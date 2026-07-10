use datax_protocol::ChatId;

/// Result type returned by thread-store operations.
pub type ChatStoreResult<T> = Result<T, ChatStoreError>;

/// Error type shared by thread-store implementations.
#[derive(Debug, thiserror::Error)]
pub enum ChatStoreError {
    /// The requested thread does not exist in this store.
    #[error("thread {chat_id} not found")]
    ChatNotFound {
        /// Thread id requested by the caller.
        chat_id: ChatId,
    },

    /// The caller supplied invalid request data.
    #[error("invalid thread-store request: {message}")]
    InvalidRequest {
        /// User-facing explanation of the invalid request.
        message: String,
    },

    /// The operation conflicted with current store state.
    #[error("thread-store conflict: {message}")]
    Conflict {
        /// User-facing explanation of the conflict.
        message: String,
    },

    /// The store implementation does not support this operation yet.
    #[error("thread-store unsupported operation: {operation}")]
    Unsupported {
        /// Stable operation name for callers that need to map unsupported operations.
        operation: &'static str,
    },

    /// Catch-all for implementation failures that do not fit a more specific category.
    #[error("thread-store internal error: {message}")]
    Internal {
        /// User-facing explanation of the implementation failure.
        message: String,
    },
}
