use anyhow::Result;
use app_test_support::TestAppServer;
use app_test_support::create_mock_responses_server_repeating_assistant;
use app_test_support::to_response;
use datax_app_server::in_process;
use datax_app_server::in_process::InProcessStartArgs;
use datax_app_server_protocol::ChatArchiveParams;
use datax_app_server_protocol::ChatArchiveResponse;
use datax_app_server_protocol::ChatStartParams;
use datax_app_server_protocol::ChatStartResponse;
use datax_app_server_protocol::ChatStatus;
use datax_app_server_protocol::ChatUnarchiveParams;
use datax_app_server_protocol::ChatUnarchiveResponse;
use datax_app_server_protocol::ChatUnarchivedNotification;
use datax_app_server_protocol::ClientInfo;
use datax_app_server_protocol::ClientRequest;
use datax_app_server_protocol::ClientRequest::*;
use datax_app_server_protocol::InitializeCapabilities;
use datax_app_server_protocol::InitializeParams;
use datax_app_server_protocol::InteractionStartParams;
use datax_app_server_protocol::InteractionStartResponse;
use datax_app_server_protocol::JSONRPCResponse;
use datax_app_server_protocol::RequestId;
use datax_app_server_protocol::UserInput;
use datax_arg0::Arg0DispatchPaths;
use datax_config::CloudConfigBundleLoader;
use datax_config::LoaderOverrides;
use datax_core::config::ConfigBuilder;
use datax_core::find_archived_thread_path_by_id_str;
use datax_core::find_thread_path_by_id_str;
use datax_exec_server::EnvironmentManager;
use datax_feedback::CodexFeedback;
use datax_protocol::ChatId;
use datax_protocol::models::BaseInstructions;
use datax_protocol::protocol::SessionSource;
use datax_protocol::protocol::ThreadMemoryMode;
use datax_thread_store::CreateChatParams;
use datax_thread_store::InMemoryChatStore;
use datax_thread_store::ChatMetadataPatch;
use datax_thread_store::ChatPersistenceMetadata;
use datax_thread_store::ChatStore;
use datax_thread_store::UpdateChatMetadataParams;
use pretty_assertions::assert_eq;
use serde_json::Value;
use std::fs::FileTimes;
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use tempfile::TempDir;
use tokio::time::timeout;
use uuid::Uuid;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

#[tokio::test]
async fn thread_unarchive_moves_rollout_back_into_sessions_directory() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

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
    let ChatStartResponse { chat: thread, .. } = to_response::<ChatStartResponse>(start_resp)?;

    let rollout_path = thread.path.clone().expect("thread path");

    let turn_start_id = mcp
        .send_interaction_start_request(InteractionStartParams {
            chat_id: thread.id.clone(),
            client_user_message_id: None,
            input: vec![UserInput::Text {
                text: "materialize".to_string(),
                text_elements: Vec::new(),
            }],
            ..Default::default()
        })
        .await?;
    let turn_start_response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn_start_id)),
    )
    .await??;
    let _: InteractionStartResponse = to_response::<InteractionStartResponse>(turn_start_response)?;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("interaction/completed"),
    )
    .await??;

    let found_rollout_path =
        find_thread_path_by_id_str(codex_home.path(), &thread.id, /*state_db_ctx*/ None)
            .await?
            .expect("expected rollout path for thread id to exist");
    assert_paths_match_on_disk(&found_rollout_path, &rollout_path)?;

    let archive_id = mcp
        .send_chat_archive_request(ChatArchiveParams {
            chat_id: thread.id.clone(),
        })
        .await?;
    let archive_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(archive_id)),
    )
    .await??;
    let _: ChatArchiveResponse = to_response::<ChatArchiveResponse>(archive_resp)?;

    let archived_path = find_archived_thread_path_by_id_str(
        codex_home.path(),
        &thread.id,
        /*state_db_ctx*/ None,
    )
    .await?
    .expect("expected archived rollout path for thread id to exist");
    let archived_path_display = archived_path.display();
    assert!(
        archived_path.exists(),
        "expected {archived_path_display} to exist"
    );
    let old_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
    let old_timestamp = old_time
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("old timestamp")
        .as_secs() as i64;
    let times = FileTimes::new().set_modified(old_time);
    OpenOptions::new()
        .append(true)
        .open(&archived_path)?
        .set_times(times)?;

    let unarchive_id = mcp
        .send_chat_unarchive_request(ChatUnarchiveParams {
            chat_id: thread.id.clone(),
        })
        .await?;
    let unarchive_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(unarchive_id)),
    )
    .await??;
    let unarchive_result = unarchive_resp.result.clone();
    let ChatUnarchiveResponse {
        chat: unarchived_thread,
    } = to_response::<ChatUnarchiveResponse>(unarchive_resp)?;
    let unarchive_notification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("chat/unarchived"),
    )
    .await??;
    let unarchived_notification: ChatUnarchivedNotification = serde_json::from_value(
        unarchive_notification
            .params
            .expect("chat/unarchived notification params"),
    )?;
    assert_eq!(unarchived_notification.chat_id, thread.id);
    assert!(
        unarchived_thread.updated_at > old_timestamp,
        "expected updated_at to be bumped on unarchive"
    );
    assert_eq!(unarchived_thread.status, ChatStatus::NotLoaded);

    // Wire contract: thread title field is `name`, serialized as null when unset.
    let thread_json = unarchive_result
        .get("thread")
        .and_then(Value::as_object)
        .expect("chat/unarchive result.chat must be an object");
    assert_eq!(unarchived_thread.name, None);
    assert_eq!(
        thread_json.get("name"),
        Some(&Value::Null),
        "chat/unarchive must serialize `name: null` when unset"
    );

    let rollout_path_display = rollout_path.display();
    assert!(
        rollout_path.exists(),
        "expected rollout path {rollout_path_display} to be restored"
    );
    assert!(
        !archived_path.exists(),
        "expected archived rollout path {archived_path_display} to be moved"
    );

    Ok(())
}

