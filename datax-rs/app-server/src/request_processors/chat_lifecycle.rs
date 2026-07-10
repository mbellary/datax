use super::*;
use datax_protocol::config_types::MultiAgentMode;

pub(super) const THREAD_UNLOADING_DELAY: Duration = Duration::from_secs(30 * 60);

#[derive(Clone)]
pub(super) struct ListenerTaskContext {
    pub(super) chat_manager: Arc<ChatManager>,
    pub(super) chat_state_manager: ChatStateManager,
    pub(super) outgoing: Arc<OutgoingMessageSender>,
    pub(super) pending_chat_unloads: Arc<Mutex<HashSet<ChatId>>>,
    pub(super) chat_watch_manager: ChatWatchManager,
    pub(super) thread_list_state_permit: Arc<Semaphore>,
    pub(super) fallback_model_provider: String,
    pub(super) codex_home: PathBuf,
    pub(super) skills_watcher: Arc<SkillsWatcher>,
}

struct UnloadingState {
    delay: Duration,
    has_subscribers_rx: watch::Receiver<bool>,
    has_subscribers: (bool, Instant),
    chat_status_rx: watch::Receiver<ChatStatus>,
    is_active: (bool, Instant),
}

impl UnloadingState {
    async fn new(
        listener_task_context: &ListenerTaskContext,
        chat_id: ChatId,
        delay: Duration,
    ) -> Option<Self> {
        let has_subscribers_rx = listener_task_context
            .chat_state_manager
            .subscribe_to_has_connections(chat_id)
            .await?;
        let chat_status_rx = listener_task_context
            .chat_watch_manager
            .subscribe(chat_id)
            .await?;
        let has_subscribers = (*has_subscribers_rx.borrow(), Instant::now());
        let is_active = (
            matches!(*chat_status_rx.borrow(), ChatStatus::Active { .. }),
            Instant::now(),
        );
        Some(Self {
            delay,
            has_subscribers_rx,
            has_subscribers,
            chat_status_rx,
            is_active,
        })
    }

    fn unloading_target(&self) -> Option<Instant> {
        match (self.has_subscribers, self.is_active) {
            ((false, has_no_subscribers_since), (false, is_inactive_since)) => {
                Some(std::cmp::max(has_no_subscribers_since, is_inactive_since) + self.delay)
            }
            _ => None,
        }
    }

    fn sync_receiver_values(&mut self) {
        let has_subscribers = *self.has_subscribers_rx.borrow();
        if self.has_subscribers.0 != has_subscribers {
            self.has_subscribers = (has_subscribers, Instant::now());
        }

        let is_active = matches!(*self.chat_status_rx.borrow(), ChatStatus::Active { .. });
        if self.is_active.0 != is_active {
            self.is_active = (is_active, Instant::now());
        }
    }

    fn should_unload_now(&mut self) -> bool {
        self.sync_receiver_values();
        self.unloading_target()
            .is_some_and(|target| target <= Instant::now())
    }

    fn note_thread_activity_observed(&mut self) {
        if !self.is_active.0 {
            self.is_active = (false, Instant::now());
        }
    }

    async fn wait_for_unloading_trigger(&mut self) -> bool {
        loop {
            self.sync_receiver_values();
            let unloading_target = self.unloading_target();
            if let Some(target) = unloading_target
                && target <= Instant::now()
            {
                return true;
            }
            let unloading_sleep = async {
                if let Some(target) = unloading_target {
                    tokio::time::sleep_until(target.into()).await;
                } else {
                    futures::future::pending::<()>().await;
                }
            };
            tokio::select! {
                _ = unloading_sleep => return true,
                changed = self.has_subscribers_rx.changed() => {
                    if changed.is_err() {
                        return false;
                    }
                    self.sync_receiver_values();
                },
                changed = self.chat_status_rx.changed() => {
                    if changed.is_err() {
                        return false;
                    }
                    self.sync_receiver_values();
                },
            }
        }
    }
}

pub(super) enum ChatShutdownResult {
    Complete,
    SubmitFailed,
    TimedOut,
}

pub(super) enum EnsureConversationListenerResult {
    Attached,
    ConnectionClosed,
}

