use anyhow::Result;
use app_test_support::TestAppServer;
use app_test_support::create_final_assistant_message_sse_response;
use app_test_support::create_mock_responses_server_sequence_unchecked;
use app_test_support::to_response;
use datax_app_server_protocol::ChatResumeParams;
use datax_app_server_protocol::ChatResumeResponse;
use datax_app_server_protocol::ChatRollbackParams;
use datax_app_server_protocol::ChatRollbackResponse;
use datax_app_server_protocol::ChatStartParams;
use datax_app_server_protocol::ChatStartResponse;
use datax_app_server_protocol::ChatStatus;
use datax_app_server_protocol::InteractionStartParams;
use datax_app_server_protocol::JSONRPCResponse;
use datax_app_server_protocol::Message;
use datax_app_server_protocol::RequestId;
use datax_app_server_protocol::UserInput as V2UserInput;
use pretty_assertions::assert_eq;
use serde_json::Value;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[tokio::test]
async fn thread_rollback_drops_last_turns_and_persists_to_rollout() -> Result<()> {
    // Three Codex interactions hit the mock model (session start + two interaction/start calls).
    let responses = vec![
        create_final_assistant_message_sse_response("Done")?,
        create_final_assistant_message_sse_response("Done")?,
        create_final_assistant_message_sse_response("Done")?,
    ];
    let server = create_mock_responses_server_sequence_unchecked(responses).await;

    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    // Start a thread.
    let start_id = mcp
        .send_chat_start_request(ChatStartParams {
            model: Some("mock-model".to_string()),
            ..Default::default()
        })
        .await?;
    let start_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_id)),
    )
    .await??;
    let ChatStartResponse { thread, .. } = to_response::<ChatStartResponse>(start_resp)?;

    // Two interactions.
    let first_text = "First";
    let turn1_id = mcp
        .send_interaction_start_request(InteractionStartParams {
            chat_id: thread.id.clone(),
            client_user_message_id: None,
            input: vec![V2UserInput::Text {
                text: first_text.to_string(),
                text_elements: Vec::new(),
            }],
            ..Default::default()
        })
        .await?;
    let _turn1_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn1_id)),
    )
    .await??;
    let _completed1 = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("interaction/completed"),
    )
    .await??;

    let turn2_id = mcp
        .send_interaction_start_request(InteractionStartParams {
            chat_id: thread.id.clone(),
            client_user_message_id: None,
            input: vec![V2UserInput::Text {
                text: "Second".to_string(),
                text_elements: Vec::new(),
            }],
            ..Default::default()
        })
        .await?;
    let _turn2_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn2_id)),
    )
    .await??;
    let _completed2 = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("interaction/completed"),
    )
    .await??;

    // Roll back the last turn.
    let rollback_id = mcp
        .send_chat_rollback_request(ChatRollbackParams {
            chat_id: thread.id.clone(),
            num_turns: 1,
        })
        .await?;
    let rollback_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(rollback_id)),
    )
    .await??;
    let rollback_result = rollback_resp.result.clone();
    let ChatRollbackResponse {
        thread: rolled_back_thread,
    } = to_response::<ChatRollbackResponse>(rollback_resp)?;

    // Wire contract: thread title field is `name`, serialized as null when unset.
    let thread_json = rollback_result
        .get("thread")
        .and_then(Value::as_object)
        .expect("chat/rollback result.thread must be an object");
    assert_eq!(rolled_back_thread.name, None);
    assert_eq!(rolled_back_thread.session_id, thread.session_id);
    assert_eq!(
        thread_json.get("name"),
        Some(&Value::Null),
        "chat/rollback must serialize `name: null` when unset"
    );
    assert_eq!(
        thread_json.get("sessionId").and_then(Value::as_str),
        Some(thread.session_id.as_str())
    );

    assert_eq!(rolled_back_thread.interactions.len(), 1);
    assert_eq!(rolled_back_thread.status, ChatStatus::Idle);
    assert_eq!(rolled_back_thread.interactions[0].messages.len(), 2);
    match &rolled_back_thread.interactions[0].messages[0] {
        Message::UserMessage { content, .. } => {
            assert_eq!(
                content,
                &vec![V2UserInput::Text {
                    text: first_text.to_string(),
                    text_elements: Vec::new(),
                }]
            );
        }
        other => panic!("expected user message item, got {other:?}"),
    }

    // Resume and confirm the history is pruned.
    let resume_id = mcp
        .send_chat_resume_request(ChatResumeParams {
            chat_id: thread.id,
            ..Default::default()
        })
        .await?;
    let resume_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(resume_id)),
    )
    .await??;
    let ChatResumeResponse { thread, .. } = to_response::<ChatResumeResponse>(resume_resp)?;

    assert_eq!(thread.interactions.len(), 1);
    assert_eq!(thread.status, ChatStatus::Idle);
    assert_eq!(thread.interactions[0].messages.len(), 2);
    match &thread.interactions[0].messages[0] {
        Message::UserMessage { content, .. } => {
            assert_eq!(
                content,
                &vec![V2UserInput::Text {
                    text: first_text.to_string(),
                    text_elements: Vec::new(),
                }]
            );
        }
        other => panic!("expected user message item, got {other:?}"),
    }

    Ok(())
}

fn create_config_toml(codex_home: &std::path::Path, server_uri: &str) -> std::io::Result<()> {
    let config_toml = codex_home.join("config.toml");
    std::fs::write(
        config_toml,
        format!(
            r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "read-only"

model_provider = "mock_provider"

[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "{server_uri}/v1"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
"#
        ),
    )
}
