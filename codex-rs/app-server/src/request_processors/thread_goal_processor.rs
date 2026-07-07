use super::*;
use datax_goal_extension::GoalObjectiveUpdate;
use datax_goal_extension::GoalService;
use datax_goal_extension::GoalServiceError;
use datax_goal_extension::GoalSetRequest;
use datax_goal_extension::GoalTokenBudgetUpdate;

#[derive(Clone)]
pub(crate) struct ThreadGoalRequestProcessor {
    thread_manager: Arc<ThreadManager>,
    outgoing: Arc<OutgoingMessageSender>,
    config: Arc<Config>,
    thread_state_manager: ThreadStateManager,
    state_db: Option<StateDbHandle>,
    goal_service: Arc<GoalService>,
}

impl ThreadGoalRequestProcessor {
    pub(crate) fn new(
        thread_manager: Arc<ThreadManager>,
        outgoing: Arc<OutgoingMessageSender>,
        config: Arc<Config>,
        thread_state_manager: ThreadStateManager,
        state_db: Option<StateDbHandle>,
        goal_service: Arc<GoalService>,
    ) -> Self {
        Self {
            thread_manager,
            outgoing,
            config,
            thread_state_manager,
            state_db,
            goal_service,
        }
    }

    pub(crate) async fn thread_goal_set(
        &self,
        request_id: ConnectionRequestId,
        params: ChatGoalSetParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_goal_set_inner(request_id, params)
            .await
            .map(|()| None)
    }

