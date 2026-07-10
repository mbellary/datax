use anyhow::Result;
use app_test_support::TestAppServer;
use app_test_support::create_fake_rollout_with_text_elements;
use app_test_support::create_mock_responses_server_repeating_assistant;
use app_test_support::rollout_path;
use app_test_support::test_absolute_path;
use app_test_support::to_response;
use core_test_support::responses;
use datax_app_server::in_process;
use datax_app_server::in_process::InProcessStartArgs;
use datax_app_server_protocol::ChatForkParams;
use datax_app_server_protocol::ChatForkResponse;
use datax_app_server_protocol::ChatInteractionsListParams;
use datax_app_server_protocol::ChatInteractionsListResponse;
use datax_app_server_protocol::ChatInteractionsMessagesListParams;
use datax_app_server_protocol::ChatListParams;
use datax_app_server_protocol::ChatListResponse;
use datax_app_server_protocol::ChatNameUpdatedNotification;
use datax_app_server_protocol::ChatReadParams;
use datax_app_server_protocol::ChatReadResponse;
use datax_app_server_protocol::ChatResumeInitialInteractionsPageParams;
use datax_app_server_protocol::ChatResumeParams;
use datax_app_server_protocol::ChatResumeResponse;
use datax_app_server_protocol::ChatSetNameParams;
use datax_app_server_protocol::ChatSetNameResponse;
use datax_app_server_protocol::ChatStartParams;
use datax_app_server_protocol::ChatStartResponse;
use datax_app_server_protocol::ChatStatus;
use datax_app_server_protocol::ClientInfo;
use datax_app_server_protocol::ClientRequest;
use datax_app_server_protocol::ClientRequest::*;
use datax_app_server_protocol::InitializeCapabilities;
use datax_app_server_protocol::InitializeParams;
use datax_app_server_protocol::InteractionMessagesView;
use datax_app_server_protocol::InteractionStartParams;
use datax_app_server_protocol::InteractionStartResponse;
use datax_app_server_protocol::InteractionStatus;
use datax_app_server_protocol::JSONRPCError;
use datax_app_server_protocol::JSONRPCResponse;
use datax_app_server_protocol::Message;
use datax_app_server_protocol::RequestId;
use datax_app_server_protocol::SessionSource;
use datax_app_server_protocol::SortDirection;
use datax_app_server_protocol::UserInput;
use datax_arg0::Arg0DispatchPaths;
use datax_config::CloudConfigBundleLoader;
use datax_config::LoaderOverrides;
use datax_core::ARCHIVED_SESSIONS_SUBDIR;
use datax_core::config::ConfigBuilder;
use datax_exec_server::EnvironmentManager;
use datax_feedback::CodexFeedback;
use datax_protocol::models::BaseInstructions;
use datax_protocol::protocol::AgentMessageEvent;
use datax_protocol::protocol::EventMsg;
use datax_protocol::protocol::RolloutMessage;
use datax_protocol::protocol::SessionSource as ProtocolSessionSource;
use datax_protocol::protocol::ThreadMemoryMode;
use datax_protocol::protocol::UserMessageEvent;
use datax_protocol::user_input::ByteRange;
use datax_protocol::user_input::TextElement;
use datax_thread_store::AppendChatMessagesParams;
use datax_thread_store::CreateChatParams;
use datax_thread_store::InMemoryChatStore;
use datax_thread_store::ChatMetadataPatch;
use datax_thread_store::ChatPersistenceMetadata;
use datax_thread_store::ChatStore;
use datax_thread_store::UpdateChatMetadataParams;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::timeout;
use uuid::Uuid;

