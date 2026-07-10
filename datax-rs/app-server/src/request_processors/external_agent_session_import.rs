use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use datax_arg0::Arg0DispatchPaths;
use datax_core::ChatManager;
use datax_core::config::ConfigOverrides;
use datax_external_agent_sessions::CompletedExternalAgentSessionImport;
use datax_external_agent_sessions::ExternalAgentSessionMigration;
use datax_external_agent_sessions::ImportedExternalAgentSession;
use datax_external_agent_sessions::PendingSessionImport;
use datax_external_agent_sessions::prepare_validated_session_import;
use datax_external_agent_sessions::record_completed_session_imports;
use datax_models_manager::manager::RefreshStrategy;
use datax_protocol::ChatId;
use datax_protocol::models::BaseInstructions;
use datax_protocol::protocol::MultiAgentVersion;
use datax_protocol::protocol::ThreadMemoryMode;
use datax_rollout::is_persisted_rollout_item;
use datax_thread_store::AppendChatMessagesParams;
use datax_thread_store::CreateChatParams;
use datax_thread_store::ChatMetadataPatch;
use datax_thread_store::ChatPersistenceMetadata;
use datax_thread_store::ChatStore;
use datax_thread_store::UpdateChatMetadataParams;
use futures::StreamExt;
use tokio::sync::Semaphore;

use crate::config::external_agent_config::ExternalAgentConfigImportItemResult;
use crate::config::external_agent_config::record_import_error;
use crate::config_manager::ConfigManager;

const SESSION_IMPORT_CONCURRENCY: usize = 5;

#[derive(Clone)]
pub(super) struct ExternalAgentSessionImporter {
    codex_home: PathBuf,
    permits: Arc<Semaphore>,
    chat_manager: Arc<ChatManager>,
    chat_store: Arc<dyn ChatStore>,
    config_manager: ConfigManager,
    arg0_paths: Arg0DispatchPaths,
}

impl ExternalAgentSessionImporter {
    pub(super) fn new(
        codex_home: PathBuf,
        chat_manager: Arc<ChatManager>,
        chat_store: Arc<dyn ChatStore>,
        config_manager: ConfigManager,
        arg0_paths: Arg0DispatchPaths,
    ) -> Self {
        Self {
            codex_home,
            permits: Arc::new(Semaphore::new(1)),
            chat_manager,
            chat_store,
            config_manager,
            arg0_paths,
        }
    }

    pub(super) async fn import_sessions(
        &self,
        sessions: Vec<ExternalAgentSessionMigration>,
        mut item_result: ExternalAgentConfigImportItemResult,
    ) -> ExternalAgentConfigImportItemResult {
        if sessions.is_empty() {
            return item_result;
        }
        let Ok(_permit) = self.permits.acquire().await else {
            record_import_error(
                &mut item_result,
                "session_permit",
                "external agent session import permit could not be acquired",
                /*source*/ None,
            );
            return item_result;
        };
        let import_results = futures::stream::iter(sessions)
            .map(|session| {
                let importer = self.clone();
                async move { importer.import_requested_session(session).await }
            })
            .buffer_unordered(SESSION_IMPORT_CONCURRENCY);
        futures::pin_mut!(import_results);

        let mut completed_imports = Vec::new();
        while let Some(result) = import_results.next().await {
            match result {
                Ok(Some(completed_import)) => {
                    item_result.record_success(
                        Some(completed_import.source_path.display().to_string()),
                        Some(completed_import.imported_chat_id.to_string()),
                    );
                    completed_imports.push(completed_import);
                }
                Ok(None) => {}
                Err(failure) => {
                    record_import_error(
                        &mut item_result,
                        failure.stage,
                        failure.message.clone(),
                        Some(failure.source_path.display().to_string()),
                    );
                }
            }
        }
        if let Err(err) = record_completed_session_imports(&self.codex_home, completed_imports) {
            record_import_error(
                &mut item_result,
                "session_ledger_update",
                err.to_string(),
                /*source*/ None,
            );
        }
        item_result
    }

    async fn import_requested_session(
        &self,
        session: ExternalAgentSessionMigration,
    ) -> Result<Option<CompletedExternalAgentSessionImport>, SessionImportFailure> {
        let source_path = session.path.clone();
        let Some(pending_import) =
            self.prepare_session_import(session)
                .await
                .map_err(|message| SessionImportFailure {
                    source_path: source_path.clone(),
                    message,
                    stage: "session_prepare",
                })?
        else {
            return Ok(None);
        };
        let imported_chat_id =
            self.persist_session(pending_import.session)
                .await
                .map_err(|message| SessionImportFailure {
                    source_path: pending_import.source_path.clone(),
                    message,
                    stage: "session_persist",
                })?;
        Ok(Some(CompletedExternalAgentSessionImport {
            source_path: pending_import.source_path,
            source_content_sha256: pending_import.source_content_sha256,
            imported_chat_id,
        }))
    }

