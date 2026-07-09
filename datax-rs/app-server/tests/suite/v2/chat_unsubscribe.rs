use anyhow::Result;
use app_test_support::TestAppServer;
use app_test_support::create_mock_responses_server_repeating_assistant;
use app_test_support::to_response;
use core_test_support::responses;
use core_test_support::streaming_sse::StreamingSseChunk;
use core_test_support::streaming_sse::start_streaming_sse_server;
use datax_app_server_protocol::ChatLoadedListParams;
use datax_app_server_protocol::ChatLoadedListResponse;
use datax_app_server_protocol::ChatReadParams;
use datax_app_server_protocol::ChatReadResponse;
use datax_app_server_protocol::ChatResumeParams;
use datax_app_server_protocol::ChatResumeResponse;
use datax_app_server_protocol::ChatStartParams;
use datax_app_server_protocol::ChatStartResponse;
use datax_app_server_protocol::ChatStatus;
use datax_app_server_protocol::ChatUnsubscribeParams;
use datax_app_server_protocol::ChatUnsubscribeResponse;
use datax_app_server_protocol::ChatUnsubscribeStatus;
use datax_app_server_protocol::DynamicToolCallOutputContentItem;
use datax_app_server_protocol::DynamicToolCallParams;
use datax_app_server_protocol::DynamicToolCallResponse;
use datax_app_server_protocol::DynamicToolFunctionSpec;
use datax_app_server_protocol::DynamicToolSpec;
use datax_app_server_protocol::InteractionStartParams;
use datax_app_server_protocol::InteractionStartResponse;
use datax_app_server_protocol::JSONRPCResponse;
use datax_app_server_protocol::Message;
use datax_app_server_protocol::MessageStartedNotification;
use datax_app_server_protocol::RequestId;
use datax_app_server_protocol::ServerRequest;
use datax_app_server_protocol::UserInput as V2UserInput;
use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
#[tokio::test]
async fn thread_unsubscribe_keeps_thread_loaded_until_idle_timeout() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let chat_id = start_chat(&mut mcp).await?;

    let unsubscribe_id = mcp
        .send_chat_unsubscribe_request(ChatUnsubscribeParams {
            chat_id: chat_id.clone(),
        })
        .await?;
    let unsubscribe_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(unsubscribe_id)),
    )
    .await??;
    let unsubscribe = to_response::<ChatUnsubscribeResponse>(unsubscribe_resp)?;
    assert_eq!(unsubscribe.status, ChatUnsubscribeStatus::Unsubscribed);

    assert!(
        timeout(
            std::time::Duration::from_millis(250),
            mcp.read_stream_until_notification_message("chat/closed"),
        )
        .await
        .is_err()
    );

    let list_id = mcp
        .send_chat_loaded_list_request(ChatLoadedListParams::default())
        .await?;
    let list_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_id)),
    )
    .await??;
    let ChatLoadedListResponse { data, next_cursor } =
        to_response::<ChatLoadedListResponse>(list_resp)?;
    assert_eq!(data, vec![chat_id]);
    assert_eq!(next_cursor, None);

    Ok(())
}

