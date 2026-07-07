use anyhow::Result;
use app_test_support::TestAppServer;
use app_test_support::create_fake_rollout;
use app_test_support::create_mock_responses_server_repeating_assistant;
use app_test_support::to_response;
use datax_app_server_protocol::ChatArchiveParams;
use datax_app_server_protocol::ChatArchiveResponse;
use datax_app_server_protocol::ChatArchivedNotification;
use datax_app_server_protocol::ChatResumeParams;
use datax_app_server_protocol::ChatResumeResponse;
use datax_app_server_protocol::ChatStartParams;
use datax_app_server_protocol::ChatStartResponse;
use datax_app_server_protocol::ChatStatus;
use datax_app_server_protocol::ChatUnarchiveParams;
use datax_app_server_protocol::ChatUnarchiveResponse;
use datax_app_server_protocol::InteractionStartParams;
use datax_app_server_protocol::InteractionStartResponse;
use datax_app_server_protocol::JSONRPCError;
use datax_app_server_protocol::JSONRPCResponse;
use datax_app_server_protocol::RequestId;
use datax_app_server_protocol::UserInput;
use datax_core::ARCHIVED_SESSIONS_SUBDIR;
use datax_core::find_archived_thread_path_by_id_str;
use datax_core::find_thread_path_by_id_str;
use datax_protocol::ThreadId;
use datax_state::DirectionalThreadSpawnEdgeStatus;
use datax_state::StateRuntime;
use pretty_assertions::assert_eq;
use std::path::Path;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[tokio::test]
async fn thread_archive_requires_materialized_rollout() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
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
    assert!(!thread.id.is_empty());

    let rollout_path = thread.path.clone().expect("thread path");
    assert!(
        !rollout_path.exists(),
        "fresh thread rollout should not exist yet at {}",
        rollout_path.display()
    );
    assert!(
        find_thread_path_by_id_str(codex_home.path(), &thread.id, /*state_db_ctx*/ None)
            .await?
            .is_none(),
        "thread id should not be discoverable before rollout materialization"
    );

    // Archive should fail before the rollout is materialized.
    let archive_id = mcp
        .send_chat_archive_request(ChatArchiveParams {
            chat_id: thread.id.clone(),
        })
        .await?;
    let archive_err: JSONRPCError = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(archive_id)),
    )
    .await??;
    assert!(
        archive_err
            .error
            .message
            .contains("no rollout found for thread id"),
        "unexpected archive error: {}",
        archive_err.error.message
    );

    // Materialize rollout via a real user turn and confirm archive succeeds.
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

    assert!(
        rollout_path.exists(),
        "expected rollout path {} to exist after first user message",
        rollout_path.display()
    );

    let discovered_path =
        find_thread_path_by_id_str(codex_home.path(), &thread.id, /*state_db_ctx*/ None)
            .await?
            .expect("expected rollout path for thread id to exist after materialization");
    assert_paths_match_on_disk(&discovered_path, &rollout_path)?;

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
    let archive_notification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("chat/archived"),
    )
    .await??;
    let archived_notification: ChatArchivedNotification = serde_json::from_value(
        archive_notification
            .params
            .expect("chat/archived notification params"),
    )?;
    assert_eq!(archived_notification.chat_id, thread.id);

    // Verify file moved.
    let archived_directory = codex_home.path().join(ARCHIVED_SESSIONS_SUBDIR);
    // The archived file keeps the original filename (rollout-...-<id>.jsonl).
    let archived_rollout_path =
        archived_directory.join(rollout_path.file_name().expect("rollout file name"));
    assert!(
        !rollout_path.exists(),
        "expected rollout path {} to be moved",
        rollout_path.display()
    );
    assert!(
        archived_rollout_path.exists(),
        "expected archived rollout path {} to exist",
        archived_rollout_path.display()
    );

    Ok(())
}

