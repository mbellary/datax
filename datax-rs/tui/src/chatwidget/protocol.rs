use super::*;

impl ChatWidget {
    pub(crate) fn handle_server_notification(
        &mut self,
        notification: ServerNotification,
        replay_kind: Option<ReplayKind>,
    ) {
        // Reject misrouted child updates before shared notification handling mutates parent state.
        if let ServerNotification::McpServerStatusUpdated(notification) = &notification
            && let (Some(notification_chat_id), Some(chat_id)) =
                (notification.chat_id.as_deref(), self.chat_id())
            && notification_chat_id != chat_id.to_string()
        {
            return;
        }

        let from_replay = replay_kind.is_some();
        let is_resume_initial_replay =
            matches!(replay_kind, Some(ReplayKind::ResumeInitialMessages));
        let is_retry_error = matches!(
            &notification,
            ServerNotification::Error(ErrorNotification {
                will_retry: true,
                ..
            })
        );
        if !is_resume_initial_replay && !is_retry_error {
            self.restore_retry_status_header_if_present();
        }
        match notification {
            ServerNotification::ChatTokenUsageUpdated(notification) => {
                self.set_token_info(Some(token_usage_info_from_app_server(
                    notification.token_usage,
                )));
            }
            ServerNotification::ChatNameUpdated(notification) => {
                match ChatId::from_string(&notification.chat_id) {
                    Ok(chat_id) => {
                        self.on_thread_name_updated(chat_id, notification.thread_name)
                    }
                    Err(err) => {
                        tracing::warn!(
                            chat_id = notification.chat_id,
                            error = %err,
                            "ignoring app-server ThreadNameUpdated with invalid chat_id"
                        );
                    }
                }
            }
            ServerNotification::ChatGoalUpdated(notification) => {
                self.on_thread_goal_updated(notification.goal, notification.interaction_id);
            }
            ServerNotification::ChatGoalCleared(notification) => {
                self.on_thread_goal_cleared(notification.chat_id.as_str());
            }
            ServerNotification::ChatSettingsUpdated(notification) => {
                self.on_thread_settings_updated(notification);
            }
            ServerNotification::InteractionStarted(notification) => {
                self.turn_lifecycle.last_interaction_id = Some(notification.turn.id);
                self.last_non_retry_error = None;
                if !matches!(replay_kind, Some(ReplayKind::ResumeInitialMessages)) {
                    self.on_task_started();
                }
            }
            ServerNotification::InteractionCompleted(notification) => {
                self.handle_turn_completed_notification(notification, replay_kind);
            }
            ServerNotification::MessageStarted(notification) => {
                self.handle_item_started_notification(notification, replay_kind.is_some());
            }
            ServerNotification::MessageCompleted(notification) => {
                self.handle_item_completed_notification(notification, replay_kind);
            }
            ServerNotification::AgentMessageDelta(notification) => {
                self.on_agent_message_delta(notification.delta);
            }
            ServerNotification::PlanDelta(notification) => self.on_plan_delta(notification.delta),
            ServerNotification::ReasoningSummaryTextDelta(notification) => {
                self.on_agent_reasoning_delta(notification.delta);
            }
            ServerNotification::ReasoningTextDelta(notification) => {
                if self.config.show_raw_agent_reasoning {
                    self.on_agent_reasoning_delta(notification.delta);
                }
            }
            ServerNotification::ReasoningSummaryPartAdded(_) => self.on_reasoning_section_break(),
            ServerNotification::TerminalInteraction(notification) => {
                self.on_terminal_interaction(notification.process_id, notification.stdin)
            }
            ServerNotification::CommandExecutionOutputDelta(notification) => {
                self.on_exec_command_output_delta(&notification.message_id, &notification.delta);
            }
            ServerNotification::FileChangeOutputDelta(notification) => {
                self.on_patch_apply_output_delta(notification.message_id, notification.delta);
            }
            ServerNotification::InteractionDiffUpdated(notification) => {
                self.on_turn_diff(notification.diff)
            }
            ServerNotification::InteractionPlanUpdated(notification) => {
                self.on_plan_update(UpdatePlanArgs {
                    explanation: notification.explanation,
                    plan: notification
                        .plan
                        .into_iter()
                        .map(|step| UpdatePlanItemArg {
                            step: step.step,
                            status: match step.status {
                                InteractionPlanStepStatus::Pending => UpdatePlanItemStatus::Pending,
                                InteractionPlanStepStatus::InProgress => {
                                    UpdatePlanItemStatus::InProgress
                                }
                                InteractionPlanStepStatus::Completed => {
                                    UpdatePlanItemStatus::Completed
                                }
                            },
                        })
                        .collect(),
                })
            }
            ServerNotification::HookStarted(notification) => {
                self.on_hook_started(notification.run);
            }
            ServerNotification::HookCompleted(notification) => {
                self.on_hook_completed(notification.run);
            }
            ServerNotification::Error(notification) => {
                if notification.will_retry {
                    if !from_replay {
                        self.on_stream_error(
                            notification.error.message,
                            notification.error.additional_details,
                        );
                    }
                } else {
                    self.last_non_retry_error = Some((
                        notification.interaction_id.clone(),
                        notification.error.message.clone(),
                    ));
                    self.handle_non_retry_error(
                        notification.error.message,
                        notification.error.codex_error_info,
                    );
                }
            }
            ServerNotification::SkillsChanged(_) => {
                self.refresh_skills_for_current_cwd(/*force_reload*/ true);
            }
            ServerNotification::ModelRerouted(_) => {}
            ServerNotification::ModelVerification(notification) => {
                self.on_app_server_model_verification(&notification.verifications)
            }
            ServerNotification::Warning(notification) => self.on_warning(notification.message),
            ServerNotification::GuardianWarning(notification) => {
                self.on_warning(notification.message)
            }
            ServerNotification::DeprecationNotice(notification) => {
                self.on_deprecation_notice(notification.summary, notification.details)
            }
            ServerNotification::ConfigWarning(notification) => self.on_warning(
                notification
                    .details
                    .map(|details| format!("{}: {details}", notification.summary))
                    .unwrap_or(notification.summary),
            ),
            ServerNotification::McpServerStatusUpdated(notification) => {
                self.on_mcp_server_status_updated(notification)
            }
            ServerNotification::MessageGuardianApprovalReviewStarted(notification) => {
                self.on_guardian_review_notification(
                    notification.review_id,
                    notification.interaction_id,
                    notification.started_at_ms,
                    notification.review,
                    /*completion*/ None,
                    notification.action,
                );
            }
            ServerNotification::MessageGuardianApprovalReviewCompleted(notification) => {
                self.on_guardian_review_notification(
                    notification.review_id,
                    notification.interaction_id,
                    notification.started_at_ms,
                    notification.review,
                    Some((notification.completed_at_ms, notification.decision_source)),
                    notification.action,
                );
            }
            ServerNotification::ChatClosed(_) => {
                if !from_replay {
                    self.on_shutdown_complete();
                }
            }
            ServerNotification::ServerRequestResolved(_)
            | ServerNotification::AccountUpdated(_)
            | ServerNotification::AccountRateLimitsUpdated(_)
            | ServerNotification::ChatStarted(_)
            | ServerNotification::ChatStatusChanged(_)
            | ServerNotification::ChatArchived(_)
            | ServerNotification::ChatDeleted(_)
            | ServerNotification::ChatUnarchived(_)
            | ServerNotification::RawResponseMessageCompleted(_)
            | ServerNotification::CommandExecOutputDelta(_)
            | ServerNotification::ProcessOutputDelta(_)
            | ServerNotification::ProcessExited(_)
            | ServerNotification::FileChangePatchUpdated(_)
            | ServerNotification::McpToolCallProgress(_)
            | ServerNotification::McpServerOauthLoginCompleted(_)
            | ServerNotification::AppListUpdated(_)
            | ServerNotification::RemoteControlStatusChanged(_)
            | ServerNotification::ExternalAgentConfigImportProgress(_)
            | ServerNotification::ExternalAgentConfigImportCompleted(_)
            | ServerNotification::FsChanged(_)
            | ServerNotification::ModelSafetyBufferingUpdated(_)
            | ServerNotification::InteractionModerationMetadata(_)
            | ServerNotification::FuzzyFileSearchSessionUpdated(_)
            | ServerNotification::FuzzyFileSearchSessionCompleted(_)
            | ServerNotification::ChatRealtimeStarted(_)
            | ServerNotification::ChatRealtimeMessageAdded(_)
            | ServerNotification::ChatRealtimeOutputAudioDelta(_)
            | ServerNotification::ChatRealtimeError(_)
            | ServerNotification::ChatRealtimeClosed(_)
            | ServerNotification::ChatRealtimeSdp(_)
            | ServerNotification::ChatRealtimeTranscriptDelta(_)
            | ServerNotification::ChatRealtimeTranscriptDone(_)
            | ServerNotification::WindowsWorldWritableWarning(_)
            | ServerNotification::WindowsSandboxSetupCompleted(_)
            | ServerNotification::AccountLoginCompleted(_) => {}
            ServerNotification::ContextCompacted(_) => {}
        }
    }

