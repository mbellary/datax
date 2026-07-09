use super::ActivePermissionProfile;
use super::ApprovalsReviewer;
use super::AskForApproval;
use super::Chat;
use super::ChatSource;
use super::Interaction;
use super::InteractionEnvironmentParams;
use super::InteractionMessagesView;
use super::Message;
use super::SandboxMode;
use super::SandboxPolicy;
use super::shared::v2_enum_from_core;
use datax_experimental_api_macros::ExperimentalApi;
pub use datax_protocol::capabilities::CapabilityRootLocation;
pub use datax_protocol::capabilities::SelectedCapabilityRoot;
use datax_protocol::config_types::CollaborationMode;
use datax_protocol::config_types::MultiAgentMode;
use datax_protocol::config_types::Personality;
use datax_protocol::config_types::ReasoningSummary;
pub use datax_protocol::dynamic_tools::DynamicToolFunctionSpec;
pub use datax_protocol::dynamic_tools::DynamicToolNamespaceSpec;
pub use datax_protocol::dynamic_tools::DynamicToolNamespaceTool;
pub use datax_protocol::dynamic_tools::DynamicToolSpec;
use datax_protocol::models::ResponseItem;
use datax_protocol::openai_models::ReasoningEffort;
use datax_protocol::protocol::ThreadGoalStatus as CoreThreadGoalStatus;
use datax_protocol::protocol::TokenUsage as CoreTokenUsage;
use datax_protocol::protocol::TokenUsageInfo as CoreTokenUsageInfo;
use datax_utils_absolute_path::AbsolutePathBuf;
use datax_utils_path_uri::LegacyAppPathString;
use datax_utils_path_uri::PathUri;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum ChatStartSource {
    Startup,
    Clear,
}