#[expect(
    clippy::await_holding_invalid_type,
    reason = "listener subscription must be serialized against pending unloads"
)]
pub(super) async fn ensure_conversation_listener(
    listener_task_context: ListenerTaskContext,
    conversation_id: ChatId,
    connection_id: ConnectionId,
    raw_events_enabled: bool,
) -> Result<EnsureConversationListenerResult, JSONRPCErrorError> {
    let conversation = match listener_task_context
        .chat_manager
        .get_chat(conversation_id)
        .await
    {
        Ok(conv) => conv,
        Err(_) => {
            return Err(invalid_request(format!(
                "thread not found: {conversation_id}"
            )));
        }
    };
    let chat_state = {
        let pending_chat_unloads = listener_task_context.pending_chat_unloads.lock().await;
        if pending_chat_unloads.contains(&conversation_id) {
            return Err(invalid_request(format!(
                "thread {conversation_id} is closing; retry after the thread is closed"
            )));
        }
        let Some(chat_state) = listener_task_context
            .chat_state_manager
            .try_ensure_connection_subscribed(conversation_id, connection_id, raw_events_enabled)
            .await
        else {
            return Ok(EnsureConversationListenerResult::ConnectionClosed);
        };
        chat_state
    };
    if let Err(error) = ensure_listener_task_running(
        listener_task_context.clone(),
        conversation_id,
        conversation,
        chat_state,
    )
    .await
    {
        let _ = listener_task_context
            .chat_state_manager
            .unsubscribe_connection_from_chat(conversation_id, connection_id)
            .await;
        return Err(error);
    }
    Ok(EnsureConversationListenerResult::Attached)
}

pub(super) fn log_listener_attach_result(
    result: Result<EnsureConversationListenerResult, JSONRPCErrorError>,
    chat_id: ChatId,
    connection_id: ConnectionId,
    thread_kind: &'static str,
) {
    match result {
        Ok(EnsureConversationListenerResult::Attached) => {}
        Ok(EnsureConversationListenerResult::ConnectionClosed) => {
            tracing::debug!(
                chat_id = %chat_id,
                connection_id = ?connection_id,
                "skipping auto-attach for closed connection"
            );
        }
        Err(err) => {
            tracing::warn!(
                "failed to attach listener for {thread_kind} {chat_id}: {message}",
                message = err.message
            );
        }
    }
}