#[tokio::test]
async fn thread_archive_archives_spawned_descendants() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let parent_id = create_fake_rollout(
        codex_home.path(),
        "2025-01-01T00-00-00",
        "2025-01-01T00:00:00Z",
        "parent",
        Some("mock_provider"),
        /*git_info*/ None,
    )?;
    let child_id = create_fake_rollout(
        codex_home.path(),
        "2025-01-01T00-01-00",
        "2025-01-01T00:01:00Z",
        "child",
        Some("mock_provider"),
        /*git_info*/ None,
    )?;
    let grandchild_id = create_fake_rollout(
        codex_home.path(),
        "2025-01-01T00-02-00",
        "2025-01-01T00:02:00Z",
        "grandchild",
        Some("mock_provider"),
        /*git_info*/ None,
    )?;

    let parent_chat_id = ThreadId::from_string(&parent_id)?;
    let child_thread_id = ThreadId::from_string(&child_id)?;
    let grandchild_thread_id = ThreadId::from_string(&grandchild_id)?;
    let state_db =
        StateRuntime::init(codex_home.path().to_path_buf(), "mock_provider".into()).await?;
    state_db
        .mark_backfill_complete(/*last_watermark*/ None)
        .await?;
    state_db
        .upsert_thread_spawn_edge(
            parent_chat_id,
            child_thread_id,
            DirectionalThreadSpawnEdgeStatus::Closed,
        )
        .await?;
    state_db
        .upsert_thread_spawn_edge(
            child_thread_id,
            grandchild_thread_id,
            DirectionalThreadSpawnEdgeStatus::Open,
        )
        .await?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let archive_id = mcp
        .send_chat_archive_request(ChatArchiveParams {
            chat_id: parent_id.clone(),
        })
        .await?;
    let archive_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(archive_id)),
    )
    .await??;
    let _: ChatArchiveResponse = to_response::<ChatArchiveResponse>(archive_resp)?;

    let mut archived_ids = Vec::new();
    for _ in 0..3 {
        let notification = timeout(
            DEFAULT_READ_TIMEOUT,
            mcp.read_stream_until_notification_message("chat/archived"),
        )
        .await??;
        let archived_notification: ChatArchivedNotification = serde_json::from_value(
            notification
                .params
                .expect("chat/archived notification params"),
        )?;
        archived_ids.push(archived_notification.chat_id);
    }
    assert_eq!(archived_ids, vec![parent_id, grandchild_id, child_id]);

    for chat_id in [parent_chat_id, child_thread_id, grandchild_thread_id] {
        assert!(
            find_thread_path_by_id_str(
                codex_home.path(),
                &chat_id.to_string(),
                /*state_db_ctx*/ None,
            )
            .await?
            .is_none(),
            "expected active rollout for {chat_id} to be archived"
        );
        assert!(
            find_archived_thread_path_by_id_str(
                codex_home.path(),
                &chat_id.to_string(),
                /*state_db_ctx*/ None,
            )
            .await?
            .is_some(),
            "expected archived rollout for {chat_id} to exist"
        );
    }

    Ok(())
}