#[cfg(windows)]
const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(25);
#[cfg(not(windows))]
const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[tokio::test]
async fn thread_read_returns_summary_without_turns() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let preview = "Saved user message";
    let text_elements = [TextElement::new(
        ByteRange { start: 0, end: 5 },
        Some("<note>".into()),
    )];
    let conversation_id = create_fake_rollout_with_text_elements(
        codex_home.path(),
        "2025-01-05T12-00-00",
        "2025-01-05T12:00:00Z",
        preview,
        text_elements
            .iter()
            .map(|elem| serde_json::to_value(elem).expect("serialize text element"))
            .collect(),
        Some("mock_provider"),
        /*git_info*/ None,
    )?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let read_id = mcp
        .send_chat_read_request(ChatReadParams {
            chat_id: conversation_id.clone(),
            include_interactions: false,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ChatReadResponse { chat: thread, .. } = to_response::<ChatReadResponse>(read_resp)?;

    assert_eq!(thread.id, conversation_id);
    assert_eq!(thread.preview, preview);
    assert_eq!(thread.model_provider, "mock_provider");
    assert!(!thread.ephemeral, "stored rollouts should not be ephemeral");
    assert!(thread.path.as_ref().expect("thread path").is_absolute());
    assert_eq!(thread.cwd, test_absolute_path("/"));
    assert_eq!(thread.cli_version, "0.0.0");
    assert_eq!(thread.source, SessionSource::Cli);
    assert_eq!(thread.git_info, None);
    assert_eq!(thread.interactions.len(), 0);
    assert_eq!(thread.status, ChatStatus::NotLoaded);

    Ok(())
}

#[tokio::test]
async fn thread_read_can_include_turns() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let preview = "Saved user message";
    let text_elements = vec![TextElement::new(
        ByteRange { start: 0, end: 5 },
        Some("<note>".into()),
    )];
    let conversation_id = create_fake_rollout_with_text_elements(
        codex_home.path(),
        "2025-01-05T12-00-00",
        "2025-01-05T12:00:00Z",
        preview,
        text_elements
            .iter()
            .map(|elem| serde_json::to_value(elem).expect("serialize text element"))
            .collect(),
        Some("mock_provider"),
        /*git_info*/ None,
    )?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let read_id = mcp
        .send_chat_read_request(ChatReadParams {
            chat_id: conversation_id.clone(),
            include_interactions: true,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ChatReadResponse { chat: thread, .. } = to_response::<ChatReadResponse>(read_resp)?;

    assert_eq!(thread.interactions.len(), 1);
    let turn = &thread.interactions[0];
    assert_eq!(turn.status, InteractionStatus::Completed);
    assert_eq!(turn.messages_view, InteractionMessagesView::Full);
    assert_eq!(turn.items.len(), 1, "expected user message item");
    match &turn.items[0] {
        Message::UserMessage { content, .. } => {
            assert_eq!(
                content,
                &vec![UserInput::Text {
                    text: preview.to_string(),
                    text_elements: text_elements.clone().into_iter().map(Into::into).collect(),
                }]
            );
        }
        other => panic!("expected user message item, got {other:?}"),
    }
    assert_eq!(thread.status, ChatStatus::NotLoaded);

    Ok(())
}

#[tokio::test]
async fn thread_turns_list_can_page_backward_and_forward() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let filename_ts = "2025-01-05T12-00-00";
    let conversation_id = create_fake_rollout_with_text_elements(
        codex_home.path(),
        filename_ts,
        "2025-01-05T12:00:00Z",
        "first",
        vec![],
        Some("mock_provider"),
        /*git_info*/ None,
    )?;
    let rollout_path = rollout_path(codex_home.path(), filename_ts, &conversation_id);
    append_user_message(rollout_path.as_path(), "2025-01-05T12:01:00Z", "second")?;
    append_user_message(rollout_path.as_path(), "2025-01-05T12:02:00Z", "third")?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let read_id = mcp
        .send_chat_interactions_list_request(ChatInteractionsListParams {
            chat_id: conversation_id.clone(),
            cursor: None,
            limit: Some(2),
            sort_direction: Some(SortDirection::Desc),
            messages_view: None,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ChatInteractionsListResponse {
        data,
        next_cursor,
        backwards_cursor,
    } = to_response::<ChatInteractionsListResponse>(read_resp)?;
    assert_eq!(turn_user_texts(&data), vec!["third", "second"]);
    assert!(
        data.iter()
            .all(|turn| turn.messages_view == InteractionMessagesView::Summary)
    );
    let next_cursor = next_cursor.expect("expected nextCursor for older interactions");
    let backwards_cursor = backwards_cursor.expect("expected backwardsCursor for newest turn");

    let read_id = mcp
        .send_chat_interactions_list_request(ChatInteractionsListParams {
            chat_id: conversation_id.clone(),
            cursor: Some(next_cursor),
            limit: Some(10),
            sort_direction: Some(SortDirection::Desc),
            messages_view: None,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ChatInteractionsListResponse { data, .. } =
        to_response::<ChatInteractionsListResponse>(read_resp)?;
    assert_eq!(turn_user_texts(&data), vec!["first"]);

    append_user_message(rollout_path.as_path(), "2025-01-05T12:03:00Z", "fourth")?;

    let read_id = mcp
        .send_chat_interactions_list_request(ChatInteractionsListParams {
            chat_id: conversation_id,
            cursor: Some(backwards_cursor),
            limit: Some(10),
            sort_direction: Some(SortDirection::Asc),
            messages_view: None,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ChatInteractionsListResponse { data, .. } =
        to_response::<ChatInteractionsListResponse>(read_resp)?;
    assert_eq!(turn_user_texts(&data), vec!["third", "fourth"]);

    Ok(())
}

#[tokio::test]
async fn thread_turns_list_supports_requested_messages_view() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let filename_ts = "2025-01-05T12-00-00";
    let conversation_id = create_fake_rollout_with_text_elements(
        codex_home.path(),
        filename_ts,
        "2025-01-05T12:00:00Z",
        "first",
        vec![],
        Some("mock_provider"),
        /*git_info*/ None,
    )?;
    let rollout_path = rollout_path(codex_home.path(), filename_ts, &conversation_id);
    append_agent_message(rollout_path.as_path(), "2025-01-05T12:01:00Z", "draft")?;
    append_agent_message(rollout_path.as_path(), "2025-01-05T12:02:00Z", "final")?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let full = read_single_turn_messages_view(
        &mut mcp,
        conversation_id.as_str(),
        Some(InteractionMessagesView::Full),
    )
    .await?;
    assert_eq!(full.messages_view, InteractionMessagesView::Full);
    assert_eq!(
        turn_agent_texts(std::slice::from_ref(&full)),
        vec!["draft", "final"]
    );

    let summary = read_single_turn_messages_view(
        &mut mcp,
        conversation_id.as_str(),
        Some(InteractionMessagesView::Summary),
    )
    .await?;
    assert_eq!(summary.messages_view, InteractionMessagesView::Summary);
    assert_eq!(
        turn_user_texts(std::slice::from_ref(&summary)),
        vec!["first"]
    );
    assert_eq!(
        turn_agent_texts(std::slice::from_ref(&summary)),
        vec!["final"]
    );

    let not_loaded = read_single_turn_messages_view(
        &mut mcp,
        conversation_id.as_str(),
        Some(InteractionMessagesView::NotLoaded),
    )
    .await?;
    assert_eq!(not_loaded.messages_view, InteractionMessagesView::NotLoaded);
    assert!(not_loaded.items.is_empty());
    assert_eq!(not_loaded.id, full.id);
    assert_eq!(not_loaded.status, full.status);
    assert_eq!(not_loaded.started_at, full.started_at);
    assert_eq!(not_loaded.completed_at, full.completed_at);
    assert_eq!(not_loaded.duration_ms, full.duration_ms);

    Ok(())
}

#[tokio::test]
async fn thread_turns_list_reads_store_history_without_rollout_path() -> Result<()> {
    let codex_home = TempDir::new()?;
    let chat_id = datax_protocol::ChatId::from_string("00000000-0000-4000-8000-000000000123")?;
    let store_id = Uuid::new_v4().to_string();
    create_config_toml_with_thread_store(codex_home.path(), &store_id)?;
    let store = InMemoryChatStore::for_id(store_id.clone());
    let _in_memory_store = InMemoryChatStoreId { store_id };
    seed_pathless_store_thread(&store, chat_id).await?;

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
        session_source: SessionSource::Cli.into(),
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
        .request(ChatInteractionsList {
            request_id: RequestId::Integer(1),
            params: ChatInteractionsListParams {
                chat_id: chat_id.to_string(),
                cursor: None,
                limit: Some(10),
                sort_direction: Some(SortDirection::Asc),
                messages_view: None,
            },
        })
        .await?
        .expect("chat/interactions/list should succeed");
    let ChatInteractionsListResponse { data, .. } = serde_json::from_value(result)?;

    assert_eq!(turn_user_texts(&data), vec!["history from store"]);

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn thread_read_loaded_include_turns_reads_store_history_without_rollout_path() -> Result<()> {
    let codex_home = TempDir::new()?;
    let store_id = Uuid::new_v4().to_string();
    create_config_toml_with_thread_store(codex_home.path(), &store_id)?;
    let store = InMemoryChatStore::for_id(store_id.clone());
    let _in_memory_store = InMemoryChatStoreId { store_id };

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
        session_source: SessionSource::Cli.into(),
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
        .request(ChatStart {
            request_id: RequestId::Integer(1),
            params: ChatStartParams {
                model: Some("mock-model".to_string()),
                ..Default::default()
            },
        })
        .await?
        .expect("chat/start should succeed");
    let ChatStartResponse { chat: thread, .. } = serde_json::from_value(result)?;
    assert_eq!(thread.path, None);

    let chat_id = datax_protocol::ChatId::from_string(&thread.id)?;
    store
        .append_items(AppendChatMessagesParams {
            chat_id: chat_id,
            items: store_history_items(),
        })
        .await?;

    let result = client
        .request(ChatRead {
            request_id: RequestId::Integer(2),
            params: ChatReadParams {
                chat_id: thread.id,
                include_interactions: true,
            },
        })
        .await?
        .expect("chat/read should succeed");
    let ChatReadResponse { chat: thread, .. } = serde_json::from_value(result)?;

    assert_eq!(
        turn_user_texts(&thread.interactions),
        vec!["history from store"]
    );

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn thread_list_includes_store_thread_without_rollout_path() -> Result<()> {
    let codex_home = TempDir::new()?;
    let chat_id = datax_protocol::ChatId::from_string("00000000-0000-4000-8000-000000000124")?;
    let store_id = Uuid::new_v4().to_string();
    create_config_toml_with_thread_store(codex_home.path(), &store_id)?;
    let store = InMemoryChatStore::for_id(store_id.clone());
    let _in_memory_store = InMemoryChatStoreId { store_id };
    seed_pathless_store_thread(&store, chat_id).await?;

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
        session_source: SessionSource::Cli.into(),
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
        .request(ChatList {
            request_id: RequestId::Integer(1),
            params: ChatListParams {
                cursor: None,
                limit: Some(10),
                sort_key: None,
                sort_direction: None,
                model_providers: Some(Vec::new()),
                source_kinds: None,
                archived: None,
                cwd: None,
                use_state_db_only: false,
                search_term: None,
                parent_chat_id: None,
            },
        })
        .await?
        .expect("chat/list should succeed");
    let ChatListResponse { data, .. } = serde_json::from_value(result)?;

    assert_eq!(data.len(), 1);
    let thread = &data[0];
    assert_eq!(thread.id, chat_id.to_string());
    assert_eq!(thread.path, None);
    assert_eq!(thread.preview, "");
    assert_eq!(thread.name.as_deref(), Some("named pathless thread"));

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn thread_read_can_return_archived_threads_by_id() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let filename_ts = "2025-01-05T12-00-00";
    let preview = "Archived saved user message";
    let conversation_id = create_fake_rollout_with_text_elements(
        codex_home.path(),
        filename_ts,
        "2025-01-05T12:00:00Z",
        preview,
        vec![],
        Some("mock_provider"),
        /*git_info*/ None,
    )?;
    let active_rollout_path = rollout_path(codex_home.path(), filename_ts, &conversation_id);
    let archived_dir = codex_home.path().join(ARCHIVED_SESSIONS_SUBDIR);
    std::fs::create_dir_all(&archived_dir)?;
    let archived_rollout_path =
        archived_dir.join(active_rollout_path.file_name().expect("rollout file name"));
    std::fs::rename(&active_rollout_path, &archived_rollout_path)?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let read_id = mcp
        .send_chat_read_request(ChatReadParams {
            chat_id: conversation_id.clone(),
            include_interactions: false,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ChatReadResponse { chat: thread } = to_response::<ChatReadResponse>(read_resp)?;

    assert_eq!(thread.id, conversation_id);
    assert_eq!(thread.preview, preview);
    let path = thread.path.expect("thread path");
    assert_eq!(path.canonicalize()?, archived_rollout_path.canonicalize()?);

    Ok(())
}

#[tokio::test]
async fn thread_resume_initial_turns_page_matches_requested_turns_list_page() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let filename_ts = "2025-01-05T12-00-00";
    let conversation_id = create_fake_rollout_with_text_elements(
        codex_home.path(),
        filename_ts,
        "2025-01-05T12:00:00Z",
        "first",
        vec![],
        Some("mock_provider"),
        /*git_info*/ None,
    )?;
    let rollout_path = rollout_path(codex_home.path(), filename_ts, &conversation_id);
    append_user_message(rollout_path.as_path(), "2025-01-05T12:01:00Z", "second")?;
    append_user_message(rollout_path.as_path(), "2025-01-05T12:02:00Z", "third")?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let turns_list_id = mcp
        .send_chat_interactions_list_request(ChatInteractionsListParams {
            chat_id: conversation_id.clone(),
            cursor: None,
            limit: Some(2),
            sort_direction: Some(SortDirection::Asc),
            messages_view: Some(InteractionMessagesView::NotLoaded),
        })
        .await?;
    let turns_list_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turns_list_id)),
    )
    .await??;
    let expected_page = to_response::<ChatInteractionsListResponse>(turns_list_resp)?;

    let resume_id = mcp
        .send_chat_resume_request(ChatResumeParams {
            chat_id: conversation_id,
            exclude_interactions: true,
            initial_interactions_page: Some(ChatResumeInitialInteractionsPageParams {
                limit: Some(2),
                sort_direction: Some(SortDirection::Asc),
                messages_view: Some(InteractionMessagesView::NotLoaded),
            }),
            ..Default::default()
        })
        .await?;
    let resume_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(resume_id)),
    )
    .await??;
    let ChatResumeResponse {
        thread,
        initial_interactions_page,
        ..
    } = to_response::<ChatResumeResponse>(resume_resp)?;

    assert!(thread.interactions.is_empty());
    assert_eq!(
        initial_interactions_page,
        Some(datax_app_server_protocol::InteractionsPage::from(
            expected_page
        ))
    );

    Ok(())
}

#[tokio::test]
async fn thread_turns_list_rejects_cursor_when_anchor_turn_is_rolled_back() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let filename_ts = "2025-01-05T12-00-00";
    let conversation_id = create_fake_rollout_with_text_elements(
        codex_home.path(),
        filename_ts,
        "2025-01-05T12:00:00Z",
        "first",
        vec![],
        Some("mock_provider"),
        /*git_info*/ None,
    )?;
    let rollout_path = rollout_path(codex_home.path(), filename_ts, &conversation_id);
    append_user_message(rollout_path.as_path(), "2025-01-05T12:01:00Z", "second")?;
    append_user_message(rollout_path.as_path(), "2025-01-05T12:02:00Z", "third")?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let read_id = mcp
        .send_chat_interactions_list_request(ChatInteractionsListParams {
            chat_id: conversation_id.clone(),
            cursor: None,
            limit: Some(2),
            sort_direction: Some(SortDirection::Desc),
            messages_view: None,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ChatInteractionsListResponse {
        backwards_cursor, ..
    } = to_response::<ChatInteractionsListResponse>(read_resp)?;
    let backwards_cursor = backwards_cursor.expect("expected backwardsCursor for newest turn");

    append_thread_rollback(
        rollout_path.as_path(),
        "2025-01-05T12:03:00Z",
        /*num_turns*/ 1,
    )?;

    let read_id = mcp
        .send_chat_interactions_list_request(ChatInteractionsListParams {
            chat_id: conversation_id,
            cursor: Some(backwards_cursor),
            limit: Some(10),
            sort_direction: Some(SortDirection::Asc),
            messages_view: None,
        })
        .await?;
    let read_err: JSONRPCError = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(read_id)),
    )
    .await??;

    assert_eq!(
        read_err.error.message,
        "invalid cursor: anchor turn is no longer present"
    );

    Ok(())
}

#[tokio::test]
async fn thread_read_returns_forked_from_id_for_forked_threads() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let conversation_id = create_fake_rollout_with_text_elements(
        codex_home.path(),
        "2025-01-05T12-00-00",
        "2025-01-05T12:00:00Z",
        "Saved user message",
        vec![],
        Some("mock_provider"),
        /*git_info*/ None,
    )?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let fork_id = mcp
        .send_chat_fork_request(ChatForkParams {
            chat_id: conversation_id.clone(),
            ..Default::default()
        })
        .await?;
    let fork_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(fork_id)),
    )
    .await??;
    let ChatForkResponse { chat: forked, .. } = to_response::<ChatForkResponse>(fork_resp)?;

    let read_id = mcp
        .send_chat_read_request(ChatReadParams {
            chat_id: forked.id,
            include_interactions: false,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ChatReadResponse { chat: thread, .. } = to_response::<ChatReadResponse>(read_resp)?;

    assert_eq!(thread.forked_from_id, Some(conversation_id));

    Ok(())
}

#[tokio::test]
async fn thread_read_loaded_thread_returns_precomputed_path_before_materialization() -> Result<()> {
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
    let thread_path = thread.path.clone().expect("thread path");
    assert!(
        !thread_path.exists(),
        "fresh thread rollout should not be materialized yet"
    );

    let read_id = mcp
        .send_chat_read_request(ChatReadParams {
            chat_id: thread.id.clone(),
            include_interactions: false,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ChatReadResponse { chat: read, .. } = to_response::<ChatReadResponse>(read_resp)?;

    assert_eq!(read.id, thread.id);
    assert_eq!(read.path, Some(thread_path));
    assert!(read.preview.is_empty());
    assert_eq!(read.interactions.len(), 0);
    assert_eq!(read.status, ChatStatus::Idle);

    Ok(())
}

#[tokio::test]
async fn thread_name_set_is_reflected_in_read_list_and_resume() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let preview = "Saved user message";
    let conversation_id = create_fake_rollout_with_text_elements(
        codex_home.path(),
        "2025-01-05T12-00-00",
        "2025-01-05T12:00:00Z",
        preview,
        vec![],
        Some("mock_provider"),
        /*git_info*/ None,
    )?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    // Set a user-facing thread title.
    let new_name = "My renamed thread";
    let set_id = mcp
        .send_chat_set_name_request(ChatSetNameParams {
            chat_id: conversation_id.clone(),
            name: new_name.to_string(),
        })
        .await?;
    let set_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(set_id)),
    )
    .await??;
    let _: ChatSetNameResponse = to_response::<ChatSetNameResponse>(set_resp)?;
    let notification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("chat/name/updated"),
    )
    .await??;
    let notification: ChatNameUpdatedNotification =
        serde_json::from_value(notification.params.expect("chat/name/updated params"))?;
    assert_eq!(notification.chat_id, conversation_id);
    assert_eq!(notification.thread_name.as_deref(), Some(new_name));

    // Read should now surface `thread.name`, and the wire payload must include `name`.
    let read_id = mcp
        .send_chat_read_request(ChatReadParams {
            chat_id: conversation_id.clone(),
            include_interactions: false,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let read_result = read_resp.result.clone();
    let ChatReadResponse { chat: thread, .. } = to_response::<ChatReadResponse>(read_resp)?;
    assert_eq!(thread.id, conversation_id);
    assert_eq!(thread.name.as_deref(), Some(new_name));
    let thread_json = read_result
        .get("thread")
        .and_then(Value::as_object)
        .expect("chat/read result.chat must be an object");
    assert_eq!(
        thread_json.get("name").and_then(Value::as_str),
        Some(new_name),
        "chat/read must serialize `thread.name` on the wire"
    );
    assert_eq!(
        thread_json.get("ephemeral").and_then(Value::as_bool),
        Some(false),
        "chat/read must serialize `thread.ephemeral` on the wire"
    );

    // List should also surface the name.
    let list_id = mcp
        .send_chat_list_request(ChatListParams {
            cursor: None,
            limit: Some(50),
            sort_key: None,
            sort_direction: None,
            model_providers: Some(vec!["mock_provider".to_string()]),
            source_kinds: None,
            archived: None,
            cwd: None,
            use_state_db_only: false,
            search_term: None,
            parent_chat_id: None,
        })
        .await?;
    let list_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_id)),
    )
    .await??;
    let list_result = list_resp.result.clone();
    let ChatListResponse { data, .. } = to_response::<ChatListResponse>(list_resp)?;
    let listed = data
        .iter()
        .find(|t| t.id == conversation_id)
        .expect("chat/list should include the created thread");
    assert_eq!(listed.name.as_deref(), Some(new_name));
    let listed_json = list_result
        .get("data")
        .and_then(Value::as_array)
        .expect("chat/list result.data must be an array")
        .iter()
        .find(|t| t.get("id").and_then(Value::as_str) == Some(&conversation_id))
        .and_then(Value::as_object)
        .expect("chat/list should include the created thread as an object");
    assert_eq!(
        listed_json.get("name").and_then(Value::as_str),
        Some(new_name),
        "chat/list must serialize `thread.name` on the wire"
    );
    assert_eq!(
        listed_json.get("ephemeral").and_then(Value::as_bool),
        Some(false),
        "chat/list must serialize `thread.ephemeral` on the wire"
    );

    // Resume should also surface the name.
    let resume_id = mcp
        .send_chat_resume_request(ChatResumeParams {
            chat_id: conversation_id.clone(),
            ..Default::default()
        })
        .await?;
    let resume_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(resume_id)),
    )
    .await??;
    let resume_result = resume_resp.result.clone();
    let ChatResumeResponse { chat: resumed, .. } = to_response::<ChatResumeResponse>(resume_resp)?;
    assert_eq!(resumed.id, conversation_id);
    assert_eq!(resumed.name.as_deref(), Some(new_name));
    let resumed_json = resume_result
        .get("thread")
        .and_then(Value::as_object)
        .expect("chat/resume result.chat must be an object");
    assert_eq!(
        resumed_json.get("name").and_then(Value::as_str),
        Some(new_name),
        "chat/resume must serialize `thread.name` on the wire"
    );
    assert_eq!(
        resumed_json.get("ephemeral").and_then(Value::as_bool),
        Some(false),
        "chat/resume must serialize `thread.ephemeral` on the wire"
    );

    Ok(())
}

