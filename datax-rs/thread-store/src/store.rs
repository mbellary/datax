use datax_protocol::ChatId;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;

use crate::AppendChatMessagesParams;
use crate::ArchiveChatParams;
use crate::CreateChatParams;
use crate::DeleteChatParams;
use crate::MessagePage;
use crate::ListMessagesParams;
use crate::ListChatsParams;
use crate::ListInteractionsParams;
use crate::LoadChatHistoryParams;
use crate::ReadChatByRolloutPathParams;
use crate::ReadChatParams;
use crate::ResumeChatParams;
use crate::SearchChatsParams;
use crate::StoredChat;
use crate::StoredChatHistory;
use crate::ChatPage;
use crate::ChatSearchPage;
use crate::ChatStoreError;
use crate::ChatStoreResult;
use crate::InteractionPage;
use crate::UpdateChatMetadataParams;

/// Future returned by [`ChatStore`] operations.
pub type ChatStoreFuture<'a, T> = Pin<Box<dyn Future<Output = ChatStoreResult<T>> + Send + 'a>>;

/// Storage-neutral chat persistence boundary.
pub trait ChatStore: Any + Send + Sync {
    /// Return this store as [`Any`] for implementation-owned escape hatches.
    fn as_any(&self) -> &dyn Any;

    /// Creates a new live thread.
    fn create_chat(&self, params: CreateChatParams) -> ChatStoreFuture<'_, ()>;

    /// Reopens an existing thread for live appends.
    fn resume_chat(&self, params: ResumeChatParams) -> ChatStoreFuture<'_, ()>;

    /// Appends raw rollout items to a live thread.
    ///
    /// Implementations should apply the shared rollout persistence policy before writing durable
    /// replay history and before updating any implementation-owned projections.
    fn append_items(&self, params: AppendChatMessagesParams) -> ChatStoreFuture<'_, ()>;

    /// Materializes the thread if persistence is lazy, then persists all queued items.
    fn persist_chat(&self, chat_id: ChatId) -> ChatStoreFuture<'_, ()>;

    /// Flushes all queued items and returns once they are durable/readable.
    fn flush_chat(&self, chat_id: ChatId) -> ChatStoreFuture<'_, ()>;

    /// Flushes pending items and closes the live thread writer.
    fn shutdown_chat(&self, chat_id: ChatId) -> ChatStoreFuture<'_, ()>;

    /// Discards the live thread writer without forcing pending in-memory items to become durable.
    ///
    /// Core calls this when session initialization fails after a live writer has been created.
    /// Implementations should release any live writer resources for the thread while preserving
    /// already-durable thread data.
    fn discard_chat(&self, chat_id: ChatId) -> ChatStoreFuture<'_, ()>;

    /// Loads persisted history for resume, fork, rollback, and memory jobs.
    fn load_history(
        &self,
        params: LoadChatHistoryParams,
    ) -> ChatStoreFuture<'_, StoredChatHistory>;

    /// Reads a thread summary and optionally its persisted history.
    fn read_chat(&self, params: ReadChatParams) -> ChatStoreFuture<'_, StoredChat>;

    /// Reads a rollout-backed thread by path when the store supports path-addressed lookups.
    ///
    /// Deprecated: new callers should use [`ChatStore::read_chat`] instead.
    fn read_chat_by_rollout_path(
        &self,
        params: ReadChatByRolloutPathParams,
    ) -> ChatStoreFuture<'_, StoredChat>;

    /// Lists stored threads matching the supplied filters.
    fn list_chats(&self, params: ListChatsParams) -> ChatStoreFuture<'_, ChatPage>;

    /// Searches stored threads and returns search-only preview metadata.
    fn search_chats(
        &self,
        _params: SearchChatsParams,
    ) -> ChatStoreFuture<'_, ChatSearchPage> {
        Box::pin(async {
            Err(ChatStoreError::Unsupported {
                operation: "thread/search",
            })
        })
    }

    /// Lists turns within a stored thread.
    fn list_interactions(&self, _params: ListInteractionsParams) -> ChatStoreFuture<'_, InteractionPage> {
        Box::pin(async {
            Err(ChatStoreError::Unsupported {
                operation: "list_interactions",
            })
        })
    }

    /// Lists persisted items within a stored thread, optionally filtered to a turn.
    fn list_messages(&self, _params: ListMessagesParams) -> ChatStoreFuture<'_, MessagePage> {
        Box::pin(async {
            Err(ChatStoreError::Unsupported {
                operation: "list_messages",
            })
        })
    }

    /// Applies a literal metadata patch and returns the updated thread.
    ///
    /// Implementations should apply the supplied fields directly. Policy such as deciding whether
    /// an append-derived preview should be emitted belongs above the store.
    fn update_chat_metadata(
        &self,
        params: UpdateChatMetadataParams,
    ) -> ChatStoreFuture<'_, StoredChat>;

    /// Archives a thread.
    fn archive_chat(&self, params: ArchiveChatParams) -> ChatStoreFuture<'_, ()>;

    /// Unarchives a thread and returns its updated metadata.
    fn unarchive_chat(&self, params: ArchiveChatParams) -> ChatStoreFuture<'_, StoredChat>;

    /// Deletes a thread's persisted rollout data and associated metadata.
    fn delete_chat(&self, params: DeleteChatParams) -> ChatStoreFuture<'_, ()>;
}
