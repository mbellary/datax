use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::turn_context::InteractionContext;
use datax_protocol::ChatId;
use std::sync::Arc;

/// Resolves a single tool-facing agent target to a thread id.
pub(crate) async fn resolve_agent_target(
    session: &Arc<Session>,
    turn: &Arc<InteractionContext>,
    target: &str,
) -> Result<ChatId, FunctionCallError> {
    register_session_root(session, turn);
    if let Ok(chat_id) = ChatId::from_string(target) {
        return Ok(chat_id);
    }

    session
        .services
        .agent_control
        .resolve_agent_reference(session.chat_id, &turn.session_source, target)
        .await
        .map_err(|err| match err {
            datax_protocol::error::CodexErr::UnsupportedOperation(message) => {
                FunctionCallError::RespondToModel(message)
            }
            other => FunctionCallError::RespondToModel(other.to_string()),
        })
}

fn register_session_root(session: &Arc<Session>, turn: &Arc<InteractionContext>) {
    session
        .services
        .agent_control
        .register_session_root(session.chat_id, turn.parent_chat_id);
}
