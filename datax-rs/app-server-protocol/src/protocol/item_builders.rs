//! Shared builders for app-server [`Message`] values derived from compatibility events.
//!
//! Most live tool messages now come from first-class core `MessageStarted` / `MessageCompleted` events.
//! These builders remain for approval flows, rebuilt legacy history, and other pre-execution
//! paths where the underlying tool has not started or never starts at all.
//!
//! Keeping these builders in one place is useful for two reasons:
//! - Live notifications and rebuilt `chat/read` history both need to construct the same
//!   synthetic messages, so sharing the logic avoids drift between those paths.
//! - The projection is presentation-specific. Core protocol events stay generic, while the
//!   app-server protocol decides how to surface those events as `Message`s for clients.
use crate::protocol::common::ServerNotification;
use crate::protocol::v2::AutoReviewDecisionSource;
use crate::protocol::v2::CommandAction;
use crate::protocol::v2::CommandExecutionSource;
use crate::protocol::v2::CommandExecutionStatus;
use crate::protocol::v2::FileUpdateChange;
use crate::protocol::v2::GuardianApprovalReview;
use crate::protocol::v2::GuardianApprovalReviewStatus;
use crate::protocol::v2::Message;
use crate::protocol::v2::MessageGuardianApprovalReviewCompletedNotification;
use crate::protocol::v2::MessageGuardianApprovalReviewStartedNotification;
use crate::protocol::v2::PatchApplyStatus;
use crate::protocol::v2::PatchChangeKind;
use datax_protocol::ChatId;
use datax_protocol::parse_command::ParsedCommand;
use datax_protocol::protocol::ApplyPatchApprovalRequestEvent;
use datax_protocol::protocol::ExecApprovalRequestEvent;
use datax_protocol::protocol::ExecCommandBeginEvent;
use datax_protocol::protocol::ExecCommandEndEvent;
use datax_protocol::protocol::FileChange;
use datax_protocol::protocol::GuardianAssessmentAction;
use datax_protocol::protocol::GuardianAssessmentEvent;
use datax_protocol::protocol::PatchApplyBeginEvent;
use datax_protocol::protocol::PatchApplyEndEvent;
use datax_shell_command::parse_command::parse_command;
use datax_shell_command::parse_command::shlex_join;
use datax_utils_path_uri::PathConvention;
use datax_utils_path_uri::PathUri;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::warn;

pub fn build_file_change_approval_request_item(
    payload: &ApplyPatchApprovalRequestEvent,
) -> Message {
    Message::FileChange {
        id: payload.call_id.clone(),
        changes: convert_patch_changes(&payload.changes),
        status: PatchApplyStatus::InProgress,
    }
}

pub fn build_file_change_begin_item(payload: &PatchApplyBeginEvent) -> Message {
    Message::FileChange {
        id: payload.call_id.clone(),
        changes: convert_patch_changes(&payload.changes),
        status: PatchApplyStatus::InProgress,
    }
}

pub fn build_file_change_end_item(payload: &PatchApplyEndEvent) -> Message {
    Message::FileChange {
        id: payload.call_id.clone(),
        changes: convert_patch_changes(&payload.changes),
        status: (&payload.status).into(),
    }
}

pub fn build_command_execution_approval_request_item(
    payload: &ExecApprovalRequestEvent,
) -> Message {
    Message::CommandExecution {
        id: payload.call_id.clone(),
        command: shlex_join(&payload.command),
        cwd: payload.cwd.clone().into(),
        process_id: None,
        source: CommandExecutionSource::Agent,
        status: CommandExecutionStatus::InProgress,
        command_actions: payload
            .parsed_cmd
            .iter()
            .cloned()
            .map(|parsed| CommandAction::from_core_with_cwd(parsed, &payload.cwd))
            .collect(),
        aggregated_output: None,
        exit_code: None,
        duration_ms: None,
    }
}

pub fn build_command_execution_begin_item(payload: &ExecCommandBeginEvent) -> Message {
    let command_actions = command_actions_for_path_uri(&payload.parsed_cmd, &payload.cwd);
    Message::CommandExecution {
        id: payload.call_id.clone(),
        command: shlex_join(&payload.command),
        cwd: payload.cwd.clone().into(),
        process_id: payload.process_id.clone(),
        source: payload.source.into(),
        status: CommandExecutionStatus::InProgress,
        command_actions,
        aggregated_output: None,
        exit_code: None,
        duration_ms: None,
    }
}

