use crate::bespoke_event_handling::apply_bespoke_event_handling;
use crate::bespoke_event_handling::maybe_emit_hook_prompt_item_completed;
use crate::command_exec::CommandExecManager;
use crate::command_exec::StartCommandExecParams;
use crate::config_manager::ConfigManager;
use crate::error_code::INPUT_TOO_LARGE_ERROR_CODE;
use crate::error_code::invalid_params;
use crate::models::supported_models;
use crate::outgoing_message::ConnectionId;
use crate::outgoing_message::ConnectionRequestId;
use crate::outgoing_message::OutgoingMessageSender;
use crate::outgoing_message::RequestContext;
use crate::outgoing_message::ThreadScopedOutgoingMessageSender;
use crate::skills_watcher::SkillsWatcher;
use crate::thread_status::ThreadWatchManager;
use crate::thread_status::resolve_thread_status;
use chrono::Duration as ChronoDuration;
use chrono::SecondsFormat;
use datax_analytics::AnalyticsEventsClient;
use datax_analytics::AnalyticsJsonRpcError;
use datax_analytics::InputError;
use datax_analytics::TurnSteerRequestError;
use datax_app_server_protocol::Account;
use datax_app_server_protocol::AccountLoginCompletedNotification;
use datax_app_server_protocol::AccountTokenUsageDailyBucket;
use datax_app_server_protocol::AccountTokenUsageSummary;
use datax_app_server_protocol::AccountUpdatedNotification;
use datax_app_server_protocol::AddCreditsNudgeCreditType;
use datax_app_server_protocol::AddCreditsNudgeEmailStatus;
use datax_app_server_protocol::AdditionalContextEntry;
use datax_app_server_protocol::AdditionalContextKind;
use datax_app_server_protocol::AppInfo;
use datax_app_server_protocol::AppListUpdatedNotification;
use datax_app_server_protocol::AppSummary;
use datax_app_server_protocol::AppTemplateSummary;
use datax_app_server_protocol::AppTemplateUnavailableReason;
use datax_app_server_protocol::AppsListParams;
use datax_app_server_protocol::AppsListResponse;
use datax_app_server_protocol::AskForApproval;
use datax_app_server_protocol::AuthMode;
use datax_app_server_protocol::CancelLoginAccountParams;
use datax_app_server_protocol::CancelLoginAccountResponse;
use datax_app_server_protocol::CancelLoginAccountStatus;
use datax_app_server_protocol::Chat;
use datax_app_server_protocol::ChatApproveGuardianDeniedActionParams;
use datax_app_server_protocol::ChatApproveGuardianDeniedActionResponse;
use datax_app_server_protocol::ChatArchiveParams;
use datax_app_server_protocol::ChatArchiveResponse;
use datax_app_server_protocol::ChatArchivedNotification;
use datax_app_server_protocol::ChatBackgroundTerminal;
use datax_app_server_protocol::ChatBackgroundTerminalsCleanParams;
use datax_app_server_protocol::ChatBackgroundTerminalsCleanResponse;
use datax_app_server_protocol::ChatBackgroundTerminalsListParams;
use datax_app_server_protocol::ChatBackgroundTerminalsListResponse;
use datax_app_server_protocol::ChatBackgroundTerminalsTerminateParams;
use datax_app_server_protocol::ChatBackgroundTerminalsTerminateResponse;
use datax_app_server_protocol::ChatClosedNotification;
use datax_app_server_protocol::ChatCompactStartParams;
use datax_app_server_protocol::ChatCompactStartResponse;
use datax_app_server_protocol::ChatDecrementElicitationParams;
use datax_app_server_protocol::ChatDecrementElicitationResponse;
use datax_app_server_protocol::ChatDeleteParams;
use datax_app_server_protocol::ChatDeleteResponse;
use datax_app_server_protocol::ChatDeletedNotification;
use datax_app_server_protocol::ChatForkParams;
use datax_app_server_protocol::ChatForkResponse;
use datax_app_server_protocol::ChatGoal;
use datax_app_server_protocol::ChatGoalClearParams;
use datax_app_server_protocol::ChatGoalClearResponse;
use datax_app_server_protocol::ChatGoalClearedNotification;
use datax_app_server_protocol::ChatGoalGetParams;
use datax_app_server_protocol::ChatGoalGetResponse;
use datax_app_server_protocol::ChatGoalSetParams;
use datax_app_server_protocol::ChatGoalSetResponse;
use datax_app_server_protocol::ChatGoalStatus;
use datax_app_server_protocol::ChatGoalUpdatedNotification;
use datax_app_server_protocol::ChatHistoryBuilder;
use datax_app_server_protocol::ChatIncrementElicitationParams;
use datax_app_server_protocol::ChatIncrementElicitationResponse;
use datax_app_server_protocol::ChatInjectMessagesParams;
use datax_app_server_protocol::ChatInjectMessagesResponse;
use datax_app_server_protocol::ChatInteractionsListParams;
use datax_app_server_protocol::ChatInteractionsListResponse;
use datax_app_server_protocol::ChatInteractionsMessagesListParams;
use datax_app_server_protocol::ChatListCwdFilter;
use datax_app_server_protocol::ChatListParams;
use datax_app_server_protocol::ChatListResponse;
use datax_app_server_protocol::ChatLoadedListParams;
use datax_app_server_protocol::ChatLoadedListResponse;
use datax_app_server_protocol::ChatMemoryModeSetParams;
use datax_app_server_protocol::ChatMemoryModeSetResponse;
use datax_app_server_protocol::ChatMetadataGitInfoUpdateParams;
use datax_app_server_protocol::ChatMetadataUpdateParams;
use datax_app_server_protocol::ChatMetadataUpdateResponse;
use datax_app_server_protocol::ChatNameUpdatedNotification;
use datax_app_server_protocol::ChatReadParams;
use datax_app_server_protocol::ChatReadResponse;
use datax_app_server_protocol::ChatRealtimeAppendAudioParams;
use datax_app_server_protocol::ChatRealtimeAppendAudioResponse;
use datax_app_server_protocol::ChatRealtimeAppendSpeechParams;
use datax_app_server_protocol::ChatRealtimeAppendSpeechResponse;
use datax_app_server_protocol::ChatRealtimeAppendTextParams;
use datax_app_server_protocol::ChatRealtimeAppendTextResponse;
use datax_app_server_protocol::ChatRealtimeListVoicesResponse;
use datax_app_server_protocol::ChatRealtimeStartParams;
use datax_app_server_protocol::ChatRealtimeStartResponse;
use datax_app_server_protocol::ChatRealtimeStartTransport;
use datax_app_server_protocol::ChatRealtimeStopParams;
use datax_app_server_protocol::ChatRealtimeStopResponse;
use datax_app_server_protocol::ChatResumeInitialInteractionsPageParams;
use datax_app_server_protocol::ChatResumeParams;
use datax_app_server_protocol::ChatResumeResponse;
use datax_app_server_protocol::ChatRollbackParams;
use datax_app_server_protocol::ChatSearchParams;
use datax_app_server_protocol::ChatSearchResponse;
use datax_app_server_protocol::ChatSearchResult;
use datax_app_server_protocol::ChatSetNameParams;
use datax_app_server_protocol::ChatSetNameResponse;
use datax_app_server_protocol::ChatSettings;
use datax_app_server_protocol::ChatSettingsUpdateParams;
use datax_app_server_protocol::ChatSettingsUpdateResponse;
use datax_app_server_protocol::ChatShellCommandParams;
use datax_app_server_protocol::ChatShellCommandResponse;
use datax_app_server_protocol::ChatSortKey;
use datax_app_server_protocol::ChatSourceKind;
use datax_app_server_protocol::ChatStartParams;
use datax_app_server_protocol::ChatStartResponse;
use datax_app_server_protocol::ChatStartedNotification;
use datax_app_server_protocol::ChatStatus;
use datax_app_server_protocol::ChatUnarchiveParams;
use datax_app_server_protocol::ChatUnarchiveResponse;
use datax_app_server_protocol::ChatUnarchivedNotification;
use datax_app_server_protocol::ChatUnsubscribeParams;
use datax_app_server_protocol::ChatUnsubscribeResponse;
use datax_app_server_protocol::ChatUnsubscribeStatus;
use datax_app_server_protocol::ClientInfo;
use datax_app_server_protocol::ClientRequest;
use datax_app_server_protocol::ClientResponsePayload;
use datax_app_server_protocol::CodexErrorInfo;
use datax_app_server_protocol::CollaborationModeListParams;
use datax_app_server_protocol::CollaborationModeListResponse;
use datax_app_server_protocol::CommandExecParams;
use datax_app_server_protocol::CommandExecResizeParams;
use datax_app_server_protocol::CommandExecTerminateParams;
use datax_app_server_protocol::CommandExecWriteParams;
use datax_app_server_protocol::ConfigWarningNotification;
use datax_app_server_protocol::ConsumeAccountRateLimitResetCreditOutcome;
use datax_app_server_protocol::ConsumeAccountRateLimitResetCreditParams;
use datax_app_server_protocol::ConsumeAccountRateLimitResetCreditResponse;
use datax_app_server_protocol::ConversationGitInfo;
use datax_app_server_protocol::ConversationSummary;
use datax_app_server_protocol::DynamicToolFunctionSpec;
use datax_app_server_protocol::DynamicToolNamespaceTool;
use datax_app_server_protocol::DynamicToolSpec;
use datax_app_server_protocol::EnvironmentAddParams;
use datax_app_server_protocol::EnvironmentAddResponse;
use datax_app_server_protocol::ExperimentalFeature as ApiExperimentalFeature;
use datax_app_server_protocol::ExperimentalFeatureListParams;
use datax_app_server_protocol::ExperimentalFeatureListResponse;
use datax_app_server_protocol::ExperimentalFeatureStage as ApiExperimentalFeatureStage;
use datax_app_server_protocol::FeedbackUploadParams;
use datax_app_server_protocol::FeedbackUploadResponse;
use datax_app_server_protocol::GetAccountParams;
use datax_app_server_protocol::GetAccountRateLimitsResponse;
use datax_app_server_protocol::GetAccountResponse;
use datax_app_server_protocol::GetAccountTokenUsageResponse;
use datax_app_server_protocol::GetAuthStatusParams;
use datax_app_server_protocol::GetAuthStatusResponse;
use datax_app_server_protocol::GetConversationSummaryParams;
use datax_app_server_protocol::GetConversationSummaryResponse;
use datax_app_server_protocol::GetWorkspaceMessagesResponse;
use datax_app_server_protocol::GitDiffToRemoteParams;
use datax_app_server_protocol::GitDiffToRemoteResponse;
use datax_app_server_protocol::GitInfo as ApiGitInfo;
use datax_app_server_protocol::HookMetadata;
use datax_app_server_protocol::HooksListParams;
use datax_app_server_protocol::HooksListResponse;
use datax_app_server_protocol::InitializeParams;
use datax_app_server_protocol::InitializeResponse;
use datax_app_server_protocol::Interaction;
use datax_app_server_protocol::InteractionEnvironmentParams;
use datax_app_server_protocol::InteractionError;
use datax_app_server_protocol::InteractionInterruptParams;
use datax_app_server_protocol::InteractionInterruptResponse;
use datax_app_server_protocol::InteractionMessagesView;
use datax_app_server_protocol::InteractionStartParams;
use datax_app_server_protocol::InteractionStartResponse;
use datax_app_server_protocol::InteractionStatus;
use datax_app_server_protocol::InteractionSteerParams;
use datax_app_server_protocol::InteractionSteerResponse;
use datax_app_server_protocol::JSONRPCErrorError;
use datax_app_server_protocol::ListMcpServerStatusParams;
use datax_app_server_protocol::ListMcpServerStatusResponse;
use datax_app_server_protocol::LoginAccountParams;
use datax_app_server_protocol::LoginAccountResponse;
use datax_app_server_protocol::LoginApiKeyParams;
use datax_app_server_protocol::LogoutAccountResponse;
use datax_app_server_protocol::MarketplaceAddParams;
use datax_app_server_protocol::MarketplaceAddResponse;
use datax_app_server_protocol::MarketplaceInterface;
use datax_app_server_protocol::MarketplaceRemoveParams;
use datax_app_server_protocol::MarketplaceRemoveResponse;
use datax_app_server_protocol::MarketplaceUpgradeErrorInfo;
use datax_app_server_protocol::MarketplaceUpgradeParams;
use datax_app_server_protocol::MarketplaceUpgradeResponse;
use datax_app_server_protocol::McpResourceReadParams;
use datax_app_server_protocol::McpResourceReadResponse;
use datax_app_server_protocol::McpServerOauthLoginCompletedNotification;
use datax_app_server_protocol::McpServerOauthLoginParams;
use datax_app_server_protocol::McpServerOauthLoginResponse;
use datax_app_server_protocol::McpServerRefreshResponse;
use datax_app_server_protocol::McpServerStatus;
use datax_app_server_protocol::McpServerStatusDetail;
use datax_app_server_protocol::McpServerToolCallParams;
use datax_app_server_protocol::McpServerToolCallResponse;
use datax_app_server_protocol::MemoryResetResponse;
use datax_app_server_protocol::Message;
use datax_app_server_protocol::MockExperimentalMethodParams;
use datax_app_server_protocol::MockExperimentalMethodResponse;
use datax_app_server_protocol::ModelListParams;
use datax_app_server_protocol::ModelListResponse;
use datax_app_server_protocol::PermissionProfileListParams;
use datax_app_server_protocol::PermissionProfileListResponse;
use datax_app_server_protocol::PermissionProfileSummary;
use datax_app_server_protocol::PluginDetail;
use datax_app_server_protocol::PluginInstallParams;
use datax_app_server_protocol::PluginInstallResponse;
use datax_app_server_protocol::PluginInstalledParams;
use datax_app_server_protocol::PluginInstalledResponse;
use datax_app_server_protocol::PluginInterface;
use datax_app_server_protocol::PluginListMarketplaceKind;
use datax_app_server_protocol::PluginListParams;
use datax_app_server_protocol::PluginListResponse;
use datax_app_server_protocol::PluginMarketplaceEntry;
use datax_app_server_protocol::PluginReadParams;
use datax_app_server_protocol::PluginReadResponse;
use datax_app_server_protocol::PluginShareCheckoutParams;
use datax_app_server_protocol::PluginShareCheckoutResponse;
use datax_app_server_protocol::PluginShareContext;
use datax_app_server_protocol::PluginShareDeleteParams;
use datax_app_server_protocol::PluginShareDeleteResponse;
use datax_app_server_protocol::PluginShareDiscoverability;
use datax_app_server_protocol::PluginShareListItem;
use datax_app_server_protocol::PluginShareListParams;
use datax_app_server_protocol::PluginShareListResponse;
use datax_app_server_protocol::PluginSharePrincipal;
use datax_app_server_protocol::PluginSharePrincipalType;
use datax_app_server_protocol::PluginShareSaveParams;
use datax_app_server_protocol::PluginShareSaveResponse;
use datax_app_server_protocol::PluginShareTarget;
use datax_app_server_protocol::PluginShareUpdateDiscoverability;
use datax_app_server_protocol::PluginShareUpdateTargetsParams;
use datax_app_server_protocol::PluginShareUpdateTargetsResponse;
use datax_app_server_protocol::PluginSkillReadParams;
use datax_app_server_protocol::PluginSkillReadResponse;
use datax_app_server_protocol::PluginSource;
use datax_app_server_protocol::PluginSummary;
use datax_app_server_protocol::PluginUninstallParams;
use datax_app_server_protocol::PluginUninstallResponse;
use datax_app_server_protocol::RateLimitResetCreditsSummary;
use datax_app_server_protocol::RequestId;
use datax_app_server_protocol::ReviewDelivery as ApiReviewDelivery;
use datax_app_server_protocol::ReviewStartParams;
use datax_app_server_protocol::ReviewStartResponse;
use datax_app_server_protocol::ReviewTarget as ApiReviewTarget;
use datax_app_server_protocol::SandboxMode;
use datax_app_server_protocol::SendAddCreditsNudgeEmailParams;
use datax_app_server_protocol::SendAddCreditsNudgeEmailResponse;
use datax_app_server_protocol::ServerNotification;
use datax_app_server_protocol::ServerNotification::*;
use datax_app_server_protocol::ServerRequestResolvedNotification;
use datax_app_server_protocol::SkillSummary;
use datax_app_server_protocol::SkillsConfigWriteParams;
use datax_app_server_protocol::SkillsConfigWriteResponse;
use datax_app_server_protocol::SkillsExtraRootsSetParams;
use datax_app_server_protocol::SkillsExtraRootsSetResponse;
use datax_app_server_protocol::SkillsListParams;
use datax_app_server_protocol::SkillsListResponse;
use datax_app_server_protocol::SortDirection;
use datax_app_server_protocol::UserInput as V2UserInput;
use datax_app_server_protocol::WindowsSandboxReadiness;
use datax_app_server_protocol::WindowsSandboxReadinessResponse;
use datax_app_server_protocol::WindowsSandboxSetupCompletedNotification;
use datax_app_server_protocol::WindowsSandboxSetupMode;
use datax_app_server_protocol::WindowsSandboxSetupStartParams;
use datax_app_server_protocol::WindowsSandboxSetupStartResponse;
use datax_app_server_protocol::WorkspaceMessage;
use datax_app_server_protocol::WorkspaceMessageType;
use datax_arg0::Arg0DispatchPaths;
use datax_backend_client::AddCreditsNudgeCreditType as BackendAddCreditsNudgeCreditType;
use datax_backend_client::Client as BackendClient;
use datax_backend_client::CodexWorkspaceMessage as BackendWorkspaceMessage;
use datax_backend_client::CodexWorkspaceMessageType as BackendWorkspaceMessageType;
use datax_backend_client::CodexWorkspaceMessagesResponse as BackendWorkspaceMessagesResponse;
use datax_backend_client::ConsumeRateLimitResetCreditCode as BackendConsumeRateLimitResetCreditCode;
use datax_backend_client::RequestError as BackendRequestError;
use datax_backend_client::TokenUsageProfile;
use datax_chatgpt::connectors;
use datax_chatgpt::workspace_settings;
use datax_config::CloudConfigBundleLoadError;
use datax_config::CloudConfigBundleLoadErrorCode;
use datax_config::ConfigLayerStack;
use datax_config::loader::project_trust_key;
use datax_config::types::McpServerTransportConfig;
use datax_core::CodexThread;
use datax_core::CodexThreadSettingsOverrides;
use datax_core::ForkSnapshot;
use datax_core::McpManager;
use datax_core::NewThread;
#[cfg(test)]
use datax_core::SessionMeta;
use datax_core::StartThreadOptions;
use datax_core::SteerInputError;
use datax_core::ThreadConfigSnapshot;
use datax_core::ThreadManager;
use datax_core::config::Config;
use datax_core::config::ConfigOverrides;
use datax_core::config::NetworkProxyAuditMetadata;
use datax_core::config::edit::ConfigEdit;
use datax_core::config::edit::ConfigEditsBuilder;
use datax_core::connectors::AccessibleConnectorsStatus;
use datax_core::exec::ExecCapturePolicy;
use datax_core::exec::ExecExpiration;
use datax_core::exec::ExecParams;
use datax_core::exec_env::create_env;
use datax_core::path_utils;
#[cfg(test)]
use datax_core::read_head_for_summary;
use datax_core::sandboxing::SandboxPermissions;
use datax_core::windows_sandbox::WindowsSandboxLevelExt;
use datax_core::windows_sandbox::WindowsSandboxSetupMode as CoreWindowsSandboxSetupMode;
use datax_core::windows_sandbox::WindowsSandboxSetupRequest;
use datax_core::windows_sandbox::sandbox_setup_is_complete;
use datax_core_plugins::PluginInstallError as CorePluginInstallError;
use datax_core_plugins::PluginInstallRequest;
use datax_core_plugins::PluginReadRequest;
use datax_core_plugins::PluginUninstallError as CorePluginUninstallError;
use datax_core_plugins::PluginsManager;
use datax_core_plugins::loader::load_plugin_apps;
use datax_core_plugins::loader::load_plugin_mcp_servers;
use datax_core_plugins::manifest::PluginManifestInterface;
use datax_core_plugins::marketplace::MarketplaceError;
use datax_core_plugins::marketplace::MarketplacePluginSource;
use datax_core_plugins::marketplace_add::MarketplaceAddError;
use datax_core_plugins::marketplace_add::MarketplaceAddRequest;
use datax_core_plugins::marketplace_add::add_marketplace as add_marketplace_to_codex_home;
use datax_core_plugins::marketplace_remove::MarketplaceRemoveError;
use datax_core_plugins::marketplace_remove::MarketplaceRemoveRequest as CoreMarketplaceRemoveRequest;
use datax_core_plugins::marketplace_remove::remove_marketplace;
use datax_core_plugins::remote::RemoteMarketplace;
use datax_core_plugins::remote::RemoteMarketplaceSource;
use datax_core_plugins::remote::RemotePluginCatalogError;
use datax_core_plugins::remote::RemotePluginDetail as RemoteCatalogPluginDetail;
use datax_core_plugins::remote::RemotePluginServiceConfig;
use datax_core_plugins::remote::RemotePluginShareContext as RemoteCatalogPluginShareContext;
use datax_core_plugins::remote::RemotePluginShareSummary as RemoteCatalogPluginShareSummary;
use datax_core_plugins::remote::RemotePluginSummary as RemoteCatalogPluginSummary;
use datax_exec_server::EnvironmentManager;
use datax_exec_server::LOCAL_ENVIRONMENT_ID;
use datax_exec_server::LOCAL_FS;
use datax_features::FEATURES;
use datax_features::Feature;
use datax_features::Stage;
use datax_feedback::CodexFeedback;
use datax_feedback::FeedbackAttachmentPath;
use datax_feedback::FeedbackUploadOptions;
use datax_git_utils::git_diff_to_remote;
use datax_git_utils::resolve_root_git_project_for_trust;
use datax_login::AuthManager;
use datax_login::CodexAuth;
use datax_login::ServerOptions as LoginServerOptions;
use datax_login::ShutdownHandle;
use datax_login::auth::login_with_chatgpt_auth_tokens;
use datax_login::complete_device_code_login;
use datax_login::login_with_api_key;
use datax_login::oauth_client_id;
use datax_login::request_device_code;
use datax_login::run_login_server;
use datax_mcp::McpRuntimeContext;
use datax_mcp::McpServerStatusSnapshot;
use datax_mcp::McpSnapshotDetail;
use datax_mcp::collect_mcp_server_status_snapshot_with_detail;
use datax_mcp::discover_supported_scopes;
use datax_mcp::read_mcp_resource as read_mcp_resource_without_thread;
use datax_mcp::resolve_oauth_scopes;
use datax_memories_write::clear_memory_roots_contents;
use datax_model_provider::create_model_provider;
use datax_models_manager::collaboration_mode_presets::builtin_collaboration_mode_presets;
use datax_protocol::ChatId;
use datax_protocol::config_types::CollaborationMode;
use datax_protocol::config_types::ForcedLoginMethod;
use datax_protocol::config_types::Personality;
use datax_protocol::config_types::ReasoningSummary;
use datax_protocol::config_types::TrustLevel;
use datax_protocol::config_types::WindowsSandboxLevel;
use datax_protocol::error::CodexErr;
use datax_protocol::error::Result as CodexResult;
#[cfg(test)]
use datax_protocol::items::TurnItem;
use datax_protocol::models::ResponseItem;
use datax_protocol::openai_models::ReasoningEffort;
#[cfg(test)]
use datax_protocol::permissions::FileSystemSandboxPolicy;
use datax_protocol::protocol::AgentStatus;
use datax_protocol::protocol::ConversationAudioParams;
use datax_protocol::protocol::ConversationSpeechParams;
use datax_protocol::protocol::ConversationStartParams;
use datax_protocol::protocol::ConversationStartTransport;
use datax_protocol::protocol::ConversationTextParams;
use datax_protocol::protocol::EventMsg;
#[cfg(test)]
use datax_protocol::protocol::GitInfo as CoreGitInfo;
use datax_protocol::protocol::InitialHistory;
use datax_protocol::protocol::McpAuthStatus as CoreMcpAuthStatus;
use datax_protocol::protocol::Op;
use datax_protocol::protocol::RealtimeVoicesList;
use datax_protocol::protocol::ResumedHistory;
use datax_protocol::protocol::ReviewDelivery as CoreReviewDelivery;
use datax_protocol::protocol::ReviewRequest;
use datax_protocol::protocol::ReviewTarget as CoreReviewTarget;
use datax_protocol::protocol::RolloutItem;
use datax_protocol::protocol::SessionConfiguredEvent;
#[cfg(test)]
use datax_protocol::protocol::SessionMetaLine;
use datax_protocol::protocol::TurnEnvironmentSelection;
use datax_protocol::protocol::TurnEnvironmentSelections;
use datax_protocol::protocol::USER_MESSAGE_BEGIN;
use datax_protocol::protocol::W3cTraceContext;
use datax_protocol::user_input::MAX_USER_INPUT_TEXT_CHARS;
use datax_protocol::user_input::UserInput as CoreInputItem;
use datax_rmcp_client::perform_oauth_login_return_url;
use datax_rollout::is_persisted_rollout_item;
use datax_rollout::state_db::StateDbHandle;
use datax_rollout::state_db::reconcile_rollout;
use datax_state::ThreadMetadata;
use datax_state::log_db::LogDbLayer;
use datax_thread_store::ArchiveThreadParams as StoreArchiveThreadParams;
use datax_thread_store::DeleteThreadParams as StoreDeleteThreadParams;
use datax_thread_store::GitInfoPatch as StoreGitInfoPatch;
use datax_thread_store::ListThreadsParams as StoreListThreadsParams;
use datax_thread_store::LocalThreadStore;
use datax_thread_store::ReadThreadByRolloutPathParams as StoreReadThreadByRolloutPathParams;
use datax_thread_store::ReadThreadParams as StoreReadThreadParams;
use datax_thread_store::SearchThreadsParams as StoreSearchThreadsParams;
use datax_thread_store::SortDirection as StoreSortDirection;
use datax_thread_store::StoredThread;
use datax_thread_store::ThreadMetadataPatch as StoreThreadMetadataPatch;
use datax_thread_store::ThreadSortKey as StoreThreadSortKey;
use datax_thread_store::ThreadStore;
use datax_thread_store::ThreadStoreError;
use datax_utils_absolute_path::AbsolutePathBuf;
use datax_utils_pty::DEFAULT_OUTPUT_BYTES_CAP;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Error as IoError;
use std::path::Path;
use std::path::PathBuf;
use std::result::Result;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio::sync::Semaphore;
use tokio::sync::SemaphorePermit;
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tokio_util::sync::DropGuard;
use tokio_util::task::TaskTracker;
use toml::Value as TomlValue;
use tracing::Instrument;
use tracing::error;
use tracing::info;
use tracing::warn;
use uuid::Uuid;

