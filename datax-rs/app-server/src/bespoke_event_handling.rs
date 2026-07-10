use crate::error_code::internal_error;
use crate::error_code::invalid_request;
use crate::outgoing_message::ClientRequestResult;
use crate::outgoing_message::ThreadScopedOutgoingMessageSender;
use crate::request_processors::chat_from_stored_chat;
use crate::request_processors::populate_chat_interactions_from_history;
use crate::request_processors::chat_settings_from_core_snapshot;
use crate::server_request_error::is_turn_transition_server_request_error;
use crate::chat_state::ChatState;
use crate::chat_state::InteractionSummary;
use crate::chat_state::resolve_server_request_on_chat_listener;
use crate::chat_status::ChatWatchActiveGuard;
use crate::chat_status::ChatWatchManager;
use datax_app_server_protocol::AccountRateLimitsUpdatedNotification;
use datax_app_server_protocol::AdditionalPermissionProfile as V2AdditionalPermissionProfile;
use datax_app_server_protocol::ChatGoalUpdatedNotification;
use datax_app_server_protocol::ChatRealtimeClosedNotification;
use datax_app_server_protocol::ChatRealtimeErrorNotification;
use datax_app_server_protocol::ChatRealtimeMessageAddedNotification;
use datax_app_server_protocol::ChatRealtimeOutputAudioDeltaNotification;
use datax_app_server_protocol::ChatRealtimeSdpNotification;
use datax_app_server_protocol::ChatRealtimeStartedNotification;
use datax_app_server_protocol::ChatRealtimeTranscriptDeltaNotification;
use datax_app_server_protocol::ChatRealtimeTranscriptDoneNotification;
use datax_app_server_protocol::ChatRollbackResponse;
use datax_app_server_protocol::ChatSettingsUpdatedNotification;
use datax_app_server_protocol::ChatStatus;
use datax_app_server_protocol::ChatTokenUsage;
use datax_app_server_protocol::ChatTokenUsageUpdatedNotification;
use datax_app_server_protocol::CodexErrorInfo as V2CodexErrorInfo;
use datax_app_server_protocol::CommandAction as V2ParsedCommand;
use datax_app_server_protocol::CommandExecutionApprovalDecision;
use datax_app_server_protocol::CommandExecutionRequestApprovalParams;
use datax_app_server_protocol::CommandExecutionRequestApprovalResponse;
use datax_app_server_protocol::CommandExecutionSource;
use datax_app_server_protocol::CommandExecutionStatus;
use datax_app_server_protocol::DeprecationNoticeNotification;
use datax_app_server_protocol::DynamicToolCallParams;
use datax_app_server_protocol::DynamicToolCallStatus;
use datax_app_server_protocol::ErrorNotification;
use datax_app_server_protocol::ExecPolicyAmendment as V2ExecPolicyAmendment;
use datax_app_server_protocol::FileChangeApprovalDecision;
use datax_app_server_protocol::FileChangeRequestApprovalParams;
use datax_app_server_protocol::FileChangeRequestApprovalResponse;
use datax_app_server_protocol::GrantedPermissionProfile as V2GrantedPermissionProfile;
use datax_app_server_protocol::GuardianWarningNotification;
use datax_app_server_protocol::HookCompletedNotification;
use datax_app_server_protocol::HookStartedNotification;
use datax_app_server_protocol::Interaction;
use datax_app_server_protocol::InteractionCompletedNotification;
use datax_app_server_protocol::InteractionDiffUpdatedNotification;
use datax_app_server_protocol::InteractionError;
use datax_app_server_protocol::InteractionInterruptResponse;
use datax_app_server_protocol::InteractionMessagesView;
use datax_app_server_protocol::InteractionModerationMetadataNotification;
use datax_app_server_protocol::InteractionPlanStep;
use datax_app_server_protocol::InteractionPlanUpdatedNotification;
use datax_app_server_protocol::InteractionStartedNotification;
use datax_app_server_protocol::InteractionStatus;
use datax_app_server_protocol::McpServerElicitationAction;
use datax_app_server_protocol::McpServerElicitationRequestParams;
use datax_app_server_protocol::McpServerElicitationRequestResponse;
use datax_app_server_protocol::McpServerStartupState;
use datax_app_server_protocol::McpServerStatusUpdatedNotification;
use datax_app_server_protocol::Message;
use datax_app_server_protocol::MessageCompletedNotification;
use datax_app_server_protocol::MessageStartedNotification;
use datax_app_server_protocol::ModelReroutedNotification;
use datax_app_server_protocol::ModelSafetyBufferingUpdatedNotification;
use datax_app_server_protocol::ModelVerificationNotification;
use datax_app_server_protocol::NetworkApprovalContext as V2NetworkApprovalContext;
use datax_app_server_protocol::NetworkPolicyAmendment as V2NetworkPolicyAmendment;
use datax_app_server_protocol::NetworkPolicyRuleAction as V2NetworkPolicyRuleAction;
use datax_app_server_protocol::PermissionsRequestApprovalParams;
use datax_app_server_protocol::PermissionsRequestApprovalResponse;
use datax_app_server_protocol::RawResponseMessageCompletedNotification;
use datax_app_server_protocol::RequestId;
use datax_app_server_protocol::ServerNotification;
use datax_app_server_protocol::ServerNotification::*;
use datax_app_server_protocol::ServerRequestPayload;
use datax_app_server_protocol::ToolRequestUserInputOption;
use datax_app_server_protocol::ToolRequestUserInputParams;
use datax_app_server_protocol::ToolRequestUserInputQuestion;
use datax_app_server_protocol::ToolRequestUserInputResponse;
use datax_app_server_protocol::WarningNotification;
use datax_app_server_protocol::build_item_from_guardian_event;
use datax_app_server_protocol::guardian_auto_approval_review_notification;
use datax_app_server_protocol::item_event_to_server_notification;
use datax_core::ChatManager;
use datax_core::DataxChat;
use datax_core::review_format::format_review_findings_block;
use datax_core::review_prompts;
use datax_protocol::ChatId;
use datax_protocol::items::parse_hook_prompt_message;
use datax_protocol::models::AdditionalPermissionProfile as CoreAdditionalPermissionProfile;
use datax_protocol::plan_tool::UpdatePlanArgs;
use datax_protocol::protocol::CodexErrorInfo as CoreCodexErrorInfo;
use datax_protocol::protocol::Event;
use datax_protocol::protocol::EventMsg;
use datax_protocol::protocol::ExecApprovalRequestEvent;
use datax_protocol::protocol::InteractionAbortedEvent;
use datax_protocol::protocol::InteractionCompleteEvent;
use datax_protocol::protocol::Op;
use datax_protocol::protocol::RealtimeEvent;
use datax_protocol::protocol::ReviewDecision;
use datax_protocol::protocol::ReviewOutputEvent;
use datax_protocol::protocol::SubAgentActivityKind;
use datax_protocol::protocol::TokenCountEvent;
use datax_protocol::protocol::InteractionDiffEvent;
use datax_protocol::request_permissions::PermissionGrantScope as CorePermissionGrantScope;
use datax_protocol::request_permissions::RequestPermissionProfile as CoreRequestPermissionProfile;
use datax_protocol::request_permissions::RequestPermissionsResponse as CoreRequestPermissionsResponse;
use datax_protocol::request_user_input::RequestUserInputAnswer as CoreRequestUserInputAnswer;
use datax_protocol::request_user_input::RequestUserInputResponse as CoreRequestUserInputResponse;
use datax_sandboxing::policy_transforms::intersect_permission_profiles;
use datax_shell_command::parse_command::shlex_join;
use datax_utils_absolute_path::AbsolutePathBuf;
use datax_utils_path_uri::LegacyAppPathString;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tokio::sync::Mutex;
use tokio::sync::oneshot;
use tracing::error;

enum CommandExecutionApprovalPresentation {
    Network(V2NetworkApprovalContext),
    Command(CommandExecutionCompletionItem),
}