pub fn build_command_execution_end_item(payload: &ExecCommandEndEvent) -> Message {
    let aggregated_output = if payload.aggregated_output.is_empty() {
        None
    } else {
        Some(payload.aggregated_output.clone())
    };
    let duration_ms = i64::try_from(payload.duration.as_millis()).unwrap_or(i64::MAX);
    let command_actions = command_actions_for_path_uri(&payload.parsed_cmd, &payload.cwd);

    Message::CommandExecution {
        id: payload.call_id.clone(),
        command: shlex_join(&payload.command),
        cwd: payload.cwd.clone().into(),
        process_id: payload.process_id.clone(),
        source: payload.source.into(),
        status: (&payload.status).into(),
        command_actions,
        aggregated_output,
        exit_code: Some(payload.exit_code),
        duration_ms: Some(duration_ms),
    }
}

fn command_actions_for_path_uri(parsed_cmd: &[ParsedCommand], cwd: &PathUri) -> Vec<CommandAction> {
    // TODO(anp): Carry PathUri into CommandAction so foreign Read actions retain resolved paths.
    // Until then, omit those actions rather than project a foreign cwd onto the host.
    let native_cwd = if cwd.infer_path_convention() == Some(PathConvention::native()) {
        cwd.to_abs_path().ok()
    } else {
        None
    };

    parsed_cmd
        .iter()
        .cloned()
        .filter_map(|parsed| match parsed {
            ParsedCommand::Read { cmd, name, path } => match native_cwd.as_ref() {
                Some(native_cwd) => Some(CommandAction::Read {
                    command: cmd,
                    name,
                    path: native_cwd.join(path),
                }),
                None => {
                    warn!(
                        command = cmd,
                        %cwd,
                        "omitting read command action whose path cannot be resolved against a foreign cwd"
                    );
                    None
                }
            },
            ParsedCommand::ListFiles { cmd, path } => {
                Some(CommandAction::ListFiles { command: cmd, path })
            }
            ParsedCommand::Search { cmd, query, path } => Some(CommandAction::Search {
                command: cmd,
                query,
                path,
            }),
            ParsedCommand::Unknown { cmd } => Some(CommandAction::Unknown { command: cmd }),
        })
        .collect()
}

/// Build a guardian-derived [`Message`].
///
/// Currently this only synthesizes [`Message::CommandExecution`] for
/// [`GuardianAssessmentAction::Command`] and [`GuardianAssessmentAction::Execve`].
pub fn build_item_from_guardian_event(
    assessment: &GuardianAssessmentEvent,
    status: CommandExecutionStatus,
) -> Option<Message> {
    match &assessment.action {
        GuardianAssessmentAction::Command { command, cwd, .. } => {
            let id = assessment.target_item_id.as_ref()?;
            let command = command.clone();
            let command_actions = vec![CommandAction::Unknown {
                command: command.clone(),
            }];
            Some(Message::CommandExecution {
                id: id.clone(),
                command,
                cwd: cwd.clone().into(),
                process_id: None,
                source: CommandExecutionSource::Agent,
                status,
                command_actions,
                aggregated_output: None,
                exit_code: None,
                duration_ms: None,
            })
        }
        GuardianAssessmentAction::Execve {
            program, argv, cwd, ..
        } => {
            let id = assessment.target_item_id.as_ref()?;
            let argv = if argv.is_empty() {
                vec![program.clone()]
            } else {
                std::iter::once(program.clone())
                    .chain(argv.iter().skip(1).cloned())
                    .collect::<Vec<_>>()
            };
            let command = shlex_join(&argv);
            let parsed_cmd = parse_command(&argv);
            let command_actions = if parsed_cmd.is_empty() {
                vec![CommandAction::Unknown {
                    command: command.clone(),
                }]
            } else {
                parsed_cmd
                    .into_iter()
                    .map(|parsed| CommandAction::from_core_with_cwd(parsed, cwd))
                    .collect()
            };
            Some(Message::CommandExecution {
                id: id.clone(),
                command,
                cwd: cwd.clone().into(),
                process_id: None,
                source: CommandExecutionSource::Agent,
                status,
                command_actions,
                aggregated_output: None,
                exit_code: None,
                duration_ms: None,
            })
        }
        GuardianAssessmentAction::ApplyPatch { .. }
        | GuardianAssessmentAction::NetworkAccess { .. }
        | GuardianAssessmentAction::McpToolCall { .. }
        | GuardianAssessmentAction::RequestPermissions { .. } => None,
    }
}

