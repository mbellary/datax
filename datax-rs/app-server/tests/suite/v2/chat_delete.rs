use anyhow::Result;
use app_test_support::TestAppServer;
use app_test_support::create_fake_rollout;
use app_test_support::to_response;
use datax_app_server_protocol::ChatDeleteParams;
use datax_app_server_protocol::ChatDeleteResponse;
use datax_app_server_protocol::ChatDeletedNotification;
use datax_app_server_protocol::ChatLoadedListParams;
use datax_app_server_protocol::ChatLoadedListResponse;
use datax_app_server_protocol::ChatStartParams;
use datax_app_server_protocol::ChatStartResponse;
use datax_app_server_protocol::JSONRPCError;
use datax_app_server_protocol::JSONRPCResponse;
use datax_app_server_protocol::RequestId;
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
async fn thread_delete_deletes_spawned_descendants() -> Result<()> {
    let codex_home = TempDir::new()?;

    let parent_id = create_delete_test_rollout(codex_home.path(), /*minute*/ 0, "parent")?;
    let child_id = create_delete_test_rollout(codex_home.path(), /*minute*/ 1, "child")?;
    let grandchild_id =
        create_delete_test_rollout(codex_home.path(), /*minute*/ 2, "grandchild")?;

    let state_db =
        StateRuntime::init(codex_home.path().to_path_buf(), "mock_provider".into()).await?;
    let parent_chat_id = ThreadId::from_string(&parent_id)?;
    let child_thread_id = ThreadId::from_string(&child_id)?;
    let grandchild_thread_id = ThreadId::from_string(&grandchild_id)?;

    for (parent, child, status) in [
        (
            parent_chat_id,
            child_thread_id,
            DirectionalThreadSpawnEdgeStatus::Closed,
        ),
        (
            child_thread_id,
            grandchild_thread_id,
            DirectionalThreadSpawnEdgeStatus::Open,
        ),
    ] {
        state_db
            .upsert_thread_spawn_edge(parent, child, status)
            .await?;
    }

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let delete_id = mcp
        .send_chat_delete_request(ChatDeleteParams {
            chat_id: parent_id.clone(),
        })
        .await?;
    let delete_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(delete_id)),
    )
    .await??;
    let _: ChatDeleteResponse = to_response::<ChatDeleteResponse>(delete_resp)?;

    let mut deleted_ids = Vec::new();
    for _ in 0..3 {
        let notification = timeout(
            DEFAULT_READ_TIMEOUT,
            mcp.read_stream_until_notification_message("chat/deleted"),
        )
        .await??;
        let deleted_notification: ChatDeletedNotification = serde_json::from_value(
            notification
                .params
                .expect("chat/deleted notification params"),
        )?;
        deleted_ids.push(deleted_notification.chat_id);
    }
    assert_eq!(deleted_ids, vec![grandchild_id, child_id, parent_id]);

    for chat_id in [parent_chat_id, child_thread_id, grandchild_thread_id] {
        let rollout_path = find_thread_path_by_id_str(
            codex_home.path(),
            &chat_id.to_string(),
            /*state_db_ctx*/ None,
        )
        .await?;
        assert!(
            rollout_path.is_none(),
            "expected active rollout for {chat_id} to be deleted"
        );
    }
    assert_eq!(
        state_db
            .list_thread_spawn_descendants(parent_chat_id)
            .await?,
        Vec::<ThreadId>::new()
    );
    Ok(())
}

fn create_delete_test_rollout(codex_home: &Path, minute: u8, preview: &str) -> Result<String> {
    create_fake_rollout(
        codex_home,
        &format!("2025-01-01T00-{minute:02}-00"),
        &format!("2025-01-01T00:{minute:02}:00Z"),
        preview,
        Some("mock_provider"),
        /*git_info*/ None,
    )
}

#[tokio::test]
async fn thread_delete_handles_live_threads_before_rollout_exists() -> Result<()> {
    let codex_home = TempDir::new()?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let start_id = mcp
        .send_chat_start_request(ChatStartParams::default())
        .await?;
    let start_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_id)),
    )
    .await??;
    let persisted_thread = to_response::<ChatStartResponse>(start_resp)?.thread;
    let rollout_path = find_thread_path_by_id_str(
        codex_home.path(),
        &persisted_thread.id,
        /*state_db_ctx*/ None,
    )
    .await?;
    assert_eq!(rollout_path, None);

    let delete_id = mcp
        .send_chat_delete_request(ChatDeleteParams {
            chat_id: persisted_thread.id,
        })
        .await?;
    let delete_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(delete_id)),
    )
    .await??;
    let _: ChatDeleteResponse = to_response::<ChatDeleteResponse>(delete_resp)?;

    let start_id = mcp
        .send_chat_start_request(ChatStartParams {
            ephemeral: Some(true),
            ..Default::default()
        })
        .await?;
    let start_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_id)),
    )
    .await??;
    let ChatStartResponse { thread, .. } = to_response::<ChatStartResponse>(start_resp)?;

    let delete_id = mcp
        .send_chat_delete_request(ChatDeleteParams {
            chat_id: thread.id.clone(),
        })
        .await?;
    let delete_err: JSONRPCError = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(delete_id)),
    )
    .await??;
    let expected_message = format!(
        "thread is not persisted and cannot be deleted: {}",
        thread.id
    );
    assert_eq!(delete_err.error.message, expected_message);

    let list_id = mcp
        .send_chat_loaded_list_request(ChatLoadedListParams::default())
        .await?;
    let list_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_id)),
    )
    .await??;
    let ChatLoadedListResponse { mut data, .. } = to_response::<ChatLoadedListResponse>(list_resp)?;
    data.sort();
    assert_eq!(data, vec![thread.id]);

    Ok(())
}