#[derive(Debug, PartialEq)]
struct CommandExecutionCompletionItem {
    command: String,
    cwd: LegacyAppPathString,
    command_actions: Vec<V2ParsedCommand>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn apply_bespoke_event_handling(
    event: Event,
    conversation_id: ChatId,
    conversation: Arc<DataxChat>,
    chat_manager: Arc<ChatManager>,
    outgoing: ThreadScopedOutgoingMessageSender,
    chat_state: Arc<tokio::sync::Mutex<ChatState>>,
    chat_watch_manager: ChatWatchManager,
    thread_list_state_permit: Arc<tokio::sync::Semaphore>,
    fallback_model_provider: String,
) {
    let Event {
        id: event_interaction_id,
        msg,
    } = event;
    match msg {
        EventMsg::InteractionStarted(payload) => {
            // While not technically necessary as it was already done on InteractionComplete, be extra cautios and abort any pending server requests.
            outgoing.abort_pending_server_requests().await;
            chat_watch_manager
                .note_interaction_started(&conversation_id.to_string())
                .await;
            let turn = {
                let state = chat_state.lock().await;
                let mut turn = state.active_interaction_snapshot().unwrap_or_else(|| Interaction {
                    id: payload.interaction_id.clone(),
                    messages: Vec::new(),
                    messages_view: InteractionMessagesView::NotLoaded,
                    error: None,
                    status: InteractionStatus::InProgress,
                    started_at: payload.started_at,
                    completed_at: None,
                    duration_ms: None,
                });
                turn.messages.clear();
                turn.messages_view = InteractionMessagesView::NotLoaded;
                turn
            };
            let notification = InteractionStartedNotification {
                chat_id: conversation_id.to_string(),
                interaction: turn,
            };
            outgoing
                .send_server_notification(InteractionStarted(notification))
                .await;
        }
        EventMsg::InteractionComplete(turn_complete_event) => {
            // All per-chat requests are bound to a turn, so abort them.
            outgoing.abort_pending_server_requests().await;
            respond_to_pending_interrupts(&chat_state, &outgoing).await;
            let turn_failed = chat_state.lock().await.interaction_summary.last_error.is_some();
            chat_watch_manager
                .note_interaction_completed(&conversation_id.to_string(), turn_failed)
                .await;
            handle_turn_complete(
                conversation_id,
                event_interaction_id,
                turn_complete_event,
                &outgoing,
                &chat_state,
            )
            .await;
        }
        EventMsg::McpStartupUpdate(update) => {
            let (status, error) = match update.status {
                datax_protocol::protocol::McpStartupStatus::Starting => {
                    (McpServerStartupState::Starting, None)
                }
                datax_protocol::protocol::McpStartupStatus::Ready => {
                    (McpServerStartupState::Ready, None)
                }
                datax_protocol::protocol::McpStartupStatus::Failed { error } => {
                    (McpServerStartupState::Failed, Some(error))
                }
                datax_protocol::protocol::McpStartupStatus::Cancelled => {
                    (McpServerStartupState::Cancelled, None)
                }
            };
            let notification = McpServerStatusUpdatedNotification {
                chat_id: Some(conversation_id.to_string()),
                name: update.server,
                status,
                error,
            };
            outgoing
                .send_server_notification(ServerNotification::McpServerStatusUpdated(notification))
                .await;
        }
        EventMsg::Warning(warning_event) => {
            let notification = WarningNotification {
                chat_id: Some(conversation_id.to_string()),
                message: warning_event.message,
            };
            outgoing
                .send_server_notification(ServerNotification::Warning(notification))
                .await;
        }
        EventMsg::GuardianWarning(warning_event) => {
            let notification = GuardianWarningNotification {
                chat_id: conversation_id.to_string(),
                message: warning_event.message,
            };
            outgoing
                .send_server_notification(ServerNotification::GuardianWarning(notification))
                .await;
        }
        EventMsg::GuardianAssessment(assessment) => {
            let pending_command_execution = match build_item_from_guardian_event(
                &assessment,
                CommandExecutionStatus::InProgress,
            ) {
                Some(Message::CommandExecution {
                    id,
                    command,
                    cwd,
                    command_actions,
                    ..
                }) => Some((
                    id,
                    CommandExecutionCompletionItem {
                        command,
                        cwd,
                        command_actions,
                    },
                )),
                Some(_) | None => None,
            };
            let assessment_interaction_id = if assessment.interaction_id.is_empty() {
                event_interaction_id.clone()
            } else {
                assessment.interaction_id.clone()
            };
            if assessment.status == datax_protocol::protocol::GuardianAssessmentStatus::InProgress
                && let Some((target_message_id, completion_item)) =
                    pending_command_execution.as_ref()
            {
                start_command_execution_item(
                    &conversation_id,
                    assessment_interaction_id.clone(),
                    target_message_id.clone(),
                    completion_item.command.clone(),
                    completion_item.cwd.clone(),
                    completion_item.command_actions.clone(),
                    CommandExecutionSource::Agent,
                    &outgoing,
                    &chat_state,
                )
                .await;
            }
            let notification = guardian_auto_approval_review_notification(
                &conversation_id,
                &event_interaction_id,
                &assessment,
            );
            outgoing.send_server_notification(notification).await;
            let completion_status = match assessment.status {
                datax_protocol::protocol::GuardianAssessmentStatus::Denied
                | datax_protocol::protocol::GuardianAssessmentStatus::Aborted => {
                    Some(CommandExecutionStatus::Declined)
                }
                datax_protocol::protocol::GuardianAssessmentStatus::TimedOut => {
                    Some(CommandExecutionStatus::Failed)
                }
                datax_protocol::protocol::GuardianAssessmentStatus::InProgress
                | datax_protocol::protocol::GuardianAssessmentStatus::Approved => None,
            };
            if let Some(completion_status) = completion_status
                && let Some((target_message_id, completion_item)) = pending_command_execution
            {
                complete_command_execution_item(
                    &conversation_id,
                    assessment_interaction_id,
                    target_message_id,
                    completion_item.command,
                    completion_item.cwd,
                    /*process_id*/ None,
                    CommandExecutionSource::Agent,
                    completion_item.command_actions,
                    completion_status,
                    &outgoing,
                    &chat_state,
                )
                .await;
            }
        }
        EventMsg::ModelReroute(event) => {
            let notification = ModelReroutedNotification {
                chat_id: conversation_id.to_string(),
                interaction_id: event_interaction_id.clone(),
                from_model: event.from_model,
                to_model: event.to_model,
                reason: event.reason.into(),
            };
            outgoing
                .send_server_notification(ServerNotification::ModelRerouted(notification))
                .await;
        }
        EventMsg::ModelVerification(event) => {
            let notification = ModelVerificationNotification {
                chat_id: conversation_id.to_string(),
                interaction_id: event_interaction_id.clone(),
                verifications: event.verifications.into_iter().map(Into::into).collect(),
            };
            outgoing
                .send_server_notification(ServerNotification::ModelVerification(notification))
                .await;
        }
        EventMsg::TurnModerationMetadata(event) => {
            let notification = InteractionModerationMetadataNotification {
                chat_id: conversation_id.to_string(),
                interaction_id: event_interaction_id.clone(),
                metadata: event.metadata,
            };
            outgoing
                .send_server_notification(ServerNotification::InteractionModerationMetadata(
                    notification,
                ))
                .await;
        }
        EventMsg::SafetyBuffering(event) => {
            let notification = ModelSafetyBufferingUpdatedNotification {
                chat_id: conversation_id.to_string(),
                interaction_id: event_interaction_id.clone(),
                model: event.model,
                use_cases: event.use_cases,
                reasons: event.reasons,
                show_buffering_ui: event.show_buffering_ui,
                faster_model: event.faster_model,
            };
            outgoing
                .send_server_notification(ServerNotification::ModelSafetyBufferingUpdated(
                    notification,
                ))
                .await;
        }
        EventMsg::RealtimeConversationStarted(event) => {
            let notification = ChatRealtimeStartedNotification {
                chat_id: conversation_id.to_string(),
                realtime_session_id: event.realtime_session_id,
                version: event.version,
            };
            outgoing
                .send_server_notification(ServerNotification::ChatRealtimeStarted(notification))
                .await;
        }
        EventMsg::RealtimeConversationSdp(event) => {
            let notification = ChatRealtimeSdpNotification {
                chat_id: conversation_id.to_string(),
                sdp: event.sdp,
            };
            outgoing
                .send_server_notification(ServerNotification::ChatRealtimeSdp(notification))
                .await;
        }
        EventMsg::RealtimeConversationRealtime(event) => match event.payload {
            RealtimeEvent::SessionUpdated { .. } => {}
            RealtimeEvent::InputAudioSpeechStarted(event) => {
                let notification = ChatRealtimeMessageAddedNotification {
                    chat_id: conversation_id.to_string(),
                    item: serde_json::json!({
                        "type": "input_audio_buffer.speech_started",
                        "message_id": event.item_id,
                    }),
                };
                outgoing
                    .send_server_notification(ServerNotification::ChatRealtimeMessageAdded(
                        notification,
                    ))
                    .await;
            }
            RealtimeEvent::InputTranscriptDelta(event) => {
                let notification = ChatRealtimeTranscriptDeltaNotification {
                    chat_id: conversation_id.to_string(),
                    role: "user".to_string(),
                    delta: event.delta,
                };
                outgoing
                    .send_server_notification(ServerNotification::ChatRealtimeTranscriptDelta(
                        notification,
                    ))
                    .await;
            }
            RealtimeEvent::InputTranscriptDone(event) => {
                let notification = ChatRealtimeTranscriptDoneNotification {
                    chat_id: conversation_id.to_string(),
                    role: "user".to_string(),
                    text: event.text,
                };
                outgoing
                    .send_server_notification(ServerNotification::ChatRealtimeTranscriptDone(
                        notification,
                    ))
                    .await;
            }
            RealtimeEvent::OutputTranscriptDelta(event) => {
                let notification = ChatRealtimeTranscriptDeltaNotification {
                    chat_id: conversation_id.to_string(),
                    role: "assistant".to_string(),
                    delta: event.delta,
                };
                outgoing
                    .send_server_notification(ServerNotification::ChatRealtimeTranscriptDelta(
                        notification,
                    ))
                    .await;
            }
            RealtimeEvent::OutputTranscriptDone(event) => {
                let notification = ChatRealtimeTranscriptDoneNotification {
                    chat_id: conversation_id.to_string(),
                    role: "assistant".to_string(),
                    text: event.text,
                };
                outgoing
                    .send_server_notification(ServerNotification::ChatRealtimeTranscriptDone(
                        notification,
                    ))
                    .await;
            }
            RealtimeEvent::AudioOut(audio) => {
                let notification = ChatRealtimeOutputAudioDeltaNotification {
                    chat_id: conversation_id.to_string(),
                    audio: audio.into(),
                };
                outgoing
                    .send_server_notification(ServerNotification::ChatRealtimeOutputAudioDelta(
                        notification,
                    ))
                    .await;
            }
            RealtimeEvent::ResponseCreated(_) => {}
            RealtimeEvent::ResponseCancelled(event) => {
                let notification = ChatRealtimeMessageAddedNotification {
                    chat_id: conversation_id.to_string(),
                    item: serde_json::json!({
                        "type": "response.cancelled",
                        "response_id": event.response_id,
                    }),
                };
                outgoing
                    .send_server_notification(ServerNotification::ChatRealtimeMessageAdded(
                        notification,
                    ))
                    .await;
            }
            RealtimeEvent::ResponseDone(_) => {}
            RealtimeEvent::ConversationItemAdded(item) => {
                let notification = ChatRealtimeMessageAddedNotification {
                    chat_id: conversation_id.to_string(),
                    item,
                };
                outgoing
                    .send_server_notification(ServerNotification::ChatRealtimeMessageAdded(
                        notification,
                    ))
                    .await;
            }
            RealtimeEvent::ConversationItemDone { .. } | RealtimeEvent::NoopRequested(_) => {}
            RealtimeEvent::HandoffRequested(handoff) => {
                let notification = ChatRealtimeMessageAddedNotification {
                    chat_id: conversation_id.to_string(),
                    item: serde_json::json!({
                        "type": "handoff_request",
                        "handoff_id": handoff.handoff_id,
                        "message_id": handoff.item_id,
                        "input_transcript": handoff.input_transcript,
                        "active_transcript": handoff.active_transcript,
                    }),
                };
                outgoing
                    .send_server_notification(ServerNotification::ChatRealtimeMessageAdded(
                        notification,
                    ))
                    .await;
            }
            RealtimeEvent::Error(message) => {
                let notification = ChatRealtimeErrorNotification {
                    chat_id: conversation_id.to_string(),
                    message,
                };
                outgoing
                    .send_server_notification(ServerNotification::ChatRealtimeError(notification))
                    .await;
            }
        },
        EventMsg::RealtimeConversationClosed(event) => {
            let notification = ChatRealtimeClosedNotification {
                chat_id: conversation_id.to_string(),
                reason: event.reason,
            };
            outgoing
                .send_server_notification(ServerNotification::ChatRealtimeClosed(notification))
                .await;
        }
        EventMsg::ApplyPatchApprovalRequest(event) => {
            let permission_guard = chat_watch_manager
                .note_permission_requested(&conversation_id.to_string())
                .await;
            let message_id = event.call_id.clone();

            let params = FileChangeRequestApprovalParams {
                chat_id: conversation_id.to_string(),
                interaction_id: event.interaction_id.clone(),
                message_id: message_id.clone(),
                started_at_ms: event.started_at_ms,
                reason: event.reason.clone(),
                grant_root: event.grant_root.clone(),
            };
            let (pending_request_id, rx) = outgoing
                .send_request(ServerRequestPayload::FileChangeRequestApproval(params))
                .await;
            tokio::spawn(async move {
                on_file_change_request_approval_response(
                    message_id,
                    pending_request_id,
                    rx,
                    conversation,
                    chat_state.clone(),
                    permission_guard,
                )
                .await;
            });
        }
        EventMsg::ExecApprovalRequest(ev) => {
            let permission_guard = chat_watch_manager
                .note_permission_requested(&conversation_id.to_string())
                .await;
            let available_decisions = ev
                .effective_available_decisions()
                .into_iter()
                .map(CommandExecutionApprovalDecision::from)
                .collect::<Vec<_>>();
            let ExecApprovalRequestEvent {
                call_id,
                approval_id,
                interaction_id: interaction_id,
                environment_id,
                started_at_ms,
                command,
                cwd,
                reason,
                network_approval_context,
                proposed_execpolicy_amendment,
                proposed_network_policy_amendments,
                additional_permissions,
                parsed_cmd,
                ..
            } = ev;
            let command_actions = parsed_cmd
                .iter()
                .cloned()
                .map(|parsed| V2ParsedCommand::from_core_with_cwd(parsed, &cwd))
                .collect::<Vec<_>>();
            let presentation = if let Some(network_approval_context) =
                network_approval_context.map(V2NetworkApprovalContext::from)
            {
                CommandExecutionApprovalPresentation::Network(network_approval_context)
            } else {
                let command_string = shlex_join(&command);
                let completion_item = CommandExecutionCompletionItem {
                    command: command_string,
                    cwd: cwd.clone().into(),
                    command_actions: command_actions.clone(),
                };
                CommandExecutionApprovalPresentation::Command(completion_item)
            };
            let (network_approval_context, command, cwd, command_actions, completion_item) =
                match presentation {
                    CommandExecutionApprovalPresentation::Network(network_approval_context) => {
                        (Some(network_approval_context), None, None, None, None)
                    }
                    CommandExecutionApprovalPresentation::Command(completion_item) => (
                        None,
                        Some(completion_item.command.clone()),
                        Some(completion_item.cwd.clone()),
                        Some(completion_item.command_actions.clone()),
                        Some(completion_item),
                    ),
                };
            if approval_id.is_none()
                && let Some(completion_item) = completion_item.as_ref()
            {
                start_command_execution_item(
                    &conversation_id,
                    event_interaction_id.clone(),
                    call_id.clone(),
                    completion_item.command.clone(),
                    completion_item.cwd.clone(),
                    completion_item.command_actions.clone(),
                    CommandExecutionSource::Agent,
                    &outgoing,
                    &chat_state,
                )
                .await;
            }
            let proposed_execpolicy_amendment_v2 =
                proposed_execpolicy_amendment.map(V2ExecPolicyAmendment::from);
            let proposed_network_policy_amendments_v2 =
                proposed_network_policy_amendments.map(|amendments| {
                    amendments
                        .into_iter()
                        .map(V2NetworkPolicyAmendment::from)
                        .collect()
                });
            let additional_permissions =
                additional_permissions.map(V2AdditionalPermissionProfile::from);

            let params = CommandExecutionRequestApprovalParams {
                chat_id: conversation_id.to_string(),
                interaction_id: interaction_id.clone(),
                message_id: call_id.clone(),
                started_at_ms,
                approval_id: approval_id.clone(),
                environment_id,
                reason,
                network_approval_context,
                command,
                cwd,
                command_actions,
                additional_permissions,
                proposed_execpolicy_amendment: proposed_execpolicy_amendment_v2,
                proposed_network_policy_amendments: proposed_network_policy_amendments_v2,
                available_decisions: Some(available_decisions),
            };
            let (pending_request_id, rx) = outgoing
                .send_request(ServerRequestPayload::CommandExecutionRequestApproval(
                    params,
                ))
                .await;
            tokio::spawn(async move {
                on_command_execution_request_approval_response(
                    event_interaction_id,
                    conversation_id,
                    approval_id,
                    call_id,
                    completion_item,
                    pending_request_id,
                    rx,
                    conversation,
                    outgoing,
                    chat_state.clone(),
                    permission_guard,
                )
                .await;
            });
        }
        EventMsg::RequestUserInput(request) => {
            let user_input_guard = chat_watch_manager
                .note_user_input_requested(&conversation_id.to_string())
                .await;
            let questions = request
                .questions
                .into_iter()
                .map(|question| ToolRequestUserInputQuestion {
                    id: question.id,
                    header: question.header,
                    question: question.question,
                    is_other: question.is_other,
                    is_secret: question.is_secret,
                    options: question.options.map(|options| {
                        options
                            .into_iter()
                            .map(|option| ToolRequestUserInputOption {
                                label: option.label,
                                description: option.description,
                            })
                            .collect()
                    }),
                })
                .collect();
            let params = ToolRequestUserInputParams {
                chat_id: conversation_id.to_string(),
                interaction_id: request.interaction_id,
                message_id: request.call_id,
                questions,
                auto_resolution_ms: request.auto_resolution_ms,
            };
            let (pending_request_id, rx) = outgoing
                .send_request(ServerRequestPayload::ToolRequestUserInput(params))
                .await;
            tokio::spawn(async move {
                on_request_user_input_response(
                    event_interaction_id,
                    pending_request_id,
                    rx,
                    conversation,
                    chat_state,
                    user_input_guard,
                )
                .await;
            });
        }
        EventMsg::ElicitationRequest(request) => {
            let permission_guard = chat_watch_manager
                .note_permission_requested(&conversation_id.to_string())
                .await;
            let interaction_id = match request.interaction_id.clone() {
                Some(interaction_id) => Some(interaction_id),
                None => {
                    let state = chat_state.lock().await;
                    state.active_interaction_snapshot().map(|turn| turn.id)
                }
            };
            let server_name = request.server_name.clone();
            let request_body = match request.request.try_into() {
                Ok(request_body) => request_body,
                Err(err) => {
                    error!(
                        error = %err,
                        server_name,
                        request_id = ?request.id,
                        "failed to parse typed MCP elicitation schema"
                    );
                    if let Err(err) = conversation
                        .submit(Op::ResolveElicitation {
                            server_name: request.server_name,
                            request_id: request.id,
                            decision: datax_protocol::approvals::ElicitationAction::Cancel,
                            content: None,
                            meta: None,
                        })
                        .await
                    {
                        error!("failed to submit ResolveElicitation: {err}");
                    }
                    return;
                }
            };
            let params = McpServerElicitationRequestParams {
                chat_id: conversation_id.to_string(),
                interaction_id,
                server_name: request.server_name.clone(),
                request: request_body,
            };
            let (pending_request_id, rx) = outgoing
                .send_request(ServerRequestPayload::McpServerElicitationRequest(params))
                .await;
            tokio::spawn(async move {
                on_mcp_server_elicitation_response(
                    request.server_name,
                    request.id,
                    pending_request_id,
                    rx,
                    conversation,
                    chat_state,
                    permission_guard,
                )
                .await;
            });
        }
        EventMsg::RequestPermissions(request) => {
            let permission_guard = chat_watch_manager
                .note_permission_requested(&conversation_id.to_string())
                .await;
            let requested_permissions = request.permissions.clone();
            let request_cwd = match request.cwd.clone() {
                Some(cwd) => cwd,
                None => conversation.config_snapshot().await.cwd().clone(),
            };
            let params = PermissionsRequestApprovalParams {
                chat_id: conversation_id.to_string(),
                interaction_id: request.interaction_id.clone(),
                message_id: request.call_id.clone(),
                environment_id: request.environment_id.clone(),
                started_at_ms: request.started_at_ms,
                cwd: request_cwd.clone(),
                reason: request.reason,
                permissions: request.permissions.into(),
            };
            let (pending_request_id, rx) = outgoing
                .send_request(ServerRequestPayload::PermissionsRequestApproval(params))
                .await;
            let pending_response = PendingRequestPermissionsResponse {
                call_id: request.call_id,
                conversation_id,
                interaction_id: request.interaction_id,
                requested_permissions,
                request_cwd,
                pending_request_id,
                outgoing,
                receiver: rx,
                request_permissions_guard: permission_guard,
            };
            tokio::spawn(async move {
                on_request_permissions_response(pending_response, conversation, chat_state).await;
            });
        }
        EventMsg::DynamicToolCallRequest(request) => {
            let call_id = request.call_id;
            let interaction_id = request.interaction_id;
            let namespace = request.namespace;
            let tool = request.tool;
            let arguments = request.arguments;
            let item = Message::DynamicToolCall {
                id: call_id.clone(),
                namespace: namespace.clone(),
                tool: tool.clone(),
                arguments: arguments.clone(),
                status: DynamicToolCallStatus::InProgress,
                content_items: None,
                success: None,
                duration_ms: None,
            };
            let notification = MessageStartedNotification {
                chat_id: conversation_id.to_string(),
                interaction_id: interaction_id.clone(),
                started_at_ms: request.started_at_ms,
                item,
            };
            outgoing
                .send_server_notification(MessageStarted(notification))
                .await;
            let params = DynamicToolCallParams {
                chat_id: conversation_id.to_string(),
                interaction_id: interaction_id.clone(),
                call_id: call_id.clone(),
                namespace,
                tool: tool.clone(),
                arguments: arguments.clone(),
            };
            let (_pending_request_id, rx) = outgoing
                .send_request(ServerRequestPayload::DynamicToolCall(params))
                .await;
            tokio::spawn(async move {
                crate::dynamic_tools::on_call_response(call_id, rx, conversation).await;
            });
        }
        EventMsg::McpToolCallBegin(_) | EventMsg::McpToolCallEnd(_) => {
            // Deprecated MCP tool-call events are still fanned out for legacy clients.
            // App-server v2 receives the canonical InteractionMessage::McpToolCall lifecycle instead.
        }
        msg @ (EventMsg::DynamicToolCallResponse(_)
        | EventMsg::CollabAgentSpawnBegin(_)
        | EventMsg::CollabAgentSpawnEnd(_)
        | EventMsg::CollabAgentInteractionBegin(_)
        | EventMsg::CollabAgentInteractionEnd(_)
        | EventMsg::CollabWaitingBegin(_)
        | EventMsg::CollabWaitingEnd(_)
        | EventMsg::CollabCloseBegin(_)
        | EventMsg::CollabResumeBegin(_)
        | EventMsg::CollabResumeEnd(_)
        | EventMsg::AgentMessageContentDelta(_)
        | EventMsg::PlanDelta(_)
        | EventMsg::ReasoningContentDelta(_)
        | EventMsg::ReasoningRawContentDelta(_)
        | EventMsg::AgentReasoningSectionBreak(_)) => {
            let notification = item_event_to_server_notification(
                msg,
                &conversation_id.to_string(),
                &event_interaction_id,
            );
            outgoing.send_server_notification(notification).await;
        }
        EventMsg::SubAgentActivity(activity) => {
            if activity.kind == SubAgentActivityKind::Interrupted
                && chat_manager.get_chat(activity.agent_chat_id).await.is_err()
            {
                chat_watch_manager
                    .remove_chat(&activity.agent_chat_id.to_string())
                    .await;
            }
            let notification = item_event_to_server_notification(
                EventMsg::SubAgentActivity(activity),
                &conversation_id.to_string(),
                &event_interaction_id,
            );
            outgoing.send_server_notification(notification).await;
        }
        EventMsg::CollabCloseEnd(end_event) => {
            if chat_manager
                .get_chat(end_event.receiver_chat_id)
                .await
                .is_err()
            {
                chat_watch_manager
                    .remove_chat(&end_event.receiver_chat_id.to_string())
                    .await;
            }
            let notification = item_event_to_server_notification(
                EventMsg::CollabCloseEnd(end_event),
                &conversation_id.to_string(),
                &event_interaction_id,
            );
            outgoing.send_server_notification(notification).await;
        }
        EventMsg::ContextCompacted(..) => {
            // Core still fans out this deprecated event for legacy clients;
            // v2 clients receive the canonical ContextCompaction item instead.
        }
        EventMsg::DeprecationNotice(event) => {
            let notification = DeprecationNoticeNotification {
                summary: event.summary,
                details: event.details,
            };
            outgoing
                .send_server_notification(ServerNotification::DeprecationNotice(notification))
                .await;
        }
        EventMsg::TokenCount(token_count_event) => {
            handle_token_count_event(
                conversation_id,
                event_interaction_id,
                token_count_event,
                &outgoing,
            )
            .await;
        }
        EventMsg::Error(ev) => {
            chat_watch_manager
                .note_system_error(&conversation_id.to_string())
                .await;

            let message = ev.message.clone();
            let codex_error_info = ev.codex_error_info.clone();
            // If this error belongs to an in-flight `chat/rollback` request, fail that request
            // (and clear pending state) so subsequent rollbacks are unblocked.
            //
            // Don't send a notification for this error.
            if matches!(
                codex_error_info,
                Some(CoreCodexErrorInfo::ThreadRollbackFailed)
            ) {
                return handle_thread_rollback_failed(
                    conversation_id,
                    message,
                    &chat_state,
                    &outgoing,
                )
                .await;
            };

            if !ev.affects_turn_status() {
                return;
            }

            let turn_error = InteractionError {
                message: ev.message,
                codex_error_info: ev.codex_error_info.map(V2CodexErrorInfo::from),
                additional_details: None,
            };
            handle_error_notification(
                conversation_id,
                &event_interaction_id,
                turn_error,
                &outgoing,
                &chat_state,
            )
            .await;
        }
        EventMsg::StreamError(ev) => {
            // We don't need to update the turn summary store for stream errors as they are intermediate error states for retries,
            // but we notify the client.
            let turn_error = InteractionError {
                message: ev.message,
                codex_error_info: ev.codex_error_info.map(V2CodexErrorInfo::from),
                additional_details: ev.additional_details,
            };
            outgoing
                .send_server_notification(ServerNotification::Error(ErrorNotification {
                    error: turn_error,
                    will_retry: true,
                    chat_id: conversation_id.to_string(),
                    interaction_id: event_interaction_id.clone(),
                }))
                .await;
        }
        EventMsg::ViewImageToolCall(_) => {}
        EventMsg::EnteredReviewMode(review_request) => {
            let review = review_request
                .user_facing_hint
                .unwrap_or_else(|| review_prompts::user_facing_hint(&review_request.target));
            let item = Message::EnteredReviewMode {
                id: event_interaction_id.clone(),
                review,
            };
            let started = MessageStartedNotification {
                chat_id: conversation_id.to_string(),
                interaction_id: event_interaction_id.clone(),
                started_at_ms: now_unix_timestamp_ms(),
                item: item.clone(),
            };
            outgoing
                .send_server_notification(MessageStarted(started))
                .await;
            let completed = MessageCompletedNotification {
                chat_id: conversation_id.to_string(),
                interaction_id: event_interaction_id.clone(),
                completed_at_ms: now_unix_timestamp_ms(),
                item,
            };
            outgoing
                .send_server_notification(MessageCompleted(completed))
                .await;
        }
        msg @ (EventMsg::MessageStarted(_)
        | EventMsg::MessageCompleted(_)
        | EventMsg::PatchApplyUpdated(_)
        | EventMsg::TerminalInteraction(_)) => {
            let notification = item_event_to_server_notification(
                msg,
                &conversation_id.to_string(),
                &event_interaction_id,
            );
            outgoing.send_server_notification(notification).await;
        }
        EventMsg::HookStarted(event) => {
            let notification = HookStartedNotification {
                chat_id: conversation_id.to_string(),
                interaction_id: event.interaction_id,
                run: event.run.into(),
            };
            outgoing
                .send_server_notification(ServerNotification::HookStarted(notification))
                .await;
        }
        EventMsg::HookCompleted(event) => {
            let notification = HookCompletedNotification {
                chat_id: conversation_id.to_string(),
                interaction_id: event.interaction_id,
                run: event.run.into(),
            };
            outgoing
                .send_server_notification(ServerNotification::HookCompleted(notification))
                .await;
        }
        EventMsg::ExitedReviewMode(review_event) => {
            let review = match review_event.review_output {
                Some(output) => render_review_output_text(&output),
                None => REVIEW_FALLBACK_MESSAGE.to_string(),
            };
            let item = Message::ExitedReviewMode {
                id: event_interaction_id.clone(),
                review,
            };
            let started = MessageStartedNotification {
                chat_id: conversation_id.to_string(),
                interaction_id: event_interaction_id.clone(),
                started_at_ms: now_unix_timestamp_ms(),
                item: item.clone(),
            };
            outgoing
                .send_server_notification(MessageStarted(started))
                .await;
            let completed = MessageCompletedNotification {
                chat_id: conversation_id.to_string(),
                interaction_id: event_interaction_id.clone(),
                completed_at_ms: now_unix_timestamp_ms(),
                item,
            };
            outgoing
                .send_server_notification(MessageCompleted(completed))
                .await;
        }
        EventMsg::RawResponseItem(raw_response_item_event) => {
            maybe_emit_hook_prompt_item_completed(
                conversation_id,
                &event_interaction_id,
                &raw_response_item_event.item,
                &outgoing,
            )
            .await;
            maybe_emit_raw_response_item_completed(
                conversation_id,
                &event_interaction_id,
                raw_response_item_event.item,
                &outgoing,
            )
            .await;
        }
        EventMsg::PatchApplyBegin(_) | EventMsg::PatchApplyEnd(_) => {
            // Core still fans out these deprecated events for legacy clients;
            // v2 clients receive the canonical FileChange item instead.
        }
        EventMsg::ExecCommandBegin(exec_command_begin_event) => {
            if matches!(
                exec_command_begin_event.source,
                datax_protocol::protocol::ExecCommandSource::UnifiedExecInteraction
            ) {
                // TerminalInteraction is the v2 surface for unified exec
                // stdin/poll events. Suppress the legacy CommandExecution
                // item so clients do not render the same wait twice.
                return;
            }
            let message_id = exec_command_begin_event.call_id.clone();
            let first_start = {
                let mut state = chat_state.lock().await;
                state
                    .interaction_summary
                    .command_execution_started
                    .insert(message_id.clone())
            };
            if first_start {
                let notification = item_event_to_server_notification(
                    EventMsg::ExecCommandBegin(exec_command_begin_event),
                    &conversation_id.to_string(),
                    &event_interaction_id,
                );
                outgoing.send_server_notification(notification).await;
            }
        }
        EventMsg::ExecCommandOutputDelta(exec_command_output_delta_event) => {
            let notification = item_event_to_server_notification(
                EventMsg::ExecCommandOutputDelta(exec_command_output_delta_event),
                &conversation_id.to_string(),
                &event_interaction_id,
            );
            outgoing.send_server_notification(notification).await;
        }
        EventMsg::ExecCommandEnd(exec_command_end_event) => {
            let call_id = exec_command_end_event.call_id.clone();
            {
                let mut state = chat_state.lock().await;
                state
                    .interaction_summary
                    .command_execution_started
                    .remove(&call_id);
            }
            if matches!(
                exec_command_end_event.source,
                datax_protocol::protocol::ExecCommandSource::UnifiedExecInteraction
            ) {
                // The paired begin event is suppressed above; keep the
                // completion out of v2 as well so no orphan legacy item is
                // emitted for unified exec interactions.
                return;
            }
            let notification = item_event_to_server_notification(
                EventMsg::ExecCommandEnd(exec_command_end_event),
                &conversation_id.to_string(),
                &event_interaction_id,
            );
            outgoing.send_server_notification(notification).await;
        }
        // If this is a InteractionAborted, reply to any pending interrupt requests.
        EventMsg::InteractionAborted(turn_aborted_event) => {
            // All per-chat requests are bound to a turn, so abort them.
            outgoing.abort_pending_server_requests().await;
            respond_to_pending_interrupts(&chat_state, &outgoing).await;

            chat_watch_manager
                .note_interaction_interrupted(&conversation_id.to_string())
                .await;
            handle_turn_interrupted(
                conversation_id,
                event_interaction_id,
                turn_aborted_event,
                &outgoing,
                &chat_state,
            )
            .await;
        }
        EventMsg::ThreadRolledBack(_rollback_event) => {
            let pending = {
                let mut state = chat_state.lock().await;
                state.pending_rollbacks.take()
            };

            if let Some(request_id) = pending {
                let _thread_list_state_permit = match thread_list_state_permit.acquire().await {
                    Ok(permit) => permit,
                    Err(err) => {
                        outgoing
                            .send_error(
                                request_id,
                                internal_error(format!(
                                    "failed to acquire thread list state permit: {err}"
                                )),
                            )
                            .await;
                        return;
                    }
                };
                let fallback_cwd = conversation.config_snapshot().await.cwd().clone();
                let stored_chat = match conversation
                    .read_chat(
                        /*include_archived*/ true, /*include_history*/ true,
                    )
                    .await
                {
                    Ok(stored_chat) => stored_chat,
                    Err(err) => {
                        outgoing
                            .send_error(
                                request_id.clone(),
                                internal_error(format!(
                                    "failed to read thread {conversation_id} after rollback: {err}"
                                )),
                            )
                            .await;
                        return;
                    }
                };
                let loaded_status = chat_watch_manager
                    .loaded_status_for_chat(&conversation_id.to_string())
                    .await;
                let response = match chat_rollback_response_from_stored_chat(
                    stored_chat,
                    conversation.session_configured().session_id.to_string(),
                    fallback_model_provider.as_str(),
                    &fallback_cwd,
                    loaded_status,
                ) {
                    Ok(response) => response,
                    Err(err) => {
                        outgoing
                            .send_error(request_id.clone(), internal_error(err))
                            .await;
                        return;
                    }
                };

                outgoing.send_response(request_id, response).await;
            }
        }
        EventMsg::ThreadGoalUpdated(chat_goal_event) => {
            let notification = ChatGoalUpdatedNotification {
                chat_id: chat_goal_event.chat_id.to_string(),
                interaction_id: chat_goal_event.interaction_id,
                goal: chat_goal_event.goal.clone().into(),
            };
            outgoing
                .send_global_server_notification(ChatGoalUpdated(notification))
                .await;
        }
        EventMsg::ThreadSettingsApplied(chat_settings_event) => {
            let chat_settings =
                chat_settings_from_core_snapshot(chat_settings_event.thread_settings);
            let changed = {
                let mut state = chat_state.lock().await;
                state.note_chat_settings(chat_settings.clone())
            };
            if changed {
                outgoing
                    .send_server_notification(ChatSettingsUpdated(
                        ChatSettingsUpdatedNotification {
                            chat_id: conversation_id.to_string(),
                            thread_settings: chat_settings,
                        },
                    ))
                    .await;
            }
        }
        EventMsg::InteractionDiff(turn_diff_event) => {
            handle_turn_diff(
                conversation_id,
                &event_interaction_id,
                turn_diff_event,
                &outgoing,
            )
            .await;
        }
        EventMsg::PlanUpdate(plan_update_event) => {
            handle_turn_plan_update(
                conversation_id,
                &event_interaction_id,
                plan_update_event,
                &outgoing,
            )
            .await;
        }
        EventMsg::ShutdownComplete => {
            chat_watch_manager
                .note_chat_shutdown(&conversation_id.to_string())
                .await;
        }

        _ => {}
    }
}

async fn handle_turn_diff(
    conversation_id: ChatId,
    event_interaction_id: &str,
    turn_diff_event: InteractionDiffEvent,
    outgoing: &ThreadScopedOutgoingMessageSender,
) {
    let notification = InteractionDiffUpdatedNotification {
        chat_id: conversation_id.to_string(),
        interaction_id: event_interaction_id.to_string(),
        diff: turn_diff_event.unified_diff,
    };
    outgoing
        .send_server_notification(InteractionDiffUpdated(notification))
        .await;
}

async fn handle_turn_plan_update(
    conversation_id: ChatId,
    event_interaction_id: &str,
    plan_update_event: UpdatePlanArgs,
    outgoing: &ThreadScopedOutgoingMessageSender,
) {
    // `update_plan` is a todo/checklist tool; it is not related to plan-mode updates
    let notification = InteractionPlanUpdatedNotification {
        chat_id: conversation_id.to_string(),
        interaction_id: event_interaction_id.to_string(),
        explanation: plan_update_event.explanation,
        plan: plan_update_event
            .plan
            .into_iter()
            .map(InteractionPlanStep::from)
            .collect(),
    };
    outgoing
        .send_server_notification(InteractionPlanUpdated(notification))
        .await;
}

struct TurnCompletionMetadata {
    status: InteractionStatus,
    error: Option<InteractionError>,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    duration_ms: Option<i64>,
}

async fn emit_turn_completed_with_status(
    conversation_id: ChatId,
    event_interaction_id: String,
    turn_completion_metadata: TurnCompletionMetadata,
    outgoing: &ThreadScopedOutgoingMessageSender,
) {
    let notification = InteractionCompletedNotification {
        chat_id: conversation_id.to_string(),
        interaction: Interaction {
            id: event_interaction_id,
            messages: vec![],
            messages_view: InteractionMessagesView::NotLoaded,
            error: turn_completion_metadata.error,
            status: turn_completion_metadata.status,
            started_at: turn_completion_metadata.started_at,
            completed_at: turn_completion_metadata.completed_at,
            duration_ms: turn_completion_metadata.duration_ms,
        },
    };
    outgoing
        .send_server_notification(InteractionCompleted(notification))
        .await;
}

#[allow(clippy::too_many_arguments)]
async fn start_command_execution_item(
    conversation_id: &ChatId,
    interaction_id: String,
    message_id: String,
    command: String,
    cwd: LegacyAppPathString,
    command_actions: Vec<V2ParsedCommand>,
    source: CommandExecutionSource,
    outgoing: &ThreadScopedOutgoingMessageSender,
    chat_state: &Arc<Mutex<ChatState>>,
) -> bool {
    let first_start = {
        let mut state = chat_state.lock().await;
        state
            .interaction_summary
            .command_execution_started
            .insert(message_id.clone())
    };
    if first_start {
        let notification = MessageStartedNotification {
            chat_id: conversation_id.to_string(),
            interaction_id,
            started_at_ms: now_unix_timestamp_ms(),
            item: Message::CommandExecution {
                id: message_id,
                command,
                cwd,
                process_id: None,
                source,
                status: CommandExecutionStatus::InProgress,
                command_actions,
                aggregated_output: None,
                exit_code: None,
                duration_ms: None,
            },
        };
        outgoing
            .send_server_notification(MessageStarted(notification))
            .await;
    }
    first_start
}

#[allow(clippy::too_many_arguments)]
async fn complete_command_execution_item(
    conversation_id: &ChatId,
    interaction_id: String,
    message_id: String,
    command: String,
    cwd: LegacyAppPathString,
    process_id: Option<String>,
    source: CommandExecutionSource,
    command_actions: Vec<V2ParsedCommand>,
    status: CommandExecutionStatus,
    outgoing: &ThreadScopedOutgoingMessageSender,
    chat_state: &Arc<Mutex<ChatState>>,
) {
    let should_emit = chat_state
        .lock()
        .await
        .interaction_summary
        .command_execution_started
        .remove(&message_id);
    if !should_emit {
        return;
    }

    let item = Message::CommandExecution {
        id: message_id,
        command,
        cwd,
        process_id,
        source,
        status,
        command_actions,
        aggregated_output: None,
        exit_code: None,
        duration_ms: None,
    };
    let notification = MessageCompletedNotification {
        chat_id: conversation_id.to_string(),
        interaction_id,
        completed_at_ms: now_unix_timestamp_ms(),
        item,
    };
    outgoing
        .send_server_notification(MessageCompleted(notification))
        .await;
}

async fn maybe_emit_raw_response_item_completed(
    conversation_id: ChatId,
    interaction_id: &str,
    item: datax_protocol::models::ResponseItem,
    outgoing: &ThreadScopedOutgoingMessageSender,
) {
    let notification = RawResponseMessageCompletedNotification {
        chat_id: conversation_id.to_string(),
        interaction_id: interaction_id.to_string(),
        item,
    };
    outgoing
        .send_server_notification(ServerNotification::RawResponseMessageCompleted(notification))
        .await;
}

pub(crate) async fn maybe_emit_hook_prompt_item_completed(
    conversation_id: ChatId,
    interaction_id: &str,
    item: &datax_protocol::models::ResponseItem,
    outgoing: &ThreadScopedOutgoingMessageSender,
) {
    let datax_protocol::models::ResponseItem::Message {
        role, content, id, ..
    } = item
    else {
        return;
    };

    if role != "user" {
        return;
    }

    let Some(hook_prompt) = parse_hook_prompt_message(id.as_ref(), content) else {
        return;
    };

    let notification = MessageCompletedNotification {
        chat_id: conversation_id.to_string(),
        interaction_id: interaction_id.to_string(),
        completed_at_ms: now_unix_timestamp_ms(),
        item: Message::HookPrompt {
            id: hook_prompt.id,
            fragments: hook_prompt
                .fragments
                .into_iter()
                .map(datax_app_server_protocol::HookPromptFragment::from)
                .collect(),
        },
    };
    outgoing
        .send_server_notification(MessageCompleted(notification))
        .await;
}

async fn find_and_remove_interaction_summary(
    _conversation_id: ChatId,
    chat_state: &Arc<Mutex<ChatState>>,
) -> InteractionSummary {
    let mut state = chat_state.lock().await;
    std::mem::take(&mut state.interaction_summary)
}

async fn handle_turn_complete(
    conversation_id: ChatId,
    event_interaction_id: String,
    turn_complete_event: InteractionCompleteEvent,
    outgoing: &ThreadScopedOutgoingMessageSender,
    chat_state: &Arc<Mutex<ChatState>>,
) {
    let interaction_summary = find_and_remove_interaction_summary(conversation_id, chat_state).await;

    let (status, error) = match interaction_summary.last_error {
        Some(error) => (InteractionStatus::Failed, Some(error)),
        None => (InteractionStatus::Completed, None),
    };

    emit_turn_completed_with_status(
        conversation_id,
        event_interaction_id,
        TurnCompletionMetadata {
            status,
            error,
            started_at: interaction_summary.started_at,
            completed_at: turn_complete_event.completed_at,
            duration_ms: turn_complete_event.duration_ms,
        },
        outgoing,
    )
    .await;
}

async fn handle_turn_interrupted(
    conversation_id: ChatId,
    event_interaction_id: String,
    turn_aborted_event: InteractionAbortedEvent,
    outgoing: &ThreadScopedOutgoingMessageSender,
    chat_state: &Arc<Mutex<ChatState>>,
) {
    let interaction_summary = find_and_remove_interaction_summary(conversation_id, chat_state).await;

    emit_turn_completed_with_status(
        conversation_id,
        event_interaction_id,
        TurnCompletionMetadata {
            status: InteractionStatus::Interrupted,
            error: None,
            started_at: interaction_summary.started_at,
            completed_at: turn_aborted_event.completed_at,
            duration_ms: turn_aborted_event.duration_ms,
        },
        outgoing,
    )
    .await;
}

async fn handle_thread_rollback_failed(
    _conversation_id: ChatId,
    message: String,
    chat_state: &Arc<Mutex<ChatState>>,
    outgoing: &ThreadScopedOutgoingMessageSender,
) {
    let pending_rollback = chat_state.lock().await.pending_rollbacks.take();

    if let Some(request_id) = pending_rollback {
        outgoing
            .send_error(request_id, invalid_request(message))
            .await;
    }
}

fn chat_rollback_response_from_stored_chat(
    stored_chat: datax_thread_store::StoredChat,
    session_id: String,
    fallback_model_provider: &str,
    fallback_cwd: &AbsolutePathBuf,
    loaded_status: ChatStatus,
) -> std::result::Result<ChatRollbackResponse, String> {
    let chat_id = stored_chat.chat_id;
    let (mut thread, history) =
        chat_from_stored_chat(stored_chat, fallback_model_provider, fallback_cwd);
    thread.session_id = session_id;
    let Some(history) = history else {
        return Err(format!(
            "thread {chat_id} did not include persisted history after rollback"
        ));
    };
    populate_chat_interactions_from_history(&mut thread, &history.items, /*active_interaction*/ None);
    thread.status = loaded_status;
    Ok(ChatRollbackResponse { chat: thread })
}

async fn respond_to_pending_interrupts(
    chat_state: &Arc<Mutex<ChatState>>,
    outgoing: &ThreadScopedOutgoingMessageSender,
) {
    let pending = {
        let mut state = chat_state.lock().await;
        std::mem::take(&mut state.pending_interrupts)
    };

    for request_id in pending {
        outgoing
            .send_response(request_id, InteractionInterruptResponse {})
            .await;
    }
}

async fn handle_token_count_event(
    conversation_id: ChatId,
    interaction_id: String,
    token_count_event: TokenCountEvent,
    outgoing: &ThreadScopedOutgoingMessageSender,
) {
    let TokenCountEvent { info, rate_limits } = token_count_event;
    if let Some(token_usage) = info.map(ChatTokenUsage::from) {
        let notification = ChatTokenUsageUpdatedNotification {
            chat_id: conversation_id.to_string(),
            interaction_id,
            token_usage,
        };
        outgoing
            .send_server_notification(ChatTokenUsageUpdated(notification))
            .await;
    }
    if let Some(rate_limits) = rate_limits {
        outgoing
            .send_server_notification(ServerNotification::AccountRateLimitsUpdated(
                AccountRateLimitsUpdatedNotification {
                    rate_limits: rate_limits.into(),
                },
            ))
            .await;
    }
}

async fn handle_error(
    _conversation_id: ChatId,
    error: InteractionError,
    chat_state: &Arc<Mutex<ChatState>>,
) {
    let mut state = chat_state.lock().await;
    state.interaction_summary.last_error = Some(error);
}

async fn handle_error_notification(
    conversation_id: ChatId,
    event_interaction_id: &str,
    error: InteractionError,
    outgoing: &ThreadScopedOutgoingMessageSender,
    chat_state: &Arc<Mutex<ChatState>>,
) {
    handle_error(conversation_id, error.clone(), chat_state).await;
    outgoing
        .send_server_notification(ServerNotification::Error(ErrorNotification {
            error,
            will_retry: false,
            chat_id: conversation_id.to_string(),
            interaction_id: event_interaction_id.to_string(),
        }))
        .await;
}

async fn on_request_user_input_response(
    event_interaction_id: String,
    pending_request_id: RequestId,
    receiver: oneshot::Receiver<ClientRequestResult>,
    conversation: Arc<DataxChat>,
    chat_state: Arc<Mutex<ChatState>>,
    user_input_guard: ChatWatchActiveGuard,
) {
    let response = receiver.await;
    resolve_server_request_on_chat_listener(&chat_state, pending_request_id).await;
    drop(user_input_guard);
    let value = match response {
        Ok(Ok(value)) => value,
        Ok(Err(err)) if is_turn_transition_server_request_error(&err) => return,
        Ok(Err(err)) => {
            error!("request failed with client error: {err:?}");
            let empty = CoreRequestUserInputResponse {
                answers: HashMap::new(),
            };
            if let Err(err) = conversation
                .submit(Op::UserInputAnswer {
                    id: event_interaction_id,
                    response: empty,
                })
                .await
            {
                error!("failed to submit UserInputAnswer: {err}");
            }
            return;
        }
        Err(err) => {
            error!("request failed: {err:?}");
            let empty = CoreRequestUserInputResponse {
                answers: HashMap::new(),
            };
            if let Err(err) = conversation
                .submit(Op::UserInputAnswer {
                    id: event_interaction_id,
                    response: empty,
                })
                .await
            {
                error!("failed to submit UserInputAnswer: {err}");
            }
            return;
        }
    };

    let response =
        serde_json::from_value::<ToolRequestUserInputResponse>(value).unwrap_or_else(|err| {
            error!("failed to deserialize ToolRequestUserInputResponse: {err}");
            ToolRequestUserInputResponse {
                answers: HashMap::new(),
            }
        });
    let response = CoreRequestUserInputResponse {
        answers: response
            .answers
            .into_iter()
            .map(|(id, answer)| {
                (
                    id,
                    CoreRequestUserInputAnswer {
                        answers: answer.answers,
                    },
                )
            })
            .collect(),
    };

    if let Err(err) = conversation
        .submit(Op::UserInputAnswer {
            id: event_interaction_id,
            response,
        })
        .await
    {
        error!("failed to submit UserInputAnswer: {err}");
    }
}

async fn on_mcp_server_elicitation_response(
    server_name: String,
    request_id: datax_protocol::mcp::RequestId,
    pending_request_id: RequestId,
    receiver: oneshot::Receiver<ClientRequestResult>,
    conversation: Arc<DataxChat>,
    chat_state: Arc<Mutex<ChatState>>,
    permission_guard: ChatWatchActiveGuard,
) {
    let response = receiver.await;
    resolve_server_request_on_chat_listener(&chat_state, pending_request_id).await;
    drop(permission_guard);
    let response = mcp_server_elicitation_response_from_client_result(response);

    if let Err(err) = conversation
        .submit(Op::ResolveElicitation {
            server_name,
            request_id,
            decision: response.action.to_core(),
            content: response.content,
            meta: response.meta,
        })
        .await
    {
        error!("failed to submit ResolveElicitation: {err}");
    }
}

fn mcp_server_elicitation_response_from_client_result(
    response: std::result::Result<ClientRequestResult, oneshot::error::RecvError>,
) -> McpServerElicitationRequestResponse {
    match response {
        Ok(Ok(value)) => serde_json::from_value::<McpServerElicitationRequestResponse>(value)
            .unwrap_or_else(|err| {
                error!("failed to deserialize McpServerElicitationRequestResponse: {err}");
                McpServerElicitationRequestResponse {
                    action: McpServerElicitationAction::Decline,
                    content: None,
                    meta: None,
                }
            }),
        Ok(Err(err)) if is_turn_transition_server_request_error(&err) => {
            McpServerElicitationRequestResponse {
                action: McpServerElicitationAction::Cancel,
                content: None,
                meta: None,
            }
        }
        Ok(Err(err)) => {
            error!("request failed with client error: {err:?}");
            McpServerElicitationRequestResponse {
                action: McpServerElicitationAction::Decline,
                content: None,
                meta: None,
            }
        }
        Err(err) => {
            error!("request failed: {err:?}");
            McpServerElicitationRequestResponse {
                action: McpServerElicitationAction::Decline,
                content: None,
                meta: None,
            }
        }
    }
}

async fn on_request_permissions_response(
    pending_response: PendingRequestPermissionsResponse,
    conversation: Arc<DataxChat>,
    chat_state: Arc<Mutex<ChatState>>,
) {
    let PendingRequestPermissionsResponse {
        call_id,
        conversation_id,
        interaction_id,
        requested_permissions,
        request_cwd,
        pending_request_id,
        outgoing,
        receiver,
        request_permissions_guard,
    } = pending_response;
    let response = receiver.await;
    resolve_server_request_on_chat_listener(&chat_state, pending_request_id.clone()).await;
    drop(request_permissions_guard);
    let response = match request_permissions_response_from_client_result(
        requested_permissions,
        response,
        request_cwd.as_path(),
    ) {
        Ok(Some(response)) => response,
        Ok(None) => return,
        // TODO(anp): Remove this native-path localization error path once core permission paths
        // remain PathUri after crossing the app-server boundary.
        Err(err) => {
            let message = format!("failed to localize granted filesystem paths: {err}");
            handle_error_notification(
                conversation_id,
                &interaction_id,
                InteractionError {
                    message,
                    codex_error_info: None,
                    additional_details: None,
                },
                &outgoing,
                &chat_state,
            )
            .await;
            if let Err(err) = conversation.submit(Op::Interrupt).await {
                error!("failed to interrupt turn after invalid permission paths: {err}");
            }
            return;
        }
    };
    outgoing.track_effective_permissions_approval_response(pending_request_id, response.clone());

    if let Err(err) = conversation
        .submit(Op::RequestPermissionsResponse {
            id: call_id,
            response,
        })
        .await
    {
        error!("failed to submit RequestPermissionsResponse: {err}");
    }
}

struct PendingRequestPermissionsResponse {
    call_id: String,
    conversation_id: ChatId,
    interaction_id: String,
    requested_permissions: CoreRequestPermissionProfile,
    request_cwd: AbsolutePathBuf,
    pending_request_id: RequestId,
    outgoing: ThreadScopedOutgoingMessageSender,
    receiver: oneshot::Receiver<ClientRequestResult>,
    request_permissions_guard: ChatWatchActiveGuard,
}

fn request_permissions_response_from_client_result(
    requested_permissions: CoreRequestPermissionProfile,
    response: std::result::Result<ClientRequestResult, oneshot::error::RecvError>,
    cwd: &std::path::Path,
) -> std::io::Result<Option<CoreRequestPermissionsResponse>> {
    let value = match response {
        Ok(Ok(value)) => value,
        Ok(Err(err)) if is_turn_transition_server_request_error(&err) => return Ok(None),
        Ok(Err(err)) => {
            error!("request failed with client error: {err:?}");
            return Ok(Some(CoreRequestPermissionsResponse {
                permissions: Default::default(),
                scope: CorePermissionGrantScope::Turn,
                strict_auto_review: false,
            }));
        }
        Err(err) => {
            error!("request failed: {err:?}");
            return Ok(Some(CoreRequestPermissionsResponse {
                permissions: Default::default(),
                scope: CorePermissionGrantScope::Turn,
                strict_auto_review: false,
            }));
        }
    };

    let response = serde_json::from_value::<PermissionsRequestApprovalResponse>(value)
        .unwrap_or_else(|err| {
            error!("failed to deserialize PermissionsRequestApprovalResponse: {err}");
            PermissionsRequestApprovalResponse {
                permissions: V2GrantedPermissionProfile::default(),
                scope: datax_app_server_protocol::PermissionGrantScope::Interaction,
                strict_auto_review: None,
            }
        });
    let strict_auto_review = response.strict_auto_review.unwrap_or(false);
    if strict_auto_review
        && matches!(
            response.scope,
            datax_app_server_protocol::PermissionGrantScope::Session
        )
    {
        error!("strict auto review is only supported for turn-scoped permission grants");
        return Ok(Some(CoreRequestPermissionsResponse {
            permissions: Default::default(),
            scope: CorePermissionGrantScope::Turn,
            strict_auto_review: false,
        }));
    }
    let granted_permissions: CoreAdditionalPermissionProfile = response.permissions.try_into()?;
    let permissions = if granted_permissions.is_empty() {
        CoreRequestPermissionProfile::default()
    } else {
        intersect_permission_profiles(requested_permissions.into(), granted_permissions, cwd).into()
    };
    Ok(Some(CoreRequestPermissionsResponse {
        permissions,
        scope: response.scope.to_core(),
        strict_auto_review,
    }))
}

const REVIEW_FALLBACK_MESSAGE: &str = "Reviewer failed to output a response.";

fn render_review_output_text(output: &ReviewOutputEvent) -> String {
    let mut sections = Vec::new();
    let explanation = output.overall_explanation.trim();
    if !explanation.is_empty() {
        sections.push(explanation.to_string());
    }
    if !output.findings.is_empty() {
        let findings = format_review_findings_block(&output.findings, /*selection*/ None);
        let trimmed = findings.trim();
        if !trimmed.is_empty() {
            sections.push(trimmed.to_string());
        }
    }
    if sections.is_empty() {
        REVIEW_FALLBACK_MESSAGE.to_string()
    } else {
        sections.join("\n\n")
    }
}

fn map_file_change_approval_decision(decision: FileChangeApprovalDecision) -> ReviewDecision {
    match decision {
        FileChangeApprovalDecision::Accept => ReviewDecision::Approved,
        FileChangeApprovalDecision::AcceptForSession => ReviewDecision::ApprovedForSession,
        FileChangeApprovalDecision::Decline => ReviewDecision::Denied,
        FileChangeApprovalDecision::Cancel => ReviewDecision::Abort,
    }
}

#[allow(clippy::too_many_arguments)]
async fn on_file_change_request_approval_response(
    message_id: String,
    pending_request_id: RequestId,
    receiver: oneshot::Receiver<ClientRequestResult>,
    codex: Arc<DataxChat>,
    chat_state: Arc<Mutex<ChatState>>,
    permission_guard: ChatWatchActiveGuard,
) {
    let response = receiver.await;
    resolve_server_request_on_chat_listener(&chat_state, pending_request_id).await;
    drop(permission_guard);
    let decision = match response {
        Ok(Ok(value)) => {
            let response = serde_json::from_value::<FileChangeRequestApprovalResponse>(value)
                .unwrap_or_else(|err| {
                    error!("failed to deserialize FileChangeRequestApprovalResponse: {err}");
                    FileChangeRequestApprovalResponse {
                        decision: FileChangeApprovalDecision::Decline,
                    }
                });

            map_file_change_approval_decision(response.decision)
        }
        Ok(Err(err)) if is_turn_transition_server_request_error(&err) => return,
        Ok(Err(err)) => {
            error!("request failed with client error: {err:?}");
            ReviewDecision::Denied
        }
        Err(err) => {
            error!("request failed: {err:?}");
            ReviewDecision::Denied
        }
    };

    if let Err(err) = codex
        .submit(Op::PatchApproval {
            id: message_id,
            decision,
        })
        .await
    {
        error!("failed to submit PatchApproval: {err}");
    }
}

#[allow(clippy::too_many_arguments)]
async fn on_command_execution_request_approval_response(
    event_interaction_id: String,
    conversation_id: ChatId,
    approval_id: Option<String>,
    message_id: String,
    completion_item: Option<CommandExecutionCompletionItem>,
    pending_request_id: RequestId,
    receiver: oneshot::Receiver<ClientRequestResult>,
    conversation: Arc<DataxChat>,
    outgoing: ThreadScopedOutgoingMessageSender,
    chat_state: Arc<Mutex<ChatState>>,
    permission_guard: ChatWatchActiveGuard,
) {
    let response = receiver.await;
    resolve_server_request_on_chat_listener(&chat_state, pending_request_id).await;
    drop(permission_guard);
    let (decision, completion_status) = match response {
        Ok(Ok(value)) => {
            let response = serde_json::from_value::<CommandExecutionRequestApprovalResponse>(value)
                .unwrap_or_else(|err| {
                    error!("failed to deserialize CommandExecutionRequestApprovalResponse: {err}");
                    CommandExecutionRequestApprovalResponse {
                        decision: CommandExecutionApprovalDecision::Decline,
                    }
                });

            let decision = response.decision;

            let (decision, completion_status) = match decision {
                CommandExecutionApprovalDecision::Accept => (ReviewDecision::Approved, None),
                CommandExecutionApprovalDecision::AcceptForSession => {
                    (ReviewDecision::ApprovedForSession, None)
                }
                CommandExecutionApprovalDecision::AcceptWithExecpolicyAmendment {
                    execpolicy_amendment,
                } => (
                    ReviewDecision::ApprovedExecpolicyAmendment {
                        proposed_execpolicy_amendment: execpolicy_amendment.into_core(),
                    },
                    None,
                ),
                CommandExecutionApprovalDecision::ApplyNetworkPolicyAmendment {
                    network_policy_amendment,
                } => {
                    let completion_status = match network_policy_amendment.action {
                        V2NetworkPolicyRuleAction::Allow => None,
                        V2NetworkPolicyRuleAction::Deny => Some(CommandExecutionStatus::Declined),
                    };
                    (
                        ReviewDecision::NetworkPolicyAmendment {
                            network_policy_amendment: network_policy_amendment.into_core(),
                        },
                        completion_status,
                    )
                }
                CommandExecutionApprovalDecision::Decline => (
                    ReviewDecision::Denied,
                    Some(CommandExecutionStatus::Declined),
                ),
                CommandExecutionApprovalDecision::Cancel => (
                    ReviewDecision::Abort,
                    Some(CommandExecutionStatus::Declined),
                ),
            };
            (decision, completion_status)
        }
        Ok(Err(err)) if is_turn_transition_server_request_error(&err) => return,
        Ok(Err(err)) => {
            error!("request failed with client error: {err:?}");
            (ReviewDecision::Denied, Some(CommandExecutionStatus::Failed))
        }
        Err(err) => {
            error!("request failed: {err:?}");
            (ReviewDecision::Denied, Some(CommandExecutionStatus::Failed))
        }
    };

    let suppress_subcommand_completion_item = {
        // For regular shell/unified_exec approvals, approval_id is null.
        // For zsh-fork subcommand approvals, approval_id is present and
        // message_id points to the parent command item.
        if approval_id.is_some() {
            let state = chat_state.lock().await;
            state
                .interaction_summary
                .command_execution_started
                .contains(&message_id)
        } else {
            false
        }
    };

    if let Some(status) = completion_status
        && !suppress_subcommand_completion_item
        && let Some(completion_item) = completion_item
    {
        complete_command_execution_item(
            &conversation_id,
            event_interaction_id.clone(),
            message_id.clone(),
            completion_item.command,
            completion_item.cwd,
            /*process_id*/ None,
            CommandExecutionSource::Agent,
            completion_item.command_actions,
            status,
            &outgoing,
            &chat_state,
        )
        .await;
    }

    if let Err(err) = conversation
        .submit(Op::ExecApproval {
            id: approval_id.unwrap_or_else(|| message_id.clone()),
            interaction_id: Some(event_interaction_id),
            decision,
        })
        .await
    {
        error!("failed to submit ExecApproval: {err}");
    }
}

fn now_unix_timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CHANNEL_CAPACITY;
    use crate::outgoing_message::ConnectionId;
    use crate::outgoing_message::OutgoingEnvelope;
    use crate::outgoing_message::OutgoingMessage;
    use crate::outgoing_message::OutgoingMessageSender;
    use anyhow::Result;
    use anyhow::anyhow;
    use anyhow::bail;
    use chrono::Utc;
    use core_test_support::load_default_config_for_test;
    use datax_app_server_protocol::AutoReviewDecisionSource;
    use datax_app_server_protocol::GuardianApprovalReviewStatus;
    use datax_app_server_protocol::InteractionPlanStepStatus;
    use datax_app_server_protocol::JSONRPCErrorError;
    use datax_login::CodexAuth;
    use datax_protocol::AgentPath;
    use datax_protocol::items::HookPromptFragment;
    use datax_protocol::items::build_hook_prompt_message;
    use datax_protocol::models::FileSystemPermissions as CoreFileSystemPermissions;
    use datax_protocol::models::NetworkPermissions as CoreNetworkPermissions;
    use datax_protocol::models::PermissionProfile;
    use datax_protocol::permissions::FileSystemAccessMode;
    use datax_protocol::permissions::FileSystemPath;
    use datax_protocol::permissions::FileSystemSandboxEntry;
    use datax_protocol::permissions::FileSystemSpecialPath;
    use datax_protocol::plan_tool::PlanItemArg;
    use datax_protocol::plan_tool::StepStatus;
    use datax_protocol::protocol::AgentMessageEvent;
    use datax_protocol::protocol::AskForApproval;
    use datax_protocol::protocol::CreditsSnapshot;
    use datax_protocol::protocol::EventMsg;
    use datax_protocol::protocol::GuardianAssessmentEvent;
    use datax_protocol::protocol::GuardianAssessmentStatus;
    use datax_protocol::protocol::RateLimitSnapshot;
    use datax_protocol::protocol::RateLimitWindow;
    use datax_protocol::protocol::RolloutMessage;
    use datax_protocol::protocol::SessionSource;
    use datax_protocol::protocol::SubAgentActivityEvent;
    use datax_protocol::protocol::TokenUsage;
    use datax_protocol::protocol::TokenUsageInfo;
    use datax_protocol::protocol::UserMessageEvent;
    use datax_thread_store::StoredChat;
    use datax_thread_store::StoredChatHistory;
    use datax_utils_absolute_path::AbsolutePathBuf;
    use datax_utils_absolute_path::test_support::PathBufExt;
    use datax_utils_absolute_path::test_support::test_path_buf;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tempfile::TempDir;
    use tokio::sync::Mutex;
    use tokio::sync::mpsc;

