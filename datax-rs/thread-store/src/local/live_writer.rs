use std::path::PathBuf;

use datax_protocol::ChatId;
use datax_protocol::protocol::ThreadMemoryMode;
use datax_rollout::RolloutConfig;
use datax_rollout::RolloutRecorder;
use datax_rollout::RolloutRecorderParams;
use datax_rollout::persisted_rollout_items;
use tracing::warn;

use super::LocalChatStore;
use super::create_chat;
use crate::AppendChatMessagesParams;
use crate::CreateChatParams;
use crate::ReadChatParams;
use crate::ResumeChatParams;
use crate::ChatStoreError;
use crate::ChatStoreResult;

pub(super) async fn create_chat(
    store: &LocalChatStore,
    params: CreateChatParams,
) -> ChatStoreResult<()> {
    let chat_id = params.chat_id;
    store.ensure_live_recorder_absent(chat_id).await?;
    let recorder = create_chat::create_chat(store, params).await?;
    store.insert_live_recorder(chat_id, recorder).await
}

pub(super) async fn resume_chat(
    store: &LocalChatStore,
    params: ResumeChatParams,
) -> ChatStoreResult<()> {
    store.ensure_live_recorder_absent(params.chat_id).await?;
    let rollout_path = match (params.rollout_path, params.history) {
        (Some(rollout_path), _history) => rollout_path,
        (None, history) => {
            let thread = super::read_chat::read_chat(
                store,
                ReadChatParams {
                    chat_id: params.chat_id,
                    include_archived: params.include_archived,
                    include_history: history.is_none(),
                },
            )
            .await?;

            thread
                .rollout_path
                .ok_or_else(|| ChatStoreError::Internal {
                    message: format!("thread {} does not have a rollout path", params.chat_id),
                })?
        }
    };
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
    let recorder = RolloutRecorder::new(&config, RolloutRecorderParams::resume(rollout_path))
        .await
        .map_err(|err| ChatStoreError::Internal {
            message: format!("failed to resume local thread recorder: {err}"),
        })?;
    store.insert_live_recorder(params.chat_id, recorder).await
}

#[tracing::instrument(
    level = "trace",
    skip_all,
    fields(item_count = params.items.len())
)]
pub(super) async fn append_items(
    store: &LocalChatStore,
    params: AppendChatMessagesParams,
) -> ChatStoreResult<()> {
    let canonical_items = persisted_rollout_items(params.items.as_slice());
    if canonical_items.is_empty() {
        return Ok(());
    }
    let recorder = store.live_recorder(params.chat_id).await?;
    recorder
        .record_canonical_items(canonical_items.as_slice())
        .await
        .map_err(chat_store_io_error)?;
    // LiveChat applies metadata immediately after append_items returns. Wait for the local
    // writer so SQLite never gets ahead of JSONL for accepted live appends.
    recorder.flush().await.map_err(chat_store_io_error)
}

pub(super) async fn persist_chat(
    store: &LocalChatStore,
    chat_id: ChatId,
) -> ChatStoreResult<()> {
    store
        .live_recorder(chat_id)
        .await?
        .persist()
        .await
        .map_err(chat_store_io_error)?;
    sync_materialized_rollout_path(store, chat_id).await
}

pub(super) async fn flush_chat(
    store: &LocalChatStore,
    chat_id: ChatId,
) -> ChatStoreResult<()> {
    store
        .live_recorder(chat_id)
        .await?
        .flush()
        .await
        .map_err(chat_store_io_error)?;
    sync_materialized_rollout_path(store, chat_id).await
}

pub(super) async fn shutdown_chat(
    store: &LocalChatStore,
    chat_id: ChatId,
) -> ChatStoreResult<()> {
    let recorder = store.live_recorder(chat_id).await?;
    recorder.shutdown().await.map_err(chat_store_io_error)?;
    sync_materialized_rollout_path(store, chat_id).await?;
    store.live_recorders.lock().await.remove(&chat_id);
    Ok(())
}

pub(super) async fn discard_chat(
    store: &LocalChatStore,
    chat_id: ChatId,
) -> ChatStoreResult<()> {
    store
        .live_recorders
        .lock()
        .await
        .remove(&chat_id)
        .map(|_| ())
        .ok_or(ChatStoreError::ChatNotFound { chat_id })
}

pub(super) async fn rollout_path(
    store: &LocalChatStore,
    chat_id: ChatId,
) -> ChatStoreResult<PathBuf> {
    Ok(store
        .live_recorders
        .lock()
        .await
        .get(&chat_id)
        .ok_or(ChatStoreError::ChatNotFound { chat_id })?
        .rollout_path()
        .to_path_buf())
}

async fn sync_materialized_rollout_path(
    store: &LocalChatStore,
    chat_id: ChatId,
) -> ChatStoreResult<()> {
    let rollout_path = rollout_path(store, chat_id).await?;
    if datax_rollout::existing_rollout_path(rollout_path.as_path())
        .await
        .is_none()
    {
        return Ok(());
    }
    let Some(state_db) = store.state_db().await else {
        return Ok(());
    };
    let result: ChatStoreResult<()> = async {
        let Some(mut metadata) =
            state_db
                .get_thread(chat_id)
                .await
                .map_err(|err| ChatStoreError::Internal {
                    message: format!("failed to read chat metadata for {chat_id}: {err}"),
                })?
        else {
            return Ok(());
        };
        if metadata.rollout_path != rollout_path {
            metadata.rollout_path = rollout_path;
            state_db
                .upsert_thread(&metadata)
                .await
                .map_err(|err| ChatStoreError::Internal {
                    message: format!("failed to update chat metadata for {chat_id}: {err}"),
                })?;
        }
        Ok(())
    }
    .await;
    if let Err(err) = result {
        warn!("failed to sync materialized rollout path for thread {chat_id}: {err}");
    }
    Ok(())
}

fn chat_store_io_error(err: std::io::Error) -> ChatStoreError {
    ChatStoreError::Internal {
        message: err.to_string(),
    }
}
