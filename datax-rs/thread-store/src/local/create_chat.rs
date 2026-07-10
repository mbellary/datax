use super::LocalChatStore;
use crate::CreateChatParams;
use crate::ChatStoreError;
use crate::ChatStoreResult;
use datax_protocol::protocol::ThreadMemoryMode;
use datax_rollout::RolloutConfig;
use datax_rollout::RolloutRecorder;
use datax_rollout::RolloutRecorderParams;

pub(super) async fn create_chat(
    store: &LocalChatStore,
    params: CreateChatParams,
) -> ChatStoreResult<RolloutRecorder> {
    let cwd = params
        .metadata
        .cwd
        .clone()
        .ok_or_else(|| ChatStoreError::InvalidRequest {
            message: "local chat store requires a cwd".to_string(),
        })?;
    let config = RolloutConfig {
        codex_home: store.config.codex_home.clone(),
        sqlite_home: store.config.sqlite_home.clone(),
        cwd,
        model_provider_id: params.metadata.model_provider.clone(),
        generate_memories: matches!(params.metadata.memory_mode, ThreadMemoryMode::Enabled),
    };
    RolloutRecorder::new(
        &config,
        RolloutRecorderParams::new(
            params.chat_id,
            params.forked_from_id,
            params.parent_chat_id,
            params.source,
            params.chat_source,
            params.base_instructions,
            params.dynamic_tools,
        )
        .with_session_id(params.session_id)
        .with_multi_agent_version(params.multi_agent_version),
    )
    .await
    .map_err(|err| ChatStoreError::Internal {
        message: format!("failed to initialize local thread recorder: {err}"),
    })
}
