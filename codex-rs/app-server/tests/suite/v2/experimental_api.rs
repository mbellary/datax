use anyhow::Result;
use app_test_support::DEFAULT_CLIENT_NAME;
use app_test_support::TestAppServer;
use app_test_support::create_mock_responses_server_sequence_unchecked;
use app_test_support::to_response;
use datax_app_server_protocol::AskForApproval;
use datax_app_server_protocol::ChatMemoryMode;
use datax_app_server_protocol::ChatMemoryModeSetParams;
use datax_app_server_protocol::ChatRealtimeStartParams;
use datax_app_server_protocol::ChatRealtimeStartTransport;
use datax_app_server_protocol::ChatSettingsUpdateParams;
use datax_app_server_protocol::ChatStartParams;
use datax_app_server_protocol::ChatStartResponse;
use datax_app_server_protocol::ClientInfo;
use datax_app_server_protocol::InitializeCapabilities;
use datax_app_server_protocol::JSONRPCError;
use datax_app_server_protocol::JSONRPCMessage;
use datax_app_server_protocol::JSONRPCResponse;
use datax_app_server_protocol::MockExperimentalMethodParams;
use datax_app_server_protocol::RequestId;
use datax_protocol::protocol::RealtimeOutputModality;
use pretty_assertions::assert_eq;
use std::path::Path;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::test]
async fn mock_experimental_method_requires_experimental_api_capability() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    let init = mcp
        .initialize_with_capabilities(
            default_client_info(),
            Some(InitializeCapabilities {
                experimental_api: false,
                request_attestation: false,
                opt_out_notification_methods: None,
                mcp_server_openai_form_elicitation: false,
            }),
        )
        .await?;
    let JSONRPCMessage::Response(_) = init else {
        anyhow::bail!("expected initialize response, got {init:?}");
    };

    let request_id = mcp
        .send_mock_experimental_method_request(MockExperimentalMethodParams::default())
        .await?;
    let error = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;
    assert_experimental_capability_error(error, "mock/experimentalMethod");
    Ok(())
}

#[tokio::test]
async fn realtime_conversation_start_requires_experimental_api_capability() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    let init = mcp
        .initialize_with_capabilities(
            default_client_info(),
            Some(InitializeCapabilities {
                experimental_api: false,
                request_attestation: false,
                opt_out_notification_methods: None,
                mcp_server_openai_form_elicitation: false,
            }),
        )
        .await?;
    let JSONRPCMessage::Response(_) = init else {
        anyhow::bail!("expected initialize response, got {init:?}");
    };

    let request_id = mcp
        .send_chat_realtime_start_request(ChatRealtimeStartParams {
            client_managed_handoffs: None,
            codex_responses_as_messages: None,
            codex_response_message_prefix: None,
            codex_response_handoff_prefix: None,
            chat_id: "thr_123".to_string(),
            model: None,
            output_modality: RealtimeOutputModality::Audio,
            include_startup_context: None,
            prompt: Some(Some("hello".to_string())),
            realtime_session_id: None,
            transport: None,
            version: None,
            voice: None,
        })
        .await?;
    let error = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;
    assert_experimental_capability_error(error, "chat/realtime/start");
    Ok(())
}

#[tokio::test]
async fn thread_memory_mode_set_requires_experimental_api_capability() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    let init = mcp
        .initialize_with_capabilities(
            default_client_info(),
            Some(InitializeCapabilities {
                experimental_api: false,
                request_attestation: false,
                opt_out_notification_methods: None,
                mcp_server_openai_form_elicitation: false,
            }),
        )
        .await?;
    let JSONRPCMessage::Response(_) = init else {
        anyhow::bail!("expected initialize response, got {init:?}");
    };

    let request_id = mcp
        .send_chat_memory_mode_set_request(ChatMemoryModeSetParams {
            chat_id: "thr_123".to_string(),
            mode: ChatMemoryMode::Disabled,
        })
        .await?;
    let error = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;
    assert_experimental_capability_error(error, "chat/memoryMode/set");
    Ok(())
}

#[tokio::test]
async fn thread_settings_update_requires_experimental_api_capability() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    let init = mcp
        .initialize_with_capabilities(
            default_client_info(),
            Some(InitializeCapabilities {
                experimental_api: false,
                request_attestation: false,
                opt_out_notification_methods: None,
                mcp_server_openai_form_elicitation: false,
            }),
        )
        .await?;
    let JSONRPCMessage::Response(_) = init else {
        anyhow::bail!("expected initialize response, got {init:?}");
    };

    let request_id = mcp
        .send_chat_settings_update_request(ChatSettingsUpdateParams {
            chat_id: "thr_123".to_string(),
            ..Default::default()
        })
        .await?;
    let error = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;
    assert_experimental_capability_error(error, "chat/settings/update");
    Ok(())
}