// === Chats, Interactions, and Messages ===
// Chat APIs
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema, TS, ExperimentalApi,
)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatStartParams {
    #[ts(optional = nullable)]
    pub model: Option<String>,
    #[ts(optional = nullable)]
    pub model_provider: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(optional = nullable)]
    pub service_tier: Option<Option<String>>,
    #[ts(optional = nullable)]
    pub cwd: Option<String>,
    /// Replace the chat's runtime workspace roots. Paths must be absolute.
    #[experimental("chat/start.runtimeWorkspaceRoots")]
    #[ts(optional = nullable)]
    pub runtime_workspace_roots: Option<Vec<AbsolutePathBuf>>,
    #[experimental(nested)]
    #[ts(optional = nullable)]
    pub approval_policy: Option<AskForApproval>,
    /// Override where approval requests are routed for review on this chat
    /// and subsequent interactions.
    #[ts(optional = nullable)]
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    #[ts(optional = nullable)]
    pub sandbox: Option<SandboxMode>,
    /// Named profile id for this chat. Cannot be combined with `sandbox`.
    #[experimental("chat/start.permissions")]
    #[ts(optional = nullable)]
    pub permissions: Option<String>,
    #[ts(optional = nullable)]
    pub config: Option<HashMap<String, JsonValue>>,
    #[ts(optional = nullable)]
    pub service_name: Option<String>,
    #[ts(optional = nullable)]
    pub base_instructions: Option<String>,
    #[ts(optional = nullable)]
    pub developer_instructions: Option<String>,
    #[ts(optional = nullable)]
    pub personality: Option<Personality>,
    /// @deprecated Ignored. Use Ultra reasoning effort for proactive multi-agent behavior.
    #[experimental("chat/start.multiAgentMode")]
    #[ts(optional = nullable)]
    pub multi_agent_mode: Option<MultiAgentMode>,
    #[ts(optional = nullable)]
    pub ephemeral: Option<bool>,
    #[ts(optional = nullable)]
    pub session_start_source: Option<ChatStartSource>,
    /// Optional client-supplied analytics source classification for this chat.
    #[ts(optional = nullable)]
    pub chat_source: Option<ChatSource>,
    /// Optional sticky environments for this chat.
    ///
    /// Omitted selects the default environment when environment access is
    /// enabled. Empty disables environment access for interactions that do not
    /// provide an interaction override. Non-empty selects the first environment as the
    /// current interaction environment.
    #[experimental("chat/start.environments")]
    #[ts(optional = nullable)]
    pub environments: Option<Vec<InteractionEnvironmentParams>>,
    #[experimental("chat/start.dynamicTools")]
    #[serde(
        default,
        deserialize_with = "datax_protocol::dynamic_tools::deserialize_dynamic_tool_specs"
    )]
    #[ts(optional = nullable)]
    pub dynamic_tools: Option<Vec<DynamicToolSpec>>,
    /// Capability roots selected for this chat by the hosting platform.
    #[experimental("chat/start.selectedCapabilityRoots")]
    #[ts(optional = nullable)]
    pub selected_capability_roots: Option<Vec<SelectedCapabilityRoot>>,
    /// Test-only experimental field used to validate experimental gating and
    /// schema filtering behavior in a stable way.
    #[experimental("chat/start.mockExperimentalField")]
    #[ts(optional = nullable)]
    pub mock_experimental_field: Option<String>,
    /// If true, opt into emitting raw Responses API messages on the event stream.
    /// This is for internal use only (e.g. Codex Cloud).
    #[experimental("chat/start.experimentalRawEvents")]
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub experimental_raw_events: bool,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct MockExperimentalMethodParams {
    /// Test-only payload field.
    #[ts(optional = nullable)]
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct MockExperimentalMethodResponse {
    /// Echoes the input `value`.
    pub echoed: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, ExperimentalApi)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatStartResponse {
    #[serde(rename = "chat")]
    #[ts(rename = "chat")]
    pub chat: Chat,
    pub model: String,
    pub model_provider: String,
    pub service_tier: Option<String>,
    pub cwd: AbsolutePathBuf,
    /// Chat-scoped runtime workspace roots used to materialize
    /// `:workspace_roots`.
    #[experimental("chat/start.runtimeWorkspaceRoots")]
    #[serde(default)]
    pub runtime_workspace_roots: Vec<AbsolutePathBuf>,
    /// Environment-native paths to instruction source files currently loaded for this chat.
    #[serde(default)]
    pub instruction_sources: Vec<LegacyAppPathString>,
    #[experimental(nested)]
    pub approval_policy: AskForApproval,
    /// Reviewer currently used for approval requests on this chat.
    pub approvals_reviewer: ApprovalsReviewer,
    /// Legacy sandbox policy retained for compatibility. Experimental clients
    /// should prefer `activePermissionProfile` for profile provenance.
    pub sandbox: SandboxPolicy,
    /// Named or implicit built-in profile that produced the active
    /// permissions, when known.
    #[experimental("chat/start.activePermissionProfile")]
    #[serde(default)]
    pub active_permission_profile: Option<ActivePermissionProfile>,
    pub reasoning_effort: Option<ReasoningEffort>,
    /// @deprecated Always `explicitRequestOnly`. Use `reasoningEffort` for Ultra behavior.
    #[experimental("chat/start.multiAgentMode")]
    #[serde(default)]
    pub multi_agent_mode: MultiAgentMode,
}

impl ChatStartResponse {
    /// Parses valid absolute instruction source paths and omits malformed legacy values.
    pub fn instruction_source_path_uris(&self) -> Vec<PathUri> {
        instruction_source_path_uris(&self.instruction_sources)
    }
}

#[derive(
    Serialize, Deserialize, Debug, Default, Clone, PartialEq, JsonSchema, TS, ExperimentalApi,
)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatSettingsUpdateParams {
    pub chat_id: String,
    /// Override the working directory for subsequent interactions.
    #[ts(optional = nullable)]
    pub cwd: Option<PathBuf>,
    /// Override the approval policy for subsequent interactions.
    #[experimental(nested)]
    #[ts(optional = nullable)]
    pub approval_policy: Option<AskForApproval>,
    /// Override where approval requests are routed for subsequent interactions.
    #[ts(optional = nullable)]
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    /// Override the sandbox policy for subsequent interactions.
    #[ts(optional = nullable)]
    pub sandbox_policy: Option<SandboxPolicy>,
    /// Select a named permissions profile id for subsequent interactions. Cannot be
    /// combined with `sandboxPolicy`.
    #[experimental("chat/settings/update.permissions")]
    #[ts(optional = nullable)]
    pub permissions: Option<String>,
    /// Override the model for subsequent interactions.
    #[ts(optional = nullable)]
    pub model: Option<String>,
    /// Override the service tier for subsequent interactions. `null` clears the
    /// current service tier; omission leaves it unchanged.
    #[serde(
        default,
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(optional = nullable)]
    pub service_tier: Option<Option<String>>,
    /// Override the reasoning effort for subsequent interactions.
    #[ts(optional = nullable)]
    pub effort: Option<ReasoningEffort>,
    /// Override the reasoning summary for subsequent interactions.
    #[ts(optional = nullable)]
    pub summary: Option<ReasoningSummary>,
    /// EXPERIMENTAL - Set a pre-set collaboration mode for subsequent interactions.
    ///
    /// For `collaboration_mode.settings.developer_instructions`, `null` means
    /// "use the built-in instructions for the selected mode".
    #[experimental("chat/settings/update.collaborationMode")]
    #[ts(optional = nullable)]
    pub collaboration_mode: Option<CollaborationMode>,
    /// @deprecated Ignored. Use `effort: "ultra"` for proactive multi-agent behavior.
    #[experimental("chat/settings/update.multiAgentMode")]
    #[ts(optional = nullable)]
    pub multi_agent_mode: Option<MultiAgentMode>,
    /// Override the personality for subsequent interactions.
    #[ts(optional = nullable)]
    pub personality: Option<Personality>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatSettingsUpdateResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, ExperimentalApi)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatSettings {
    pub cwd: AbsolutePathBuf,
    pub approval_policy: AskForApproval,
    pub approvals_reviewer: ApprovalsReviewer,
    pub sandbox_policy: SandboxPolicy,
    pub active_permission_profile: Option<ActivePermissionProfile>,
    pub model: String,
    pub model_provider: String,
    pub service_tier: Option<String>,
    pub effort: Option<ReasoningEffort>,
    pub summary: Option<ReasoningSummary>,
    pub collaboration_mode: CollaborationMode,
    /// @deprecated Always `explicitRequestOnly`. Use `effort` for Ultra behavior.
    #[experimental("chat/settings.multiAgentMode")]
    #[serde(default)]
    pub multi_agent_mode: MultiAgentMode,
    pub personality: Option<Personality>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatSettingsUpdatedNotification {
    pub chat_id: String,
    #[serde(rename = "chatSettings")]
    #[ts(rename = "chatSettings")]
    pub thread_settings: ChatSettings,
}

#[derive(
    Serialize, Deserialize, Debug, Default, Clone, PartialEq, JsonSchema, TS, ExperimentalApi,
)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
/// There are three ways to resume a chat:
/// 1. By chat_id: load the chat from disk by chat_id and resume it.
/// 2. By history: instantiate the chat from memory and resume it.
/// 3. By path: load the chat from disk by path and resume it.
///
/// For non-running chats, the precedence is: history > non-empty path > chat_id.
/// If using history or a non-empty path for a non-running chat, the chat_id
/// param will be ignored.
///
/// If chat_id identifies a running chat, app-server rejoins that chat and
/// treats a non-empty path as a consistency check against the active rollout path.
/// Empty string path values are treated as absent.
///
/// Prefer using chat_id whenever possible.
pub struct ChatResumeParams {
    pub chat_id: String,