#[cfg(test)]
use datax_app_server_protocol::ServerRequest;

mod account_processor;
mod apps_processor;
mod catalog_processor;
mod chat_processor;
mod command_exec_processor;
mod config_processor;
mod environment_processor;
mod external_agent_config_processor;
mod external_agent_session_import;
mod feedback_doctor_report;
mod feedback_processor;
mod fs_processor;
mod git_processor;
mod initialize_processor;
mod interaction_processor;
mod marketplace_processor;
mod mcp_processor;
mod plugins;
mod process_exec_processor;
mod remote_control_processor;
mod search;
mod token_usage_replay;
mod windows_sandbox_processor;

pub(crate) use account_processor::AccountRequestProcessor;
pub(crate) use apps_processor::AppsRequestProcessor;
pub(crate) use catalog_processor::CatalogRequestProcessor;
pub(crate) use chat_processor::ChatRequestProcessor;
pub(crate) use command_exec_processor::CommandExecRequestProcessor;
pub(crate) use config_processor::ConfigRequestProcessor;
pub(crate) use environment_processor::EnvironmentRequestProcessor;
pub(crate) use external_agent_config_processor::ExternalAgentConfigRequestProcessor;
pub(crate) use external_agent_config_processor::ExternalAgentConfigRequestProcessorArgs;
pub(crate) use feedback_processor::FeedbackRequestProcessor;
pub(crate) use fs_processor::FsRequestProcessor;
pub(crate) use git_processor::GitRequestProcessor;
pub(crate) use initialize_processor::InitializeRequestProcessor;
pub(crate) use interaction_processor::InteractionRequestProcessor;
pub(crate) use marketplace_processor::MarketplaceRequestProcessor;
pub(crate) use mcp_processor::McpRequestProcessor;
pub(crate) use plugins::PluginRequestProcessor;
pub(crate) use process_exec_processor::ProcessExecRequestProcessor;
pub(crate) use remote_control_processor::RemoteControlRequestProcessor;
pub(crate) use search::SearchRequestProcessor;
pub(crate) use thread_goal_processor::ThreadGoalRequestProcessor;
pub(crate) use windows_sandbox_processor::WindowsSandboxRequestProcessor;