#[tokio::test]
async fn realtime_webrtc_start_requires_experimental_api_capability() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut mcp = TestAppServer::new(codex_home.path()).await?;

    let init = mcp
        .initialize_with_capabilities(
            default_client_info(),
            Some(InitializeCapabilities {
                experimental_api: false,
                request_attestation: false,
                opt_out_notification_methods: None,
                mcp_server_openai_form_elicitation: false,
            }),
        )
        .await?;
    let JSONRPCMessage::Response(_) = init else {
        anyhow::bail!("expected initialize response, got {init:?}");
    };

    let request_id = mcp
        .send_chat_realtime_start_request(ChatRealtimeStartParams {
            client_managed_handoffs: None,
            codex_responses_as_messages: None,
            codex_response_message_prefix: None,
            codex_response_handoff_prefix: None,
            chat_id: "thr_123".to_string(),
            model: None,
            output_modality: RealtimeOutputModality::Audio,
            include_startup_context: None,
            prompt: Some(Some("hello".to_string())),
            realtime_session_id: None,
            transport: Some(ChatRealtimeStartTransport::Webrtc {
                sdp: "v=offer\r\n".to_string(),
            }),
            version: None,
            voice: None,
        })
        .await?;
    let error = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;
    assert_experimental_capability_error(error, "chat/realtime/start");
    Ok(())
}

#[tokio::test]
async fn thread_start_mock_field_requires_experimental_api_capability() -> Result<()> {
    let server = create_mock_responses_server_sequence_unchecked(Vec::new()).await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    let init = mcp
        .initialize_with_capabilities(
            default_client_info(),
            Some(InitializeCapabilities {
                experimental_api: false,
                request_attestation: false,
                opt_out_notification_methods: None,
                mcp_server_openai_form_elicitation: false,
            }),
        )
        .await?;
    let JSONRPCMessage::Response(_) = init else {
        anyhow::bail!("expected initialize response, got {init:?}");
    };

    let request_id = mcp
        .send_chat_start_request(ChatStartParams {
            mock_experimental_field: Some("mock".to_string()),
            ..Default::default()
        })
        .await?;

    let error = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;
    assert_experimental_capability_error(error, "chat/start.mockExperimentalField");
    Ok(())
}

#[tokio::test]
async fn thread_start_without_dynamic_tools_allows_without_experimental_api_capability()
-> Result<()> {
    let server = create_mock_responses_server_sequence_unchecked(Vec::new()).await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    let init = mcp
        .initialize_with_capabilities(
            default_client_info(),
            Some(InitializeCapabilities {
                experimental_api: false,
                request_attestation: false,
                opt_out_notification_methods: None,
                mcp_server_openai_form_elicitation: false,
            }),
        )
        .await?;
    let JSONRPCMessage::Response(_) = init else {
        anyhow::bail!("expected initialize response, got {init:?}");
    };

    let request_id = mcp
        .send_chat_start_request(ChatStartParams {
            model: Some("mock-model".to_string()),
            ..Default::default()
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let _: ChatStartResponse = to_response(response)?;
    Ok(())
}

#[tokio::test]
async fn thread_start_granular_approval_policy_requires_experimental_api_capability() -> Result<()>
{
    let server = create_mock_responses_server_sequence_unchecked(Vec::new()).await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    let init = mcp
        .initialize_with_capabilities(
            default_client_info(),
            Some(InitializeCapabilities {
                experimental_api: false,
                request_attestation: false,
                opt_out_notification_methods: None,
                mcp_server_openai_form_elicitation: false,
            }),
        )
        .await?;
    let JSONRPCMessage::Response(_) = init else {
        anyhow::bail!("expected initialize response, got {init:?}");
    };

    let request_id = mcp
        .send_chat_start_request(ChatStartParams {
            approval_policy: Some(AskForApproval::Granular {
                sandbox_approval: true,
                rules: false,
                skill_approval: false,
                request_permissions: true,
                mcp_elicitations: false,
            }),
            ..Default::default()
        })
        .await?;

    let error = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;
    assert_experimental_capability_error(error, "askForApproval.granular");
    Ok(())
}

fn default_client_info() -> ClientInfo {
    ClientInfo {
        name: DEFAULT_CLIENT_NAME.to_string(),
        title: None,
        version: "0.1.0".to_string(),
    }
}

fn assert_experimental_capability_error(error: JSONRPCError, reason: &str) {
    assert_eq!(error.error.code, -32600);
    assert_eq!(
        error.error.message,
        format!("{reason} requires experimentalApi capability")
    );
    assert_eq!(error.error.data, None);
}

fn create_config_toml(codex_home: &Path, server_uri: &str) -> std::io::Result<()> {
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