    fn new_chat_state() -> Arc<Mutex<ChatState>> {
        Arc::new(Mutex::new(ChatState::default()))
    }

    const TEST_TURN_COMPLETED_AT: i64 = 1_716_000_456;
    const TEST_TURN_DURATION_MS: i64 = 1_234;

    async fn recv_broadcast_message(
        rx: &mut mpsc::Receiver<OutgoingEnvelope>,
    ) -> Result<OutgoingMessage> {
        let envelope = rx
            .recv()
            .await
            .ok_or_else(|| anyhow!("should send one message"))?;
        match envelope {
            OutgoingEnvelope::Broadcast { message } => Ok(message),
            OutgoingEnvelope::ToConnection { message, .. } => Ok(message),
        }
    }

    #[test]
    fn rollback_response_rebuilds_pathless_thread_from_stored_history() -> Result<()> {
        let chat_id = ChatId::from_string("00000000-0000-0000-0000-000000000789")?;
        let created_at = Utc::now();
        let history_items = vec![
            RolloutMessage::EventMsg(EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "before rollback".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
                ..Default::default()
            })),
            RolloutMessage::EventMsg(EventMsg::AgentMessage(AgentMessageEvent {
                message: "after rollback".to_string(),
                phase: None,
                memory_citation: None,
            })),
        ];
        let stored_chat = StoredChat {
            chat_id: chat_id,
            extra_config: None,
            rollout_path: None,
            forked_from_id: None,
            parent_chat_id: None,
            preview: "fallback preview".to_string(),
            name: Some("Rollback thread".to_string()),
            model_provider: "openai".to_string(),
            model: None,
            reasoning_effort: None,
            created_at,
            updated_at: created_at,
            recency_at: created_at,
            archived_at: None,
            cwd: test_path_buf("/tmp").abs().into(),
            cli_version: "0.0.0".to_string(),
            source: SessionSource::Cli,
            thread_source: None,
            agent_nickname: None,
            agent_role: None,
            agent_path: None,
            git_info: None,
            approval_mode: AskForApproval::OnRequest,
            permission_profile: PermissionProfile::read_only(),
            token_usage: None,
            first_user_message: Some("before rollback".to_string()),
            history: Some(StoredChatHistory {
                chat_id: chat_id,
                items: history_items,
            }),
        };
        let fallback_cwd = test_path_buf("/tmp").abs();

        let response = chat_rollback_response_from_stored_chat(
            stored_chat,
            chat_id.to_string(),
            "fallback-provider",
            &fallback_cwd,
            ChatStatus::NotLoaded,
        )
        .expect("rollback response should rebuild from stored history");

        assert_eq!(response.chat.id, chat_id.to_string());
        assert_eq!(response.chat.path, None);
        assert_eq!(response.chat.preview, "fallback preview");
        assert_eq!(response.chat.name.as_deref(), Some("Rollback thread"));
        assert_eq!(response.chat.status, ChatStatus::NotLoaded);
        assert_eq!(response.chat.interactions.len(), 1);
        assert_eq!(response.chat.interactions[0].messages.len(), 2);
        Ok(())
    }

    fn turn_complete_event(interaction_id: &str) -> InteractionCompleteEvent {
        InteractionCompleteEvent {
            interaction_id: interaction_id.to_string(),
            last_agent_message: None,
            completed_at: Some(TEST_TURN_COMPLETED_AT),
            duration_ms: Some(TEST_TURN_DURATION_MS),
            time_to_first_token_ms: None,
        }
    }

    fn turn_aborted_event(interaction_id: &str) -> InteractionAbortedEvent {
        InteractionAbortedEvent {
            interaction_id: Some(interaction_id.to_string()),
            reason: datax_protocol::protocol::InteractionAbortReason::Interrupted,
            completed_at: Some(TEST_TURN_COMPLETED_AT),
            duration_ms: Some(TEST_TURN_DURATION_MS),
        }
    }

    fn command_execution_completion_item(command: &str) -> CommandExecutionCompletionItem {
        CommandExecutionCompletionItem {
            command: command.to_string(),
            cwd: test_path_buf("/tmp").abs().into(),
            command_actions: vec![V2ParsedCommand::Unknown {
                command: command.to_string(),
            }],
        }
    }

    fn guardian_command_assessment(
        id: &str,
        interaction_id: &str,
        status: GuardianAssessmentStatus,
    ) -> GuardianAssessmentEvent {
        let (risk_level, user_authorization, rationale) = match status {
            GuardianAssessmentStatus::InProgress => (None, None, None),
            GuardianAssessmentStatus::Approved => (
                Some(datax_protocol::protocol::GuardianRiskLevel::Low),
                Some(datax_protocol::protocol::GuardianUserAuthorization::High),
                Some("looks safe".to_string()),
            ),
            GuardianAssessmentStatus::Denied => (
                Some(datax_protocol::protocol::GuardianRiskLevel::High),
                Some(datax_protocol::protocol::GuardianUserAuthorization::Low),
                Some("too risky".to_string()),
            ),
            GuardianAssessmentStatus::TimedOut => {
                (None, None, Some("review timed out".to_string()))
            }
            GuardianAssessmentStatus::Aborted => (None, None, None),
        };
        GuardianAssessmentEvent {
            id: format!("review-{id}"),
            target_item_id: Some(id.to_string()),
            interaction_id: interaction_id.to_string(),
            started_at_ms: 1_000,
            completed_at_ms: (!matches!(status, GuardianAssessmentStatus::InProgress))
                .then_some(1_042),
            status,
            risk_level,
            user_authorization,
            rationale,
            decision_source: if matches!(status, GuardianAssessmentStatus::InProgress) {
                None
            } else {
                Some(datax_protocol::protocol::GuardianAssessmentDecisionSource::Agent)
            },
            action: serde_json::from_value(json!({
                "type": "command",
                "source": "shell",
                "command": format!("rm -f /tmp/{id}.sqlite"),
                "cwd": test_path_buf("/tmp"),
            }))
            .expect("guardian action"),
        }
    }

    struct GuardianAssessmentTestContext {
        conversation_id: ChatId,
        conversation: Arc<DataxChat>,
        chat_manager: Arc<ChatManager>,
        outgoing: ThreadScopedOutgoingMessageSender,
        chat_state: Arc<Mutex<ChatState>>,
        chat_watch_manager: ChatWatchManager,
    }

    impl GuardianAssessmentTestContext {
        async fn apply_guardian_assessment_event(&self, assessment: GuardianAssessmentEvent) {
            let event_interaction_id = assessment.interaction_id.clone();
            apply_bespoke_event_handling(
                Event {
                    id: event_interaction_id,
                    msg: EventMsg::GuardianAssessment(assessment),
                },
                self.conversation_id,
                self.conversation.clone(),
                self.chat_manager.clone(),
                self.outgoing.clone(),
                self.chat_state.clone(),
                self.chat_watch_manager.clone(),
                Arc::new(tokio::sync::Semaphore::new(/*permits*/ 1)),
                "test-provider".to_string(),
            )
            .await;
        }
    }

    #[test]
    fn guardian_assessment_started_uses_event_interaction_id_fallback() {
        let conversation_id = ChatId::new();
        let action = datax_protocol::protocol::GuardianAssessmentAction::Command {
            source: datax_protocol::protocol::GuardianCommandSource::Shell,
            command: "rm -rf /tmp/example.sqlite".to_string(),
            cwd: test_path_buf("/tmp").abs(),
        };
        let notification = guardian_auto_approval_review_notification(
            &conversation_id,
            "turn-from-event",
            &GuardianAssessmentEvent {
                id: "review-1".to_string(),
                target_item_id: Some("item-1".to_string()),
                interaction_id: String::new(),
                started_at_ms: 1_000,
                completed_at_ms: None,
                status: datax_protocol::protocol::GuardianAssessmentStatus::InProgress,
                risk_level: None,
                user_authorization: None,
                rationale: None,
                decision_source: None,
                action: action.clone(),
            },
        );

        match notification {
            MessageGuardianApprovalReviewStarted(payload) => {
                assert_eq!(payload.chat_id, conversation_id.to_string());
                assert_eq!(payload.interaction_id, "turn-from-event");
                assert_eq!(payload.started_at_ms, 1_000);
                assert_eq!(payload.review_id, "review-1");
                assert_eq!(payload.target_message_id.as_deref(), Some("item-1"));
                assert_eq!(
                    payload.review.status,
                    GuardianApprovalReviewStatus::InProgress
                );
                assert_eq!(payload.review.risk_level, None);
                assert_eq!(payload.review.user_authorization, None);
                assert_eq!(payload.review.rationale, None);
                assert_eq!(payload.action, action.into());
            }
            other => panic!("unexpected notification: {other:?}"),
        }
    }

    #[test]
    fn guardian_assessment_completed_emits_review_payload() {
        let conversation_id = ChatId::new();
        let action = datax_protocol::protocol::GuardianAssessmentAction::Command {
            source: datax_protocol::protocol::GuardianCommandSource::Shell,
            command: "rm -rf /tmp/example.sqlite".to_string(),
            cwd: test_path_buf("/tmp").abs(),
        };
        let notification = guardian_auto_approval_review_notification(
            &conversation_id,
            "turn-from-event",
            &GuardianAssessmentEvent {
                id: "review-2".to_string(),
                target_item_id: Some("item-2".to_string()),
                interaction_id: "turn-from-assessment".to_string(),
                started_at_ms: 1_000,
                completed_at_ms: Some(1_042),
                status: datax_protocol::protocol::GuardianAssessmentStatus::Denied,
                risk_level: Some(datax_protocol::protocol::GuardianRiskLevel::High),
                user_authorization: Some(datax_protocol::protocol::GuardianUserAuthorization::Low),
                rationale: Some("too risky".to_string()),
                decision_source: Some(
                    datax_protocol::protocol::GuardianAssessmentDecisionSource::Agent,
                ),
                action: action.clone(),
            },
        );

        match notification {
            MessageGuardianApprovalReviewCompleted(payload) => {
                assert_eq!(payload.chat_id, conversation_id.to_string());
                assert_eq!(payload.interaction_id, "turn-from-assessment");
                assert_eq!(payload.started_at_ms, 1_000);
                assert_eq!(payload.completed_at_ms, 1_042);
                assert_eq!(payload.review_id, "review-2");
                assert_eq!(payload.target_message_id.as_deref(), Some("item-2"));
                assert_eq!(payload.decision_source, AutoReviewDecisionSource::Agent);
                assert_eq!(payload.review.status, GuardianApprovalReviewStatus::Denied);
                assert_eq!(
                    payload.review.risk_level,
                    Some(datax_app_server_protocol::GuardianRiskLevel::High)
                );
                assert_eq!(
                    payload.review.user_authorization,
                    Some(datax_app_server_protocol::GuardianUserAuthorization::Low)
                );
                assert_eq!(payload.review.rationale.as_deref(), Some("too risky"));
                assert_eq!(payload.action, action.into());
            }
            other => panic!("unexpected notification: {other:?}"),
        }
    }

    #[test]
    fn guardian_assessment_aborted_emits_completed_review_payload() {
        let conversation_id = ChatId::new();
        let action = datax_protocol::protocol::GuardianAssessmentAction::NetworkAccess {
            target: "api.openai.com:443".to_string(),
            host: "api.openai.com".to_string(),
            protocol: datax_protocol::protocol::NetworkApprovalProtocol::Https,
            port: 443,
        };
        let notification = guardian_auto_approval_review_notification(
            &conversation_id,
            "turn-from-event",
            &GuardianAssessmentEvent {
                id: "review-3".to_string(),
                target_item_id: None,
                interaction_id: "turn-from-assessment".to_string(),
                started_at_ms: 1_000,
                completed_at_ms: Some(1_042),
                status: datax_protocol::protocol::GuardianAssessmentStatus::Aborted,
                risk_level: None,
                user_authorization: None,
                rationale: None,
                decision_source: Some(
                    datax_protocol::protocol::GuardianAssessmentDecisionSource::Agent,
                ),
                action: action.clone(),
            },
        );

        match notification {
            MessageGuardianApprovalReviewCompleted(payload) => {
                assert_eq!(payload.chat_id, conversation_id.to_string());
                assert_eq!(payload.interaction_id, "turn-from-assessment");
                assert_eq!(payload.review_id, "review-3");
                assert_eq!(payload.target_message_id, None);
                assert_eq!(payload.decision_source, AutoReviewDecisionSource::Agent);
                assert_eq!(payload.review.status, GuardianApprovalReviewStatus::Aborted);
                assert_eq!(payload.review.risk_level, None);
                assert_eq!(payload.review.user_authorization, None);
                assert_eq!(payload.review.rationale, None);
                assert_eq!(payload.action, action.into());
            }
            other => panic!("unexpected notification: {other:?}"),
        }
    }

    #[tokio::test]
    async fn command_execution_started_helper_emits_once() -> Result<()> {
        let conversation_id = ChatId::new();
        let chat_state = new_chat_state();
        let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        ));
        let outgoing =
            ThreadScopedOutgoingMessageSender::new(outgoing, vec![ConnectionId(1)], ChatId::new());
        let completion_item = command_execution_completion_item("printf hi");

        let first_start = start_command_execution_item(
            &conversation_id,
            "turn-1".to_string(),
            "cmd-1".to_string(),
            completion_item.command.clone(),
            completion_item.cwd.clone(),
            completion_item.command_actions.clone(),
            CommandExecutionSource::Agent,
            &outgoing,
            &chat_state,
        )
        .await;
        assert!(first_start);

        let msg = recv_broadcast_message(&mut rx).await?;
        match msg {
            OutgoingMessage::AppServerNotification(MessageStarted(payload)) => {
                assert_eq!(payload.chat_id, conversation_id.to_string());
                assert_eq!(payload.interaction_id, "turn-1");
                assert_eq!(
                    payload.item,
                    Message::CommandExecution {
                        id: "cmd-1".to_string(),
                        command: completion_item.command.clone(),
                        cwd: completion_item.cwd.clone(),
                        process_id: None,
                        source: CommandExecutionSource::Agent,
                        status: CommandExecutionStatus::InProgress,
                        command_actions: completion_item.command_actions.clone(),
                        aggregated_output: None,
                        exit_code: None,
                        duration_ms: None,
                    }
                );
            }
            other => bail!("unexpected message: {other:?}"),
        }

        let second_start = start_command_execution_item(
            &conversation_id,
            "turn-1".to_string(),
            "cmd-1".to_string(),
            completion_item.command.clone(),
            completion_item.cwd.clone(),
            completion_item.command_actions.clone(),
            CommandExecutionSource::Agent,
            &outgoing,
            &chat_state,
        )
        .await;
        assert!(!second_start);
        assert!(rx.try_recv().is_err(), "duplicate start should not emit");
        Ok(())
    }

    #[tokio::test]
    async fn complete_command_execution_item_emits_declined_once_for_pending_command() -> Result<()>
    {
        let conversation_id = ChatId::new();
        let chat_state = new_chat_state();
        let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        ));
        let outgoing =
            ThreadScopedOutgoingMessageSender::new(outgoing, vec![ConnectionId(1)], ChatId::new());
        let completion_item = command_execution_completion_item("printf hi");

        start_command_execution_item(
            &conversation_id,
            "turn-1".to_string(),
            "cmd-1".to_string(),
            completion_item.command.clone(),
            completion_item.cwd.clone(),
            completion_item.command_actions.clone(),
            CommandExecutionSource::Agent,
            &outgoing,
            &chat_state,
        )
        .await;
        let _started = recv_broadcast_message(&mut rx).await?;

        complete_command_execution_item(
            &conversation_id,
            "turn-1".to_string(),
            "cmd-1".to_string(),
            completion_item.command.clone(),
            completion_item.cwd.clone(),
            /*process_id*/ None,
            CommandExecutionSource::Agent,
            completion_item.command_actions.clone(),
            CommandExecutionStatus::Declined,
            &outgoing,
            &chat_state,
        )
        .await;

        let completed = recv_broadcast_message(&mut rx).await?;
        match completed {
            OutgoingMessage::AppServerNotification(MessageCompleted(payload)) => {
                let Message::CommandExecution { id, status, .. } = payload.item else {
                    bail!("expected command execution completion");
                };
                assert_eq!(id, "cmd-1");
                assert_eq!(status, CommandExecutionStatus::Declined);
            }
            other => bail!("unexpected message: {other:?}"),
        }

        complete_command_execution_item(
            &conversation_id,
            "turn-1".to_string(),
            "cmd-1".to_string(),
            completion_item.command,
            completion_item.cwd,
            /*process_id*/ None,
            CommandExecutionSource::Agent,
            completion_item.command_actions,
            CommandExecutionStatus::Declined,
            &outgoing,
            &chat_state,
        )
        .await;
        assert!(
            rx.try_recv().is_err(),
            "completion should not emit after the pending item is cleared"
        );
        Ok(())
    }

    #[tokio::test]
    async fn guardian_command_execution_notifications_wrap_review_lifecycle() -> Result<()> {
        let codex_home = TempDir::new()?;
        let config = load_default_config_for_test(&codex_home).await;
        let chat_manager = Arc::new(
            datax_core::test_support::chat_manager_with_models_provider_and_home(
                CodexAuth::create_dummy_chatgpt_auth_for_testing(),
                config.model_provider.clone(),
                config.codex_home.to_path_buf(),
                Arc::new(datax_exec_server::EnvironmentManager::default_for_tests()),
            ),
        );
        let datax_core::NewChat {
            chat_id: conversation_id,
            chat: conversation,
            ..
        } = chat_manager.start_chat(config.clone()).await?;
        let chat_state = new_chat_state();
        let chat_watch_manager = ChatWatchManager::new();
        let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        ));
        let outgoing = ThreadScopedOutgoingMessageSender::new(
            outgoing,
            vec![ConnectionId(1)],
            conversation_id,
        );
        let guardian_context = GuardianAssessmentTestContext {
            conversation_id,
            conversation: conversation.clone(),
            chat_manager: chat_manager.clone(),
            outgoing: outgoing.clone(),
            chat_state: chat_state.clone(),
            chat_watch_manager: chat_watch_manager.clone(),
        };

        guardian_context
            .apply_guardian_assessment_event(guardian_command_assessment(
                "cmd-guardian-approved",
                "turn-guardian-approved",
                GuardianAssessmentStatus::InProgress,
            ))
            .await;
        let first = recv_broadcast_message(&mut rx).await?;
        match first {
            OutgoingMessage::AppServerNotification(MessageStarted(payload)) => {
                assert_eq!(payload.interaction_id, "turn-guardian-approved");
                let Message::CommandExecution { id, status, .. } = payload.item else {
                    bail!("expected command execution item");
                };
                assert_eq!(id, "cmd-guardian-approved");
                assert_eq!(status, CommandExecutionStatus::InProgress);
            }
            other => bail!("unexpected message: {other:?}"),
        }
        let second = recv_broadcast_message(&mut rx).await?;
        match second {
            OutgoingMessage::AppServerNotification(MessageGuardianApprovalReviewStarted(
                payload,
            )) => {
                assert_eq!(payload.review_id, "review-cmd-guardian-approved");
                assert_eq!(
                    payload.target_message_id.as_deref(),
                    Some("cmd-guardian-approved")
                );
                assert_eq!(
                    payload.review.status,
                    GuardianApprovalReviewStatus::InProgress
                );
            }
            other => bail!("unexpected message: {other:?}"),
        }

        guardian_context
            .apply_guardian_assessment_event(guardian_command_assessment(
                "cmd-guardian-approved",
                "turn-guardian-approved",
                GuardianAssessmentStatus::Approved,
            ))
            .await;
        let third = recv_broadcast_message(&mut rx).await?;
        match third {
            OutgoingMessage::AppServerNotification(MessageGuardianApprovalReviewCompleted(
                payload,
            )) => {
                assert_eq!(payload.review_id, "review-cmd-guardian-approved");
                assert_eq!(
                    payload.target_message_id.as_deref(),
                    Some("cmd-guardian-approved")
                );
                assert_eq!(payload.decision_source, AutoReviewDecisionSource::Agent);
                assert_eq!(
                    payload.review.status,
                    GuardianApprovalReviewStatus::Approved
                );
            }
            other => bail!("unexpected message: {other:?}"),
        }
        assert!(
            rx.try_recv().is_err(),
            "approved review should not complete the command item"
        );

        guardian_context
            .apply_guardian_assessment_event(guardian_command_assessment(
                "cmd-guardian-denied",
                "turn-guardian-denied",
                GuardianAssessmentStatus::InProgress,
            ))
            .await;
        let fourth = recv_broadcast_message(&mut rx).await?;
        match fourth {
            OutgoingMessage::AppServerNotification(MessageStarted(payload)) => {
                assert_eq!(payload.interaction_id, "turn-guardian-denied");
                let Message::CommandExecution { id, status, .. } = payload.item else {
                    bail!("expected command execution item");
                };
                assert_eq!(id, "cmd-guardian-denied");
                assert_eq!(status, CommandExecutionStatus::InProgress);
            }
            other => bail!("unexpected message: {other:?}"),
        }
        let fifth = recv_broadcast_message(&mut rx).await?;
        match fifth {
            OutgoingMessage::AppServerNotification(MessageGuardianApprovalReviewStarted(
                payload,
            )) => {
                assert_eq!(payload.review_id, "review-cmd-guardian-denied");
                assert_eq!(
                    payload.target_message_id.as_deref(),
                    Some("cmd-guardian-denied")
                );
                assert_eq!(
                    payload.review.status,
                    GuardianApprovalReviewStatus::InProgress
                );
            }
            other => bail!("unexpected message: {other:?}"),
        }

        guardian_context
            .apply_guardian_assessment_event(guardian_command_assessment(
                "cmd-guardian-denied",
                "turn-guardian-denied",
                GuardianAssessmentStatus::Denied,
            ))
            .await;
        let sixth = recv_broadcast_message(&mut rx).await?;
        match sixth {
            OutgoingMessage::AppServerNotification(MessageGuardianApprovalReviewCompleted(
                payload,
            )) => {
                assert_eq!(payload.review_id, "review-cmd-guardian-denied");
                assert_eq!(
                    payload.target_message_id.as_deref(),
                    Some("cmd-guardian-denied")
                );
                assert_eq!(payload.decision_source, AutoReviewDecisionSource::Agent);
                assert_eq!(payload.review.status, GuardianApprovalReviewStatus::Denied);
            }
            other => bail!("unexpected message: {other:?}"),
        }
        let seventh = recv_broadcast_message(&mut rx).await?;
        match seventh {
            OutgoingMessage::AppServerNotification(MessageCompleted(payload)) => {
                let Message::CommandExecution { id, status, .. } = payload.item else {
                    bail!("expected command execution completion");
                };
                assert_eq!(id, "cmd-guardian-denied");
                assert_eq!(status, CommandExecutionStatus::Declined);
            }
            other => bail!("unexpected message: {other:?}"),
        }

        let mut missing_target = guardian_command_assessment(
            "cmd-guardian-missing-target",
            "turn-guardian-missing-target",
            GuardianAssessmentStatus::InProgress,
        );
        missing_target.target_item_id = None;
        guardian_context
            .apply_guardian_assessment_event(missing_target)
            .await;
        let eighth = recv_broadcast_message(&mut rx).await?;
        match eighth {
            OutgoingMessage::AppServerNotification(MessageGuardianApprovalReviewStarted(
                payload,
            )) => {
                assert_eq!(payload.review_id, "review-cmd-guardian-missing-target");
                assert_eq!(payload.target_message_id, None);
                assert_eq!(
                    payload.review.status,
                    GuardianApprovalReviewStatus::InProgress
                );
            }
            other => bail!("unexpected message: {other:?}"),
        }

        assert!(rx.try_recv().is_err(), "no extra messages expected");
        conversation.shutdown_and_wait().await?;
        Ok(())
    }

    #[test]
    fn file_change_accept_for_session_maps_to_approved_for_session() {
        let decision =
            map_file_change_approval_decision(FileChangeApprovalDecision::AcceptForSession);
        assert_eq!(decision, ReviewDecision::ApprovedForSession);
    }

    #[test]
    fn mcp_server_elicitation_turn_transition_error_maps_to_cancel() {
        let error = JSONRPCErrorError {
            code: -1,
            message: "client request resolved because the turn state was changed".to_string(),
            data: Some(serde_json::json!({ "reason": "turnTransition" })),
        };

        let response = mcp_server_elicitation_response_from_client_result(Ok(Err(error)));

        assert_eq!(
            response,
            McpServerElicitationRequestResponse {
                action: McpServerElicitationAction::Cancel,
                content: None,
                meta: None,
            }
        );
    }

    #[test]
    fn request_permissions_turn_transition_error_is_ignored() {
        let error = JSONRPCErrorError {
            code: -1,
            message: "client request resolved because the turn state was changed".to_string(),
            data: Some(serde_json::json!({ "reason": "turnTransition" })),
        };

        let response = request_permissions_response_from_client_result(
            CoreRequestPermissionProfile::default(),
            Ok(Err(error)),
            std::env::current_dir().expect("current dir").as_path(),
        )
        .expect("paths should localize");

        assert_eq!(response, None);
    }

    #[test]
    fn request_permissions_response_accepts_partial_network_and_file_system_grants() {
        let input_path = if cfg!(target_os = "windows") {
            r"C:\tmp\input"
        } else {
            "/tmp/input"
        };
        let output_path = if cfg!(target_os = "windows") {
            r"C:\tmp\output"
        } else {
            "/tmp/output"
        };
        let ignored_path = if cfg!(target_os = "windows") {
            r"C:\tmp\ignored"
        } else {
            "/tmp/ignored"
        };
        let absolute_path = |path: &str| {
            AbsolutePathBuf::try_from(std::path::PathBuf::from(path)).expect("absolute path")
        };
        let requested_permissions = CoreRequestPermissionProfile {
            network: Some(CoreNetworkPermissions {
                enabled: Some(true),
            }),
            file_system: Some(CoreFileSystemPermissions::from_read_write_roots(
                Some(vec![absolute_path(input_path)]),
                Some(vec![absolute_path(output_path)]),
            )),
        };
        let cases = vec![
            (
                serde_json::json!({}),
                CoreRequestPermissionProfile::default(),
            ),
            (
                serde_json::json!({
                    "network": {
                        "enabled": true,
                    },
                }),
                CoreRequestPermissionProfile {
                    network: Some(CoreNetworkPermissions {
                        enabled: Some(true),
                    }),
                    ..CoreRequestPermissionProfile::default()
                },
            ),
            (
                serde_json::json!({
                    "fileSystem": {
                        "write": [output_path],
                    },
                }),
                CoreRequestPermissionProfile {
                    file_system: Some(CoreFileSystemPermissions::from_read_write_roots(
                        /*read*/ None,
                        Some(vec![absolute_path(output_path)]),
                    )),
                    ..CoreRequestPermissionProfile::default()
                },
            ),
            (
                serde_json::json!({
                    "fileSystem": {
                        "read": [input_path],
                        "write": [output_path, ignored_path],
                    },
                    "macos": {
                        "calendar": true,
                    },
                }),
                CoreRequestPermissionProfile {
                    file_system: Some(CoreFileSystemPermissions::from_read_write_roots(
                        Some(vec![absolute_path(input_path)]),
                        Some(vec![absolute_path(output_path)]),
                    )),
                    ..CoreRequestPermissionProfile::default()
                },
            ),
        ];

        let cwd = std::env::current_dir().expect("current dir");
        for (granted_permissions, expected_permissions) in cases {
            let response = request_permissions_response_from_client_result(
                requested_permissions.clone(),
                Ok(Ok(serde_json::json!({
                    "permissions": granted_permissions,
                }))),
                cwd.as_path(),
            )
            .expect("paths should localize")
            .expect("response should be accepted");

            assert_eq!(
                response,
                CoreRequestPermissionsResponse {
                    permissions: expected_permissions,
                    scope: CorePermissionGrantScope::Turn,
                    strict_auto_review: false,
                }
            );
        }
    }

    #[test]
    fn request_permissions_response_preserves_session_scope() {
        let response = request_permissions_response_from_client_result(
            CoreRequestPermissionProfile::default(),
            Ok(Ok(serde_json::json!({
                "scope": "session",
                "permissions": {},
            }))),
            std::env::current_dir().expect("current dir").as_path(),
        )
        .expect("paths should localize")
        .expect("response should be accepted");

        assert_eq!(
            response,
            CoreRequestPermissionsResponse {
                permissions: CoreRequestPermissionProfile::default(),
                scope: CorePermissionGrantScope::Session,
                strict_auto_review: false,
            }
        );
    }

    #[test]
    fn request_permissions_response_rejects_session_scoped_strict_auto_review() {
        let response = request_permissions_response_from_client_result(
            CoreRequestPermissionProfile::default(),
            Ok(Ok(serde_json::json!({
                "scope": "session",
                "strictAutoReview": true,
                "permissions": {
                    "network": {
                        "enabled": true,
                    },
                },
            }))),
            std::env::current_dir().expect("current dir").as_path(),
        )
        .expect("paths should localize")
        .expect("response should be accepted");

        assert_eq!(
            response,
            CoreRequestPermissionsResponse {
                permissions: CoreRequestPermissionProfile::default(),
                scope: CorePermissionGrantScope::Turn,
                strict_auto_review: false,
            }
        );
    }

    #[test]
    fn request_permissions_response_preserves_turn_scoped_strict_auto_review() {
        let response = request_permissions_response_from_client_result(
            CoreRequestPermissionProfile {
                network: Some(datax_protocol::models::NetworkPermissions {
                    enabled: Some(true),
                }),
                ..Default::default()
            },
            Ok(Ok(serde_json::json!({
                "strictAutoReview": true,
                "permissions": {
                    "network": {
                        "enabled": true,
                    },
                },
            }))),
            std::env::current_dir().expect("current dir").as_path(),
        )
        .expect("paths should localize")
        .expect("response should be accepted");

        assert_eq!(response.scope, CorePermissionGrantScope::Turn);
        assert!(response.strict_auto_review);
    }

    #[test]
    fn request_permissions_response_accepts_explicit_child_grant_for_requested_cwd_scope() {
        let temp_dir = TempDir::new().expect("temp dir");
        let cwd = AbsolutePathBuf::from_absolute_path(temp_dir.path()).expect("absolute cwd");
        let child = cwd.join("child");
        let requested_permissions = CoreRequestPermissionProfile {
            file_system: Some(CoreFileSystemPermissions {
                entries: vec![FileSystemSandboxEntry {
                    path: FileSystemPath::Special {
                        value: FileSystemSpecialPath::project_roots(/*subpath*/ None),
                    },
                    access: FileSystemAccessMode::Write,
                }],
                glob_scan_max_depth: None,
            }),
            ..Default::default()
        };

        let response = request_permissions_response_from_client_result(
            requested_permissions,
            Ok(Ok(serde_json::json!({
                "permissions": {
                    "fileSystem": {
                        "write": [child],
                    },
                },
            }))),
            cwd.as_path(),
        )
        .expect("paths should localize")
        .expect("response should be accepted");

        assert_eq!(
            response.permissions,
            CoreRequestPermissionProfile {
                file_system: Some(CoreFileSystemPermissions::from_read_write_roots(
                    /*read*/ None,
                    Some(vec![child]),
                )),
                ..Default::default()
            }
        );
    }

    #[test]
    fn request_permissions_response_rejects_child_grant_outside_requested_cwd_scope() {
        let temp_dir = TempDir::new().expect("temp dir");
        let request_cwd = AbsolutePathBuf::from_absolute_path(temp_dir.path().join("request-cwd"))
            .expect("absolute request cwd");
        let later_cwd = AbsolutePathBuf::from_absolute_path(temp_dir.path().join("later-cwd"))
            .expect("absolute later cwd");
        let later_child = later_cwd.join("child");
        let requested_permissions = CoreRequestPermissionProfile {
            file_system: Some(CoreFileSystemPermissions {
                entries: vec![FileSystemSandboxEntry {
                    path: FileSystemPath::Special {
                        value: FileSystemSpecialPath::project_roots(/*subpath*/ None),
                    },
                    access: FileSystemAccessMode::Write,
                }],
                glob_scan_max_depth: None,
            }),
            ..Default::default()
        };

        let response = request_permissions_response_from_client_result(
            requested_permissions,
            Ok(Ok(serde_json::json!({
                "permissions": {
                    "fileSystem": {
                        "write": [later_child],
                    },
                },
            }))),
            request_cwd.as_path(),
        )
        .expect("paths should localize")
        .expect("response should be accepted");

        assert_eq!(
            response.permissions,
            CoreRequestPermissionProfile::default()
        );
    }

    #[test]
    fn request_permissions_response_ignores_broader_cwd_grant_for_requested_child_path() {
        let temp_dir = TempDir::new().expect("temp dir");
        let cwd = AbsolutePathBuf::from_absolute_path(temp_dir.path()).expect("absolute cwd");
        let child = cwd.join("child");
        let requested_permissions = CoreRequestPermissionProfile {
            file_system: Some(CoreFileSystemPermissions::from_read_write_roots(
                /*read*/ None,
                Some(vec![child]),
            )),
            ..Default::default()
        };

        let response = request_permissions_response_from_client_result(
            requested_permissions,
            Ok(Ok(serde_json::json!({
                "permissions": {
                    "fileSystem": {
                        "entries": [{
                            "path": {
                                "type": "special",
                                "value": {
                                    "kind": "project_roots",
                                    "subpath": null
                                }
                            },
                            "access": "write"
                        }],
                    },
                },
            }))),
            cwd.as_path(),
        )
        .expect("paths should localize")
        .expect("response should be accepted");

        assert_eq!(
            response.permissions,
            CoreRequestPermissionProfile::default()
        );
    }

    #[tokio::test]
    async fn test_handle_error_records_message() -> Result<()> {
        let conversation_id = ChatId::new();
        let chat_state = new_chat_state();

        handle_error(
            conversation_id,
            InteractionError {
                message: "boom".to_string(),
                codex_error_info: Some(V2CodexErrorInfo::InternalServerError),
                additional_details: None,
            },
            &chat_state,
        )
        .await;

        let interaction_summary = find_and_remove_interaction_summary(conversation_id, &chat_state).await;
        assert_eq!(
            interaction_summary.last_error,
            Some(InteractionError {
                message: "boom".to_string(),
                codex_error_info: Some(V2CodexErrorInfo::InternalServerError),
                additional_details: None,
            })
        );
        Ok(())
    }

    #[tokio::test]
    async fn turn_started_omits_active_snapshot_items() -> Result<()> {
        let codex_home = TempDir::new()?;
        let config = load_default_config_for_test(&codex_home).await;
        let chat_manager = Arc::new(
            datax_core::test_support::chat_manager_with_models_provider_and_home(
                CodexAuth::create_dummy_chatgpt_auth_for_testing(),
                config.model_provider.clone(),
                config.codex_home.to_path_buf(),
                Arc::new(datax_exec_server::EnvironmentManager::default_for_tests()),
            ),
        );
        let datax_core::NewChat {
            chat_id: conversation_id,
            chat: conversation,
            ..
        } = chat_manager.start_chat(config.clone()).await?;
        let chat_state = new_chat_state();
        {
            let mut state = chat_state.lock().await;
            state.track_current_interaction_event(
                "turn-1",
                &EventMsg::InteractionStarted(datax_protocol::protocol::InteractionStartedEvent {
                    interaction_id: "turn-1".to_string(),
                    trace_id: None,
                    started_at: Some(42),
                    model_context_window: None,
                    collaboration_mode_kind: Default::default(),
                }),
            );
            state.track_current_interaction_event(
                "turn-1",
                &EventMsg::UserMessage(datax_protocol::protocol::UserMessageEvent {
                    client_id: None,
                    message: "already tracked".to_string(),
                    images: None,
                    local_images: Vec::new(),
                    text_elements: Vec::new(),
                    ..Default::default()
                }),
            );
        }
        let chat_watch_manager = ChatWatchManager::new();
        let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        ));
        let outgoing = ThreadScopedOutgoingMessageSender::new(
            outgoing,
            vec![ConnectionId(1)],
            conversation_id,
        );

        apply_bespoke_event_handling(
            Event {
                id: "turn-1".to_string(),
                msg: EventMsg::InteractionStarted(
                    datax_protocol::protocol::InteractionStartedEvent {
                        interaction_id: "turn-1".to_string(),
                        trace_id: None,
                        started_at: Some(42),
                        model_context_window: None,
                        collaboration_mode_kind: Default::default(),
                    },
                ),
            },
            conversation_id,
            conversation,
            chat_manager,
            outgoing,
            chat_state,
            chat_watch_manager,
            Arc::new(tokio::sync::Semaphore::new(/*permits*/ 1)),
            "test-provider".to_string(),
        )
        .await;

        let msg = recv_broadcast_message(&mut rx).await?;
        match msg {
            OutgoingMessage::AppServerNotification(InteractionStarted(n)) => {
                assert_eq!(n.interaction.id, "turn-1");
                assert_eq!(
                    n.interaction.messages_view,
                    InteractionMessagesView::NotLoaded
                );
                assert!(n.interaction.messages.is_empty());
            }
            other => bail!("unexpected message: {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn interrupted_subagent_activity_removes_missing_thread_watch() -> Result<()> {
        let codex_home = TempDir::new()?;
        let config = load_default_config_for_test(&codex_home).await;
        let chat_manager = Arc::new(
            datax_core::test_support::chat_manager_with_models_provider_and_home(
                CodexAuth::create_dummy_chatgpt_auth_for_testing(),
                config.model_provider.clone(),
                config.codex_home.to_path_buf(),
                Arc::new(datax_exec_server::EnvironmentManager::default_for_tests()),
            ),
        );
        let datax_core::NewChat {
            chat_id: conversation_id,
            chat: conversation,
            ..
        } = chat_manager.start_chat(config).await?;
        let child_chat_id = ChatId::new();
        let child_chat_id_string = child_chat_id.to_string();
        let chat_watch_manager = ChatWatchManager::new();
        chat_watch_manager
            .note_interaction_started(&child_chat_id_string)
            .await;
        assert_eq!(chat_watch_manager.running_interaction_count().await, 1);
        let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        ));
        let outgoing = ThreadScopedOutgoingMessageSender::new(
            outgoing,
            vec![ConnectionId(1)],
            conversation_id,
        );

        apply_bespoke_event_handling(
            Event {
                id: "turn-1".to_string(),
                msg: EventMsg::SubAgentActivity(SubAgentActivityEvent {
                    event_id: "activity-1".to_string(),
                    occurred_at_ms: 42,
                    agent_chat_id: child_chat_id,
                    agent_path: AgentPath::try_from("/root/worker")
                        .expect("agent path should parse"),
                    kind: SubAgentActivityKind::Interrupted,
                }),
            },
            conversation_id,
            conversation,
            chat_manager,
            outgoing,
            new_chat_state(),
            chat_watch_manager.clone(),
            Arc::new(tokio::sync::Semaphore::new(/*permits*/ 1)),
            "test-provider".to_string(),
        )
        .await;

        assert_eq!(
            chat_watch_manager
                .loaded_status_for_chat(&child_chat_id_string)
                .await,
            ChatStatus::NotLoaded
        );
        assert_eq!(chat_watch_manager.running_interaction_count().await, 0);
        let message = recv_broadcast_message(&mut rx).await?;
        let OutgoingMessage::AppServerNotification(MessageCompleted(payload)) = message else {
            bail!("unexpected message: {message:?}");
        };
        assert_eq!(
            payload,
            MessageCompletedNotification {
                item: Message::SubAgentActivity {
                    id: "activity-1".to_string(),
                    kind: datax_app_server_protocol::SubAgentActivityKind::Interrupted,
                    agent_chat_id: child_chat_id_string,
                    agent_path: "/root/worker".to_string(),
                },
                chat_id: conversation_id.to_string(),
                interaction_id: "turn-1".to_string(),
                completed_at_ms: 42,
            }
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_handle_turn_complete_emits_completed_without_error() -> Result<()> {
        let conversation_id = ChatId::new();
        let event_interaction_id = "complete1".to_string();
        let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        ));
        let outgoing =
            ThreadScopedOutgoingMessageSender::new(outgoing, vec![ConnectionId(1)], ChatId::new());
        let chat_state = new_chat_state();
        {
            let mut state = chat_state.lock().await;
            state.track_current_interaction_event(
                &event_interaction_id,
                &EventMsg::InteractionStarted(datax_protocol::protocol::InteractionStartedEvent {
                    interaction_id: event_interaction_id.clone(),
                    trace_id: None,
                    started_at: Some(42),
                    model_context_window: None,
                    collaboration_mode_kind: Default::default(),
                }),
            );
            state.track_current_interaction_event(
                &event_interaction_id,
                &EventMsg::InteractionComplete(turn_complete_event(&event_interaction_id)),
            );
        }

        handle_turn_complete(
            conversation_id,
            event_interaction_id.clone(),
            turn_complete_event(&event_interaction_id),
            &outgoing,
            &chat_state,
        )
        .await;

        let msg = recv_broadcast_message(&mut rx).await?;
        match msg {
            OutgoingMessage::AppServerNotification(InteractionCompleted(n)) => {
                assert_eq!(n.interaction.id, event_interaction_id);
                assert_eq!(n.interaction.status, InteractionStatus::Completed);
                assert_eq!(
                    n.interaction.messages_view,
                    InteractionMessagesView::NotLoaded
                );
                assert!(n.interaction.messages.is_empty());
                assert_eq!(n.interaction.error, None);
                assert_eq!(n.interaction.started_at, Some(42));
                assert_eq!(n.interaction.completed_at, Some(TEST_TURN_COMPLETED_AT));
                assert_eq!(n.interaction.duration_ms, Some(TEST_TURN_DURATION_MS));
            }
            other => bail!("unexpected message: {other:?}"),
        }
        assert!(rx.try_recv().is_err(), "no extra messages expected");
        Ok(())
    }

    #[tokio::test]
    async fn test_handle_turn_interrupted_emits_interrupted_with_error() -> Result<()> {
        let conversation_id = ChatId::new();
        let event_interaction_id = "interrupt1".to_string();
        let chat_state = new_chat_state();
        handle_error(
            conversation_id,
            InteractionError {
                message: "oops".to_string(),
                codex_error_info: None,
                additional_details: None,
            },
            &chat_state,
        )
        .await;
        let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        ));
        let outgoing =
            ThreadScopedOutgoingMessageSender::new(outgoing, vec![ConnectionId(1)], ChatId::new());

        handle_turn_interrupted(
            conversation_id,
            event_interaction_id.clone(),
            turn_aborted_event(&event_interaction_id),
            &outgoing,
            &chat_state,
        )
        .await;

        let msg = recv_broadcast_message(&mut rx).await?;
        match msg {
            OutgoingMessage::AppServerNotification(InteractionCompleted(n)) => {
                assert_eq!(n.interaction.id, event_interaction_id);
                assert_eq!(n.interaction.status, InteractionStatus::Interrupted);
                assert_eq!(n.interaction.error, None);
                assert_eq!(n.interaction.completed_at, Some(TEST_TURN_COMPLETED_AT));
                assert_eq!(n.interaction.duration_ms, Some(TEST_TURN_DURATION_MS));
            }
            other => bail!("unexpected message: {other:?}"),
        }
        assert!(rx.try_recv().is_err(), "no extra messages expected");
        Ok(())
    }

    #[tokio::test]
    async fn test_handle_turn_complete_emits_failed_with_error() -> Result<()> {
        let conversation_id = ChatId::new();
        let event_interaction_id = "complete_err1".to_string();
        let chat_state = new_chat_state();
        handle_error(
            conversation_id,
            InteractionError {
                message: "bad".to_string(),
                codex_error_info: Some(V2CodexErrorInfo::Other),
                additional_details: None,
            },
            &chat_state,
        )
        .await;
        let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        ));
        let outgoing =
            ThreadScopedOutgoingMessageSender::new(outgoing, vec![ConnectionId(1)], ChatId::new());

        handle_turn_complete(
            conversation_id,
            event_interaction_id.clone(),
            turn_complete_event(&event_interaction_id),
            &outgoing,
            &chat_state,
        )
        .await;

        let msg = recv_broadcast_message(&mut rx).await?;
        match msg {
            OutgoingMessage::AppServerNotification(InteractionCompleted(n)) => {
                assert_eq!(n.interaction.id, event_interaction_id);
                assert_eq!(n.interaction.status, InteractionStatus::Failed);
                assert_eq!(
                    n.interaction.error,
                    Some(InteractionError {
                        message: "bad".to_string(),
                        codex_error_info: Some(V2CodexErrorInfo::Other),
                        additional_details: None,
                    })
                );
                assert_eq!(n.interaction.completed_at, Some(TEST_TURN_COMPLETED_AT));
                assert_eq!(n.interaction.duration_ms, Some(TEST_TURN_DURATION_MS));
            }
            other => bail!("unexpected message: {other:?}"),
        }
        assert!(rx.try_recv().is_err(), "no extra messages expected");
        Ok(())
    }

    #[tokio::test]
    async fn test_handle_turn_plan_update_emits_notification_for_v2() -> Result<()> {
        let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        ));
        let outgoing =
            ThreadScopedOutgoingMessageSender::new(outgoing, vec![ConnectionId(1)], ChatId::new());
        let update = UpdatePlanArgs {
            explanation: Some("need plan".to_string()),
            plan: vec![
                PlanItemArg {
                    step: "first".to_string(),
                    status: StepStatus::Pending,
                },
                PlanItemArg {
                    step: "second".to_string(),
                    status: StepStatus::Completed,
                },
            ],
        };

        let conversation_id = ChatId::new();

        handle_turn_plan_update(conversation_id, "turn-123", update, &outgoing).await;

        let msg = recv_broadcast_message(&mut rx).await?;
        match msg {
            OutgoingMessage::AppServerNotification(InteractionPlanUpdated(n)) => {
                assert_eq!(n.chat_id, conversation_id.to_string());
                assert_eq!(n.interaction_id, "turn-123");
                assert_eq!(n.explanation.as_deref(), Some("need plan"));
                assert_eq!(n.plan.len(), 2);
                assert_eq!(n.plan[0].step, "first");
                assert_eq!(n.plan[0].status, InteractionPlanStepStatus::Pending);
                assert_eq!(n.plan[1].step, "second");
                assert_eq!(n.plan[1].status, InteractionPlanStepStatus::Completed);
            }
            other => bail!("unexpected message: {other:?}"),
        }
        assert!(rx.try_recv().is_err(), "no extra messages expected");
        Ok(())
    }

    #[tokio::test]
    async fn test_handle_token_count_event_emits_usage_and_rate_limits() -> Result<()> {
        let conversation_id = ChatId::new();
        let interaction_id = "turn-123".to_string();
        let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        ));
        let outgoing =
            ThreadScopedOutgoingMessageSender::new(outgoing, vec![ConnectionId(1)], ChatId::new());

        let info = TokenUsageInfo {
            total_token_usage: TokenUsage {
                input_tokens: 100,
                cached_input_tokens: 25,
                output_tokens: 50,
                reasoning_output_tokens: 9,
                total_tokens: 200,
            },
            last_token_usage: TokenUsage {
                input_tokens: 10,
                cached_input_tokens: 5,
                output_tokens: 7,
                reasoning_output_tokens: 1,
                total_tokens: 23,
            },
            model_context_window: Some(4096),
        };
        let rate_limits = RateLimitSnapshot {
            limit_id: Some("codex".to_string()),
            limit_name: None,
            primary: Some(RateLimitWindow {
                used_percent: 42.5,
                window_minutes: Some(15),
                resets_at: Some(1700000000),
            }),
            secondary: None,
            credits: Some(CreditsSnapshot {
                has_credits: true,
                unlimited: false,
                balance: Some("5".to_string()),
            }),
            individual_limit: None,
            plan_type: None,
            rate_limit_reached_type: None,
        };

        handle_token_count_event(
            conversation_id,
            interaction_id.clone(),
            TokenCountEvent {
                info: Some(info),
                rate_limits: Some(rate_limits),
            },
            &outgoing,
        )
        .await;

        let first = recv_broadcast_message(&mut rx).await?;
        match first {
            OutgoingMessage::AppServerNotification(ChatTokenUsageUpdated(payload)) => {
                assert_eq!(payload.chat_id, conversation_id.to_string());
                assert_eq!(payload.interaction_id, interaction_id);
                let usage = payload.token_usage;
                assert_eq!(usage.total.total_tokens, 200);
                assert_eq!(usage.total.cached_input_tokens, 25);
                assert_eq!(usage.last.output_tokens, 7);
                assert_eq!(usage.model_context_window, Some(4096));
            }
            other => bail!("unexpected notification: {other:?}"),
        }

        let second = recv_broadcast_message(&mut rx).await?;
        match second {
            OutgoingMessage::AppServerNotification(
                ServerNotification::AccountRateLimitsUpdated(payload),
            ) => {
                assert_eq!(payload.rate_limits.limit_id.as_deref(), Some("codex"));
                assert_eq!(payload.rate_limits.limit_name, None);
                assert!(payload.rate_limits.primary.is_some());
                assert!(payload.rate_limits.credits.is_some());
            }
            other => bail!("unexpected notification: {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_handle_token_count_event_without_usage_info() -> Result<()> {
        let conversation_id = ChatId::new();
        let interaction_id = "turn-456".to_string();
        let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        ));
        let outgoing =
            ThreadScopedOutgoingMessageSender::new(outgoing, vec![ConnectionId(1)], ChatId::new());

        handle_token_count_event(
            conversation_id,
            interaction_id.clone(),
            TokenCountEvent {
                info: None,
                rate_limits: None,
            },
            &outgoing,
        )
        .await;

        assert!(
            rx.try_recv().is_err(),
            "no notifications should be emitted when token usage info is absent"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_handle_turn_complete_emits_error_multiple_turns() -> Result<()> {
        // Conversation A will have two interactions; Conversation B will have one turn.
        let conversation_a = ChatId::new();
        let conversation_b = ChatId::new();
        let chat_state = new_chat_state();

        let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        ));
        let outgoing =
            ThreadScopedOutgoingMessageSender::new(outgoing, vec![ConnectionId(1)], ChatId::new());

        // Interaction 1 on conversation A
        let a_turn1 = "a_turn1".to_string();
        handle_error(
            conversation_a,
            InteractionError {
                message: "a1".to_string(),
                codex_error_info: Some(V2CodexErrorInfo::BadRequest),
                additional_details: None,
            },
            &chat_state,
        )
        .await;
        handle_turn_complete(
            conversation_a,
            a_turn1.clone(),
            turn_complete_event(&a_turn1),
            &outgoing,
            &chat_state,
        )
        .await;

        // Interaction 1 on conversation B
        let b_turn1 = "b_turn1".to_string();
        handle_error(
            conversation_b,
            InteractionError {
                message: "b1".to_string(),
                codex_error_info: None,
                additional_details: None,
            },
            &chat_state,
        )
        .await;
        handle_turn_complete(
            conversation_b,
            b_turn1.clone(),
            turn_complete_event(&b_turn1),
            &outgoing,
            &chat_state,
        )
        .await;

        // Interaction 2 on conversation A
        let a_turn2 = "a_turn2".to_string();
        handle_turn_complete(
            conversation_a,
            a_turn2.clone(),
            turn_complete_event(&a_turn2),
            &outgoing,
            &chat_state,
        )
        .await;

        // Verify: A turn 1
        let msg = recv_broadcast_message(&mut rx).await?;
        match msg {
            OutgoingMessage::AppServerNotification(InteractionCompleted(n)) => {
                assert_eq!(n.interaction.id, a_turn1);
                assert_eq!(n.interaction.status, InteractionStatus::Failed);
                assert_eq!(
                    n.interaction.error,
                    Some(InteractionError {
                        message: "a1".to_string(),
                        codex_error_info: Some(V2CodexErrorInfo::BadRequest),
                        additional_details: None,
                    })
                );
            }
            other => bail!("unexpected message: {other:?}"),
        }

        // Verify: B turn 1
        let msg = recv_broadcast_message(&mut rx).await?;
        match msg {
            OutgoingMessage::AppServerNotification(InteractionCompleted(n)) => {
                assert_eq!(n.interaction.id, b_turn1);
                assert_eq!(n.interaction.status, InteractionStatus::Failed);
                assert_eq!(
                    n.interaction.error,
                    Some(InteractionError {
                        message: "b1".to_string(),
                        codex_error_info: None,
                        additional_details: None,
                    })
                );
            }
            other => bail!("unexpected message: {other:?}"),
        }

        // Verify: A turn 2
        let msg = recv_broadcast_message(&mut rx).await?;
        match msg {
            OutgoingMessage::AppServerNotification(InteractionCompleted(n)) => {
                assert_eq!(n.interaction.id, a_turn2);
                assert_eq!(n.interaction.status, InteractionStatus::Completed);
                assert_eq!(n.interaction.error, None);
            }
            other => bail!("unexpected message: {other:?}"),
        }

        assert!(rx.try_recv().is_err(), "no extra messages expected");
        Ok(())
    }

    #[tokio::test]
    async fn test_handle_turn_diff_emits_v2_notification() -> Result<()> {
        let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        ));
        let outgoing =
            ThreadScopedOutgoingMessageSender::new(outgoing, vec![ConnectionId(1)], ChatId::new());
        let unified_diff = "--- a\n+++ b\n".to_string();
        let conversation_id = ChatId::new();

        handle_turn_diff(
            conversation_id,
            "turn-1",
            InteractionDiffEvent {
                unified_diff: unified_diff.clone(),
            },
            &outgoing,
        )
        .await;

        let msg = recv_broadcast_message(&mut rx).await?;
        match msg {
            OutgoingMessage::AppServerNotification(InteractionDiffUpdated(notification)) => {
                assert_eq!(notification.chat_id, conversation_id.to_string());
                assert_eq!(notification.interaction_id, "turn-1");
                assert_eq!(notification.diff, unified_diff);
            }
            other => bail!("unexpected message: {other:?}"),
        }
        assert!(rx.try_recv().is_err(), "no extra messages expected");
        Ok(())
    }

    #[tokio::test]
    async fn test_hook_prompt_raw_response_emits_item_completed() -> Result<()> {
        let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        ));
        let conversation_id = ChatId::new();
        let outgoing = ThreadScopedOutgoingMessageSender::new(
            outgoing,
            vec![ConnectionId(1)],
            conversation_id,
        );
        let item = build_hook_prompt_message(&[
            HookPromptFragment::from_single_hook("Retry with tests.", "hook-run-1"),
            HookPromptFragment::from_single_hook("Then summarize cleanly.", "hook-run-2"),
        ])
        .expect("hook prompt message");

        maybe_emit_hook_prompt_item_completed(conversation_id, "turn-1", &item, &outgoing).await;

        let msg = recv_broadcast_message(&mut rx).await?;
        match msg {
            OutgoingMessage::AppServerNotification(MessageCompleted(notification)) => {
                assert_eq!(notification.chat_id, conversation_id.to_string());
                assert_eq!(notification.interaction_id, "turn-1");
                assert_eq!(
                    notification.item,
                    Message::HookPrompt {
                        id: notification.item.id().to_string(),
                        fragments: vec![
                            datax_app_server_protocol::HookPromptFragment {
                                text: "Retry with tests.".into(),
                                hook_run_id: "hook-run-1".into(),
                            },
                            datax_app_server_protocol::HookPromptFragment {
                                text: "Then summarize cleanly.".into(),
                                hook_run_id: "hook-run-2".into(),
                            },
                        ],
                    }
                );
            }
            other => bail!("unexpected message: {other:?}"),
        }
        assert!(rx.try_recv().is_err(), "no extra messages expected");
        Ok(())
    }
}