pub(super) async fn ensure_listener_task_running(
    listener_task_context: ListenerTaskContext,
    conversation_id: ChatId,
    conversation: Arc<DataxChat>,
    chat_state: Arc<Mutex<ChatState>>,
) -> Result<(), JSONRPCErrorError> {
    let (cancel_tx, mut cancel_rx) = oneshot::channel();
    let Some(mut unloading_state) = UnloadingState::new(
        &listener_task_context,
        conversation_id,
        THREAD_UNLOADING_DELAY,
    )
    .await
    else {
        return Err(invalid_request(format!(
            "thread {conversation_id} is closing; retry after the thread is closed"
        )));
    };
    let config = conversation.config().await;
    let environments = conversation.environment_selections().await;
    let watch_registration = listener_task_context
        .skills_watcher
        .register_thread_config(
            config.as_ref(),
            listener_task_context.chat_manager.as_ref(),
            &environments,
        )
        .await;
    let chat_settings_baseline =
        chat_settings_from_config_snapshot(&conversation.config_snapshot().await);
    let (mut listener_command_rx, listener_generation) = {
        let mut chat_state = chat_state.lock().await;
        if chat_state.listener_matches(&conversation) {
            return Ok(());
        }
        let (listener_command_rx, listener_generation) = chat_state.set_listener(
            cancel_tx,
            &conversation,
            watch_registration,
            chat_settings_baseline,
        );
        let Some(listener_command_tx) = chat_state.listener_command_tx() else {
            tracing::warn!(
                "chat listener command sender missing immediately after listener registration"
            );
            return Ok(());
        };
        listener_task_context
            .chat_state_manager
            .register_listener_command_tx(conversation_id, listener_command_tx);
        (listener_command_rx, listener_generation)
    };
    let ListenerTaskContext {
        outgoing,
        chat_manager,
        chat_state_manager,
        pending_chat_unloads,
        chat_watch_manager,
        thread_list_state_permit,
        fallback_model_provider,
        codex_home,
        ..
    } = listener_task_context;
    let outgoing_for_task = Arc::clone(&outgoing);
    tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = &mut cancel_rx => {
                    // Listener was superseded or the thread is being torn down.
                    break;
                }
                listener_command = listener_command_rx.recv() => {
                    let Some(listener_command) = listener_command else {
                        break;
                    };
                    handle_chat_listener_command(
                        conversation_id,
                        &conversation,
                        codex_home.as_path(),
                        &chat_state_manager,
                        &chat_state,
                        &chat_watch_manager,
                        &outgoing_for_task,
                        &pending_chat_unloads,
                        listener_command,
                    )
                    .await;
                }
                event = conversation.next_event() => {
                    let event = match event {
                        Ok(event) => event,
                        Err(err) => {
                            tracing::warn!("thread.next_event() failed with: {err}");
                            break;
                        }
                    };

                    // Track the event before emitting any typed translations
                    // so thread-local state such as raw event opt-in stays
                    // synchronized with the conversation.
                    let raw_events_enabled = {
                        let mut chat_state = chat_state.lock().await;
                        chat_state.track_current_interaction_event(&event.id, &event.msg);
                        chat_state.experimental_raw_events
                    };
                    let subscribed_connection_ids = chat_state_manager
                        .subscribed_connection_ids(conversation_id)
                        .await;
                    let thread_outgoing = ThreadScopedOutgoingMessageSender::new(
                        outgoing_for_task.clone(),
                        subscribed_connection_ids,
                        conversation_id,
                    );

                    if let EventMsg::RawResponseItem(raw_response_item_event) = &event.msg
                        && !raw_events_enabled
                    {
                        maybe_emit_hook_prompt_item_completed(
                            conversation_id,
                            &event.id,
                            &raw_response_item_event.item,
                            &thread_outgoing,
                        )
                        .await;
                        continue;
                    }

                    apply_bespoke_event_handling(
                        event.clone(),
                        conversation_id,
                        conversation.clone(),
                        chat_manager.clone(),
                        thread_outgoing,
                        chat_state.clone(),
                        chat_watch_manager.clone(),
                        thread_list_state_permit.clone(),
                        fallback_model_provider.clone(),
                    )
                    .await;
                }
                unloading_watchers_open = unloading_state.wait_for_unloading_trigger() => {
                    if !unloading_watchers_open {
                        break;
                    }
                    if !unloading_state.should_unload_now() {
                        continue;
                    }
                    if matches!(conversation.agent_status().await, AgentStatus::Running) {
                        unloading_state.note_thread_activity_observed();
                        continue;
                    }
                    {
                        let mut pending_chat_unloads = pending_chat_unloads.lock().await;
                        if pending_chat_unloads.contains(&conversation_id) {
                            continue;
                        }
                        if !unloading_state.should_unload_now() {
                            continue;
                        }
                        pending_chat_unloads.insert(conversation_id);
                    }
                    unload_thread_without_subscribers(
                        chat_manager.clone(),
                        outgoing_for_task.clone(),
                        pending_chat_unloads.clone(),
                        chat_state_manager.clone(),
                        chat_watch_manager.clone(),
                        conversation_id,
                        conversation.clone(),
                    )
                    .await;
                    break;
                }
            }
        }

        let mut chat_state = chat_state.lock().await;
        if chat_state.listener_generation == listener_generation {
            chat_state_manager.unregister_listener_command_tx(conversation_id);
            chat_state.clear_listener();
        }
    });
    Ok(())
}

pub(super) async fn wait_for_chat_shutdown(thread: &Arc<DataxChat>) -> ChatShutdownResult {
    match tokio::time::timeout(Duration::from_secs(10), thread.shutdown_and_wait()).await {
        Ok(Ok(())) => ChatShutdownResult::Complete,
        Ok(Err(_)) => ChatShutdownResult::SubmitFailed,
        Err(_) => ChatShutdownResult::TimedOut,
    }
}