#[tokio::test]
async fn thread_archive_succeeds_when_descendant_archive_fails() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let parent_id = create_fake_rollout(
        codex_home.path(),
        "2025-01-01T00-00-00",
        "2025-01-01T00:00:00Z",
        "parent",
        Some("mock_provider"),
        /*git_info*/ None,
    )?;
    let child_id = create_fake_rollout(
        codex_home.path(),
        "2025-01-01T00-01-00",
        "2025-01-01T00:01:00Z",
        "child",
        Some("mock_provider"),
        /*git_info*/ None,
    )?;
    let grandchild_id = create_fake_rollout(
        codex_home.path(),
        "2025-01-01T00-02-00",
        "2025-01-01T00:02:00Z",
        "grandchild",
        Some("mock_provider"),
        /*git_info*/ None,
    )?;

    let parent_chat_id = ThreadId::from_string(&parent_id)?;
    let child_thread_id = ThreadId::from_string(&child_id)?;
    let grandchild_thread_id = ThreadId::from_string(&grandchild_id)?;
    let state_db =
        StateRuntime::init(codex_home.path().to_path_buf(), "mock_provider".into()).await?;
    state_db
        .mark_backfill_complete(/*last_watermark*/ None)
        .await?;
    state_db
        .upsert_thread_spawn_edge(
            parent_chat_id,
            child_thread_id,
            DirectionalThreadSpawnEdgeStatus::Closed,
        )
        .await?;
    state_db
        .upsert_thread_spawn_edge(
            child_thread_id,
            grandchild_thread_id,
            DirectionalThreadSpawnEdgeStatus::Open,
        )
        .await?;

    let child_rollout_path =
        find_thread_path_by_id_str(codex_home.path(), &child_id, /*state_db_ctx*/ None)
            .await?
            .expect("child rollout path");
    let archived_child_path = codex_home
        .path()
        .join(ARCHIVED_SESSIONS_SUBDIR)
        .join(child_rollout_path.file_name().expect("rollout file name"));
    std::fs::create_dir_all(&archived_child_path)?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let archive_id = mcp
        .send_chat_archive_request(ChatArchiveParams {
            chat_id: parent_id.clone(),
        })
        .await?;
    let archive_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(archive_id)),
    )
    .await??;
    let _: ChatArchiveResponse = to_response::<ChatArchiveResponse>(archive_resp)?;

    let mut archived_ids = Vec::new();
    for _ in 0..2 {
        let notification = timeout(
            DEFAULT_READ_TIMEOUT,
            mcp.read_stream_until_notification_message("chat/archived"),
        )
        .await??;
        let archived_notification: ChatArchivedNotification = serde_json::from_value(
            notification
                .params
                .expect("chat/archived notification params"),
        )?;
        archived_ids.push(archived_notification.chat_id);
    }
    assert_eq!(archived_ids, vec![parent_id, grandchild_id]);

    assert!(
        timeout(
            std::time::Duration::from_millis(250),
            mcp.read_stream_until_notification_message("chat/archived"),
        )
        .await
        .is_err()
    );

    assert!(
        child_rollout_path.exists(),
        "child should stay active after descendant archive failure"
    );
    assert!(
        archived_child_path.is_dir(),
        "test conflict should remain in archived sessions"
    );
    for chat_id in [parent_chat_id, grandchild_thread_id] {
        assert!(
            find_thread_path_by_id_str(
                codex_home.path(),
                &chat_id.to_string(),
                /*state_db_ctx*/ None,
            )
            .await?
            .is_none(),
            "expected active rollout for {chat_id} to be archived"
        );
        assert!(
            find_archived_thread_path_by_id_str(
                codex_home.path(),
                &chat_id.to_string(),
                /*state_db_ctx*/ None,
            )
            .await?
            .is_some(),
            "expected archived rollout for {chat_id} to exist"
        );
    }

    Ok(())
}

#[tokio::test]
async fn thread_archive_succeeds_when_spawned_descendant_is_missing() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let parent_id = create_fake_rollout(
        codex_home.path(),
        "2025-01-01T00-00-00",
        "2025-01-01T00:00:00Z",
        "parent",
        Some("mock_provider"),
        /*git_info*/ None,
    )?;
    let parent_chat_id = ThreadId::from_string(&parent_id)?;
    let missing_child_thread_id = ThreadId::from_string("00000000-0000-0000-0000-000000000901")?;

    let state_db =
        StateRuntime::init(codex_home.path().to_path_buf(), "mock_provider".into()).await?;
    state_db
        .mark_backfill_complete(/*last_watermark*/ None)
        .await?;
    state_db
        .upsert_thread_spawn_edge(
            parent_chat_id,
            missing_child_thread_id,
            DirectionalThreadSpawnEdgeStatus::Closed,
        )
        .await?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let archive_id = mcp
        .send_chat_archive_request(ChatArchiveParams {
            chat_id: parent_id.clone(),
        })
        .await?;
    let archive_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(archive_id)),
    )
    .await??;
    let _: ChatArchiveResponse = to_response::<ChatArchiveResponse>(archive_resp)?;

    let notification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("chat/archived"),
    )
    .await??;
    let archived_notification: ChatArchivedNotification = serde_json::from_value(
        notification
            .params
            .expect("chat/archived notification params"),
    )?;
    assert_eq!(archived_notification.chat_id, parent_id);

    assert!(
        find_thread_path_by_id_str(codex_home.path(), &parent_id, /*state_db_ctx*/ None)
            .await?
            .is_none(),
        "parent should be archived even when a descendant is missing"
    );
    assert!(
        find_archived_thread_path_by_id_str(
            codex_home.path(),
            &parent_id,
            /*state_db_ctx*/ None,
        )
        .await?
        .is_some(),
        "parent should be moved into archived sessions"
    );

    Ok(())
}