    pub(crate) async fn thread_goal_get(
        &self,
        params: ChatGoalGetParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_goal_get_inner(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn thread_goal_clear(
        &self,
        request_id: ConnectionRequestId,
        params: ChatGoalClearParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_goal_clear_inner(request_id, params)
            .await
            .map(|()| None)
    }

    pub(crate) async fn emit_resume_goal_snapshot_and_continue(
        &self,
        chat_id: ThreadId,
        thread: &CodexThread,
    ) {
        if !self.config.features.enabled(Feature::Goals) {
            return;
        }
        self.emit_thread_goal_snapshot(chat_id).await;
        // App-server owns resume response and snapshot ordering, so wait until
        // those are sent before letting extensions react to the idle thread.
        thread.emit_thread_idle_lifecycle_if_idle().await;
    }

    pub(crate) async fn pending_resume_goal_state(
        &self,
        thread: &CodexThread,
    ) -> (bool, Option<StateDbHandle>) {
        let emit_thread_goal_update = self.config.features.enabled(Feature::Goals);
        let thread_goal_state_db = if emit_thread_goal_update {
            if let Some(state_db) = thread.state_db() {
                Some(state_db)
            } else {
                self.state_db.clone()
            }
        } else {
            None
        };
        (emit_thread_goal_update, thread_goal_state_db)
    }

    async fn thread_goal_set_inner(
        &self,
        request_id: ConnectionRequestId,
        params: ChatGoalSetParams,
    ) -> Result<(), JSONRPCErrorError> {
        if !self.config.features.enabled(Feature::Goals) {
            return Err(invalid_request("goals feature is disabled"));
        }

        let chat_id = parse_thread_id_for_request(params.chat_id.as_str())?;
        let state_db = self.state_db_for_materialized_thread(chat_id).await?;
        self.reconcile_thread_goal_rollout(chat_id, &state_db)
            .await?;

        let listener_command_tx = {
            let thread_state = self.thread_state_manager.thread_state(chat_id).await;
            let thread_state = thread_state.lock().await;
            thread_state.listener_command_tx()
        };
        let status = params.status.map(ChatGoalStatus::to_core);
        let objective = params.objective.as_deref();

        let outcome = self
            .goal_service
            .set_thread_goal(
                &state_db,
                GoalSetRequest {
                    chat_id,
                    objective: objective
                        .map(GoalObjectiveUpdate::Set)
                        .unwrap_or(GoalObjectiveUpdate::Keep),
                    status,
                    token_budget: match params.token_budget {
                        Some(token_budget) => GoalTokenBudgetUpdate::Set(token_budget),
                        None => GoalTokenBudgetUpdate::Keep,
                    },
                },
            )
            .await
            .map_err(goal_service_error)?;
        let goal = ChatGoal::from(outcome.goal.clone());

        let persist_result = match self.thread_manager.get_thread(chat_id).await {
            Ok(thread) => {
                // Live goal-first threads can be listed before any user turn is written.
                // Use the live path so JSONL and SQLite preview metadata stay in sync.
                thread
                    .append_rollout_items(&[outcome.thread_goal_updated_item()])
                    .await
            }
            Err(_) => Ok(()),
        };
        if let Err(err) = persist_result {
            warn!("failed to persist goal update for live thread {chat_id}: {err}");
        }

        self.outgoing
            .send_response(
                request_id.clone(),
                ChatGoalSetResponse { goal: goal.clone() },
            )
            .await;
        self.emit_thread_goal_updated_ordered(chat_id, goal, listener_command_tx)
            .await;
        outcome.apply_runtime_effects(&self.goal_service).await;
        Ok(())
    }

    async fn thread_goal_get_inner(
        &self,
        params: ChatGoalGetParams,
    ) -> Result<ChatGoalGetResponse, JSONRPCErrorError> {
        if !self.config.features.enabled(Feature::Goals) {
            return Err(invalid_request("goals feature is disabled"));
        }

        let chat_id = parse_thread_id_for_request(params.chat_id.as_str())?;
        let state_db = self.state_db_for_materialized_thread(chat_id).await?;
        let goal = self
            .goal_service
            .get_thread_goal(&state_db, chat_id)
            .await
            .map_err(goal_service_error)?
            .map(ChatGoal::from);
        Ok(ChatGoalGetResponse { goal })
    }

    async fn thread_goal_clear_inner(
        &self,
        request_id: ConnectionRequestId,
        params: ChatGoalClearParams,
    ) -> Result<(), JSONRPCErrorError> {
        if !self.config.features.enabled(Feature::Goals) {
            return Err(invalid_request("goals feature is disabled"));
        }

        let chat_id = parse_thread_id_for_request(params.chat_id.as_str())?;
        let state_db = self.state_db_for_materialized_thread(chat_id).await?;
        self.reconcile_thread_goal_rollout(chat_id, &state_db)
            .await?;

        let listener_command_tx = {
            let thread_state = self.thread_state_manager.thread_state(chat_id).await;
            let thread_state = thread_state.lock().await;
            thread_state.listener_command_tx()
        };
        let cleared = self
            .goal_service
            .clear_thread_goal(&state_db, chat_id)
            .await
            .map_err(goal_service_error)?;

        self.outgoing
            .send_response(request_id, ChatGoalClearResponse { cleared })
            .await;
        if cleared {
            self.emit_thread_goal_cleared_ordered(chat_id, listener_command_tx)
                .await;
        }
        Ok(())
    }

    async fn state_db_for_materialized_thread(
        &self,
        chat_id: ThreadId,
    ) -> Result<StateDbHandle, JSONRPCErrorError> {
        if let Ok(thread) = self.thread_manager.get_thread(chat_id).await {
            if thread.rollout_path().is_none() {
                return Err(invalid_request(format!(
                    "ephemeral thread does not support goals: {chat_id}"
                )));
            }
            if let Some(state_db) = thread.state_db() {
                return Ok(state_db);
            }
        } else {
            datax_rollout::find_thread_path_by_id_str(
                &self.config.codex_home,
                &chat_id.to_string(),
                self.state_db.as_deref(),
            )
            .await
            .map_err(|err| internal_error(format!("failed to locate thread id {chat_id}: {err}")))?
            .ok_or_else(|| invalid_request(format!("thread not found: {chat_id}")))?;
        }

        self.state_db
            .clone()
            .ok_or_else(|| internal_error("sqlite state db unavailable for thread goals"))
    }

    async fn reconcile_thread_goal_rollout(
        &self,
        chat_id: ThreadId,
        state_db: &StateDbHandle,
    ) -> Result<(), JSONRPCErrorError> {
        let running_thread = self.thread_manager.get_thread(chat_id).await.ok();
        let rollout_path = match running_thread.as_ref() {
            Some(thread) => thread.rollout_path().ok_or_else(|| {
                invalid_request(format!(
                    "ephemeral thread does not support goals: {chat_id}"
                ))
            })?,
            None => datax_rollout::find_thread_path_by_id_str(
                &self.config.codex_home,
                &chat_id.to_string(),
                self.state_db.as_deref(),
            )
            .await
            .map_err(|err| internal_error(format!("failed to locate thread id {chat_id}: {err}")))?
            .ok_or_else(|| invalid_request(format!("thread not found: {chat_id}")))?,
        };
        reconcile_rollout(
            Some(state_db),
            rollout_path.as_path(),
            self.config.model_provider_id.as_str(),
            /*builder*/ None,
            &[],
            /*archived_only*/ None,
            /*new_thread_memory_mode*/ None,
        )
        .await;
        Ok(())
    }

    async fn emit_thread_goal_snapshot(&self, chat_id: ThreadId) {
        let state_db = match self.state_db_for_materialized_thread(chat_id).await {
            Ok(state_db) => state_db,
            Err(err) => {
                warn!(
                    "failed to open state db before emitting thread goal resume snapshot for {chat_id}: {}",
                    err.message
                );
                return;
            }
        };
        let listener_command_tx = {
            let thread_state = self.thread_state_manager.thread_state(chat_id).await;
            let thread_state = thread_state.lock().await;
            thread_state.listener_command_tx()
        };
        if let Some(listener_command_tx) = listener_command_tx {
            let command = crate::thread_state::ThreadListenerCommand::EmitThreadGoalSnapshot {
                state_db: state_db.clone(),
            };
            if listener_command_tx.send(command).is_ok() {
                return;
            }
            warn!(
                "failed to enqueue thread goal snapshot for {chat_id}: listener command channel is closed"
            );
        }
        send_thread_goal_snapshot_notification(&self.outgoing, chat_id, &state_db).await;
    }

    async fn emit_thread_goal_updated_ordered(
        &self,
        chat_id: ThreadId,
        goal: ChatGoal,
        listener_command_tx: Option<tokio::sync::mpsc::UnboundedSender<ThreadListenerCommand>>,
    ) {
        if let Some(listener_command_tx) = listener_command_tx {
            let command = crate::thread_state::ThreadListenerCommand::EmitThreadGoalUpdated {
                interaction_id: None,
                goal: goal.clone(),
            };
            if listener_command_tx.send(command).is_ok() {
                return;
            }
            warn!(
                "failed to enqueue thread goal update for {chat_id}: listener command channel is closed"
            );
        }
        self.outgoing
            .send_server_notification(ChatGoalUpdated(ChatGoalUpdatedNotification {
                chat_id: chat_id.to_string(),
                interaction_id: None,
                goal,
            }))
            .await;
    }

    async fn emit_thread_goal_cleared_ordered(
        &self,
        chat_id: ThreadId,
        listener_command_tx: Option<tokio::sync::mpsc::UnboundedSender<ThreadListenerCommand>>,
    ) {
        if let Some(listener_command_tx) = listener_command_tx {
            let command = crate::thread_state::ThreadListenerCommand::EmitThreadGoalCleared;
            if listener_command_tx.send(command).is_ok() {
                return;
            }
            warn!(
                "failed to enqueue thread goal clear for {chat_id}: listener command channel is closed"
            );
        }
        self.outgoing
            .send_server_notification(ChatGoalCleared(ChatGoalClearedNotification {
                chat_id: chat_id.to_string(),
            }))
            .await;
    }
}

pub(super) fn api_thread_goal_from_state(goal: datax_state::ChatGoal) -> ChatGoal {
    ChatGoal {
        chat_id: goal.chat_id.to_string(),
        objective: goal.objective,
        status: api_thread_goal_status_from_state(goal.status),
        token_budget: goal.token_budget,
        tokens_used: goal.tokens_used,
        time_used_seconds: goal.time_used_seconds,
        created_at: goal.created_at.timestamp(),
        updated_at: goal.updated_at.timestamp(),
    }
}

fn api_thread_goal_status_from_state(status: datax_state::ChatGoalStatus) -> ChatGoalStatus {
    match status {
        datax_state::ChatGoalStatus::Active => ChatGoalStatus::Active,
        datax_state::ChatGoalStatus::Paused => ChatGoalStatus::Paused,
        datax_state::ChatGoalStatus::Blocked => ChatGoalStatus::Blocked,
        datax_state::ChatGoalStatus::UsageLimited => ChatGoalStatus::UsageLimited,
        datax_state::ChatGoalStatus::BudgetLimited => ChatGoalStatus::BudgetLimited,
        datax_state::ChatGoalStatus::Complete => ChatGoalStatus::Complete,
    }
}

fn goal_service_error(err: GoalServiceError) -> JSONRPCErrorError {
    match err {
        GoalServiceError::InvalidRequest(message) => invalid_request(message),
        GoalServiceError::Internal(message) => internal_error(message),
    }
}

fn parse_thread_id_for_request(chat_id: &str) -> Result<ThreadId, JSONRPCErrorError> {
    ThreadId::from_string(chat_id)
        .map_err(|err| invalid_request(format!("invalid thread id: {err}")))
}
