mod archive_chat;
mod create_chat;
mod delete_chat;
mod helpers;
mod list_chats;
mod live_writer;
mod read_chat;
mod search_chats;
mod unarchive_chat;
mod update_chat_metadata;

#[cfg(test)]
mod test_support;

use datax_protocol::ChatId;
use datax_rollout::RolloutRecorder;
use datax_rollout::StateDbHandle;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::AppendChatMessagesParams;
use crate::ArchiveChatParams;
use crate::CreateChatParams;
use crate::DeleteChatParams;
use crate::ListChatsParams;
use crate::LoadChatHistoryParams;
use crate::ReadChatByRolloutPathParams;
use crate::ReadChatParams;
use crate::ResumeChatParams;
use crate::SearchChatsParams;
use crate::StoredChat;
use crate::StoredChatHistory;
use crate::ChatPage;
use crate::ChatSearchPage;
use crate::ChatStore;
use crate::ChatStoreError;
use crate::ChatStoreFuture;
use crate::ChatStoreResult;
use crate::UpdateChatMetadataParams;

/// Local filesystem/SQLite-backed implementation of [`ChatStore`].
///
/// Local storage has two compatibility surfaces. Rollout JSONL files are the
/// durable replay format and remain readable without SQLite, including older
/// files that encode metadata in `SessionMeta` items and name-index entries.
/// The SQLite state DB, when available, is the queryable metadata index used by
/// list/read paths for fast lookup.
///
/// Live appends still write canonical JSONL history, but append-derived
/// metadata is observed above the store and applied through
/// [`ChatStore::update_chat_metadata`]. This implementation applies that
/// patch literally to SQLite while keeping the JSONL/name-index compatibility
/// behavior needed for SQLite-less reads, repair, and old local rollout files.
#[derive(Clone)]
pub struct LocalChatStore {
    pub(super) config: LocalChatStoreConfig,
    live_recorders: Arc<Mutex<HashMap<ChatId, RolloutRecorder>>>,
    state_db: Option<StateDbHandle>,
}

/// Process-scoped configuration for local thread storage.
///
/// This describes where local storage lives. New-thread rollout metadata such
/// as cwd, provider, and memory mode is supplied when live persistence is opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalChatStoreConfig {
    pub codex_home: PathBuf,
    pub sqlite_home: PathBuf,
    /// Provider used only when older local metadata does not contain one.
    pub default_model_provider_id: String,
}

impl LocalChatStoreConfig {
    pub fn from_config(config: &impl datax_rollout::RolloutConfigView) -> Self {
        Self {
            codex_home: config.codex_home().to_path_buf(),
            sqlite_home: config.sqlite_home().to_path_buf(),
            default_model_provider_id: config.model_provider_id().to_string(),
        }
    }
}