    pub(super) fn handle_turn_completed_notification(
        &mut self,
        notification: InteractionCompletedNotification,
        replay_kind: Option<ReplayKind>,
    ) {
        // User-message dedupe only suppresses the app-server echo of a prompt
        // this TUI already rendered locally. Once that turn ends, another
        // client can submit the same text and it still needs its own user cell.
        self.last_rendered_user_message_display = None;
        match notification.turn.status {
            InteractionStatus::Completed => {
                self.last_non_retry_error = None;
                self.on_task_complete(
                    /*last_agent_message*/ None,
                    notification.turn.duration_ms,
                    replay_kind.is_some(),
                )
            }
            InteractionStatus::Interrupted => {
                self.last_non_retry_error = None;
                let reason = if self
                    .turn_lifecycle
                    .take_budget_limited(notification.turn.id.as_str())
                {
                    InteractionAbortReason::BudgetLimited
                } else {
                    InteractionAbortReason::Interrupted
                };
                self.on_interrupted_turn(reason);
            }
            InteractionStatus::Failed => {
                if let Some(error) = notification.turn.error {
                    if self.last_non_retry_error.as_ref()
                        == Some(&(notification.turn.id.clone(), error.message.clone()))
                    {
                        self.last_non_retry_error = None;
                    } else {
                        self.handle_non_retry_error(error.message, error.codex_error_info);
                    }
                } else {
                    self.last_non_retry_error = None;
                    self.finalize_turn();
                    self.request_redraw();
                    self.maybe_send_next_queued_input();
                }
            }
            InteractionStatus::InProgress => {}
        }
    }