#[tokio::test]
async fn thread_unsubscribe_during_turn_keeps_turn_running() -> Result<()> {
    let call_id = "deterministic-wait-call";
    let tool_name = "deterministic_wait";
    let tool_args = json!({});
    let tool_call_arguments = serde_json::to_string(&tool_args)?;

    let tmp = TempDir::new()?;
    let codex_home = tmp.path().join("codex_home");
    std::fs::create_dir(&codex_home)?;
    let working_directory = tmp.path().join("workdir");
    std::fs::create_dir(&working_directory)?;

    let (server, mut completions) = start_streaming_sse_server(vec![
        vec![StreamingSseChunk {
            gate: None,
            body: responses::sse(vec![
                responses::ev_response_created("resp-1"),
                responses::ev_function_call(call_id, tool_name, &tool_call_arguments),
                responses::ev_completed("resp-1"),
            ]),
        }],
        vec![StreamingSseChunk {
            gate: None,
            body: responses::sse(vec![
                responses::ev_response_created("resp-2"),
                responses::ev_assistant_message("msg-1", "Done"),
                responses::ev_completed("resp-2"),
            ]),
        }],
    ])
    .await;
    let first_response_completed = completions.remove(0);
    let final_response_completed = completions.remove(0);
    create_config_toml(&codex_home, server.uri())?;

    let mut mcp = TestAppServer::new(&codex_home).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let thread_req = mcp
        .send_chat_start_request(ChatStartParams {
            model: Some("mock-model".to_string()),
            dynamic_tools: Some(vec![DynamicToolSpec::Function(DynamicToolFunctionSpec {
                name: tool_name.to_string(),
                description: "Deterministic wait tool".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false,
                }),
                defer_loading: false,
            })]),
            ..Default::default()
        })
        .await?;
    let thread_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(thread_req)),
    )
    .await??;
    let ChatStartResponse { chat: thread, .. } = to_response::<ChatStartResponse>(thread_resp)?;
    let chat_id = thread.id;

    let turn_req = mcp
        .send_interaction_start_request(InteractionStartParams {
            chat_id: chat_id.clone(),
            client_user_message_id: None,
            input: vec![V2UserInput::Text {
                text: "run deterministic tool".to_string(),
                text_elements: Vec::new(),
            }],
            cwd: Some(working_directory),
            ..Default::default()
        })
        .await?;
    let turn_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn_req)),
    )
    .await??;
    let _: InteractionStartResponse = to_response::<InteractionStartResponse>(turn_resp)?;

    timeout(
        DEFAULT_READ_TIMEOUT,
        server.wait_for_request_count(/*count*/ 1),
    )
    .await?;
    timeout(DEFAULT_READ_TIMEOUT, first_response_completed).await??;

    let started = timeout(
        DEFAULT_READ_TIMEOUT,
        wait_for_dynamic_tool_started(&mut mcp, call_id),
    )
    .await??;
    assert_eq!(started.chat_id, chat_id);

    let request = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_request_message(),
    )
    .await??;
    let (request_id, params) = match request {
        ServerRequest::DynamicToolCall { request_id, params } => (request_id, params),
        other => panic!("expected DynamicToolCall request, got {other:?}"),
    };
    assert_eq!(
        params,
        DynamicToolCallParams {
            chat_id: chat_id.clone(),
            interaction_id: started.interaction_id,
            call_id: call_id.to_string(),
            namespace: None,
            tool: tool_name.to_string(),
            arguments: tool_args,
        }
    );

    let unsubscribe_id = mcp
        .send_chat_unsubscribe_request(ChatUnsubscribeParams {
            chat_id: chat_id.clone(),
        })
        .await?;
    let unsubscribe_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(unsubscribe_id)),
    )
    .await??;
    let unsubscribe = to_response::<ChatUnsubscribeResponse>(unsubscribe_resp)?;
    assert_eq!(unsubscribe.status, ChatUnsubscribeStatus::Unsubscribed);

    let closed_while_tool_call_blocked = timeout(
        std::time::Duration::from_millis(250),
        mcp.read_stream_until_notification_message("chat/closed"),
    );
    let closed_while_tool_call_blocked = closed_while_tool_call_blocked.await;
    assert!(closed_while_tool_call_blocked.is_err());

    let response = DynamicToolCallResponse {
        content_items: vec![DynamicToolCallOutputContentItem::InputText {
            text: "dynamic-ok".to_string(),
        }],
        success: true,
    };
    mcp.send_response(request_id, serde_json::to_value(response)?)
        .await?;

    timeout(
        DEFAULT_READ_TIMEOUT,
        server.wait_for_request_count(/*count*/ 2),
    )
    .await?;
    timeout(DEFAULT_READ_TIMEOUT, final_response_completed).await??;
    server.shutdown().await;

    Ok(())
}