use crate::error_code::internal_error;
use crate::error_code::invalid_request;
use crate::filters::compute_source_filters;
use crate::filters::source_kind_matches;
use crate::thread_state::ConnectionCapabilities;
use crate::thread_state::ThreadListenerCommand;
use crate::thread_state::ThreadState;
use crate::thread_state::ThreadStateManager;
use token_usage_replay::latest_token_usage_interaction_id_from_rollout_items;
use token_usage_replay::send_thread_token_usage_update_to_connection;

fn resolve_request_cwd(cwd: Option<PathBuf>) -> Result<Option<AbsolutePathBuf>, JSONRPCErrorError> {
    cwd.map(|cwd| {
        AbsolutePathBuf::relative_to_current_dir(path_utils::normalize_for_native_workdir(cwd))
            .map_err(|err| invalid_request(format!("invalid cwd: {err}")))
    })
    .transpose()
}

fn resolve_turn_environment_selections(
    thread_manager: &ThreadManager,
    environments: Option<Vec<InteractionEnvironmentParams>>,
) -> Result<Option<Vec<TurnEnvironmentSelection>>, JSONRPCErrorError> {
    let Some(environments) = environments else {
        return Ok(None);
    };
    let mut selections = Vec::with_capacity(environments.len());
    for environment in environments {
        let environment_id = environment.environment_id;
        let cwd = environment
            .cwd
            .to_inferred_path_uri()
            .ok_or_else(|| {
                invalid_request(format!(
                    "invalid cwd for environment `{environment_id}`: path `{}` does not use absolute POSIX or Windows path syntax",
                    environment.cwd
                ))
            })?;
        selections.push(TurnEnvironmentSelection {
            environment_id,
            cwd,
        });
    }
    thread_manager
        .validate_environment_selections(&selections)
        .map_err(environment_selection_error)?;
    Ok(Some(selections))
}