    async fn prepare_session_import(
        &self,
        session: ExternalAgentSessionMigration,
    ) -> Result<Option<PendingSessionImport>, String> {
        let codex_home = self.codex_home.clone();
        tokio::task::spawn_blocking(move || prepare_validated_session_import(&codex_home, session))
            .await
            .map_err(|err| format!("external agent session preparation task failed: {err}"))?
            .map_err(|err| format!("failed to prepare external agent session: {err}"))
    }

    async fn persist_session(
        &self,
        session: ImportedExternalAgentSession,
    ) -> Result<ChatId, String> {
        let ImportedExternalAgentSession {
            cwd,
            title,
            first_user_message,
            mut rollout_items,
        } = session;
        let config = self
            .config_manager
            .load_with_overrides(
                /*request_overrides*/ None,
                ConfigOverrides {
                    cwd: Some(cwd),
                    codex_linux_sandbox_exe: self.arg0_paths.codex_linux_sandbox_exe.clone(),
                    main_execve_wrapper_exe: self.arg0_paths.main_execve_wrapper_exe.clone(),
                    ..Default::default()
                },
            )
            .await
            .map_err(|err| format!("failed to load imported session config: {err}"))?;
        let models_manager = self.chat_manager.get_models_manager();
        let model = models_manager
            .get_default_model(&config.model, RefreshStrategy::Offline)
            .await;
        let model_info = models_manager
            .get_model_info(model.as_str(), &config.to_models_manager_config())
            .await;
        let chat_id = ChatId::new();
        let source = self.chat_manager.session_source();
        let cwd = config.cwd.to_path_buf();
        let model_provider = config.model_provider_id.clone();
        let memory_mode = if config.memories.generate_memories {
            ThreadMemoryMode::Enabled
        } else {
            ThreadMemoryMode::Disabled
        };
        let now = Utc::now();
        let create_params = CreateChatParams {
            session_id: chat_id.into(),
            chat_id: chat_id,
            extra_config: None,
            forked_from_id: None,
            parent_chat_id: None,
            source: source.clone(),
            chat_source: None,
            base_instructions: BaseInstructions {
                text: config
                    .base_instructions
                    .clone()
                    .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
            },
            dynamic_tools: Vec::new(),
            multi_agent_version: Some(MultiAgentVersion::V1),
            metadata: ChatPersistenceMetadata {
                cwd: Some(cwd.clone()),
                model_provider: model_provider.clone(),
                memory_mode,
            },
        };
        rollout_items.retain(is_persisted_rollout_item);
        let title = title
            .as_deref()
            .and_then(datax_core::util::normalize_thread_name);
        let metadata = ChatMetadataPatch {
            title,
            preview: first_user_message.clone(),
            model_provider: Some(model_provider),
            created_at: Some(now),
            updated_at: Some(now),
            source: Some(source.clone()),
            agent_nickname: Some(source.get_nickname()),
            agent_role: Some(source.get_agent_role()),
            agent_path: Some(source.get_agent_path().map(Into::into)),
            cwd: Some(cwd),
            cli_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            first_user_message,
            memory_mode: Some(memory_mode),
            ..Default::default()
        };

        self.chat_store
            .create_chat(create_params)
            .await
            .map_err(|err| format!("failed to import session: {err}"))?;
        if !rollout_items.is_empty()
            && let Err(err) = self
                .chat_store
                .append_items(AppendChatMessagesParams {
                    chat_id: chat_id,
                    items: rollout_items,
                })
                .await
        {
            let _ = self.chat_store.discard_chat(chat_id).await;
            return Err(format!("failed to import session: {err}"));
        }

        self.chat_store
            .update_chat_metadata(UpdateChatMetadataParams {
                chat_id: chat_id,
                patch: metadata,
                include_archived: false,
            })
            .await
            .map_err(|err| format!("failed to update imported session: {err}"))?;
        self.chat_store
            .persist_chat(chat_id)
            .await
            .map_err(|err| format!("failed to persist imported session: {err}"))?;
        self.chat_store
            .shutdown_chat(chat_id)
            .await
            .map_err(|err| format!("failed to shutdown imported session: {err}"))?;
        Ok(chat_id)
    }
}

struct SessionImportFailure {
    source_path: PathBuf,
    message: String,
    stage: &'static str,
}
