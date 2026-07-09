use std::path::PathBuf;
use std::sync::Arc;

use datax_protocol::ChatId;
use datax_protocol::protocol::RolloutMessage;
use datax_protocol::protocol::ThreadMemoryMode;
use datax_rollout::persisted_rollout_items;
use tokio::sync::Mutex;
use tracing::warn;

use crate::AppendChatMessagesParams;
use crate::CreateChatParams;
use crate::LoadChatHistoryParams;
use crate::LocalChatStore;
use crate::ReadChatParams;
use crate::ResumeChatParams;
use crate::StoredChat;
use crate::StoredChatHistory;
use crate::ChatMetadataPatch;
use crate::ChatStore;
use crate::ChatStoreResult;
use crate::UpdateChatMetadataParams;
use crate::chat_metadata_sync::ChatMetadataSync;

/// Handle for an active thread's persistence lifecycle.
///
/// `LiveChat` keeps lifecycle decisions with the caller while delegating storage details to
/// [`ChatStore`]. Local stores may use a rollout file internally and remote stores may use a
/// service, but session code should only need this handle for the active thread.
#[derive(Clone)]
pub struct LiveChat {
    chat_id: ChatId,
    chat_store: Arc<dyn ChatStore>,
    metadata_sync: Arc<Mutex<ChatMetadataSync>>,
}

/// Owns a live thread while session initialization is still fallible.
///
/// If initialization returns early after persistence has been opened, dropping this guard discards
/// the live writer without forcing lazy in-memory state to become durable. Call [`commit`] once the
/// session owns the live thread for normal operation.
pub struct LiveChatInitGuard {
    live_chat: Option<LiveChat>,
}

impl LiveChatInitGuard {
    pub fn new(live_chat: Option<LiveChat>) -> Self {
        Self { live_chat }
    }

    pub fn as_ref(&self) -> Option<&LiveChat> {
        self.live_chat.as_ref()
    }

    pub fn commit(&mut self) {
        self.live_chat = None;
    }

    pub async fn discard(&mut self) {
        let Some(live_chat) = self.live_chat.take() else {
            return;
        };
        if let Err(err) = live_chat.discard().await {
            warn!("failed to discard chat persistence for failed session init: {err}");
        }
    }
}

impl Drop for LiveChatInitGuard {
    fn drop(&mut self) {
        let Some(live_chat) = self.live_chat.take() else {
            return;
        };
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            warn!("failed to discard chat persistence for failed session init: no Tokio runtime");
            return;
        };
        handle.spawn(async move {
            if let Err(err) = live_chat.discard().await {
                warn!("failed to discard chat persistence for failed session init: {err}");
            }
        });
    }
}

impl LiveChat {
    pub async fn create(
        chat_store: Arc<dyn ChatStore>,
        params: CreateChatParams,
    ) -> ChatStoreResult<Self> {
        let chat_id = params.chat_id;
        let metadata_sync = ChatMetadataSync::for_create(&params).await;
        chat_store.create_chat(params).await?;
        Ok(Self {
            chat_id,
            chat_store,
            metadata_sync: Arc::new(Mutex::new(metadata_sync)),
        })
    }

    pub async fn resume(
        chat_store: Arc<dyn ChatStore>,
        mut params: ResumeChatParams,
    ) -> ChatStoreResult<Self> {
        let chat_id = params.chat_id;
        let should_load_history = params.history.is_none();
        let include_archived = params.include_archived;
        chat_store.resume_chat(params.clone()).await?;
        if should_load_history {
            match chat_store
                .load_history(LoadChatHistoryParams {
                    chat_id,
                    include_archived,
                })
                .await
            {
                Ok(history) => params.history = Some(history.items),
                Err(err) => {
                    if let Err(discard_err) = chat_store.discard_chat(chat_id).await {
                        warn!(
                            "failed to discard chat persistence after resume history load failed: {discard_err}"
                        );
                    }
                    return Err(err);
                }
            }
        }
        let metadata_sync = ChatMetadataSync::for_resume(&params);
        Ok(Self {
            chat_id,
            chat_store,
            metadata_sync: Arc::new(Mutex::new(metadata_sync)),
        })
    }

