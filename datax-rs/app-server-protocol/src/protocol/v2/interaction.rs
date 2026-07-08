use super::ApprovalsReviewer;
use super::AskForApproval;
use super::Interaction;
use super::SandboxPolicy;
use datax_experimental_api_macros::ExperimentalApi;
use datax_protocol::config_types::CollaborationMode;
use datax_protocol::config_types::MultiAgentMode;
use datax_protocol::config_types::Personality;
use datax_protocol::config_types::ReasoningSummary;
use datax_protocol::models::ImageDetail;
use datax_protocol::openai_models::ReasoningEffort;
use datax_protocol::plan_tool::PlanItemArg as CorePlanItemArg;
use datax_protocol::plan_tool::StepStatus as CorePlanStepStatus;
use datax_protocol::user_input::ByteRange as CoreByteRange;
use datax_protocol::user_input::TextElement as CoreTextElement;
use datax_protocol::user_input::UserInput as CoreUserInput;
use datax_utils_absolute_path::AbsolutePathBuf;
use datax_utils_path_uri::LegacyAppPathString;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum InteractionStatus {
    Completed,
    Interrupted,
    Failed,
    InProgress,
}

// Interaction APIs
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, ExperimentalApi)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct InteractionEnvironmentParams {
    pub environment_id: String,
    pub cwd: LegacyAppPathString,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
#[ts(export_to = "v2/")]
pub enum AdditionalContextKind {
    Untrusted,
    Application,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct AdditionalContextEntry {
    pub value: String,
    pub kind: AdditionalContextKind,
}

#[derive(
    Serialize, Deserialize, Debug, Default, Clone, PartialEq, JsonSchema, TS, ExperimentalApi,
)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct InteractionStartParams {
    pub chat_id: String,
    #[ts(optional = nullable)]
    pub client_user_message_id: Option<String>,
    pub input: Vec<UserInput>,
    /// Optional metadata to enrich Codex's ResponsesAPI interaction metadata.
    ///
    /// Entries are flattened into the JSON string sent as
    /// `client_metadata["x-codex-turn-metadata"]` on ResponsesAPI HTTP and websocket requests.
    ///
    /// They are not sent as top-level ResponsesAPI `client_metadata` keys, and reserved keys
    /// such as `session_id`, `chat_id`, `interaction_id`, and `window_id` cannot be overridden.
    #[experimental("interaction/start.responsesapiClientMetadata")]
    #[ts(optional = nullable)]
    pub responsesapi_client_metadata: Option<HashMap<String, String>>,
    /// Optional client-provided context fragments keyed by an opaque source identifier.
    #[experimental("interaction/start.additionalContext")]
    #[ts(optional = nullable)]
    pub additional_context: Option<HashMap<String, AdditionalContextEntry>>,
    /// Optional environments for this interaction and subsequent interactions.
    ///
    /// Omitted uses the chat sticky environments. Empty disables
    /// environment access for this interaction. Non-empty selects the first
    /// environment as the current interaction environment for this interaction.
    #[experimental("interaction/start.environments")]
    #[ts(optional = nullable)]
    pub environments: Option<Vec<InteractionEnvironmentParams>>,
    /// Override the working directory for this interaction and subsequent interactions.
    #[ts(optional = nullable)]
    pub cwd: Option<PathBuf>,
    /// Replace the chat's runtime workspace roots for this interaction and
    /// subsequent interactions. Paths must be absolute.
    #[experimental("interaction/start.runtimeWorkspaceRoots")]
    #[ts(optional = nullable)]
    pub runtime_workspace_roots: Option<Vec<AbsolutePathBuf>>,
    /// Override the approval policy for this interaction and subsequent interactions.
    #[experimental(nested)]
    #[ts(optional = nullable)]
    pub approval_policy: Option<AskForApproval>,
    /// Override where approval requests are routed for review on this interaction and
    /// subsequent interactions.
    #[ts(optional = nullable)]
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    /// Override the sandbox policy for this interaction and subsequent interactions.
    #[ts(optional = nullable)]
    pub sandbox_policy: Option<SandboxPolicy>,
    /// Select a named permissions profile id for this interaction and subsequent
    /// interactions. Cannot be combined with `sandboxPolicy`.
    #[experimental("interaction/start.permissions")]
    #[ts(optional = nullable)]
    pub permissions: Option<String>,
    /// Override the model for this interaction and subsequent interactions.
    #[ts(optional = nullable)]
    pub model: Option<String>,
    /// Override the service tier for this interaction and subsequent interactions.
    #[serde(
        default,
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(optional = nullable)]
    pub service_tier: Option<Option<String>>,
    /// Override the reasoning effort for this interaction and subsequent interactions.
    #[ts(optional = nullable)]
    pub effort: Option<ReasoningEffort>,
    /// Override the reasoning summary for this interaction and subsequent interactions.
    #[ts(optional = nullable)]
    pub summary: Option<ReasoningSummary>,
    /// Override the personality for this interaction and subsequent interactions.
    #[ts(optional = nullable)]
    pub personality: Option<Personality>,
    /// Optional JSON Schema used to constrain the final assistant message for
    /// this interaction.
    #[ts(optional = nullable)]
    pub output_schema: Option<JsonValue>,

    /// EXPERIMENTAL - Set a pre-set collaboration mode.
    /// Takes precedence over model, reasoning_effort, and developer instructions if set.
    ///
    /// For `collaboration_mode.settings.developer_instructions`, `null` means
    /// "use the built-in instructions for the selected mode".
    #[experimental("interaction/start.collaborationMode")]
    #[ts(optional = nullable)]
    pub collaboration_mode: Option<CollaborationMode>,

    /// @deprecated Ignored. Use `effort: "ultra"` for proactive multi-agent behavior.
    #[experimental("interaction/start.multiAgentMode")]
    #[ts(optional = nullable)]
    pub multi_agent_mode: Option<MultiAgentMode>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct InteractionStartResponse {
    #[serde(rename = "interaction")]
    #[ts(rename = "interaction")]
    pub turn: Interaction,
}

#[derive(
    Serialize, Deserialize, Debug, Default, Clone, PartialEq, JsonSchema, TS, ExperimentalApi,
)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct InteractionSteerParams {
    pub chat_id: String,
    #[ts(optional = nullable)]
    pub client_user_message_id: Option<String>,
    pub input: Vec<UserInput>,
    /// Optional metadata to enrich Codex's ResponsesAPI interaction metadata.
    ///
    /// Entries are flattened into the JSON string sent as
    /// `client_metadata["x-codex-turn-metadata"]` on ResponsesAPI HTTP and websocket requests.
    ///
    /// They are not sent as top-level ResponsesAPI `client_metadata` keys, and reserved keys
    /// such as `session_id`, `chat_id`, `interaction_id`, and `window_id` cannot be overridden.
    #[experimental("interaction/steer.responsesapiClientMetadata")]
    #[ts(optional = nullable)]
    pub responsesapi_client_metadata: Option<HashMap<String, String>>,
    /// Optional client-provided context fragments keyed by an opaque source identifier.
    #[experimental("interaction/steer.additionalContext")]
    #[ts(optional = nullable)]
    pub additional_context: Option<HashMap<String, AdditionalContextEntry>>,
    /// Required active interaction id precondition. The request fails when it does not
    /// match the currently active interaction.
    #[serde(rename = "expectedInteractionId")]
    #[ts(rename = "expectedInteractionId")]
    pub expected_turn_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct InteractionSteerResponse {
    pub interaction_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct InteractionInterruptParams {
    pub chat_id: String,
    pub interaction_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct InteractionInterruptResponse {}

// User input types
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

impl From<CoreByteRange> for ByteRange {
    fn from(value: CoreByteRange) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl From<ByteRange> for CoreByteRange {
    fn from(value: ByteRange) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TextElement {
    /// Byte range in the parent `text` buffer that this element occupies.
    pub byte_range: ByteRange,
    /// Optional human-readable placeholder for the element, displayed in the UI.
    placeholder: Option<String>,
}

impl TextElement {
    pub fn new(byte_range: ByteRange, placeholder: Option<String>) -> Self {
        Self {
            byte_range,
            placeholder,
        }
    }

    pub fn set_placeholder(&mut self, placeholder: Option<String>) {
        self.placeholder = placeholder;
    }

    pub fn placeholder(&self) -> Option<&str> {
        self.placeholder.as_deref()
    }
}

impl From<CoreTextElement> for TextElement {
    fn from(value: CoreTextElement) -> Self {
        Self::new(
            value.byte_range.into(),
            value._placeholder_for_conversion_only().map(str::to_string),
        )
    }
}

impl From<TextElement> for CoreTextElement {
    fn from(value: TextElement) -> Self {
        Self::new(value.byte_range.into(), value.placeholder)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type")]
#[ts(export_to = "v2/")]
pub enum UserInput {
    Text {
        text: String,
        /// UI-defined spans within `text` used to render or persist special elements.
        #[serde(default)]
        text_elements: Vec<TextElement>,
    },
    Image {
        #[serde(default)]
        #[ts(optional)]
        detail: Option<ImageDetail>,
        url: String,
    },
    LocalImage {
        #[serde(default)]
        #[ts(optional)]
        detail: Option<ImageDetail>,
        path: PathBuf,
    },
    Skill {
        name: String,
        path: PathBuf,
    },
    Mention {
        name: String,
        path: String,
    },
}

impl UserInput {
    pub fn into_core(self) -> CoreUserInput {
        match self {
            UserInput::Text {
                text,
                text_elements,
            } => CoreUserInput::Text {
                text,
                text_elements: text_elements.into_iter().map(Into::into).collect(),
            },
            UserInput::Image { url, detail } => CoreUserInput::Image {
                image_url: url,
                detail,
            },
            UserInput::LocalImage { path, detail } => CoreUserInput::LocalImage { path, detail },
            UserInput::Skill { name, path } => CoreUserInput::Skill { name, path },
            UserInput::Mention { name, path } => CoreUserInput::Mention { name, path },
        }
    }
}

impl From<CoreUserInput> for UserInput {
    fn from(value: CoreUserInput) -> Self {
        match value {
            CoreUserInput::Text {
                text,
                text_elements,
            } => UserInput::Text {
                text,
                text_elements: text_elements.into_iter().map(Into::into).collect(),
            },
            CoreUserInput::Image { image_url, detail } => UserInput::Image {
                url: image_url,
                detail,
            },
            CoreUserInput::LocalImage { path, detail } => UserInput::LocalImage { path, detail },
            CoreUserInput::Skill { name, path } => UserInput::Skill { name, path },
            CoreUserInput::Mention { name, path } => UserInput::Mention { name, path },
            _ => unreachable!("unsupported user input variant"),
        }
    }
}

impl UserInput {
    pub fn text_char_count(&self) -> usize {
        match self {
            UserInput::Text { text, .. } => text.chars().count(),
            UserInput::Image { .. }
            | UserInput::LocalImage { .. }
            | UserInput::Skill { .. }
            | UserInput::Mention { .. } => 0,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct InteractionStartedNotification {
    pub chat_id: String,
    #[serde(rename = "interaction")]
    #[ts(rename = "interaction")]
    pub turn: Interaction,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct Usage {
    pub input_tokens: i32,
    pub cached_input_tokens: i32,
    pub output_tokens: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct InteractionCompletedNotification {
    pub chat_id: String,
    #[serde(rename = "interaction")]
    #[ts(rename = "interaction")]
    pub turn: Interaction,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
/// Notification that the interaction-level unified diff has changed.
/// Contains the latest aggregated diff across all file changes in the interaction.
pub struct InteractionDiffUpdatedNotification {
    pub chat_id: String,
    pub interaction_id: String,
    pub diff: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct InteractionPlanUpdatedNotification {
    pub chat_id: String,
    pub interaction_id: String,
    pub explanation: Option<String>,
    pub plan: Vec<InteractionPlanStep>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct InteractionPlanStep {
    pub step: String,
    pub status: InteractionPlanStepStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum InteractionPlanStepStatus {
    Pending,
    InProgress,
    Completed,
}

impl From<CorePlanItemArg> for InteractionPlanStep {
    fn from(value: CorePlanItemArg) -> Self {
        Self {
            step: value.step,
            status: value.status.into(),
        }
    }
}

impl From<CorePlanStepStatus> for InteractionPlanStepStatus {
    fn from(value: CorePlanStepStatus) -> Self {
        match value {
            CorePlanStepStatus::Pending => Self::Pending,
            CorePlanStepStatus::InProgress => Self::InProgress,
            CorePlanStepStatus::Completed => Self::Completed,
        }
    }
}