    /// [UNSTABLE] FOR CODEX CLOUD - DO NOT USE.
    /// If specified, the chat will be resumed with the provided history
    /// instead of loaded from disk.
    #[experimental("chat/resume.history")]
    #[ts(optional = nullable)]
    pub history: Option<Vec<ResponseItem>>,

    /// [UNSTABLE] Specify the rollout path to resume from.
    /// If specified for a non-running chat, the chat_id param will be ignored.
    /// If chat_id identifies a running chat, the path must match the active
    /// rollout path.
    #[experimental("chat/resume.path")]
    #[serde(
        default,
        deserialize_with = "crate::protocol::serde_helpers::deserialize_empty_path_as_none"
    )]
    #[ts(optional = nullable)]
    pub path: Option<PathBuf>,

    /// Configuration overrides for the resumed chat, if any.
    #[ts(optional = nullable)]
    pub model: Option<String>,
    #[ts(optional = nullable)]
    pub model_provider: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(optional = nullable)]
    pub service_tier: Option<Option<String>>,
    #[ts(optional = nullable)]
    pub cwd: Option<String>,
    /// Replace the chat's runtime workspace roots. Paths must be absolute.
    #[experimental("chat/resume.runtimeWorkspaceRoots")]
    #[ts(optional = nullable)]
    pub runtime_workspace_roots: Option<Vec<AbsolutePathBuf>>,
    #[experimental(nested)]
    #[ts(optional = nullable)]
    pub approval_policy: Option<AskForApproval>,
    /// Override where approval requests are routed for review on this chat
    /// and subsequent interactions.
    #[ts(optional = nullable)]
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    #[ts(optional = nullable)]
    pub sandbox: Option<SandboxMode>,
    /// Named profile id for the resumed chat. Cannot be combined with
    /// `sandbox`.
    #[experimental("chat/resume.permissions")]
    #[ts(optional = nullable)]
    pub permissions: Option<String>,
    #[ts(optional = nullable)]
    pub config: Option<HashMap<String, serde_json::Value>>,
    #[ts(optional = nullable)]
    pub base_instructions: Option<String>,
    #[ts(optional = nullable)]
    pub developer_instructions: Option<String>,
    #[ts(optional = nullable)]
    pub personality: Option<Personality>,
    /// When true, return only chat metadata and live-resume state without
    /// populating `chat.interactions`. This is useful when the client plans to call
    /// `chat/interactions/list` immediately after resuming.
    #[experimental("chat/resume.excludeInteractions")]
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exclude_interactions: bool,
    /// When present, include a `chat/interactions/list` page in the resume response
    /// so clients can bootstrap recent interactions without a second request.
    #[experimental("chat/resume.initialInteractionsPage")]
    #[ts(optional = nullable)]
    #[serde(rename = "initialInteractionsPage")]
    #[ts(rename = "initialInteractionsPage")]
    pub initial_interactions_page: Option<ChatResumeInitialInteractionsPageParams>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, ExperimentalApi)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatResumeResponse {
    #[serde(rename = "chat")]
    #[ts(rename = "chat")]
    pub chat: Chat,
    pub model: String,
    pub model_provider: String,
    pub service_tier: Option<String>,
    pub cwd: AbsolutePathBuf,
    /// Chat-scoped runtime workspace roots used to materialize
    /// `:workspace_roots`.
    #[experimental("chat/resume.runtimeWorkspaceRoots")]
    #[serde(default)]
    pub runtime_workspace_roots: Vec<AbsolutePathBuf>,
    /// Environment-native paths to instruction source files currently loaded for this chat.
    #[serde(default)]
    pub instruction_sources: Vec<LegacyAppPathString>,
    #[experimental(nested)]
    pub approval_policy: AskForApproval,
    /// Reviewer currently used for approval requests on this chat.
    pub approvals_reviewer: ApprovalsReviewer,
    /// Legacy sandbox policy retained for compatibility. Experimental clients
    /// should prefer `activePermissionProfile` for profile provenance.
    pub sandbox: SandboxPolicy,
    /// Named or implicit built-in profile that produced the active
    /// permissions, when known.
    #[experimental("chat/resume.activePermissionProfile")]
    #[serde(default)]
    pub active_permission_profile: Option<ActivePermissionProfile>,
    pub reasoning_effort: Option<ReasoningEffort>,
    /// @deprecated Always `explicitRequestOnly`. Use `reasoningEffort` for Ultra behavior.
    #[experimental("chat/resume.multiAgentMode")]
    #[serde(default)]
    pub multi_agent_mode: MultiAgentMode,
    /// `chat/interactions/list` page returned when requested by `initialInteractionsPage`.
    #[experimental("chat/resume.initialInteractionsPage")]
    #[serde(default)]
    #[serde(rename = "initialInteractionsPage")]
    #[ts(rename = "initialInteractionsPage")]
    pub initial_interactions_page: Option<InteractionsPage>,
}