#[tokio::test]
async fn thread_unsubscribe_preserves_cached_status_before_idle_unload() -> Result<()> {
    let server = responses::start_mock_server().await;
    let _response_mock = responses::mount_sse_once(
        &server,
        responses::sse_failed("resp-1", "server_error", "simulated failure"),
    )
    .await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let chat_id = start_chat(&mut mcp).await?;

    let turn_req = mcp
        .send_interaction_start_request(InteractionStartParams {
            chat_id: chat_id.clone(),
            client_user_message_id: None,
            input: vec![V2UserInput::Text {
                text: "fail this turn".to_string(),
                text_elements: Vec::new(),
            }],
            ..Default::default()
        })
        .await?;
    let turn_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn_req)),
    )
    .await??;
    let _: InteractionStartResponse = to_response::<InteractionStartResponse>(turn_resp)?;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("error"),
    )
    .await??;

    let read_id = mcp
        .send_chat_read_request(ChatReadParams {
            chat_id: chat_id.clone(),
            include_interactions: false,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ChatReadResponse { chat: thread, .. } = to_response::<ChatReadResponse>(read_resp)?;
    assert_eq!(thread.status, ChatStatus::SystemError);

    let unsubscribe_id = mcp
        .send_chat_unsubscribe_request(ChatUnsubscribeParams {
            chat_id: chat_id.clone(),
        })
        .await?;
    let unsubscribe_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(unsubscribe_id)),
    )
    .await??;
    let unsubscribe = to_response::<ChatUnsubscribeResponse>(unsubscribe_resp)?;
    assert_eq!(unsubscribe.status, ChatUnsubscribeStatus::Unsubscribed);
    assert!(
        timeout(
            std::time::Duration::from_millis(250),
            mcp.read_stream_until_notification_message("chat/closed"),
        )
        .await
        .is_err()
    );

    let resume_id = mcp
        .send_chat_resume_request(ChatResumeParams {
            chat_id,
            cwd: Some(codex_home.path().to_string_lossy().to_string()),
            ..Default::default()
        })
        .await?;
    let resume_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(resume_id)),
    )
    .await??;
    let resume: ChatResumeResponse = to_response::<ChatResumeResponse>(resume_resp)?;
    assert_eq!(resume.chat.status, ChatStatus::SystemError);

    Ok(())
}

#[tokio::test]
async fn thread_unsubscribe_reports_not_subscribed_before_idle_unload() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let chat_id = start_chat(&mut mcp).await?;

    let first_unsubscribe_id = mcp
        .send_chat_unsubscribe_request(ChatUnsubscribeParams {
            chat_id: chat_id.clone(),
        })
        .await?;
    let first_unsubscribe_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(first_unsubscribe_id)),
    )
    .await??;
    let first_unsubscribe = to_response::<ChatUnsubscribeResponse>(first_unsubscribe_resp)?;
    assert_eq!(
        first_unsubscribe.status,
        ChatUnsubscribeStatus::Unsubscribed
    );

    let second_unsubscribe_id = mcp
        .send_chat_unsubscribe_request(ChatUnsubscribeParams { chat_id })
        .await?;
    let second_unsubscribe_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(second_unsubscribe_id)),
    )
    .await??;
    let second_unsubscribe = to_response::<ChatUnsubscribeResponse>(second_unsubscribe_resp)?;
    assert_eq!(
        second_unsubscribe.status,
        ChatUnsubscribeStatus::NotSubscribed
    );

    Ok(())
}

async fn wait_for_dynamic_tool_started(
    mcp: &mut TestAppServer,
    call_id: &str,
) -> Result<MessageStartedNotification> {
    loop {
        let notification = mcp
            .read_stream_until_notification_message("message/started")
            .await?;
        let Some(params) = notification.params else {
            continue;
        };
        let started: MessageStartedNotification = serde_json::from_value(params)?;
        if matches!(&started.item, Message::DynamicToolCall { id, .. } if id == call_id) {
            return Ok(started);
        }
    }
}

fn create_config_toml(codex_home: &std::path::Path, server_uri: &str) -> std::io::Result<()> {
    let config_toml = codex_home.join("config.toml");
    std::fs::write(
        config_toml,
        format!(
            r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "danger-full-access"

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

async fn start_chat(mcp: &mut TestAppServer) -> Result<String> {
    let req_id = mcp
        .send_chat_start_request(ChatStartParams {
            model: Some("mock-model".to_string()),
            ..Default::default()
        })
        .await?;
    let resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(req_id)),
    )
    .await??;
    let ChatStartResponse { chat: thread, .. } = to_response::<ChatStartResponse>(resp)?;
    Ok(thread.id)
}