#[tokio::test]
async fn thread_read_include_turns_rejects_unmaterialized_loaded_thread() -> Result<()> {
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
    let thread_path = thread.path.clone().expect("thread path");
    assert!(
        !thread_path.exists(),
        "fresh thread rollout should not be materialized yet"
    );

    let read_id = mcp
        .send_chat_read_request(ChatReadParams {
            chat_id: thread.id.clone(),
            include_interactions: true,
        })
        .await?;
    let read_err: JSONRPCError = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(read_id)),
    )
    .await??;

    assert!(
        read_err
            .error
            .message
            .contains("includeInteractions is unavailable before first user message"),
        "unexpected error: {}",
        read_err.error.message
    );

    Ok(())
}

#[tokio::test]
async fn thread_turns_list_rejects_unmaterialized_loaded_thread() -> Result<()> {
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
    let thread_path = thread.path.clone().expect("thread path");
    assert!(
        !thread_path.exists(),
        "fresh thread rollout should not be materialized yet"
    );

    let read_id = mcp
        .send_chat_interactions_list_request(ChatInteractionsListParams {
            chat_id: thread.id,
            cursor: None,
            limit: None,
            sort_direction: None,
            messages_view: None,
        })
        .await?;
    let read_err: JSONRPCError = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(read_id)),
    )
    .await??;

    assert!(
        read_err
            .error
            .message
            .contains("chat/interactions/list is unavailable before first user message"),
        "unexpected error: {}",
        read_err.error.message
    );

    Ok(())
}

