use anyhow::Result;
use app_test_support::TestAppServer;
use app_test_support::create_fake_rollout;
use app_test_support::rollout_path;
use app_test_support::to_response;
use datax_app_server::in_process;
use datax_app_server::in_process::InProcessStartArgs;
use datax_app_server_protocol::ClientInfo;
use datax_app_server_protocol::ClientRequest;
use datax_app_server_protocol::ConversationSummary;
use datax_app_server_protocol::GetConversationSummaryParams;
use datax_app_server_protocol::GetConversationSummaryResponse;
use datax_app_server_protocol::InitializeCapabilities;
use datax_app_server_protocol::InitializeParams;
use datax_app_server_protocol::JSONRPCResponse;
use datax_app_server_protocol::RequestId;
use datax_arg0::Arg0DispatchPaths;
use datax_config::CloudConfigBundleLoader;
use datax_config::LoaderOverrides;
use datax_core::config::ConfigBuilder;
use datax_exec_server::EnvironmentManager;
use datax_feedback::CodexFeedback;
use datax_protocol::ChatId;
use datax_protocol::models::BaseInstructions;
use datax_protocol::protocol::SessionSource;
use datax_protocol::protocol::ThreadMemoryMode;
use datax_thread_store::CreateChatParams;
use datax_thread_store::InMemoryChatStore;
use datax_thread_store::ChatPersistenceMetadata;
use datax_thread_store::ChatStore;
use datax_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::timeout;
use uuid::Uuid;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const FILENAME_TS: &str = "2025-01-02T12-00-00";
const META_RFC3339: &str = "2025-01-02T12:00:00Z";
const CREATED_AT_RFC3339: &str = "2025-01-02T12:00:00.000Z";
const UPDATED_AT_RFC3339: &str = "2025-01-02T12:00:00.000Z";
const PREVIEW: &str = "Summarize this conversation";
const MODEL_PROVIDER: &str = "openai";

fn expected_summary(conversation_id: ChatId, path: PathBuf) -> ConversationSummary {
    ConversationSummary {
        conversation_id,
        path,
        preview: PREVIEW.to_string(),
        timestamp: Some(CREATED_AT_RFC3339.to_string()),
        updated_at: Some(UPDATED_AT_RFC3339.to_string()),
        model_provider: MODEL_PROVIDER.to_string(),
        cwd: PathBuf::from("/"),
        cli_version: "0.0.0".to_string(),
        source: SessionSource::Cli,
        git_info: None,
    }
}

fn normalized_canonical_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    Ok(AbsolutePathBuf::from_absolute_path(path.as_ref().canonicalize()?)?.into_path_buf())
}

fn normalized_summary_path(mut summary: ConversationSummary) -> Result<ConversationSummary> {
    if !summary.path.as_os_str().is_empty() {
        summary.path = normalized_canonical_path(summary.path)?;
    }
    Ok(summary)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_conversation_summary_by_chat_id_reads_rollout() -> Result<()> {
    let codex_home = TempDir::new()?;
    let conversation_id = create_fake_rollout(
        codex_home.path(),
        FILENAME_TS,
        META_RFC3339,
        PREVIEW,
        Some(MODEL_PROVIDER),
        /*git_info*/ None,
    )?;
    let chat_id = ChatId::from_string(&conversation_id)?;
    let expected = expected_summary(
        chat_id,
        normalized_canonical_path(rollout_path(
            codex_home.path(),
            FILENAME_TS,
            &conversation_id,
        ))?,
    );

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_get_conversation_summary_request(GetConversationSummaryParams::ChatId {
            conversation_id: chat_id,
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let received: GetConversationSummaryResponse = to_response(response)?;

    assert_eq!(normalized_summary_path(received.summary)?, expected);
    Ok(())
}

#[tokio::test]
async fn get_conversation_summary_by_chat_id_reads_pathless_store_thread() -> Result<()> {
    let codex_home = TempDir::new()?;
    let store_id = Uuid::new_v4().to_string();
    create_config_toml_with_in_memory_chat_store(codex_home.path(), &store_id)?;
    let store = InMemoryChatStore::for_id(store_id.clone());
    let _in_memory_store = InMemoryChatStoreId { store_id };
    let chat_id = ChatId::from_string("00000000-0000-4000-8000-000000000125")?;
    store
        .create_chat(CreateChatParams {
            session_id: chat_id.into(),
            chat_id: chat_id,
            extra_config: None,
            forked_from_id: None,
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
        .request(ClientRequest::GetConversationSummary {
            request_id: RequestId::Integer(1),
            params: GetConversationSummaryParams::ChatId {
                conversation_id: chat_id,
            },
        })
        .await?
        .expect("getConversationSummary should succeed");
    let GetConversationSummaryResponse { summary } = serde_json::from_value(result)?;

    assert_eq!(summary.conversation_id, chat_id);
    assert_eq!(summary.path, PathBuf::new());
    assert_eq!(summary.cwd, PathBuf::new());
    assert_eq!(summary.model_provider, "test");

    client.shutdown().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_conversation_summary_by_relative_rollout_path_resolves_from_codex_home() -> Result<()>
{
    let codex_home = TempDir::new()?;
    let conversation_id = create_fake_rollout(
        codex_home.path(),
        FILENAME_TS,
        META_RFC3339,
        PREVIEW,
        Some(MODEL_PROVIDER),
        /*git_info*/ None,
    )?;
    let chat_id = ChatId::from_string(&conversation_id)?;
    let rollout_path = rollout_path(codex_home.path(), FILENAME_TS, &conversation_id);
    let relative_path = rollout_path.strip_prefix(codex_home.path())?.to_path_buf();
    let expected = expected_summary(chat_id, normalized_canonical_path(rollout_path)?);

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_get_conversation_summary_request(GetConversationSummaryParams::RolloutPath {
            rollout_path: relative_path,
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let received: GetConversationSummaryResponse = to_response(response)?;

    assert_eq!(normalized_summary_path(received.summary)?, expected);
    Ok(())
}

struct InMemoryChatStoreId {
    store_id: String,
}

impl Drop for InMemoryChatStoreId {
    fn drop(&mut self) {
        InMemoryChatStore::remove_id(&self.store_id);
    }
}

fn create_config_toml_with_in_memory_chat_store(
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
