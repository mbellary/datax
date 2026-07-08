use super::ChatStatus;
use super::CodexErrorInfo;
use super::InteractionStatus;
use super::Message;
use datax_protocol::protocol::SessionSource as CoreSessionSource;
use datax_protocol::protocol::SubAgentSource as CoreSubAgentSource;
use datax_protocol::protocol::ThreadSource as CoreThreadSource;
use datax_utils_absolute_path::AbsolutePathBuf;
use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::Schema;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use thiserror::Error;
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
#[derive(Default)]
pub enum SessionSource {
    Cli,
    #[serde(rename = "vscode")]
    #[ts(rename = "vscode")]
    #[default]
    VsCode,
    Exec,
    AppServer,
    Custom(String),
    SubAgent(CoreSubAgentSource),
    #[serde(other)]
    Unknown,
}

impl From<CoreSessionSource> for SessionSource {
    fn from(value: CoreSessionSource) -> Self {
        match value {
            CoreSessionSource::Cli => SessionSource::Cli,
            CoreSessionSource::VSCode => SessionSource::VsCode,
            CoreSessionSource::Exec => SessionSource::Exec,
            CoreSessionSource::Mcp => SessionSource::AppServer,
            CoreSessionSource::Custom(source) => SessionSource::Custom(source),
            // We do not want to render those at the app-server level.
            CoreSessionSource::Internal(_) => SessionSource::Unknown,
            CoreSessionSource::SubAgent(sub) => SessionSource::SubAgent(sub),
            CoreSessionSource::Unknown => SessionSource::Unknown,
        }
    }
}

impl From<SessionSource> for CoreSessionSource {
    fn from(value: SessionSource) -> Self {
        match value {
            SessionSource::Cli => CoreSessionSource::Cli,
            SessionSource::VsCode => CoreSessionSource::VSCode,
            SessionSource::Exec => CoreSessionSource::Exec,
            SessionSource::AppServer => CoreSessionSource::Mcp,
            SessionSource::Custom(source) => CoreSessionSource::Custom(source),
            SessionSource::SubAgent(sub) => CoreSessionSource::SubAgent(sub),
            SessionSource::Unknown => CoreSessionSource::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, TS)]
#[serde(try_from = "String", into = "String")]
#[ts(type = "string")]
#[ts(export_to = "v2/")]
pub enum ChatSource {
    User,
    Subagent,
    Feature(String),
    MemoryConsolidation,
}

impl JsonSchema for ChatSource {
    fn schema_name() -> String {
        "ChatSource".to_string()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        String::json_schema(generator)
    }
}

impl TryFrom<String> for ChatSource {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse::<CoreThreadSource>().map(Into::into)
    }
}

impl From<ChatSource> for String {
    fn from(value: ChatSource) -> Self {
        CoreThreadSource::from(value).into()
    }
}

impl From<CoreThreadSource> for ChatSource {
    fn from(value: CoreThreadSource) -> Self {
        match value {
            CoreThreadSource::User => ChatSource::User,
            CoreThreadSource::Subagent => ChatSource::Subagent,
            CoreThreadSource::Feature(feature) => ChatSource::Feature(feature),
            CoreThreadSource::MemoryConsolidation => ChatSource::MemoryConsolidation,
        }
    }
}

impl From<ChatSource> for CoreThreadSource {
    fn from(value: ChatSource) -> Self {
        match value {
            ChatSource::User => CoreThreadSource::User,
            ChatSource::Subagent => CoreThreadSource::Subagent,
            ChatSource::Feature(feature) => CoreThreadSource::Feature(feature),
            ChatSource::MemoryConsolidation => CoreThreadSource::MemoryConsolidation,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct GitInfo {
    pub sha: Option<String>,
    pub branch: Option<String>,
    pub origin_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct Chat {
    pub id: String,
    /// Session id shared by threads that belong to the same session tree.
    pub session_id: String,
    /// Source chat id when this chat was created by forking another chat.
    pub forked_from_id: Option<String>,
    /// The ID of the parent chat. This will only be set if this chat is a subagent.
    pub parent_chat_id: Option<String>,
    /// Usually the first user message in the thread, if available.
    pub preview: String,
    /// Whether the chat is ephemeral and should not be materialized on disk.
    pub ephemeral: bool,
    /// Model provider used for this thread (for example, 'openai').
    pub model_provider: String,
    /// Unix timestamp (in seconds) when the chat was created.
    #[ts(type = "number")]
    pub created_at: i64,
    /// Unix timestamp (in seconds) when the chat was last updated.
    #[ts(type = "number")]
    pub updated_at: i64,
    /// Unix timestamp (in seconds) used for chat recency ordering.
    #[ts(type = "number | null")]
    pub recency_at: Option<i64>,
    /// Current runtime status for the thread.
    pub status: ChatStatus,
    /// [UNSTABLE] Path to the chat on disk.
    pub path: Option<PathBuf>,
    /// Working directory captured for the thread.
    pub cwd: AbsolutePathBuf,
    /// Version of the CLI that created the thread.
    pub cli_version: String,
    /// Origin of the thread (CLI, VSCode, codex exec, codex app-server, etc.).
    pub source: SessionSource,
    /// Optional analytics source classification for this thread.
    pub chat_source: Option<ChatSource>,
    /// Optional random unique nickname assigned to an AgentControl-spawned sub-agent.
    pub agent_nickname: Option<String>,
    /// Optional role (agent_role) assigned to an AgentControl-spawned sub-agent.
    pub agent_role: Option<String>,
    /// Optional Git metadata captured when the chat was created.
    pub git_info: Option<GitInfo>,
    /// Optional user-facing chat title.
    pub name: Option<String>,
    /// Only populated on `chat/resume`, `chat/rollback`, `chat/fork`, and `chat/read`
    /// (when `includeInteractions` is true) responses.
    /// For all other responses and notifications returning a Chat,
    /// the interactions field will be an empty list.
    pub interactions: Vec<Interaction>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct Interaction {
    pub id: String,
    /// Chat messages currently included in this turn payload.
    pub messages: Vec<Message>,
    /// Describes how much of `messages` has been loaded for this turn.
    #[serde(default)]
    pub messages_view: InteractionMessagesView,
    pub status: InteractionStatus,
    /// Only populated when the Interaction's status is failed.
    pub error: Option<InteractionError>,
    /// Unix timestamp (in seconds) when the turn started.
    #[ts(type = "number | null")]
    pub started_at: Option<i64>,
    /// Unix timestamp (in seconds) when the turn completed.
    #[ts(type = "number | null")]
    pub completed_at: Option<i64>,
    /// Duration between turn start and completion in milliseconds, if known.
    #[ts(type = "number | null")]
    pub duration_ms: Option<i64>,
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum InteractionMessagesView {
    /// `messages` was not loaded for this turn. The field is intentionally empty.
    NotLoaded,
    /// `messages` contains only a display summary for this turn.
    Summary,
    /// `messages` contains every Message available from persisted app-server history for this turn.
    #[default]
    Full,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, Error)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
#[error("{message}")]
pub struct InteractionError {
    pub message: String,
    pub codex_error_info: Option<CodexErrorInfo>,
    #[serde(default)]
    pub additional_details: Option<String>,
}