impl std::fmt::Debug for LocalChatStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalChatStore")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl LocalChatStore {
    /// Create a local store using an already initialized state DB handle.
    pub fn new(config: LocalChatStoreConfig, state_db: Option<StateDbHandle>) -> Self {
        Self {
            config,
            live_recorders: Arc::new(Mutex::new(HashMap::new())),
            state_db,
        }
    }

    /// Return the state DB handle used by local rollout writers.
    pub async fn state_db(&self) -> Option<StateDbHandle> {
        self.state_db.clone()
    }

    /// Read a local rollout-backed thread by path.
    pub async fn read_chat_by_rollout_path(
        &self,
        rollout_path: PathBuf,
        include_archived: bool,
        include_history: bool,
    ) -> ChatStoreResult<StoredChat> {
        read_chat::read_chat_by_rollout_path(
            self,
            rollout_path,
            include_archived,
            include_history,
        )
        .await
    }

    /// Return the live local rollout path for legacy local-only code paths.
    pub async fn live_rollout_path(&self, chat_id: ChatId) -> ChatStoreResult<PathBuf> {
        live_writer::rollout_path(self, chat_id).await
    }

    pub(super) async fn live_recorder(
        &self,
        chat_id: ChatId,
    ) -> ChatStoreResult<RolloutRecorder> {
        self.live_recorders
            .lock()
            .await
            .get(&chat_id)
            .cloned()
            .ok_or(ChatStoreError::ChatNotFound { chat_id })
    }

    pub(super) async fn ensure_live_recorder_absent(
        &self,
        chat_id: ChatId,
    ) -> ChatStoreResult<()> {
        if self.live_recorders.lock().await.contains_key(&chat_id) {
            return Err(ChatStoreError::InvalidRequest {
                message: format!("thread {chat_id} already has a live local writer"),
            });
        }
        Ok(())
    }

    pub(super) async fn insert_live_recorder(
        &self,
        chat_id: ChatId,
        recorder: RolloutRecorder,
    ) -> ChatStoreResult<()> {
        match self.live_recorders.lock().await.entry(chat_id) {
            Entry::Occupied(entry) => Err(ChatStoreError::InvalidRequest {
                message: format!("thread {} already has a live local writer", entry.key()),
            }),
            Entry::Vacant(entry) => {
                entry.insert(recorder);
                Ok(())
            }
        }
    }

    async fn load_history(
        &self,
        params: LoadChatHistoryParams,
    ) -> ChatStoreResult<StoredChatHistory> {
        if let Ok(rollout_path) = live_writer::rollout_path(self, params.chat_id).await {
            if !params.include_archived
                && helpers::rollout_path_is_archived(
                    self.config.codex_home.as_path(),
                    rollout_path.as_path(),
                )
            {
                return Err(ChatStoreError::InvalidRequest {
                    message: format!("thread {} is archived", params.chat_id),
                });
            }
            return read_chat::read_chat_by_rollout_path(
                self,
                rollout_path,
                /*include_archived*/ true,
                /*include_history*/ true,
            )
            .await?
            .history
            .ok_or_else(|| ChatStoreError::Internal {
                message: format!("failed to load history for thread {}", params.chat_id),
            });
        }

        read_chat::read_chat(
            self,
            ReadChatParams {
                chat_id: params.chat_id,
                include_archived: params.include_archived,
                include_history: true,
            },
        )
        .await?
        .history
        .ok_or_else(|| ChatStoreError::Internal {
            message: format!("failed to load history for thread {}", params.chat_id),
        })
    }

    async fn read_chat_by_rollout_path_params(
        &self,
        params: ReadChatByRolloutPathParams,
    ) -> ChatStoreResult<StoredChat> {
        read_chat::read_chat_by_rollout_path(
            self,
            params.rollout_path,
            params.include_archived,
            params.include_history,
        )
        .await
    }
}