impl ChatResumeResponse {
    /// Parses valid absolute instruction source paths and omits malformed legacy values.
    pub fn instruction_source_path_uris(&self) -> Vec<PathUri> {
        instruction_source_path_uris(&self.instruction_sources)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatResumeInitialInteractionsPageParams {
    /// Optional interaction page size.
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
    /// Optional interaction pagination direction; defaults to descending.
    #[ts(optional = nullable)]
    pub sort_direction: Option<SortDirection>,
    /// How much message detail to include for each returned interaction; defaults to summary.
    #[ts(optional = nullable)]
    pub messages_view: Option<InteractionMessagesView>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct InteractionsPage {
    pub data: Vec<Interaction>,
    pub next_cursor: Option<String>,
    pub backwards_cursor: Option<String>,
}

impl From<ChatInteractionsListResponse> for InteractionsPage {
    fn from(response: ChatInteractionsListResponse) -> Self {
        Self {
            data: response.data,
            next_cursor: response.next_cursor,
            backwards_cursor: response.backwards_cursor,
        }
    }
}

#[derive(
    Serialize, Deserialize, Debug, Default, Clone, PartialEq, JsonSchema, TS, ExperimentalApi,
)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
/// There are two ways to fork a chat:
/// 1. By chat_id: load the chat from disk by chat_id and fork it into a new chat.
/// 2. By path: load the chat from disk by path and fork it into a new chat.
///
/// If using a non-empty path, the chat_id param will be ignored.
/// Empty string path values are treated as absent.
///
/// Prefer using chat_id whenever possible.
pub struct ChatForkParams {
    pub chat_id: String,

    /// [UNSTABLE] Specify the rollout path to fork from.
    /// If specified, the chat_id param will be ignored.
    #[experimental("chat/fork.path")]
    #[serde(
        default,
        deserialize_with = "crate::protocol::serde_helpers::deserialize_empty_path_as_none"
    )]
    #[ts(optional = nullable)]
    pub path: Option<PathBuf>,