pub fn guardian_auto_approval_review_notification(
    conversation_id: &ChatId,
    event_interaction_id: &str,
    assessment: &GuardianAssessmentEvent,
) -> ServerNotification {
    let interaction_id = if assessment.interaction_id.is_empty() {
        event_interaction_id.to_string()
    } else {
        assessment.interaction_id.clone()
    };
    let review = GuardianApprovalReview {
        status: match assessment.status {
            datax_protocol::protocol::GuardianAssessmentStatus::InProgress => {
                GuardianApprovalReviewStatus::InProgress
            }
            datax_protocol::protocol::GuardianAssessmentStatus::Approved => {
                GuardianApprovalReviewStatus::Approved
            }
            datax_protocol::protocol::GuardianAssessmentStatus::Denied => {
                GuardianApprovalReviewStatus::Denied
            }
            datax_protocol::protocol::GuardianAssessmentStatus::TimedOut => {
                GuardianApprovalReviewStatus::TimedOut
            }
            datax_protocol::protocol::GuardianAssessmentStatus::Aborted => {
                GuardianApprovalReviewStatus::Aborted
            }
        },
        risk_level: assessment.risk_level.map(Into::into),
        user_authorization: assessment.user_authorization.map(Into::into),
        rationale: assessment.rationale.clone(),
    };
    let action = assessment.action.clone().into();
    match assessment.status {
        datax_protocol::protocol::GuardianAssessmentStatus::InProgress => {
            ServerNotification::MessageGuardianApprovalReviewStarted(
                MessageGuardianApprovalReviewStartedNotification {
                    chat_id: conversation_id.to_string(),
                    interaction_id,
                    review_id: assessment.id.clone(),
                    started_at_ms: assessment.started_at_ms,
                    target_message_id: assessment.target_item_id.clone(),
                    review,
                    action,
                },
            )
        }
        datax_protocol::protocol::GuardianAssessmentStatus::Approved
        | datax_protocol::protocol::GuardianAssessmentStatus::Denied
        | datax_protocol::protocol::GuardianAssessmentStatus::TimedOut
        | datax_protocol::protocol::GuardianAssessmentStatus::Aborted => {
            ServerNotification::MessageGuardianApprovalReviewCompleted(
                MessageGuardianApprovalReviewCompletedNotification {
                    chat_id: conversation_id.to_string(),
                    interaction_id,
                    review_id: assessment.id.clone(),
                    started_at_ms: assessment.started_at_ms,
                    completed_at_ms: assessment
                        .completed_at_ms
                        .unwrap_or(assessment.started_at_ms),
                    target_message_id: assessment.target_item_id.clone(),
                    decision_source: assessment
                        .decision_source
                        .map(AutoReviewDecisionSource::from)
                        .unwrap_or(AutoReviewDecisionSource::Agent),
                    review,
                    action,
                },
            )
        }
    }
}

pub fn convert_patch_changes(changes: &HashMap<PathBuf, FileChange>) -> Vec<FileUpdateChange> {
    let mut converted: Vec<FileUpdateChange> = changes
        .iter()
        .map(|(path, change)| FileUpdateChange {
            path: path.to_string_lossy().into_owned(),
            kind: map_patch_change_kind(change),
            diff: format_file_change_diff(change),
        })
        .collect();
    converted.sort_by(|a, b| a.path.cmp(&b.path));
    converted
}

fn map_patch_change_kind(change: &FileChange) -> PatchChangeKind {
    match change {
        FileChange::Add { .. } => PatchChangeKind::Add,
        FileChange::Delete { .. } => PatchChangeKind::Delete,
        FileChange::Update { move_path, .. } => PatchChangeKind::Update {
            move_path: move_path.clone(),
        },
    }
}

fn format_file_change_diff(change: &FileChange) -> String {
    match change {
        FileChange::Add { content } => content.clone(),
        FileChange::Delete { content } => content.clone(),
        FileChange::Update {
            unified_diff,
            move_path,
        } => {
            if let Some(path) = move_path {
                format!("{unified_diff}\n\nMoved to: {}", path.display())
            } else {
                unified_diff.clone()
            }
        }
    }
}

#[cfg(test)]
#[path = "item_builders_tests.rs"]
mod tests;
