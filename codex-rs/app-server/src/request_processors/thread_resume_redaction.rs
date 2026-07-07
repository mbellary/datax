use datax_app_server_protocol::Interaction;
use datax_app_server_protocol::McpToolCallResult;
use datax_app_server_protocol::Message;
use serde_json::Value as JsonValue;

// Temporary bandaid for remote clients: chat/resume can include large MCP and
// image-generation payloads. Keep this response-only so persisted rollout
// history, model resume history, and other APIs stay unchanged.
const REDACTED_PAYLOAD: &str = "[redacted]";
const CHATGPT_REMOTE_CLIENT_NAMES: &[&str] =
    &["codex_chatgpt_android_remote", "codex_chatgpt_ios_remote"];

pub(super) fn should_redact_thread_resume_payloads(client_name: Option<&str>) -> bool {
    client_name.is_some_and(|client_name| CHATGPT_REMOTE_CLIENT_NAMES.contains(&client_name))
}

pub(super) fn redact_thread_resume_payloads(interactions: &mut [Interaction]) {
    for turn in interactions {
        turn.messages.retain_mut(|item| match item {
            Message::McpToolCall {
                arguments,
                result,
                error,
                ..
            } => {
                *arguments = JsonValue::String(REDACTED_PAYLOAD.to_string());
                if result.is_some() {
                    *result = Some(Box::new(redacted_mcp_tool_call_result()));
                }
                if let Some(error) = error {
                    error.message = REDACTED_PAYLOAD.to_string();
                }
                true
            }
            Message::ImageGeneration { .. } => false,
            _ => true,
        });
    }
}

fn redacted_mcp_tool_call_result() -> McpToolCallResult {
    McpToolCallResult {
        content: vec![serde_json::json!({
            "type": "text",
            "text": REDACTED_PAYLOAD,
        })],
        structured_content: None,
        meta: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datax_app_server_protocol::Chat;
    use datax_app_server_protocol::ChatStatus;
    use datax_app_server_protocol::InteractionMessagesView;
    use datax_app_server_protocol::InteractionStatus;
    use datax_app_server_protocol::McpToolCallAppContext;
    use datax_app_server_protocol::McpToolCallError;
    use datax_app_server_protocol::McpToolCallStatus;
    use datax_app_server_protocol::SessionSource;
    use datax_utils_absolute_path::test_support::PathBufExt;
    use datax_utils_absolute_path::test_support::test_path_buf;
    use pretty_assertions::assert_eq;

    #[test]
    fn redacts_mcp_success_result_and_removes_image_generation() {
        let mut thread = test_thread(vec![
            Message::AgentMessage {
                id: "agent-1".to_string(),
                text: "kept".to_string(),
                phase: None,
                memory_citation: None,
            },
            Message::McpToolCall {
                id: "mcp-1".to_string(),
                server: "docs".to_string(),
                tool: "lookup".to_string(),
                status: McpToolCallStatus::Completed,
                arguments: serde_json::json!({"secret":"argument"}),
                app_context: Some(McpToolCallAppContext {
                    connector_id: "calendar".to_string(),
                    link_id: Some("link_calendar".to_string()),
                    resource_uri: Some("ui://widget/lookup.html".to_string()),
                }),
                mcp_app_resource_uri: Some("ui://widget/lookup.html".to_string()),
                plugin_id: Some("sample@test".to_string()),
                result: Some(Box::new(McpToolCallResult {
                    content: vec![serde_json::json!({
                        "type": "text",
                        "text": "secret result"
                    })],
                    structured_content: Some(serde_json::json!({"secret":"structured"})),
                    meta: Some(serde_json::json!({"secret":"meta"})),
                })),
                error: None,
                duration_ms: Some(8),
            },
            Message::ImageGeneration {
                id: "ig-1".to_string(),
                status: "completed".to_string(),
                revised_prompt: Some("revised".to_string()),
                result: "base64-result".to_string(),
                saved_path: Some(test_path_buf("/tmp/ig-1.png").abs()),
            },
        ]);

        redact_thread_resume_payloads(&mut thread.interactions);

        assert_eq!(thread.interactions[0].messages.len(), 2);
        assert_eq!(
            thread.interactions[0].messages[0],
            Message::AgentMessage {
                id: "agent-1".to_string(),
                text: "kept".to_string(),
                phase: None,
                memory_citation: None,
            }
        );
        assert_eq!(
            thread.interactions[0].messages[1],
            Message::McpToolCall {
                id: "mcp-1".to_string(),
                server: "docs".to_string(),
                tool: "lookup".to_string(),
                status: McpToolCallStatus::Completed,
                arguments: JsonValue::String(REDACTED_PAYLOAD.to_string()),
                app_context: Some(McpToolCallAppContext {
                    connector_id: "calendar".to_string(),
                    link_id: Some("link_calendar".to_string()),
                    resource_uri: Some("ui://widget/lookup.html".to_string()),
                }),
                mcp_app_resource_uri: Some("ui://widget/lookup.html".to_string()),
                plugin_id: Some("sample@test".to_string()),
                result: Some(Box::new(redacted_mcp_tool_call_result())),
                error: None,
                duration_ms: Some(8),
            }
        );
    }

    #[test]
    fn redacts_mcp_error_message() {
        let mut thread = test_thread(vec![Message::McpToolCall {
            id: "mcp-1".to_string(),
            server: "docs".to_string(),
            tool: "lookup".to_string(),
            status: McpToolCallStatus::Failed,
            arguments: serde_json::json!({"secret":"argument"}),
            app_context: None,
            mcp_app_resource_uri: None,
            plugin_id: None,
            result: None,
            error: Some(McpToolCallError {
                message: "secret error".to_string(),
            }),
            duration_ms: Some(8),
        }]);

        redact_thread_resume_payloads(&mut thread.interactions);

        assert_eq!(
            thread.interactions[0].messages[0],
            Message::McpToolCall {
                id: "mcp-1".to_string(),
                server: "docs".to_string(),
                tool: "lookup".to_string(),
                status: McpToolCallStatus::Failed,
                arguments: JsonValue::String(REDACTED_PAYLOAD.to_string()),
                app_context: None,
                mcp_app_resource_uri: None,
                plugin_id: None,
                result: None,
                error: Some(McpToolCallError {
                    message: REDACTED_PAYLOAD.to_string(),
                }),
                duration_ms: Some(8),
            }
        );
    }

    fn test_thread(messages: Vec<Message>) -> Chat {
        Chat {
            id: "thread-1".to_string(),
            session_id: "session-1".to_string(),
            forked_from_id: None,
            parent_chat_id: None,
            preview: "preview".to_string(),
            ephemeral: false,
            model_provider: "mock_provider".to_string(),
            created_at: 0,
            updated_at: 0,
            recency_at: Some(0),
            status: ChatStatus::Idle,
            path: None,
            cwd: test_path_buf("/tmp").abs(),
            cli_version: "0.0.0".to_string(),
            source: SessionSource::Cli,
            chat_source: None,
            agent_nickname: None,
            agent_role: None,
            git_info: None,
            name: None,
            interactions: vec![Interaction {
                id: "turn-1".to_string(),
                messages,
                messages_view: InteractionMessagesView::Full,
                status: InteractionStatus::Completed,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
            }],
        }
    }
}