#[tokio::test]
async fn thread_unarchive_preserves_pathless_store_metadata() -> Result<()> {
    let codex_home = TempDir::new()?;
    let store_id = Uuid::new_v4().to_string();
    create_config_toml_with_in_memory_thread_store(codex_home.path(), &store_id)?;
    let store = InMemoryChatStore::for_id(store_id.clone());
    let _in_memory_store = InMemoryChatStoreId { store_id };
    let chat_id = ChatId::from_string("00000000-0000-4000-8000-000000000126")?;
    let parent_chat_id = ChatId::from_string("00000000-0000-4000-8000-000000000127")?;
    store
        .create_chat(CreateChatParams {
            session_id: chat_id.into(),
            chat_id: chat_id,
            extra_config: None,
            forked_from_id: Some(parent_chat_id),
            parent_chat_id: None,
            source: SessionSource::Cli,
            thread_source: None,
            base_instructions: BaseInstructions::default(),
            dynamic_tools: Vec::new(),
            multi_agent_version: None,
            metadata: ChatPersistenceMetadata {
                cwd: None,
                model_provider: "test-provider".to_string(),
                memory_mode: ThreadMemoryMode::Disabled,
            },
        })
        .await?;
    store
        .update_chat_metadata(UpdateChatMetadataParams {
            chat_id: chat_id,
            patch: ChatMetadataPatch {
                name: Some(Some("named pathless thread".to_string())),
                ..Default::default()
            },
            include_archived: true,
        })
        .await?;

    let loader_overrides = LoaderOverrides::without_managed_config_for_tests();
    let config = ConfigBuilder::default()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .loader_overrides(loader_overrides.clone())
        .build()
        .await?;
    let client = in_process::start(InProcessStartArgs {
        arg0_paths: Arg0DispatchPaths::default(),
        config: Arc::new(config),
        cli_overrides: Vec::new(),
        loader_overrides,
        strict_config: false,
        cloud_config_bundle: CloudConfigBundleLoader::default(),
        thread_config_loader: Arc::new(datax_config::NoopThreadConfigLoader),
        feedback: CodexFeedback::new(),
        log_db: None,
        state_db: None,
        environment_manager: Arc::new(EnvironmentManager::default_for_tests()),
        config_warnings: Vec::new(),
        session_source: SessionSource::Cli,
        enable_codex_api_key_env: false,
        initialize: InitializeParams {
            client_info: ClientInfo {
                name: "datax-app-server-tests".to_string(),
                title: None,
                version: "0.1.0".to_string(),
            },
            capabilities: Some(InitializeCapabilities {
                experimental_api: true,
                ..Default::default()
            }),
        },
        channel_capacity: in_process::DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
    })
    .await?;

    let result = client
        .request(ChatUnarchive {
            request_id: RequestId::Integer(1),
            params: ChatUnarchiveParams {
                chat_id: chat_id.to_string(),
            },
        })
        .await?
        .expect("chat/unarchive should succeed");
    let ChatUnarchiveResponse { chat: thread } = serde_json::from_value(result)?;

    assert_eq!(thread.id, chat_id.to_string());
    assert_eq!(thread.path, None);
    assert_eq!(thread.forked_from_id, Some(parent_chat_id.to_string()));
    assert_eq!(thread.name, Some("named pathless thread".to_string()));

    client.shutdown().await?;
    Ok(())
}

fn create_config_toml(codex_home: &Path, server_uri: &str) -> std::io::Result<()> {
    let config_toml = codex_home.join("config.toml");
    std::fs::write(config_toml, config_contents(server_uri))
}

struct InMemoryChatStoreId {
    store_id: String,
}

impl Drop for InMemoryChatStoreId {
    fn drop(&mut self) {
        InMemoryChatStore::remove_id(&self.store_id);
    }
}

fn create_config_toml_with_in_memory_thread_store(
    codex_home: &Path,
    store_id: &str,
) -> std::io::Result<()> {
    std::fs::write(
        codex_home.join("config.toml"),
        format!(
            r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "read-only"
experimental_chat_store = {{ type = "in_memory", id = "{store_id}" }}

model_provider = "mock_provider"

[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "http://127.0.0.1:1/v1"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
"#
        ),
    )
}

fn config_contents(server_uri: &str) -> String {
    format!(
        r#"model = "mock-model"
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
    )
}

fn assert_paths_match_on_disk(actual: &Path, expected: &Path) -> std::io::Result<()> {
    let actual = actual.canonicalize()?;
    let expected = expected.canonicalize()?;
    assert_eq!(actual, expected);
    Ok(())
}
