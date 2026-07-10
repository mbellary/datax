//! Storage-neutral chat persistence interfaces.
//!
//! Application code should treat [`datax_protocol::ChatId`] as the only durable thread handle.
//! Implementations are responsible for resolving that id to local rollout files, RPC requests, or
//! any other backing store.

mod error;
mod in_memory;
mod live_chat;
mod local;
mod store;
mod chat_metadata_sync;
mod types;

pub use error::ChatStoreError;
pub use error::ChatStoreResult;
pub use in_memory::InMemoryChatStore;
pub use in_memory::InMemoryChatStoreCalls;
pub use live_chat::LiveChat;
pub use live_chat::LiveChatInitGuard;
pub use local::LocalChatStore;
pub use local::LocalChatStoreConfig;
pub use store::ChatStore;
pub use store::ChatStoreFuture;
pub use types::AppendChatMessagesParams;
pub use types::ArchiveChatParams;
pub use types::ClearableField;
pub use types::CreateChatParams;
pub use types::DeleteChatParams;
pub use types::ExtraConfig;
pub use types::GitInfoPatch;
pub use types::MessagePage;
pub use types::ListMessagesParams;
pub use types::ListChatsParams;
pub use types::ListInteractionsParams;
pub use types::LoadChatHistoryParams;
pub use types::ReadChatByRolloutPathParams;
pub use types::ReadChatParams;
pub use types::ResumeChatParams;
pub use types::SearchChatsParams;
pub use types::SortDirection;
pub use types::StoredChat;
pub use types::StoredChatHistory;
pub use types::StoredChatMessage;
pub use types::StoredChatSearchResult;
pub use types::StoredInteraction;
pub use types::StoredInteractionError;
pub use types::StoredInteractionMessagesView;
pub use types::StoredInteractionStatus;
pub use types::ChatMetadataPatch;
pub use types::ChatPage;
pub use types::ChatPersistenceMetadata;
pub use types::ChatSearchPage;
pub use types::ChatSortKey;
pub use types::InteractionPage;
pub use types::UpdateChatMetadataParams;