#[tokio::test]
async fn thread_archive_clears_stale_subscriptions_before_resume() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let mut primary = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, primary.initialize()).await??;

    let start_id = primary
        .send_chat_start_request(ChatStartParams {
            model: Some("mock-model".to_string()),
            ..Default::default()
        })
        .await?;
    let start_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        primary.read_stream_until_response_message(RequestId::Integer(start_id)),
    )
    .await??;
    let ChatStartResponse { thread, .. } = to_response::<ChatStartResponse>(start_resp)?;

    let turn_start_id = primary
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
        primary.read_stream_until_response_message(RequestId::Integer(turn_start_id)),
    )
    .await??;
    let _: InteractionStartResponse = to_response::<InteractionStartResponse>(turn_start_response)?;
    timeout(
        DEFAULT_READ_TIMEOUT,
        primary.read_stream_until_notification_message("interaction/completed"),
    )
    .await??;
    primary.clear_message_buffer();

    let mut secondary = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, secondary.initialize()).await??;

    let archive_id = primary
        .send_chat_archive_request(ChatArchiveParams {
            chat_id: thread.id.clone(),
        })
        .await?;
    let archive_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        primary.read_stream_until_response_message(RequestId::Integer(archive_id)),
    )
    .await??;
    let _: ChatArchiveResponse = to_response::<ChatArchiveResponse>(archive_resp)?;
    timeout(
        DEFAULT_READ_TIMEOUT,
        primary.read_stream_until_notification_message("chat/archived"),
    )
    .await??;

    let unarchive_id = primary
        .send_chat_unarchive_request(ChatUnarchiveParams {
            chat_id: thread.id.clone(),
        })
        .await?;
    let unarchive_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        primary.read_stream_until_response_message(RequestId::Integer(unarchive_id)),
    )
    .await??;
    let _: ChatUnarchiveResponse = to_response::<ChatUnarchiveResponse>(unarchive_resp)?;
    timeout(
        DEFAULT_READ_TIMEOUT,
        primary.read_stream_until_notification_message("chat/unarchived"),
    )
    .await??;
    primary.clear_message_buffer();

    let resume_id = secondary
        .send_chat_resume_request(ChatResumeParams {
            chat_id: thread.id.clone(),
            ..Default::default()
        })
        .await?;
    let resume_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        secondary.read_stream_until_response_message(RequestId::Integer(resume_id)),
    )
    .await??;
    let resume: ChatResumeResponse = to_response::<ChatResumeResponse>(resume_resp)?;
    assert_eq!(resume.thread.status, ChatStatus::Idle);
    primary.clear_message_buffer();
    secondary.clear_message_buffer();

    let resumed_turn_id = secondary
        .send_interaction_start_request(InteractionStartParams {
            chat_id: thread.id,
            client_user_message_id: None,
            input: vec![UserInput::Text {
                text: "secondary turn".to_string(),
                text_elements: Vec::new(),
            }],
            ..Default::default()
        })
        .await?;
    let resumed_turn_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        secondary.read_stream_until_response_message(RequestId::Integer(resumed_turn_id)),
    )
    .await??;
    let _: InteractionStartResponse = to_response::<InteractionStartResponse>(resumed_turn_resp)?;

    assert!(
        timeout(
            std::time::Duration::from_millis(250),
            primary.read_stream_until_notification_message("interaction/started"),
        )
        .await
        .is_err()
    );

    timeout(
        DEFAULT_READ_TIMEOUT,
        secondary.read_stream_until_notification_message("interaction/completed"),
    )
    .await??;

    Ok(())
}

fn create_config_toml(codex_home: &Path, server_uri: &str) -> std::io::Result<()> {
    let config_toml = codex_home.join("config.toml");
    std::fs::write(config_toml, config_contents(server_uri))
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
