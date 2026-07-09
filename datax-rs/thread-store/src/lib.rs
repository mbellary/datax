//! Storage-neutral thread persistence interfaces.
//!
//! Application code should treat [`datax_protocol::ChatId`] as the only durable thread handle.
//! Implementations are responsible for resolving that id to local rollout files, RPC requests, or
//! any other backing store.

mod error;
mod in_memory;
mod live_thread;
mod local;
mod store;
mod thread_metadata_sync;
mod types;

pub use error::ThreadStoreError;
pub use error::ThreadStoreResult;
pub use in_memory::InMemoryThreadStore;
pub use in_memory::InMemoryThreadStoreCalls;
pub use live_thread::LiveThread;
pub use live_thread::LiveThreadInitGuard;
pub use local::LocalThreadStore;
pub use local::LocalThreadStoreConfig;
pub use store::ThreadStore;
pub use store::ThreadStoreFuture;
pub use types::AppendChatMessagesParams;
pub use types::ArchiveThreadParams;
pub use types::ClearableField;
pub use types::CreateThreadParams;
pub use types::DeleteThreadParams;
pub use types::ExtraConfig;
pub use types::GitInfoPatch;
pub use types::MessagePage;
pub use types::ListMessagesParams;
pub use types::ListThreadsParams;
pub use types::ListInteractionsParams;
pub use types::LoadThreadHistoryParams;
pub use types::ReadThreadByRolloutPathParams;
pub use types::ReadThreadParams;
pub use types::ResumeThreadParams;
pub use types::SearchThreadsParams;
pub use types::SortDirection;
pub use types::StoredChat;
pub use types::StoredChatHistory;
pub use types::StoredChatMessage;
pub use types::StoredChatSearchResult;
pub use types::StoredInteraction;
pub use types::StoredInteractionError;
pub use types::StoredInteractionMessagesView;
pub use types::StoredInteractionStatus;
pub use types::ThreadMetadataPatch;
pub use types::ChatPage;
pub use types::ThreadPersistenceMetadata;
pub use types::ChatSearchPage;
pub use types::ThreadSortKey;
pub use types::InteractionPage;
pub use types::UpdateThreadMetadataParams;
