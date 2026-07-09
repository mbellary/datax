use crate::ChatManager;
use crate::agent::AgentControl;
use crate::chat_manager::ChatManagerState;
use crate::config::Config;
use crate::config::test_config;
use crate::datax_chat::DataxChat;
use datax_features::Feature;
use datax_login::CodexAuth;
use datax_protocol::ChatId;
use datax_protocol::error::CodexErr;
use datax_protocol::protocol::EventMsg;
use datax_protocol::protocol::InteractionAbortReason;
use datax_protocol::protocol::InteractionAbortedEvent;
use datax_protocol::protocol::InteractionCompleteEvent;
use datax_protocol::protocol::SessionSource;
use datax_protocol::protocol::SubAgentSource;
use datax_protocol::protocol::ThreadSource;
use pretty_assertions::assert_eq;
use std::sync::Arc;

#[tokio::test]
async fn residency_slot_reservation_unloads_oldest_idle_v2_agent() {
    let mut config = test_config().await;
    let _ = config.features.enable(Feature::MultiAgentV2);
    config.multi_agent_v2.max_concurrent_threads_per_session = 2;
    let temp_home = tempfile::tempdir().expect("create temp home");
    config.codex_home = temp_home.path().to_path_buf().try_into().unwrap();
    config.cwd = temp_home.path().to_path_buf().try_into().unwrap();
    let manager = ChatManager::with_models_provider_and_home_for_tests(
        CodexAuth::from_api_key("dummy"),
        config.model_provider.clone(),
        config.codex_home.to_path_buf(),
        Arc::new(datax_exec_server::EnvironmentManager::default_for_tests()),
    );
    let root = manager
        .start_chat(config.clone())
        .await
        .expect("start root thread");
    let control = manager.agent_control();
    let state = control.upgrade().expect("thread manager should be live");

    let first_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_chat_id*/ None)
        .await
        .expect("first resident slot");
    let first = spawn_v2_subagent(&control, &state, config.clone(), root.chat_id, "worker-1").await;
    first_slot.commit(first.chat_id);
    mark_thread_completed(first.chat.as_ref()).await;

    let second_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_chat_id*/ None)
        .await
        .expect("second resident slot should evict the first idle agent");
    match manager.get_chat(first.chat_id).await {
        Err(CodexErr::ThreadNotFound(chat_id)) => assert_eq!(chat_id, first.chat_id),
        Err(err) => panic!("expected evicted thread to be missing, got {err:?}"),
        Ok(_) => panic!("expected evicted thread to be missing"),
    }
    let second = spawn_v2_subagent(&control, &state, config, root.chat_id, "worker-2").await;
    second_slot.commit(second.chat_id);

    assert!(manager.get_chat(root.chat_id).await.is_ok());
    assert!(manager.get_chat(second.chat_id).await.is_ok());
}

#[tokio::test]
async fn interrupted_v2_agent_is_lost_after_residency_eviction() {
    let mut config = test_config().await;
    let _ = config.features.enable(Feature::MultiAgentV2);
    config.multi_agent_v2.max_concurrent_threads_per_session = 2;
    let temp_home = tempfile::tempdir().expect("create temp home");
    config.codex_home = temp_home.path().to_path_buf().try_into().unwrap();
    config.cwd = temp_home.path().to_path_buf().try_into().unwrap();
    let manager = ChatManager::with_models_provider_and_home_for_tests(
        CodexAuth::from_api_key("dummy"),
        config.model_provider.clone(),
        config.codex_home.to_path_buf(),
        Arc::new(datax_exec_server::EnvironmentManager::default_for_tests()),
    );
    let root = manager
        .start_chat(config.clone())
        .await
        .expect("start root thread");
    let control = manager.agent_control();
    let state = control.upgrade().expect("thread manager should be live");

    let first_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_chat_id*/ None)
        .await
        .expect("first resident slot");
    let first = spawn_v2_subagent(&control, &state, config.clone(), root.chat_id, "worker-1").await;
    first_slot.commit(first.chat_id);
    mark_thread_interrupted(first.chat.as_ref()).await;

    let second_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_chat_id*/ None)
        .await
        .expect("second resident slot should evict the first interrupted idle agent");
    match manager.get_chat(first.chat_id).await {
        Err(CodexErr::ThreadNotFound(chat_id)) => assert_eq!(chat_id, first.chat_id),
        Err(err) => panic!("expected evicted thread to be missing, got {err:?}"),
        Ok(_) => panic!("expected evicted thread to be missing"),
    }
    let second =
        spawn_v2_subagent(&control, &state, config.clone(), root.chat_id, "worker-2").await;
    second_slot.commit(second.chat_id);
    mark_thread_completed(second.chat.as_ref()).await;

    let err = control
        .ensure_v2_agent_loaded(config, first.chat_id)
        .await
        .expect_err("evicted interrupted agent should stay lost");
    match err {
        CodexErr::ThreadNotFound(chat_id) => assert_eq!(chat_id, first.chat_id),
        err => panic!("expected ThreadNotFound, got {err:?}"),
    }

    assert!(manager.get_chat(root.chat_id).await.is_ok());
    assert!(manager.get_chat(second.chat_id).await.is_ok());
    match manager.get_chat(first.chat_id).await {
        Err(CodexErr::ThreadNotFound(chat_id)) => assert_eq!(chat_id, first.chat_id),
        Err(err) => panic!("expected evicted thread to be missing, got {err:?}"),
        Ok(_) => panic!("expected evicted thread to be missing"),
    }
}

async fn spawn_v2_subagent(
    control: &AgentControl,
    state: &Arc<ChatManagerState>,
    config: Config,
    parent_chat_id: ChatId,
    label: &str,
) -> crate::chat_manager::NewChat {
    state
        .spawn_new_thread_with_source(
            config,
            control.clone(),
            SessionSource::SubAgent(SubAgentSource::Other(label.to_string())),
            Some(parent_chat_id),
            /*forked_from_chat_id*/ None,
            Some(ThreadSource::Subagent),
            /*metrics_service_name*/ None,
            /*inherited_environments*/ None,
            /*inherited_exec_policy*/ None,
            /*environments*/ None,
        )
        .await
        .expect("spawn v2 subagent")
}

async fn mark_thread_completed(thread: &DataxChat) {
    let turn = thread.codex.session.new_default_turn().await;
    thread
        .codex
        .session
        .send_event(
            turn.as_ref(),
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: turn.sub_id.clone(),
                last_agent_message: Some("done".to_string()),
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
        )
        .await;
    clear_active_turn(thread).await;
}

async fn mark_thread_interrupted(thread: &DataxChat) {
    let turn = thread.codex.session.new_default_turn().await;
    thread
        .codex
        .session
        .send_event(
            turn.as_ref(),
            EventMsg::InteractionAborted(InteractionAbortedEvent {
                interaction_id: Some(turn.sub_id.clone()),
                reason: InteractionAbortReason::Interrupted,
                completed_at: None,
                duration_ms: None,
            }),
        )
        .await;
    clear_active_turn(thread).await;
}

async fn clear_active_turn(thread: &DataxChat) {
    // The fixture has no task runner to clear the turn after the terminal event.
    *thread.codex.session.active_turn.lock().await = None;
}