    /// Configuration overrides for the forked chat, if any.
    #[ts(optional = nullable)]
    pub model: Option<String>,
    #[ts(optional = nullable)]
    pub model_provider: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(optional = nullable)]
    pub service_tier: Option<Option<String>>,
    #[ts(optional = nullable)]
    pub cwd: Option<String>,
    /// Replace the chat's runtime workspace roots. Paths must be absolute.
    #[experimental("chat/fork.runtimeWorkspaceRoots")]
    #[ts(optional = nullable)]
    pub runtime_workspace_roots: Option<Vec<AbsolutePathBuf>>,
    #[experimental(nested)]
    #[ts(optional = nullable)]
    pub approval_policy: Option<AskForApproval>,
    /// Override where approval requests are routed for review on this chat
    /// and subsequent interactions.
    #[ts(optional = nullable)]
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    #[ts(optional = nullable)]
    pub sandbox: Option<SandboxMode>,
    /// Named profile id for the forked chat. Cannot be combined with
    /// `sandbox`.
    #[experimental("chat/fork.permissions")]
    #[ts(optional = nullable)]
    pub permissions: Option<String>,
    #[ts(optional = nullable)]
    pub config: Option<HashMap<String, serde_json::Value>>,
    #[ts(optional = nullable)]
    pub base_instructions: Option<String>,
    #[ts(optional = nullable)]
    pub developer_instructions: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ephemeral: bool,
    /// Optional client-supplied analytics source classification for this forked chat.
    #[ts(optional = nullable)]
    pub chat_source: Option<ChatSource>,
    /// When true, return only chat metadata and live fork state without
    /// populating `chat.interactions`. This is useful when the client plans to call
    /// `chat/interactions/list` immediately after forking.
    #[experimental("chat/fork.excludeInteractions")]
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exclude_interactions: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, ExperimentalApi)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatForkResponse {
    #[serde(rename = "chat")]
    #[ts(rename = "chat")]
    pub chat: Chat,
    pub model: String,
    pub model_provider: String,
    pub service_tier: Option<String>,
    pub cwd: AbsolutePathBuf,
    /// Chat-scoped runtime workspace roots used to materialize
    /// `:workspace_roots`.
    #[experimental("chat/fork.runtimeWorkspaceRoots")]
    #[serde(default)]
    pub runtime_workspace_roots: Vec<AbsolutePathBuf>,
    /// Environment-native paths to instruction source files currently loaded for this chat.
    #[serde(default)]
    pub instruction_sources: Vec<LegacyAppPathString>,
    #[experimental(nested)]
    pub approval_policy: AskForApproval,
    /// Reviewer currently used for approval requests on this chat.
    pub approvals_reviewer: ApprovalsReviewer,
    /// Legacy sandbox policy retained for compatibility. Experimental clients
    /// should prefer `activePermissionProfile` for profile provenance.
    pub sandbox: SandboxPolicy,
    /// Named or implicit built-in profile that produced the active
    /// permissions, when known.
    #[experimental("chat/fork.activePermissionProfile")]
    #[serde(default)]
    pub active_permission_profile: Option<ActivePermissionProfile>,
    pub reasoning_effort: Option<ReasoningEffort>,
    /// @deprecated Always `explicitRequestOnly`. Use `reasoningEffort` for Ultra behavior.
    #[experimental("chat/fork.multiAgentMode")]
    #[serde(default)]
    pub multi_agent_mode: MultiAgentMode,
}

impl ChatForkResponse {
    /// Parses valid absolute instruction source paths and omits malformed legacy values.
    pub fn instruction_source_path_uris(&self) -> Vec<PathUri> {
        instruction_source_path_uris(&self.instruction_sources)
    }
}