    fn handle_item_started_notification(
        &mut self,
        notification: MessageStartedNotification,
        from_replay: bool,
    ) {
        match notification.item {
            item @ Message::CommandExecution { .. } => self.on_command_execution_started(item),
            Message::FileChange { id: _, changes, .. } => {
                self.on_patch_apply_begin(file_update_changes_to_display(changes));
            }
            item @ Message::McpToolCall { .. } => self.on_mcp_tool_call_started(item),
            Message::WebSearch { id, .. } => {
                self.on_web_search_begin(id);
            }
            Message::ImageGeneration { .. } => {
                self.on_image_generation_begin();
            }
            Message::CollabAgentToolCall {
                id,
                tool,
                status,
                sender_chat_id,
                receiver_chat_ids,
                prompt,
                model,
                reasoning_effort,
                agents_states,
            } => self.on_collab_agent_tool_call(Message::CollabAgentToolCall {
                id,
                tool,
                status,
                sender_chat_id,
                receiver_chat_ids,
                prompt,
                model,
                reasoning_effort,
                agents_states,
            }),
            item @ Message::SubAgentActivity { .. } => self.on_sub_agent_activity(item),
            Message::EnteredReviewMode { review, .. } if !from_replay => {
                self.enter_review_mode_with_hint(review, /*from_replay*/ false);
            }
            _ => {}
        }
    }

    fn handle_item_completed_notification(
        &mut self,
        notification: MessageCompletedNotification,
        replay_kind: Option<ReplayKind>,
    ) {
        self.handle_thread_item(
            notification.item,
            notification.interaction_id,
            replay_kind.map_or(ThreadItemRenderSource::Live, ThreadItemRenderSource::Replay),
        );
    }
}