fn resolve_runtime_workspace_roots(workspace_roots: Vec<AbsolutePathBuf>) -> Vec<AbsolutePathBuf> {
    let mut resolved_roots = Vec::new();
    for root in workspace_roots {
        if !resolved_roots.iter().any(|existing| existing == &root) {
            resolved_roots.push(root);
        }
    }
    resolved_roots
}

mod config_errors;
mod request_errors;
mod thread_delete;
mod thread_goal_processor;
mod thread_lifecycle;
mod thread_resume_redaction;
mod thread_summary;

use self::config_errors::*;
use self::request_errors::*;
use self::thread_goal_processor::api_thread_goal_from_state;
use self::thread_lifecycle::*;
use self::thread_resume_redaction::*;
use self::thread_summary::*;

pub(crate) use self::chat_processor::chat_from_stored_thread;
pub(crate) use self::thread_lifecycle::populate_thread_turns_from_history;
#[cfg(test)]
pub(crate) use self::thread_summary::read_summary_from_rollout;
#[cfg(test)]
pub(crate) use self::thread_summary::summary_to_thread;
pub(crate) use self::thread_summary::thread_settings_from_config_snapshot;
pub(crate) use self::thread_summary::thread_settings_from_core_snapshot;

pub(crate) fn build_api_turns_from_rollout_items(messages: &[RolloutItem]) -> Vec<Interaction> {
    let mut builder = ChatHistoryBuilder::new();
    for item in messages {
        if is_persisted_rollout_item(item) {
            builder.handle_rollout_item(item);
        }
    }
    builder.finish()
}