#[tokio::test]
async fn thread_turns_items_list_returns_unsupported() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let read_id = mcp
        .send_chat_interactions_messages_list_request(ChatInteractionsMessagesListParams {
            chat_id: "thr_123".to_string(),
            interaction_id: "turn_456".to_string(),
            cursor: None,
            limit: None,
            sort_direction: None,
        })
        .await?;
    let read_err: JSONRPCError = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(read_id)),
    )
    .await??;

    assert_eq!(read_err.error.code, -32601);
    assert_eq!(
        read_err.error.message,
        "chat/interactions/messages/list is not supported yet"
    );

    Ok(())
}

#[tokio::test]
async fn thread_read_reports_system_error_idle_flag_after_failed_turn() -> Result<()> {
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

    let turn_start_id = mcp
        .send_interaction_start_request(InteractionStartParams {
            chat_id: thread.id.clone(),
            client_user_message_id: None,
            input: vec![UserInput::Text {
                text: "fail this turn".to_string(),
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
        mcp.read_stream_until_notification_message("error"),
    )
    .await??;

    let read_id = mcp
        .send_chat_read_request(ChatReadParams {
            chat_id: thread.id,
            include_interactions: false,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ChatReadResponse { chat: thread, .. } = to_response::<ChatReadResponse>(read_resp)?;

    assert_eq!(thread.status, ChatStatus::SystemError,);

    Ok(())
}

fn append_user_message(path: &Path, timestamp: &str, text: &str) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new().append(true).open(path)?;
    writeln!(
        file,
        "{}",
        json!({
            "timestamp": timestamp,
            "type":"event_msg",
            "payload": {
                "type":"user_message",
                "message": text,
                "text_elements": [],
                "local_images": []
            }
        })
    )
}

fn append_agent_message(path: &Path, timestamp: &str, text: &str) -> anyhow::Result<()> {
    let mut file = std::fs::OpenOptions::new().append(true).open(path)?;
    writeln!(
        file,
        "{}",
        json!({
            "timestamp": timestamp,
            "type": "event_msg",
            "payload": serde_json::to_value(EventMsg::AgentMessage(AgentMessageEvent {
                message: text.to_string(),
                phase: None,
                memory_citation: None,
            }))?,
        })
    )?;
    Ok(())
}

fn append_thread_rollback(path: &Path, timestamp: &str, num_turns: u32) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new().append(true).open(path)?;
    writeln!(
        file,
        "{}",
        json!({
            "timestamp": timestamp,
            "type":"event_msg",
            "payload": {
                "type":"thread_rolled_back",
                "num_turns": num_turns
            }
        })
    )
}

async fn read_single_turn_messages_view(
    mcp: &mut TestAppServer,
    chat_id: &str,
    messages_view: Option<InteractionMessagesView>,
) -> anyhow::Result<datax_app_server_protocol::Interaction> {
    let read_id = mcp
        .send_chat_interactions_list_request(ChatInteractionsListParams {
            chat_id: chat_id.to_string(),
            cursor: None,
            limit: Some(10),
            sort_direction: Some(SortDirection::Asc),
            messages_view,
        })
        .await?;
    let read_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(read_id)),
    )
    .await??;
    let ChatInteractionsListResponse { mut data, .. } =
        to_response::<ChatInteractionsListResponse>(read_resp)?;
    assert_eq!(data.len(), 1);
    Ok(data.remove(0))
}

fn turn_user_texts(interactions: &[datax_app_server_protocol::Interaction]) -> Vec<&str> {
    interactions
        .iter()
        .filter_map(|turn| match turn.items.first()? {
            Message::UserMessage { content, .. } => match content.first()? {
                UserInput::Text { text, .. } => Some(text.as_str()),
                UserInput::Image { .. }
                | UserInput::LocalImage { .. }
                | UserInput::Skill { .. }
                | UserInput::Mention { .. } => None,
            },
            _ => None,
        })
        .collect()
}

fn turn_agent_texts(interactions: &[datax_app_server_protocol::Interaction]) -> Vec<&str> {
    interactions
        .iter()
        .flat_map(|turn| &turn.items)
        .filter_map(|item| match item {
            Message::AgentMessage { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

struct InMemoryChatStoreId {
    store_id: String,
}

impl Drop for InMemoryChatStoreId {
    fn drop(&mut self) {
        InMemoryChatStore::remove_id(&self.store_id);
    }
}

async fn seed_pathless_store_thread(
    store: &InMemoryChatStore,
    chat_id: datax_protocol::ChatId,
) -> Result<()> {
    store
        .create_chat(CreateChatParams {
            session_id: chat_id.into(),
            chat_id: chat_id,
            extra_config: None,
            forked_from_id: None,
            parent_chat_id: None,
            source: ProtocolSessionSource::Cli,
            chat_source: None,
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
        .append_items(AppendChatMessagesParams {
            chat_id: chat_id,
            items: store_history_items(),
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
    Ok(())
}

fn store_history_items() -> Vec<RolloutMessage> {
    vec![RolloutMessage::EventMsg(EventMsg::UserMessage(
        UserMessageEvent {
            client_id: None,
            message: "history from store".to_string(),
            images: None,
            local_images: Vec::new(),
            text_elements: Vec::new(),
            ..Default::default()
        },
    ))]
}

fn create_config_toml_with_thread_store(codex_home: &Path, store_id: &str) -> std::io::Result<()> {
    let config_toml = codex_home.join("config.toml");
    std::fs::write(
        config_toml,
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

// Helper to create a config.toml pointing at the mock model server.
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
