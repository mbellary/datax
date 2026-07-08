//! Chat targeting helpers for app-server requests and notifications.

use datax_app_server_protocol::ServerNotification;
use datax_app_server_protocol::ServerRequest;
use datax_protocol::ThreadId;

pub(super) fn server_request_thread_id(request: &ServerRequest) -> Option<ThreadId> {
    match request {
        ServerRequest::CommandExecutionRequestApproval { params, .. } => {
            ThreadId::from_string(&params.chat_id).ok()
        }
        ServerRequest::FileChangeRequestApproval { params, .. } => {
            ThreadId::from_string(&params.chat_id).ok()
        }
        ServerRequest::ToolRequestUserInput { params, .. } => {
            ThreadId::from_string(&params.chat_id).ok()
        }
        ServerRequest::McpServerElicitationRequest { params, .. } => {
            ThreadId::from_string(&params.chat_id).ok()
        }
        ServerRequest::PermissionsRequestApproval { params, .. } => {
            ThreadId::from_string(&params.chat_id).ok()
        }
        ServerRequest::DynamicToolCall { params, .. } => {
            ThreadId::from_string(&params.chat_id).ok()
        }
        ServerRequest::CurrentTimeRead { params, .. } => {
            ThreadId::from_string(&params.chat_id).ok()
        }
        ServerRequest::ChatgptAuthTokensRefresh { .. }
        | ServerRequest::AttestationGenerate { .. }
        | ServerRequest::ApplyPatchApproval { .. }
        | ServerRequest::ExecCommandApproval { .. } => None,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ServerNotificationThreadTarget {
    Chat(ThreadId),
    InvalidThreadId(String),
    AppScoped,
    Global,
}

pub(super) fn server_notification_thread_target(
    notification: &ServerNotification,
) -> ServerNotificationThreadTarget {
    let thread_id = match notification {
        ServerNotification::Error(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::ChatStarted(notification) => Some(notification.thread.id.as_str()),
        ServerNotification::ChatStatusChanged(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::ChatArchived(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::ChatDeleted(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::ChatUnarchived(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::ChatClosed(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::ChatNameUpdated(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::ChatTokenUsageUpdated(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::ChatGoalUpdated(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::ChatGoalCleared(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::ChatSettingsUpdated(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::InteractionStarted(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::HookStarted(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::InteractionCompleted(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::HookCompleted(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::InteractionDiffUpdated(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::InteractionPlanUpdated(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::MessageStarted(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::MessageGuardianApprovalReviewStarted(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::MessageGuardianApprovalReviewCompleted(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::MessageCompleted(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::RawResponseItemCompleted(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::AgentMessageDelta(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::PlanDelta(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::CommandExecutionOutputDelta(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::TerminalInteraction(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::FileChangeOutputDelta(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::FileChangePatchUpdated(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::ServerRequestResolved(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::McpToolCallProgress(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::ReasoningSummaryTextDelta(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::ReasoningSummaryPartAdded(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::ReasoningTextDelta(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::ContextCompacted(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::ModelRerouted(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::ModelVerification(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::ModelSafetyBufferingUpdated(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::InteractionModerationMetadata(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::ChatRealtimeStarted(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::ChatRealtimeMessageAdded(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::ChatRealtimeTranscriptDelta(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::ChatRealtimeTranscriptDone(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::ChatRealtimeOutputAudioDelta(notification) => {
            Some(notification.chat_id.as_str())
        }
        ServerNotification::ChatRealtimeSdp(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::ChatRealtimeError(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::ChatRealtimeClosed(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::Warning(notification) => notification.chat_id.as_deref(),
        ServerNotification::GuardianWarning(notification) => Some(notification.chat_id.as_str()),
        ServerNotification::McpServerStatusUpdated(notification) => {
            match notification.chat_id.as_deref() {
                Some(thread_id) => Some(thread_id),
                None => return ServerNotificationThreadTarget::AppScoped,
            }
        }
        ServerNotification::SkillsChanged(_)
        | ServerNotification::McpServerOauthLoginCompleted(_)
        | ServerNotification::AccountUpdated(_)
        | ServerNotification::AccountRateLimitsUpdated(_)
        | ServerNotification::AppListUpdated(_)
        | ServerNotification::RemoteControlStatusChanged(_)
        | ServerNotification::ExternalAgentConfigImportProgress(_)
        | ServerNotification::ExternalAgentConfigImportCompleted(_)
        | ServerNotification::DeprecationNotice(_)
        | ServerNotification::ConfigWarning(_)
        | ServerNotification::FuzzyFileSearchSessionUpdated(_)
        | ServerNotification::FuzzyFileSearchSessionCompleted(_)
        | ServerNotification::CommandExecOutputDelta(_)
        | ServerNotification::ProcessOutputDelta(_)
        | ServerNotification::ProcessExited(_)
        | ServerNotification::FsChanged(_)
        | ServerNotification::WindowsWorldWritableWarning(_)
        | ServerNotification::WindowsSandboxSetupCompleted(_)
        | ServerNotification::AccountLoginCompleted(_) => None,
    };

    match thread_id {
        Some(thread_id) => match ThreadId::from_string(thread_id) {
            Ok(thread_id) => ServerNotificationThreadTarget::Chat(thread_id),
            Err(_) => ServerNotificationThreadTarget::InvalidThreadId(thread_id.to_string()),
        },
        None => ServerNotificationThreadTarget::Global,
    }
}

#[cfg(test)]
mod tests {
    use super::ServerNotificationThreadTarget;
    use super::server_notification_thread_target;
    use crate::test_support::PathBufExt;
    use crate::test_support::test_path_buf;
    use datax_app_server_protocol::ChatSettings;
    use datax_app_server_protocol::ChatSettingsUpdatedNotification;
    use datax_app_server_protocol::GuardianWarningNotification;
    use datax_app_server_protocol::McpServerStartupState;
    use datax_app_server_protocol::McpServerStatusUpdatedNotification;
    use datax_app_server_protocol::ServerNotification;
    use datax_app_server_protocol::WarningNotification;
    use datax_protocol::ThreadId;
    use datax_protocol::config_types::CollaborationMode;
    use datax_protocol::config_types::ModeKind;
    use datax_protocol::config_types::Settings;
    use datax_protocol::openai_models::ReasoningEffort;
    use pretty_assertions::assert_eq;

    fn test_thread_settings() -> ChatSettings {
        ChatSettings {
            cwd: test_path_buf("/tmp/thread-settings").abs(),
            approval_policy: datax_app_server_protocol::AskForApproval::Never,
            approvals_reviewer: datax_app_server_protocol::ApprovalsReviewer::User,
            sandbox_policy: datax_app_server_protocol::SandboxPolicy::ReadOnly {
                network_access: false,
            },
            active_permission_profile: None,
            model: "gpt-5.4".to_string(),
            model_provider: "openai".to_string(),
            service_tier: None,
            effort: Some(ReasoningEffort::High),
            summary: None,
            collaboration_mode: CollaborationMode {
                mode: ModeKind::Default,
                settings: Settings {
                    model: "gpt-5.4".to_string(),
                    reasoning_effort: Some(ReasoningEffort::High),
                    developer_instructions: None,
                },
            },
            multi_agent_mode: Default::default(),
            personality: None,
        }
    }

    #[test]
    fn warning_notifications_without_threads_are_global() {
        let notification = ServerNotification::Warning(WarningNotification {
            chat_id: None,
            message: "warning".to_string(),
        });

        let target = server_notification_thread_target(&notification);

        assert_eq!(target, ServerNotificationThreadTarget::Global);
    }

    #[test]
    fn warning_notifications_route_to_threads_when_thread_id_is_present() {
        let thread_id = ThreadId::new();
        let notification = ServerNotification::Warning(WarningNotification {
            chat_id: Some(thread_id.to_string()),
            message: "warning".to_string(),
        });

        let target = server_notification_thread_target(&notification);

        assert_eq!(target, ServerNotificationThreadTarget::Chat(thread_id));
    }

    #[test]
    fn guardian_warning_notifications_route_to_threads() {
        let thread_id = ThreadId::new();
        let notification = ServerNotification::GuardianWarning(GuardianWarningNotification {
            chat_id: thread_id.to_string(),
            message: "warning".to_string(),
        });

        let target = server_notification_thread_target(&notification);

        assert_eq!(target, ServerNotificationThreadTarget::Chat(thread_id));
    }

    #[test]
    fn mcp_startup_notifications_route_to_threads() {
        let thread_id = ThreadId::new();
        let notification =
            ServerNotification::McpServerStatusUpdated(McpServerStatusUpdatedNotification {
                chat_id: Some(thread_id.to_string()),
                name: "sentry".to_string(),
                status: McpServerStartupState::Failed,
                error: Some("sentry is not logged in".to_string()),
            });

        let target = server_notification_thread_target(&notification);

        assert_eq!(target, ServerNotificationThreadTarget::Chat(thread_id));
    }

    #[test]
    fn mcp_startup_notifications_without_threads_are_app_scoped() {
        let notification =
            ServerNotification::McpServerStatusUpdated(McpServerStatusUpdatedNotification {
                chat_id: None,
                name: "sentry".to_string(),
                status: McpServerStartupState::Failed,
                error: Some("sentry is not logged in".to_string()),
            });

        let target = server_notification_thread_target(&notification);

        assert_eq!(target, ServerNotificationThreadTarget::AppScoped);
    }

    #[test]
    fn thread_settings_updated_notifications_route_to_threads() {
        let thread_id = ThreadId::new();
        let notification =
            ServerNotification::ChatSettingsUpdated(ChatSettingsUpdatedNotification {
                chat_id: thread_id.to_string(),
                thread_settings: test_thread_settings(),
            });

        let target = server_notification_thread_target(&notification);

        assert_eq!(target, ServerNotificationThreadTarget::Chat(thread_id));
    }
}