impl ChatStore for LocalChatStore {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn create_chat(&self, params: CreateChatParams) -> ChatStoreFuture<'_, ()> {
        Box::pin(async move { live_writer::create_chat(self, params).await })
    }

    fn resume_chat(&self, params: ResumeChatParams) -> ChatStoreFuture<'_, ()> {
        Box::pin(async move { live_writer::resume_chat(self, params).await })
    }

    fn append_items(&self, params: AppendChatMessagesParams) -> ChatStoreFuture<'_, ()> {
        Box::pin(async move { live_writer::append_items(self, params).await })
    }

    fn persist_chat(&self, chat_id: ChatId) -> ChatStoreFuture<'_, ()> {
        Box::pin(async move { live_writer::persist_chat(self, chat_id).await })
    }

    fn flush_chat(&self, chat_id: ChatId) -> ChatStoreFuture<'_, ()> {
        Box::pin(async move { live_writer::flush_chat(self, chat_id).await })
    }

    fn shutdown_chat(&self, chat_id: ChatId) -> ChatStoreFuture<'_, ()> {
        Box::pin(async move { live_writer::shutdown_chat(self, chat_id).await })
    }

    fn discard_chat(&self, chat_id: ChatId) -> ChatStoreFuture<'_, ()> {
        Box::pin(async move { live_writer::discard_chat(self, chat_id).await })
    }

    fn load_history(
        &self,
        params: LoadChatHistoryParams,
    ) -> ChatStoreFuture<'_, StoredChatHistory> {
        Box::pin(LocalChatStore::load_history(self, params))
    }

    fn read_chat(&self, params: ReadChatParams) -> ChatStoreFuture<'_, StoredChat> {
        Box::pin(async move { read_chat::read_chat(self, params).await })
    }

    fn read_chat_by_rollout_path(
        &self,
        params: ReadChatByRolloutPathParams,
    ) -> ChatStoreFuture<'_, StoredChat> {
        Box::pin(LocalChatStore::read_chat_by_rollout_path_params(
            self, params,
        ))
    }

    fn list_chats(&self, params: ListChatsParams) -> ChatStoreFuture<'_, ChatPage> {
        Box::pin(async move { list_chats::list_chats(self, params).await })
    }

    fn search_chats(
        &self,
        params: SearchChatsParams,
    ) -> ChatStoreFuture<'_, ChatSearchPage> {
        Box::pin(async move { search_chats::search_chats(self, params).await })
    }

    fn update_chat_metadata(
        &self,
        params: UpdateChatMetadataParams,
    ) -> ChatStoreFuture<'_, StoredChat> {
        Box::pin(async move { update_chat_metadata::update_chat_metadata(self, params).await })
    }

    fn archive_chat(&self, params: ArchiveChatParams) -> ChatStoreFuture<'_, ()> {
        Box::pin(async move { archive_chat::archive_chat(self, params).await })
    }

    fn unarchive_chat(&self, params: ArchiveChatParams) -> ChatStoreFuture<'_, StoredChat> {
        Box::pin(async move { unarchive_chat::unarchive_chat(self, params).await })
    }

    fn delete_chat(&self, params: DeleteChatParams) -> ChatStoreFuture<'_, ()> {
        Box::pin(async move { delete_chat::delete_chat(self, params).await })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use datax_protocol::ChatId;
    use datax_protocol::models::BaseInstructions;
    use datax_protocol::models::FunctionCallOutputPayload;
    use datax_protocol::models::MessagePhase;
    use datax_protocol::models::ResponseItem;
    use datax_protocol::protocol::AgentMessageEvent;
    use datax_protocol::protocol::EventMsg;
    use datax_protocol::protocol::RolloutMessage;
    use datax_protocol::protocol::SessionSource;
    use datax_protocol::protocol::ThreadMemoryMode;
    use datax_protocol::protocol::InteractionCompleteEvent;
    use datax_protocol::protocol::InteractionStartedEvent;
    use datax_protocol::protocol::UserMessageEvent;
    use tempfile::TempDir;

    use super::*;
    use crate::LiveChat;
    use crate::ChatPersistenceMetadata;
    use crate::local::test_support::test_config;
    use crate::local::test_support::write_archived_session_file;
    use crate::local::test_support::write_session_file;

    #[tokio::test]
    async fn live_writer_lifecycle_writes_and_closes() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let chat_id = ChatId::default();

        store
            .create_chat(create_chat_params(chat_id))
            .await
            .expect("create live thread");
        let rollout_path = store
            .live_rollout_path(chat_id)
            .await
            .expect("load rollout path");

        store
            .append_items(AppendChatMessagesParams {
                chat_id,
                items: vec![user_message_item("first live write")],
            })
            .await
            .expect("append live item");
        store
            .persist_chat(chat_id)
            .await
            .expect("persist live thread");
        store
            .flush_chat(chat_id)
            .await
            .expect("flush live thread");

        assert_rollout_contains_message(rollout_path.as_path(), "first live write").await;

        store
            .shutdown_chat(chat_id)
            .await
            .expect("shutdown live thread");
        let err = store
            .append_items(AppendChatMessagesParams {
                chat_id,
                items: vec![user_message_item("write after shutdown")],
            })
            .await
            .expect_err("shutdown should remove the live thread writer");
        assert!(
            matches!(err, ChatStoreError::ChatNotFound { chat_id: missing } if missing == chat_id)
        );
    }

    #[tokio::test]
    async fn raw_append_items_does_not_update_sqlite_metadata() {
        // This pins the ChatStore contract: raw appends are history-only. Callers that need
        // metadata updates must use LiveChat or call update_chat_metadata explicitly.
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = LocalChatStore::new(config, Some(runtime.clone()));
        let chat_id = ChatId::default();

        store
            .create_chat(create_chat_params(chat_id))
            .await
            .expect("create live thread");
        store
            .append_items(AppendChatMessagesParams {
                chat_id,
                items: vec![user_message_item("raw append")],
            })
            .await
            .expect("append raw item");
        store.flush_chat(chat_id).await.expect("flush thread");

        assert_eq!(
            runtime
                .get_thread(chat_id)
                .await
                .expect("sqlite metadata read"),
            None
        );
    }

    #[tokio::test]
    async fn live_chat_observes_appended_items_into_sqlite_metadata() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = Arc::new(LocalChatStore::new(config, Some(runtime.clone())));
        let chat_id = ChatId::default();
        let live_chat = LiveChat::create(store.clone(), create_chat_params(chat_id))
            .await
            .expect("create live thread");

        live_chat
            .append_items(&[user_message_item("observed append")])
            .await
            .expect("append observed item");
        live_chat.flush().await.expect("flush thread");

        let metadata = runtime
            .get_thread(chat_id)
            .await
            .expect("sqlite metadata read")
            .expect("sqlite metadata");
        assert_eq!(
            metadata.first_user_message.as_deref(),
            Some("observed append")
        );
        assert_eq!(metadata.preview.as_deref(), Some("observed append"));
        assert_eq!(metadata.title, "observed append");
    }

    #[tokio::test]
    async fn live_chat_output_advances_updated_at_but_not_recency_at() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = Arc::new(LocalChatStore::new(config, Some(runtime.clone())));
        let chat_id = ChatId::default();
        let live_chat = LiveChat::create(store, create_chat_params(chat_id))
            .await
            .expect("create live thread");

        live_chat
            .append_items(&[user_message_item("start thread")])
            .await
            .expect("append initial user message");
        live_chat.flush().await.expect("flush thread");
        let before_turn_start = runtime
            .get_thread(chat_id)
            .await
            .expect("sqlite metadata read")
            .expect("sqlite metadata");

        live_chat
            .append_items(&[RolloutMessage::EventMsg(EventMsg::InteractionStarted(
                InteractionStartedEvent {
                    interaction_id: "turn-1".to_string(),
                    trace_id: None,
                    started_at: None,
                    model_context_window: None,
                    collaboration_mode_kind: Default::default(),
                },
            ))])
            .await
            .expect("append turn start");
        live_chat.flush().await.expect("flush thread");
        let after_turn_start = runtime
            .get_thread(chat_id)
            .await
            .expect("sqlite metadata read")
            .expect("sqlite metadata");
        assert!(after_turn_start.recency_at > before_turn_start.recency_at);

        live_chat
            .append_items(&[
                RolloutMessage::EventMsg(EventMsg::AgentMessage(AgentMessageEvent {
                    message: "commentary".to_string(),
                    phase: Some(MessagePhase::Commentary),
                    memory_citation: None,
                })),
                RolloutMessage::ResponseItem(ResponseItem::FunctionCallOutput {
                    id: None,
                    call_id: "call-1".to_string(),
                    output: FunctionCallOutputPayload::from_text("tool output".to_string()),
                    internal_chat_message_metadata_passthrough: None,
                }),
                RolloutMessage::EventMsg(EventMsg::TokenCount(
                    datax_protocol::protocol::TokenCountEvent {
                        info: None,
                        rate_limits: None,
                    },
                )),
                RolloutMessage::EventMsg(EventMsg::InteractionComplete(InteractionCompleteEvent {
                    interaction_id: "turn-1".to_string(),
                    last_agent_message: None,
                    completed_at: None,
                    duration_ms: None,
                    time_to_first_token_ms: None,
                })),
            ])
            .await
            .expect("append post-start items");
        live_chat.flush().await.expect("flush thread");
        let completed = runtime
            .get_thread(chat_id)
            .await
            .expect("sqlite metadata read")
            .expect("sqlite metadata");

        assert!(completed.updated_at > after_turn_start.updated_at);
        assert_eq!(completed.recency_at, after_turn_start.recency_at);
    }

    #[tokio::test]
    async fn live_chat_shutdown_does_not_materialize_empty_chat_metadata() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = Arc::new(LocalChatStore::new(config, Some(runtime.clone())));
        let chat_id = ChatId::default();
        let live_chat = LiveChat::create(store.clone(), create_chat_params(chat_id))
            .await
            .expect("create live thread");
        let rollout_path = store
            .live_rollout_path(chat_id)
            .await
            .expect("live rollout path");

        live_chat.shutdown().await.expect("shutdown thread");

        assert!(
            !tokio::fs::try_exists(rollout_path.as_path())
                .await
                .expect("rollout path should be checkable")
        );
        assert_eq!(
            runtime
                .get_thread(chat_id)
                .await
                .expect("sqlite metadata read"),
            None
        );
    }

    #[tokio::test]
    async fn live_chat_shutdown_with_buffered_items_materializes_before_metadata_read() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = Arc::new(LocalChatStore::new(config, Some(runtime.clone())));
        let chat_id = ChatId::default();
        let live_chat = LiveChat::create(store.clone(), create_chat_params(chat_id))
            .await
            .expect("create live thread");
        let rollout_path = store
            .live_rollout_path(chat_id)
            .await
            .expect("live rollout path");

        live_chat
            .append_items(&[RolloutMessage::EventMsg(EventMsg::TokenCount(
                datax_protocol::protocol::TokenCountEvent {
                    info: None,
                    rate_limits: None,
                },
            ))])
            .await
            .expect("append metadata-only item");
        live_chat.shutdown().await.expect("shutdown thread");

        assert!(
            tokio::fs::try_exists(rollout_path.as_path())
                .await
                .expect("rollout path should be checkable")
        );
        let metadata = runtime
            .get_thread(chat_id)
            .await
            .expect("sqlite metadata read")
            .expect("sqlite metadata");
        assert_eq!(metadata.rollout_path, rollout_path);
    }

    #[tokio::test]
    async fn live_chat_resume_loads_history_before_observing_metadata() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = Arc::new(LocalChatStore::new(config, Some(runtime.clone())));
        let uuid = uuid::Uuid::from_u128(401);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path =
            write_session_file(home.path(), "2025-01-03T17-00-00", uuid).expect("session file");
        let live_chat = LiveChat::resume(
            store,
            ResumeChatParams {
                chat_id,
                rollout_path: Some(rollout_path),
                history: None,
                include_archived: false,
                metadata: ChatPersistenceMetadata {
                    cwd: Some(home.path().to_path_buf()),
                    model_provider: "different-provider".to_string(),
                    memory_mode: ThreadMemoryMode::Enabled,
                },
            },
        )
        .await
        .expect("resume live thread");

        live_chat
            .append_items(&[user_message_item("new live append")])
            .await
            .expect("append after resume");

        let metadata = runtime
            .get_thread(chat_id)
            .await
            .expect("sqlite metadata read")
            .expect("sqlite metadata");
        assert_eq!(
            metadata.created_at.to_rfc3339(),
            "2025-01-03T17:00:00+00:00"
        );
        assert_eq!(metadata.model_provider, "test-provider");
        assert_eq!(
            metadata.first_user_message.as_deref(),
            Some("Hello from user")
        );
    }

    #[tokio::test]
    async fn live_chat_resume_loads_history_from_explicit_external_rollout_path() {
        let home = TempDir::new().expect("temp dir");
        let external_home = TempDir::new().expect("external temp dir");
        let config = test_config(home.path());
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = Arc::new(LocalChatStore::new(config, Some(runtime.clone())));
        let uuid = uuid::Uuid::from_u128(402);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path = write_session_file(external_home.path(), "2025-01-03T17-30-00", uuid)
            .expect("external session file");
        let live_chat = LiveChat::resume(
            store,
            ResumeChatParams {
                chat_id,
                rollout_path: Some(rollout_path),
                history: None,
                include_archived: false,
                metadata: ChatPersistenceMetadata {
                    cwd: Some(home.path().to_path_buf()),
                    model_provider: "different-provider".to_string(),
                    memory_mode: ThreadMemoryMode::Enabled,
                },
            },
        )
        .await
        .expect("resume external live thread");

        live_chat
            .append_items(&[user_message_item("new external append")])
            .await
            .expect("append after external resume");

        let metadata = runtime
            .get_thread(chat_id)
            .await
            .expect("sqlite metadata read")
            .expect("sqlite metadata");
        assert_eq!(
            metadata.created_at.to_rfc3339(),
            "2025-01-03T17:30:00+00:00"
        );
        assert_eq!(metadata.model_provider, "test-provider");
        assert_eq!(
            metadata.first_user_message.as_deref(),
            Some("Hello from user")
        );
    }

    #[tokio::test]
    async fn create_chat_rejects_missing_cwd() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let chat_id = ChatId::default();
        let mut params = create_chat_params(chat_id);
        params.metadata.cwd = None;

        let err = store
            .create_chat(params)
            .await
            .expect_err("local chat store should require cwd");

        assert!(matches!(
            err,
            ChatStoreError::InvalidRequest { message }
                if message == "local chat store requires a cwd"
        ));
    }

    #[tokio::test]
    async fn discard_chat_drops_unmaterialized_live_writer() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let chat_id = ChatId::default();

        store
            .create_chat(create_chat_params(chat_id))
            .await
            .expect("create live thread");
        let rollout_path = store
            .live_rollout_path(chat_id)
            .await
            .expect("load rollout path");
        store
            .discard_chat(chat_id)
            .await
            .expect("discard live thread");

        assert!(
            !tokio::fs::try_exists(rollout_path.as_path())
                .await
                .expect("check rollout path")
        );
        let err = store
            .append_items(AppendChatMessagesParams {
                chat_id,
                items: vec![user_message_item("write after discard")],
            })
            .await
            .expect_err("discard should remove the live thread writer");
        assert!(
            matches!(err, ChatStoreError::ChatNotFound { chat_id: missing } if missing == chat_id)
        );
    }

    #[tokio::test]
    async fn resume_chat_reopens_live_writer_and_appends() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let chat_id = ChatId::default();

        let first_store = LocalChatStore::new(config.clone(), /*state_db*/ None);
        first_store
            .create_chat(create_chat_params(chat_id))
            .await
            .expect("create initial thread");
        first_store
            .append_items(AppendChatMessagesParams {
                chat_id,
                items: vec![user_message_item("before resume")],
            })
            .await
            .expect("append initial item");
        first_store
            .persist_chat(chat_id)
            .await
            .expect("persist initial thread");
        first_store
            .flush_chat(chat_id)
            .await
            .expect("flush initial thread");
        let rollout_path = first_store
            .live_rollout_path(chat_id)
            .await
            .expect("load rollout path");
        first_store
            .shutdown_chat(chat_id)
            .await
            .expect("shutdown initial writer");

        let resumed_store = LocalChatStore::new(config, /*state_db*/ None);
        resumed_store
            .resume_chat(ResumeChatParams {
                chat_id,
                rollout_path: None,
                history: None,
                include_archived: true,
                metadata: chat_metadata(),
            })
            .await
            .expect("resume live thread");
        resumed_store
            .append_items(AppendChatMessagesParams {
                chat_id,
                items: vec![user_message_item("after resume")],
            })
            .await
            .expect("append resumed item");
        resumed_store
            .flush_chat(chat_id)
            .await
            .expect("flush resumed thread");

        assert_rollout_contains_message(rollout_path.as_path(), "before resume").await;
        assert_rollout_contains_message(rollout_path.as_path(), "after resume").await;
    }

    #[tokio::test]
    async fn create_chat_rejects_duplicate_live_writer() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let chat_id = ChatId::default();

        store
            .create_chat(create_chat_params(chat_id))
            .await
            .expect("create live thread");

        let err = store
            .create_chat(create_chat_params(chat_id))
            .await
            .expect_err("duplicate live writer should fail");

        assert!(matches!(err, ChatStoreError::InvalidRequest { .. }));
        assert!(err.to_string().contains("already has a live local writer"));
    }

    #[tokio::test]
    async fn resume_chat_rejects_duplicate_live_writer() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let chat_id = ChatId::default();

        store
            .create_chat(create_chat_params(chat_id))
            .await
            .expect("create live thread");
        let rollout_path = store
            .live_rollout_path(chat_id)
            .await
            .expect("live rollout path");
        let err = store
            .resume_chat(ResumeChatParams {
                chat_id,
                rollout_path: Some(rollout_path),
                history: None,
                include_archived: true,
                metadata: chat_metadata(),
            })
            .await
            .expect_err("duplicate live resume should fail");
        assert!(matches!(err, ChatStoreError::InvalidRequest { .. }));
        assert!(err.to_string().contains("already has a live local writer"));
    }

    #[tokio::test]
    async fn resume_chat_rejects_missing_cwd() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let uuid = uuid::Uuid::from_u128(407);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path =
            write_session_file(home.path(), "2025-01-04T11-30-00", uuid).expect("session file");
        let err = store
            .resume_chat(ResumeChatParams {
                chat_id,
                rollout_path: Some(rollout_path),
                history: None,
                include_archived: true,
                metadata: ChatPersistenceMetadata {
                    cwd: None,
                    model_provider: "test-provider".to_string(),
                    memory_mode: ThreadMemoryMode::Enabled,
                },
            })
            .await
            .expect_err("missing cwd should fail");

        assert!(matches!(err, ChatStoreError::InvalidRequest { .. }));
        assert!(err.to_string().contains("requires a cwd"));
    }

    #[tokio::test]
    async fn load_history_uses_live_writer_rollout_path() {
        let home = TempDir::new().expect("temp dir");
        let external_home = TempDir::new().expect("external temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let uuid = uuid::Uuid::from_u128(404);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path = write_session_file(external_home.path(), "2025-01-04T10-00-00", uuid)
            .expect("external session file");

        store
            .resume_chat(ResumeChatParams {
                chat_id,
                rollout_path: Some(rollout_path),
                history: None,
                include_archived: true,
                metadata: chat_metadata(),
            })
            .await
            .expect("resume live thread");
        store
            .append_items(AppendChatMessagesParams {
                chat_id,
                items: vec![user_message_item("external history item")],
            })
            .await
            .expect("append live item");
        store
            .flush_chat(chat_id)
            .await
            .expect("flush live thread");

        let history = store
            .load_history(LoadChatHistoryParams {
                chat_id,
                include_archived: false,
            })
            .await
            .expect("load external live history");

        assert!(history.items.iter().any(|item| {
            matches!(
                item,
                RolloutMessage::EventMsg(EventMsg::UserMessage(event)) if event.message == "external history item"
            )
        }));
    }

    #[tokio::test]
    async fn read_chat_uses_live_writer_rollout_path_for_external_resume() {
        let home = TempDir::new().expect("temp dir");
        let external_home = TempDir::new().expect("external temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let uuid = uuid::Uuid::from_u128(406);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path = write_session_file(external_home.path(), "2025-01-04T11-00-00", uuid)
            .expect("external session file");

        store
            .resume_chat(ResumeChatParams {
                chat_id,
                rollout_path: Some(rollout_path.clone()),
                history: None,
                include_archived: true,
                metadata: chat_metadata(),
            })
            .await
            .expect("resume live thread");

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: false,
                include_history: true,
            })
            .await
            .expect("read external live thread");

        assert_eq!(thread.rollout_path, Some(rollout_path));
        assert!(thread.history.expect("history").items.iter().any(|item| {
            matches!(
                item,
                RolloutMessage::EventMsg(EventMsg::UserMessage(event)) if event.message == "Hello from user"
            )
        }));
    }

    #[tokio::test]
    async fn load_history_uses_live_writer_rollout_path_for_archived_source() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let uuid = uuid::Uuid::from_u128(405);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path = write_archived_session_file(home.path(), "2025-01-04T10-30-00", uuid)
            .expect("archived session file");

        store
            .resume_chat(ResumeChatParams {
                chat_id,
                rollout_path: Some(rollout_path),
                history: None,
                include_archived: true,
                metadata: chat_metadata(),
            })
            .await
            .expect("resume live archived thread");
        store
            .append_items(AppendChatMessagesParams {
                chat_id,
                items: vec![user_message_item("archived live history item")],
            })
            .await
            .expect("append live item");
        store
            .flush_chat(chat_id)
            .await
            .expect("flush live thread");

        let err = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect_err("active-only read should reject archived live thread");
        assert!(matches!(err, ChatStoreError::InvalidRequest { .. }));

        let err = store
            .load_history(LoadChatHistoryParams {
                chat_id,
                include_archived: false,
            })
            .await
            .expect_err("active-only history should reject archived live thread");
        assert!(matches!(err, ChatStoreError::InvalidRequest { .. }));
        assert!(err.to_string().contains("archived"));

        let history = store
            .load_history(LoadChatHistoryParams {
                chat_id,
                include_archived: true,
            })
            .await
            .expect("load archived live history");

        assert!(history.items.iter().any(|item| {
            matches!(
                item,
                RolloutMessage::EventMsg(EventMsg::UserMessage(event)) if event.message == "archived live history item"
            )
        }));
    }

    #[tokio::test]
    async fn read_chat_by_rollout_path_includes_history() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let chat_id = ChatId::default();

        store
            .create_chat(create_chat_params(chat_id))
            .await
            .expect("create thread");
        store
            .append_items(AppendChatMessagesParams {
                chat_id,
                items: vec![user_message_item("path read")],
            })
            .await
            .expect("append item");
        store.flush_chat(chat_id).await.expect("flush thread");
        let rollout_path = store
            .live_rollout_path(chat_id)
            .await
            .expect("load rollout path");

        let thread = store
            .read_chat_by_rollout_path(
                rollout_path,
                /*include_archived*/ true,
                /*include_history*/ true,
            )
            .await
            .expect("read thread by rollout path");

        assert_eq!(thread.chat_id, chat_id);
        assert_eq!(
            thread
                .history
                .expect("history")
                .items
                .into_iter()
                .filter(|item| matches!(item, RolloutMessage::EventMsg(EventMsg::UserMessage(_))))
                .count(),
            1
        );
    }

    fn create_chat_params(chat_id: ChatId) -> CreateChatParams {
        CreateChatParams {
            session_id: chat_id.into(),
            chat_id,
            extra_config: None,
            forked_from_id: None,
            parent_chat_id: None,
            source: SessionSource::Exec,
            thread_source: None,
            base_instructions: BaseInstructions::default(),
            dynamic_tools: Vec::new(),
            multi_agent_version: None,
            metadata: chat_metadata(),
        }
    }

    fn chat_metadata() -> ChatPersistenceMetadata {
        ChatPersistenceMetadata {
            cwd: Some(std::env::current_dir().expect("cwd")),
            model_provider: "test-provider".to_string(),
            memory_mode: ThreadMemoryMode::Enabled,
        }
    }

    fn user_message_item(message: &str) -> RolloutMessage {
        RolloutMessage::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            client_id: None,
            message: message.to_string(),
            images: None,
            local_images: Vec::new(),
            text_elements: Vec::new(),
            ..Default::default()
        }))
    }

    async fn assert_rollout_contains_message(path: &std::path::Path, expected: &str) {
        let (items, _, _) = RolloutRecorder::load_rollout_items(path)
            .await
            .expect("load rollout items");
        assert!(items.iter().any(|item| {
            matches!(
                item,
                RolloutMessage::EventMsg(EventMsg::UserMessage(event)) if event.message == expected
            )
        }));
    }
}