    #[tracing::instrument(
        level = "trace",
        skip_all,
        fields(item_count = items.len())
    )]
    pub async fn append_items(&self, items: &[RolloutMessage]) -> ChatStoreResult<()> {
        let canonical_items = persisted_rollout_items(items);
        if items.is_empty() {
            return Ok(());
        }
        self.chat_store
            .append_items(AppendChatMessagesParams {
                chat_id: self.chat_id,
                items: items.to_vec(),
            })
            .await?;
        if canonical_items.is_empty() {
            return Ok(());
        }
        let update = self
            .metadata_sync
            .lock()
            .await
            .observe_appended_items(canonical_items.as_slice());
        if let Some(update) = update {
            self.chat_store
                .update_chat_metadata(UpdateChatMetadataParams {
                    chat_id: self.chat_id,
                    patch: update.patch.clone(),
                    include_archived: true,
                })
                .await?;
            self.metadata_sync
                .lock()
                .await
                .mark_pending_update_applied(&update);
        }
        Ok(())
    }

    pub async fn persist(&self) -> ChatStoreResult<()> {
        self.chat_store.persist_chat(self.chat_id).await?;
        self.flush_pending_metadata_update().await
    }

    pub async fn flush(&self) -> ChatStoreResult<()> {
        self.chat_store.flush_chat(self.chat_id).await?;
        self.flush_pending_metadata_update_for_existing_history()
            .await
    }

    pub async fn shutdown(&self) -> ChatStoreResult<()> {
        self.flush_pending_metadata_update_for_existing_history()
            .await?;
        self.chat_store.shutdown_chat(self.chat_id).await
    }

    pub async fn discard(&self) -> ChatStoreResult<()> {
        self.chat_store.discard_chat(self.chat_id).await
    }

    pub async fn load_history(
        &self,
        include_archived: bool,
    ) -> ChatStoreResult<StoredChatHistory> {
        self.chat_store
            .load_history(LoadChatHistoryParams {
                chat_id: self.chat_id,
                include_archived,
            })
            .await
    }

    pub async fn read_chat(
        &self,
        include_archived: bool,
        include_history: bool,
    ) -> ChatStoreResult<StoredChat> {
        self.chat_store
            .read_chat(ReadChatParams {
                chat_id: self.chat_id,
                include_archived,
                include_history,
            })
            .await
    }

    pub async fn update_memory_mode(
        &self,
        mode: ThreadMemoryMode,
        include_archived: bool,
    ) -> ChatStoreResult<()> {
        self.flush_pending_metadata_update().await?;
        self.chat_store
            .update_chat_metadata(UpdateChatMetadataParams {
                chat_id: self.chat_id,
                patch: ChatMetadataPatch {
                    memory_mode: Some(mode),
                    ..Default::default()
                },
                include_archived,
            })
            .await?;
        Ok(())
    }

    pub async fn update_metadata(
        &self,
        patch: ChatMetadataPatch,
        include_archived: bool,
    ) -> ChatStoreResult<StoredChat> {
        self.flush_pending_metadata_update().await?;
        self.chat_store
            .update_chat_metadata(UpdateChatMetadataParams {
                chat_id: self.chat_id,
                patch,
                include_archived,
            })
            .await
    }

    /// Returns the live local rollout path for legacy local-only callers.
    ///
    /// Remote stores do not expose rollout files, so they return `Ok(None)`.
    pub async fn local_rollout_path(&self) -> ChatStoreResult<Option<PathBuf>> {
        let Some(local_store) = self
            .chat_store
            .as_any()
            .downcast_ref::<LocalChatStore>()
        else {
            return Ok(None);
        };
        local_store
            .live_rollout_path(self.chat_id)
            .await
            .map(Some)
    }

    async fn flush_pending_metadata_update(&self) -> ChatStoreResult<()> {
        let update = self.metadata_sync.lock().await.take_pending_update();
        self.apply_pending_metadata_update(update).await
    }

    async fn flush_pending_metadata_update_for_existing_history(&self) -> ChatStoreResult<()> {
        let update = self
            .metadata_sync
            .lock()
            .await
            .take_pending_update_for_existing_history();
        self.apply_pending_metadata_update(update).await
    }

    async fn apply_pending_metadata_update(
        &self,
        update: Option<crate::chat_metadata_sync::PendingChatMetadataPatch>,
    ) -> ChatStoreResult<()> {
        let Some(update) = update else {
            return Ok(());
        };
        self.chat_store
            .update_chat_metadata(UpdateChatMetadataParams {
                chat_id: self.chat_id,
                patch: update.patch.clone(),
                include_archived: true,
            })
            .await?;
        self.metadata_sync
            .lock()
            .await
            .mark_pending_update_applied(&update);
        Ok(())
    }
}