pub(super) async fn unload_thread_without_subscribers(
    chat_manager: Arc<ChatManager>,
    outgoing: Arc<OutgoingMessageSender>,
    pending_chat_unloads: Arc<Mutex<HashSet<ChatId>>>,
    chat_state_manager: ChatStateManager,
    chat_watch_manager: ChatWatchManager,
    chat_id: ChatId,
    thread: Arc<DataxChat>,
) {
    info!("thread {chat_id} has no subscribers and is idle; shutting down");

    // Any pending app-server -> client requests for this thread can no longer be
    // answered; cancel their callbacks before shutdown/unload.
    outgoing
        .cancel_requests_for_thread(chat_id, /*error*/ None)
        .await;
    chat_state_manager.remove_chat_state(chat_id).await;

    tokio::spawn(async move {
        match wait_for_chat_shutdown(&thread).await {
            ChatShutdownResult::Complete => {
                if chat_manager.remove_chat(&chat_id).await.is_none() {
                    info!("thread {chat_id} was already removed before teardown finalized");
                    chat_watch_manager.remove_chat(&chat_id.to_string()).await;
                    pending_chat_unloads.lock().await.remove(&chat_id);
                    return;
                }
                chat_watch_manager.remove_chat(&chat_id.to_string()).await;
                let notification = ChatClosedNotification {
                    chat_id: chat_id.to_string(),
                };
                outgoing
                    .send_server_notification(ChatClosed(notification))
                    .await;
                pending_chat_unloads.lock().await.remove(&chat_id);
            }
            ChatShutdownResult::SubmitFailed => {
                pending_chat_unloads.lock().await.remove(&chat_id);
                warn!("failed to submit Shutdown to thread {chat_id}");
            }
            ChatShutdownResult::TimedOut => {
                pending_chat_unloads.lock().await.remove(&chat_id);
                warn!("thread {chat_id} shutdown timed out; leaving thread loaded");
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_chat_listener_command(
    conversation_id: ChatId,
    conversation: &Arc<DataxChat>,
    codex_home: &Path,
    chat_state_manager: &ChatStateManager,
    chat_state: &Arc<Mutex<ChatState>>,
    chat_watch_manager: &ChatWatchManager,
    outgoing: &Arc<OutgoingMessageSender>,
    pending_chat_unloads: &Arc<Mutex<HashSet<ChatId>>>,
    listener_command: ChatListenerCommand,
) {
    match listener_command {
        ChatListenerCommand::SendChatResumeResponse(resume_request) => {
            handle_pending_chat_resume_request(
                conversation_id,
                conversation,
                codex_home,
                chat_state_manager,
                chat_state,
                chat_watch_manager,
                outgoing,
                pending_chat_unloads,
                *resume_request,
            )
            .await;
        }
        ChatListenerCommand::EmitChatGoalUpdated {
            interaction_id,
            goal,
        } => {
            outgoing
                .send_server_notification(ChatGoalUpdated(ChatGoalUpdatedNotification {
                    chat_id: conversation_id.to_string(),
                    interaction_id,
                    goal,
                }))
                .await;
        }
        ChatListenerCommand::EmitChatGoalCleared => {
            outgoing
                .send_server_notification(ChatGoalCleared(ChatGoalClearedNotification {
                    chat_id: conversation_id.to_string(),
                }))
                .await;
        }
        ChatListenerCommand::EmitChatGoalSnapshot { state_db } => {
            send_chat_goal_snapshot_notification(outgoing, conversation_id, &state_db).await;
        }
        ChatListenerCommand::ResolveServerRequest {
            request_id,
            completion_tx,
        } => {
            resolve_pending_server_request(
                conversation_id,
                chat_state_manager,
                outgoing,
                request_id,
            )
            .await;
            let _ = completion_tx.send(());
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[expect(
    clippy::await_holding_invalid_type,
    reason = "running-thread resume subscription must be serialized against pending unloads"
)]
pub(super) async fn handle_pending_chat_resume_request(
    conversation_id: ChatId,
    conversation: &Arc<DataxChat>,
    _codex_home: &Path,
    chat_state_manager: &ChatStateManager,
    chat_state: &Arc<Mutex<ChatState>>,
    chat_watch_manager: &ChatWatchManager,
    outgoing: &Arc<OutgoingMessageSender>,
    pending_chat_unloads: &Arc<Mutex<HashSet<ChatId>>>,
    pending: crate::chat_state::PendingChatResumeRequest,
) {
    let active_interaction = {
        let state = chat_state.lock().await;
        state.active_interaction_snapshot()
    };
    tracing::debug!(
        chat_id = %conversation_id,
        request_id = ?pending.request_id,
        active_interaction_present = active_interaction.is_some(),
        active_interaction_id = ?active_interaction.as_ref().map(|turn| turn.id.as_str()),
        active_interaction_status = ?active_interaction.as_ref().map(|turn| &turn.status),
        "composing running chat resume response"
    );
    let has_live_in_progress_turn =
        matches!(conversation.agent_status().await, AgentStatus::Running)
            || active_interaction
                .as_ref()
                .is_some_and(|turn| matches!(turn.status, InteractionStatus::InProgress));

    let request_id = pending.request_id;
    let connection_id = request_id.connection_id;
    let mut thread = pending.chat_summary;
    if pending.include_interactions {
        populate_chat_interactions_from_history(
            &mut thread,
            &pending.history_items,
            active_interaction.as_ref(),
        );
    }

    let chat_status = chat_watch_manager
        .loaded_status_for_chat(&thread.id)
        .await;

    set_chat_status_and_interrupt_stale_turns(
        &mut thread,
        chat_status,
        has_live_in_progress_turn,
    );
    let token_usage_thread = pending.include_interactions.then(|| thread.clone());
    let mut initial_turns_page = if let Some(params) = pending.initial_turns_page.as_ref() {
        match super::chat_processor::build_chat_resume_initial_turns_page(
            &pending.history_items,
            thread.status.clone(),
            has_live_in_progress_turn,
            active_interaction,
            params,
        ) {
            Ok(page) => Some(page),
            Err(error) => {
                outgoing.send_error(request_id, error).await;
                return;
            }
        }
    } else {
        None
    };
    if pending.redact_resume_payloads {
        redact_chat_resume_payloads(&mut thread.interactions);
        if let Some(initial_turns_page) = initial_turns_page.as_mut() {
            redact_chat_resume_payloads(&mut initial_turns_page.data);
        }
    }

    {
        let pending_chat_unloads = pending_chat_unloads.lock().await;
        if pending_chat_unloads.contains(&conversation_id) {
            drop(pending_chat_unloads);
            outgoing
                .send_error(
                    request_id,
                    invalid_request(format!(
                        "thread {conversation_id} is closing; retry chat/resume after the thread is closed"
                    )),
                )
                .await;
            return;
        }
        if !chat_state_manager
            .try_add_connection_to_chat(conversation_id, connection_id)
            .await
        {
            tracing::debug!(
                chat_id = %conversation_id,
                connection_id = ?connection_id,
                "skipping running chat resume for closed connection"
            );
            return;
        }
    }

    let config_snapshot = pending.config_snapshot;
    let cwd = config_snapshot.cwd().clone();
    let ChatConfigSnapshot {
        model,
        model_provider_id,
        service_tier,
        approval_policy,
        approvals_reviewer,
        permission_profile,
        active_permission_profile,
        workspace_roots,
        reasoning_effort,
        ..
    } = config_snapshot;
    let instruction_sources = pending.instruction_sources;
    let sandbox = thread_response_sandbox_policy(&permission_profile, cwd.as_path());
    let active_permission_profile =
        thread_response_active_permission_profile(active_permission_profile);
    let session_id = conversation.session_configured().session_id.to_string();
    thread.session_id = session_id;

    let response = ChatResumeResponse {
        chat: thread,
        model,
        model_provider: model_provider_id,
        service_tier,
        cwd,
        runtime_workspace_roots: workspace_roots,
        instruction_sources,
        approval_policy: approval_policy.into(),
        approvals_reviewer: approvals_reviewer.into(),
        sandbox,
        active_permission_profile,
        reasoning_effort,
        multi_agent_mode: MultiAgentMode::ExplicitRequestOnly,
        initial_interactions_page: initial_turns_page,
    };
    outgoing.send_response(request_id, response).await;
    // Match cold resume: metadata-only resume should attach the listener without
    // paying the cost of turn reconstruction for historical usage replay.
    if let Some(token_usage_thread) = token_usage_thread {
        let token_usage_interaction_id = latest_token_usage_interaction_id_from_rollout_items(
            &pending.history_items,
            token_usage_thread.interactions.as_slice(),
        );
        // Rejoining a loaded thread has the same UI contract as a cold resume, but
        // uses the live conversation state instead of reconstructing a new session.
        send_thread_token_usage_update_to_connection(
            outgoing,
            connection_id,
            conversation_id,
            &token_usage_thread,
            conversation.as_ref(),
            token_usage_interaction_id,
        )
        .await;
    }
    if pending.emit_chat_goal_update {
        if let Some(state_db) = pending.chat_goal_state_db {
            send_chat_goal_snapshot_notification(outgoing, conversation_id, &state_db).await;
        } else {
            tracing::warn!(
                chat_id = %conversation_id,
                "state db unavailable when reading chat goal for running chat resume"
            );
        }
    }
    outgoing
        .replay_requests_to_connection_for_thread(connection_id, conversation_id)
        .await;
    // App-server owns resume response and snapshot ordering, so wait until
    // replay completes before letting extensions react to the idle thread.
    if pending.emit_chat_goal_update {
        conversation.emit_chat_idle_lifecycle_if_idle().await;
    }
}

pub(super) async fn send_chat_goal_snapshot_notification(
    outgoing: &Arc<OutgoingMessageSender>,
    chat_id: ChatId,
    state_db: &StateDbHandle,
) {
    match state_db.thread_goals().get_thread_goal(chat_id).await {
        Ok(Some(goal)) => {
            outgoing
                .send_server_notification(ChatGoalUpdated(ChatGoalUpdatedNotification {
                    chat_id: chat_id.to_string(),
                    interaction_id: None,
                    goal: api_chat_goal_from_state(goal),
                }))
                .await;
        }
        Ok(None) => {
            outgoing
                .send_server_notification(ChatGoalCleared(ChatGoalClearedNotification {
                    chat_id: chat_id.to_string(),
                }))
                .await;
        }
        Err(err) => {
            tracing::warn!(
                chat_id = %chat_id,
                "failed to read chat goal for resume snapshot: {err}"
            );
        }
    }
}

pub(crate) fn populate_chat_interactions_from_history(
    thread: &mut Chat,
    messages: &[RolloutMessage],
    active_interaction: Option<&Interaction>,
) {
    let mut interactions = build_api_turns_from_rollout_items(messages);
    if let Some(active_interaction) = active_interaction {
        merge_interaction_history_with_active_interaction(&mut interactions, active_interaction.clone());
    }
    thread.interactions = interactions;
}

pub(super) async fn resolve_pending_server_request(
    conversation_id: ChatId,
    chat_state_manager: &ChatStateManager,
    outgoing: &Arc<OutgoingMessageSender>,
    request_id: RequestId,
) {
    let chat_id = conversation_id.to_string();
    let subscribed_connection_ids = chat_state_manager
        .subscribed_connection_ids(conversation_id)
        .await;
    let outgoing = ThreadScopedOutgoingMessageSender::new(
        outgoing.clone(),
        subscribed_connection_ids,
        conversation_id,
    );
    outgoing
        .send_server_notification(ServerNotification::ServerRequestResolved(
            ServerRequestResolvedNotification {
                chat_id,
                request_id,
            },
        ))
        .await;
}

pub(super) fn merge_interaction_history_with_active_interaction(
    interactions: &mut Vec<Interaction>,
    active_interaction: Interaction,
) {
    interactions.retain(|turn| turn.id != active_interaction.id);
    interactions.push(active_interaction);
}

pub(super) fn set_chat_status_and_interrupt_stale_turns(
    thread: &mut Chat,
    loaded_status: ChatStatus,
    has_live_in_progress_turn: bool,
) {
    let status = resolve_chat_status(loaded_status, has_live_in_progress_turn);
    if !matches!(status, ChatStatus::Active { .. }) {
        for turn in &mut thread.interactions {
            if matches!(turn.status, InteractionStatus::InProgress) {
                turn.status = InteractionStatus::Interrupted;
            }
        }
    }
    thread.status = status;
}