fn instruction_source_path_uris(sources: &[LegacyAppPathString]) -> Vec<PathUri> {
    // Instruction sources are advisory diagnostics. Warn and fail open so a malformed legacy
    // path cannot fail chat start, resume, or fork.
    sources
        .iter()
        .filter_map(|source| {
            source.to_inferred_path_uri().or_else(|| {
                tracing::warn!(
                    path = source.as_str(),
                    "ignoring invalid instruction source path from app-server"
                );
                None
            })
        })
        .collect()
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatArchiveParams {
    pub chat_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatArchiveResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatDeleteParams {
    pub chat_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatDeleteResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatUnsubscribeParams {
    pub chat_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatUnsubscribeResponse {
    pub status: ChatUnsubscribeStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum ChatUnsubscribeStatus {
    NotLoaded,
    NotSubscribed,
    Unsubscribed,
}

/// Parameters for `chat/increment_elicitation`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatIncrementElicitationParams {
    /// Chat whose out-of-band elicitation counter should be incremented.
    pub chat_id: String,
}

/// Response for `chat/increment_elicitation`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatIncrementElicitationResponse {
    /// Current out-of-band elicitation count after the increment.
    pub count: u64,
    /// Whether timeout accounting is paused after applying the increment.
    pub paused: bool,
}

/// Parameters for `chat/decrement_elicitation`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatDecrementElicitationParams {
    /// Chat whose out-of-band elicitation counter should be decremented.
    pub chat_id: String,
}

/// Response for `chat/decrement_elicitation`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatDecrementElicitationResponse {
    /// Current out-of-band elicitation count after the decrement.
    pub count: u64,
    /// Whether timeout accounting remains paused after applying the decrement.
    pub paused: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatSetNameParams {
    pub chat_id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatUnarchiveParams {
    pub chat_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatSetNameResponse {}

v2_enum_from_core! {
    pub enum ChatGoalStatus from CoreThreadGoalStatus {
        Active,
        Paused,
        Blocked,
        UsageLimited,
        BudgetLimited,
        Complete,
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatGoal {
    pub chat_id: String,
    pub objective: String,
    pub status: ChatGoalStatus,
    #[ts(type = "number | null")]
    pub token_budget: Option<i64>,
    #[ts(type = "number")]
    pub tokens_used: i64,
    #[ts(type = "number")]
    pub time_used_seconds: i64,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

impl From<datax_protocol::protocol::ThreadGoal> for ChatGoal {
    fn from(value: datax_protocol::protocol::ThreadGoal) -> Self {
        Self {
            chat_id: value.thread_id.to_string(),
            objective: value.objective,
            status: value.status.into(),
            token_budget: value.token_budget,
            tokens_used: value.tokens_used,
            time_used_seconds: value.time_used_seconds,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatGoalSetParams {
    pub chat_id: String,
    #[ts(optional = nullable)]
    pub objective: Option<String>,
    #[ts(optional = nullable)]
    pub status: Option<ChatGoalStatus>,
    #[serde(
        default,
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(optional = nullable, type = "number | null")]
    pub token_budget: Option<Option<i64>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatGoalSetResponse {
    pub goal: ChatGoal,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatGoalGetParams {
    pub chat_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatGoalGetResponse {
    pub goal: Option<ChatGoal>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatGoalClearParams {
    pub chat_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatGoalClearResponse {
    pub cleared: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatMetadataUpdateParams {
    pub chat_id: String,
    /// Patch the stored Git metadata for this chat.
    /// Omit a field to leave it unchanged, set it to `null` to clear it, or
    /// provide a string to replace the stored value.
    #[ts(optional = nullable)]
    pub git_info: Option<ChatMetadataGitInfoUpdateParams>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatMetadataGitInfoUpdateParams {
    /// Omit to leave the stored commit unchanged, set to `null` to clear it,
    /// or provide a non-empty string to replace it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option"
    )]
    #[ts(optional = nullable, type = "string | null")]
    pub sha: Option<Option<String>>,
    /// Omit to leave the stored branch unchanged, set to `null` to clear it,
    /// or provide a non-empty string to replace it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option"
    )]
    #[ts(optional = nullable, type = "string | null")]
    pub branch: Option<Option<String>>,
    /// Omit to leave the stored origin URL unchanged, set to `null` to clear it,
    /// or provide a non-empty string to replace it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option"
    )]
    #[ts(optional = nullable, type = "string | null")]
    pub origin_url: Option<Option<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatMetadataUpdateResponse {
    #[serde(rename = "chat")]
    #[ts(rename = "chat")]
    pub chat: Chat,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum ChatMemoryMode {
    Enabled,
    Disabled,
}

impl ChatMemoryMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }

    pub fn to_core(self) -> datax_protocol::protocol::ThreadMemoryMode {
        match self {
            Self::Enabled => datax_protocol::protocol::ThreadMemoryMode::Enabled,
            Self::Disabled => datax_protocol::protocol::ThreadMemoryMode::Disabled,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatMemoryModeSetParams {
    pub chat_id: String,
    pub mode: ChatMemoryMode,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatMemoryModeSetResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct MemoryResetResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatUnarchiveResponse {
    #[serde(rename = "chat")]
    #[ts(rename = "chat")]
    pub chat: Chat,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatCompactStartParams {
    pub chat_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatCompactStartResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatShellCommandParams {
    pub chat_id: String,
    /// Shell command string evaluated by the chat's configured shell.
    /// Unlike `command/exec`, this intentionally preserves shell syntax
    /// such as pipes, redirects, and quoting. This runs unsandboxed with full
    /// access rather than inheriting the chat sandbox policy.
    pub command: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatShellCommandResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatApproveGuardianDeniedActionParams {
    pub chat_id: String,
    /// Serialized `datax_protocol::protocol::GuardianAssessmentEvent`.
    pub event: JsonValue,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatApproveGuardianDeniedActionResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatBackgroundTerminalsCleanParams {
    pub chat_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatBackgroundTerminalsCleanResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatBackgroundTerminalsListParams {
    pub chat_id: String,
    /// Opaque pagination cursor returned by a previous call.
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    /// Optional page size.
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatBackgroundTerminal {
    pub message_id: String,
    pub process_id: String,
    pub command: String,
    pub cwd: AbsolutePathBuf,
    pub os_pid: Option<u32>,
    pub cpu_percent: Option<f64>,
    pub rss_kb: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatBackgroundTerminalsListResponse {
    pub data: Vec<ChatBackgroundTerminal>,
    /// Opaque cursor to pass to the next call to continue after the last item.
    /// If None, there are no more messages to return.
    pub next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatBackgroundTerminalsTerminateParams {
    pub chat_id: String,
    pub process_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatBackgroundTerminalsTerminateResponse {
    pub terminated: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatRollbackParams {
    pub chat_id: String,
    /// The number of interactions to drop from the end of the chat. Must be >= 1.
    ///
    /// This only modifies the chat's history and does not revert local file changes
    /// that have been made by the agent. Clients are responsible for reverting these changes.
    #[serde(rename = "numInteractions", alias = "numTurns")]
    #[ts(rename = "numInteractions")]
    pub num_interactions: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatRollbackResponse {
    /// The updated chat after applying the rollback, with `interactions` populated.
    ///
    /// The messages stored in each interaction are lossy since we explicitly do not
    /// persist all agent interactions, such as command executions. This is the same
    /// behavior as `chat/resume`.
    #[serde(rename = "chat")]
    #[ts(rename = "chat")]
    pub chat: Chat,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, ExperimentalApi)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatListParams {
    /// Opaque pagination cursor returned by a previous call.
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    /// Optional page size; defaults to a reasonable server-side value.
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
    /// Optional sort key; defaults to created_at.
    #[ts(optional = nullable)]
    pub sort_key: Option<ChatSortKey>,
    /// Optional sort direction; defaults to descending (newest first).
    #[ts(optional = nullable)]
    pub sort_direction: Option<SortDirection>,
    /// Optional provider filter; when set, only sessions recorded under these
    /// providers are returned. When present but empty, includes all providers.
    #[ts(optional = nullable)]
    pub model_providers: Option<Vec<String>>,
    /// Optional source filter; when set, only sessions from these source kinds
    /// are returned. When omitted or empty, defaults to interactive sources.
    #[ts(optional = nullable)]
    pub source_kinds: Option<Vec<ChatSourceKind>>,
    /// Optional archived filter; when set to true, only archived chats are returned.
    /// If false or null, only non-archived chats are returned.
    #[ts(optional = nullable)]
    pub archived: Option<bool>,
    /// Optional cwd filter or filters; when set, only chats whose session cwd
    /// exactly matches one of these paths are returned.
    #[ts(optional = nullable, type = "string | Array<string> | null")]
    pub cwd: Option<ChatListCwdFilter>,
    /// If true, return from the state DB without scanning JSONL rollouts to
    /// repair chat metadata. Omitted or false preserves scan-and-repair
    /// behavior.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub use_state_db_only: bool,
    /// Optional substring filter for the extracted chat title.
    #[ts(optional = nullable)]
    pub search_term: Option<String>,
    /// Optional direct parent chat filter.
    #[experimental("chat/list.parentChatId")]
    #[ts(optional = nullable)]
    pub parent_chat_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatSearchParams {
    /// Opaque pagination cursor returned by a previous call.
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    /// Optional page size; defaults to a reasonable server-side value.
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
    /// Optional sort key; defaults to created_at.
    #[ts(optional = nullable)]
    pub sort_key: Option<ChatSortKey>,
    /// Optional sort direction; defaults to descending (newest first).
    #[ts(optional = nullable)]
    pub sort_direction: Option<SortDirection>,
    /// Optional source filter; when set, only sessions from these source kinds
    /// are returned. When omitted or empty, defaults to interactive sources.
    #[ts(optional = nullable)]
    pub source_kinds: Option<Vec<ChatSourceKind>>,
    /// Optional archived filter; when set to true, only archived chats are returned.
    /// If false or null, only non-archived chats are returned.
    #[ts(optional = nullable)]
    pub archived: Option<bool>,
    /// Required substring/full-text query for chat search.
    pub search_term: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(untagged)]
pub enum ChatListCwdFilter {
    One(String),
    Many(Vec<String>),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum ChatSourceKind {
    Cli,
    #[serde(rename = "vscode")]
    #[ts(rename = "vscode")]
    VsCode,
    Exec,
    AppServer,
    SubAgent,
    SubAgentReview,
    SubAgentCompact,
    SubAgentThreadSpawn,
    SubAgentOther,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum ChatSortKey {
    CreatedAt,
    UpdatedAt,
    RecencyAt,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatListResponse {
    pub data: Vec<Chat>,
    /// Opaque cursor to pass to the next call to continue after the last item.
    /// if None, there are no more messages to return.
    pub next_cursor: Option<String>,
    /// Opaque cursor to pass as `cursor` when reversing `sortDirection`.
    /// This is only populated when the page contains at least one chat.
    /// Use it with the opposite `sortDirection`; for timestamp sorts it anchors
    /// at the start of the page timestamp so same-second updates are not skipped.
    pub backwards_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatSearchResult {
    #[serde(rename = "chat")]
    #[ts(rename = "chat")]
    pub chat: Chat,
    pub snippet: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatSearchResponse {
    pub data: Vec<ChatSearchResult>,
    /// Opaque cursor to pass to the next call to continue after the last item.
    /// if None, there are no more messages to return.
    pub next_cursor: Option<String>,
    /// Opaque cursor to pass as `cursor` when reversing `sortDirection`.
    /// This is only populated when the page contains at least one chat.
    /// Use it with the opposite `sortDirection`; for timestamp sorts it anchors
    /// at the start of the page timestamp so same-second updates are not skipped.
    pub backwards_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatLoadedListParams {
    /// Opaque pagination cursor returned by a previous call.
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    /// Optional page size; defaults to no limit.
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatLoadedListResponse {
    /// Chat ids for sessions currently loaded in memory.
    pub data: Vec<String>,
    /// Opaque cursor to pass to the next call to continue after the last item.
    /// if None, there are no more messages to return.
    pub next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type")]
#[ts(export_to = "v2/")]
pub enum ChatStatus {
    NotLoaded,
    Idle,
    SystemError,
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Active {
        active_flags: Vec<ChatActiveFlag>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum ChatActiveFlag {
    WaitingOnApproval,
    WaitingOnUserInput,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatReadParams {
    pub chat_id: String,
    /// When true, include interactions and their messages from rollout history.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_interactions: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatReadResponse {
    #[serde(rename = "chat")]
    #[ts(rename = "chat")]
    pub chat: Chat,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatInjectMessagesParams {
    pub chat_id: String,
    /// Raw Responses API messages to append to the chat's model-visible history.
    pub messages: Vec<JsonValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatInjectMessagesResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatInteractionsListParams {
    pub chat_id: String,
    /// Opaque cursor to pass to the next call to continue after the last interaction.
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    /// Optional interaction page size.
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
    /// Optional interaction pagination direction; defaults to descending.
    #[ts(optional = nullable)]
    pub sort_direction: Option<SortDirection>,
    /// How much message detail to include for each returned interaction; defaults to summary.
    #[ts(optional = nullable)]
    pub messages_view: Option<InteractionMessagesView>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatInteractionsListResponse {
    pub data: Vec<Interaction>,
    /// Opaque cursor to pass to the next call to continue after the last interaction.
    /// if None, there are no more interactions to return.
    pub next_cursor: Option<String>,
    /// Opaque cursor to pass as `cursor` when reversing `sortDirection`.
    /// This is only populated when the page contains at least one interaction.
    /// Use it with the opposite `sortDirection` to include the anchor interaction again
    /// and catch updates to that interaction.
    pub backwards_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatInteractionsMessagesListParams {
    pub chat_id: String,
    pub interaction_id: String,
    /// Opaque cursor to pass to the next call to continue after the last item.
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    /// Optional item page size.
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
    /// Optional item pagination direction; defaults to ascending.
    #[ts(optional = nullable)]
    pub sort_direction: Option<SortDirection>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatInteractionsMessagesListResponse {
    pub data: Vec<Message>,
    /// Opaque cursor to pass to the next call to continue after the last item.
    /// if None, there are no more messages to return.
    pub next_cursor: Option<String>,
    /// Opaque cursor to pass as `cursor` when reversing `sortDirection`.
    /// This is only populated when the page contains at least one item.
    pub backwards_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatTokenUsageUpdatedNotification {
    pub chat_id: String,
    pub interaction_id: String,
    pub token_usage: ChatTokenUsage,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatTokenUsage {
    pub total: TokenUsageBreakdown,
    pub last: TokenUsageBreakdown,
    // TODO(aibrahim): make this not optional
    #[ts(type = "number | null")]
    pub model_context_window: Option<i64>,
}

impl From<CoreTokenUsageInfo> for ChatTokenUsage {
    fn from(value: CoreTokenUsageInfo) -> Self {
        Self {
            total: value.total_token_usage.into(),
            last: value.last_token_usage.into(),
            model_context_window: value.model_context_window,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TokenUsageBreakdown {
    #[ts(type = "number")]
    pub total_tokens: i64,
    #[ts(type = "number")]
    pub input_tokens: i64,
    #[ts(type = "number")]
    pub cached_input_tokens: i64,
    #[ts(type = "number")]
    pub output_tokens: i64,
    #[ts(type = "number")]
    pub reasoning_output_tokens: i64,
}

impl From<CoreTokenUsage> for TokenUsageBreakdown {
    fn from(value: CoreTokenUsage) -> Self {
        Self {
            total_tokens: value.total_tokens,
            input_tokens: value.input_tokens,
            cached_input_tokens: value.cached_input_tokens,
            output_tokens: value.output_tokens,
            reasoning_output_tokens: value.reasoning_output_tokens,
        }
    }
}

// Chat/Interaction lifecycle notifications and item progress events
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatStartedNotification {
    #[serde(rename = "chat")]
    #[ts(rename = "chat")]
    pub chat: Chat,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatStatusChangedNotification {
    pub chat_id: String,
    pub status: ChatStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatArchivedNotification {
    pub chat_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatDeletedNotification {
    pub chat_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatUnarchivedNotification {
    pub chat_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatClosedNotification {
    pub chat_id: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatNameUpdatedNotification {
    pub chat_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    #[serde(rename = "chatName")]
    #[ts(rename = "chatName")]
    pub thread_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatGoalUpdatedNotification {
    pub chat_id: String,
    pub interaction_id: Option<String>,
    pub goal: ChatGoal,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ChatGoalClearedNotification {
    pub chat_id: String,
}

/// Deprecated: Use `ContextCompaction` item type instead.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ContextCompactedNotification {
    pub chat_id: String,
    pub interaction_id: String,
}
