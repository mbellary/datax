use super::*;
use crate::error_code::method_not_found;
use datax_app_server_protocol::SelectedCapabilityRoot;
use datax_extension_api::ExtensionDataInit;
use datax_protocol::config_types::MultiAgentMode;
use datax_protocol::models::BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS;
use datax_protocol::models::BUILT_IN_PERMISSION_PROFILE_WORKSPACE;

const THREAD_LIST_DEFAULT_LIMIT: usize = 25;
const THREAD_LIST_MAX_LIMIT: usize = 100;

struct ThreadListFilters {
    model_providers: Option<Vec<String>>,
    source_kinds: Option<Vec<ChatSourceKind>>,
    archived: bool,
    cwd_filters: Option<Vec<PathBuf>>,
    search_term: Option<String>,
    use_state_db_only: bool,
    parent_chat_id: Option<ChatId>,
}

fn collect_resume_override_mismatches(
    request: &ChatResumeParams,
    config_snapshot: &ThreadConfigSnapshot,
) -> Vec<String> {
    let mut mismatch_details = Vec::new();

    if let Some(requested_model) = request.model.as_deref()
        && requested_model != config_snapshot.model
    {
        mismatch_details.push(format!(
            "model requested={requested_model} active={}",
            config_snapshot.model
        ));
    }
    if let Some(requested_provider) = request.model_provider.as_deref()
        && requested_provider != config_snapshot.model_provider_id
    {
        mismatch_details.push(format!(
            "model_provider requested={requested_provider} active={}",
            config_snapshot.model_provider_id
        ));
    }
    if let Some(requested_service_tier) = request.service_tier.as_ref()
        && requested_service_tier != &config_snapshot.service_tier
    {
        mismatch_details.push(format!(
            "service_tier requested={requested_service_tier:?} active={:?}",
            config_snapshot.service_tier
        ));
    }
    if let Some(requested_cwd) = request.cwd.as_deref() {
        let requested_cwd_path = std::path::PathBuf::from(requested_cwd);
        if requested_cwd_path != config_snapshot.cwd().as_path() {
            mismatch_details.push(format!(
                "cwd requested={} active={}",
                requested_cwd_path.display(),
                config_snapshot.cwd().display()
            ));
        }
    }
    if let Some(requested_runtime_workspace_roots) = request.runtime_workspace_roots.as_ref() {
        let requested_runtime_workspace_roots = requested_runtime_workspace_roots.to_vec();
        if requested_runtime_workspace_roots != config_snapshot.workspace_roots {
            mismatch_details.push(format!(
                "runtime_workspace_roots requested={requested_runtime_workspace_roots:?} active={:?}",
                config_snapshot.workspace_roots
            ));
        }
    }
    if let Some(requested_approval) = request.approval_policy.as_ref() {
        let active_approval: AskForApproval = config_snapshot.approval_policy.into();
        if requested_approval != &active_approval {
            mismatch_details.push(format!(
                "approval_policy requested={requested_approval:?} active={active_approval:?}"
            ));
        }
    }
    if let Some(requested_review_policy) = request.approvals_reviewer.as_ref() {
        let active_review_policy: datax_app_server_protocol::ApprovalsReviewer =
            config_snapshot.approvals_reviewer.into();
        if requested_review_policy != &active_review_policy {
            mismatch_details.push(format!(
                "approvals_reviewer requested={requested_review_policy:?} active={active_review_policy:?}"
            ));
        }
    }
    if let Some(requested_sandbox) = request.sandbox.as_ref() {
        let active_sandbox = config_snapshot.sandbox_policy();
        let sandbox_matches = matches!(
            (requested_sandbox, &active_sandbox),
            (
                SandboxMode::ReadOnly,
                datax_protocol::protocol::SandboxPolicy::ReadOnly { .. }
            ) | (
                SandboxMode::WorkspaceWrite,
                datax_protocol::protocol::SandboxPolicy::WorkspaceWrite { .. }
            ) | (
                SandboxMode::DangerFullAccess,
                datax_protocol::protocol::SandboxPolicy::DangerFullAccess
            ) | (
                SandboxMode::DangerFullAccess,
                datax_protocol::protocol::SandboxPolicy::ExternalSandbox { .. }
            )
        );
        if !sandbox_matches {
            mismatch_details.push(format!(
                "sandbox requested={requested_sandbox:?} active={active_sandbox:?}"
            ));
        }
    }
    if request.permissions.is_some() {
        mismatch_details.push(format!(
            "permissions override was provided and ignored while running; active={:?}",
            config_snapshot.active_permission_profile
        ));
    }
    if let Some(requested_personality) = request.personality.as_ref()
        && config_snapshot.personality.as_ref() != Some(requested_personality)
    {
        mismatch_details.push(format!(
            "personality requested={requested_personality:?} active={:?}",
            config_snapshot.personality
        ));
    }

    if request.config.is_some() {
        mismatch_details
            .push("config overrides were provided and ignored while running".to_string());
    }
    if request.base_instructions.is_some() {
        mismatch_details
            .push("baseInstructions override was provided and ignored while running".to_string());
    }
    if request.developer_instructions.is_some() {
        mismatch_details.push(
            "developerInstructions override was provided and ignored while running".to_string(),
        );
    }
    mismatch_details
}

fn merge_persisted_resume_metadata(
    request_overrides: &mut Option<HashMap<String, serde_json::Value>>,
    typesafe_overrides: &mut ConfigOverrides,
    persisted_metadata: &ThreadMetadata,
) {
    if has_model_resume_override(request_overrides.as_ref(), typesafe_overrides) {
        return;
    }

    typesafe_overrides.model = persisted_metadata.model.clone();
    typesafe_overrides.model_provider = Some(persisted_metadata.model_provider.clone());

    if let Some(reasoning_effort) = persisted_metadata.reasoning_effort.as_ref() {
        request_overrides.get_or_insert_with(HashMap::new).insert(
            "model_reasoning_effort".to_string(),
            serde_json::Value::String(reasoning_effort.to_string()),
        );
    }
}

fn normalize_thread_list_cwd_filters(
    cwd: Option<ChatListCwdFilter>,
) -> Result<Option<Vec<PathBuf>>, JSONRPCErrorError> {
    let Some(cwd) = cwd else {
        return Ok(None);
    };

    let cwds = match cwd {
        ChatListCwdFilter::One(cwd) => vec![cwd],
        ChatListCwdFilter::Many(cwds) => cwds,
    };
    let mut normalized_cwds = Vec::with_capacity(cwds.len());
    for cwd in cwds {
        let cwd = AbsolutePathBuf::relative_to_current_dir(cwd.as_str())
            .map(AbsolutePathBuf::into_path_buf)
            .map_err(|err| {
                invalid_params(format!("invalid chat/list cwd filter `{cwd}`: {err}"))
            })?;
        normalized_cwds.push(cwd);
    }

    Ok(Some(normalized_cwds))
}

fn has_model_resume_override(
    request_overrides: Option<&HashMap<String, serde_json::Value>>,
    typesafe_overrides: &ConfigOverrides,
) -> bool {
    typesafe_overrides.model.is_some()
        || typesafe_overrides.model_provider.is_some()
        || request_overrides.is_some_and(|overrides| overrides.contains_key("model"))
        || request_overrides
            .is_some_and(|overrides| overrides.contains_key("model_reasoning_effort"))
}

fn validate_dynamic_tools(tools: &[DynamicToolSpec]) -> Result<(), String> {
    const DYNAMIC_TOOL_NAME_MAX_LEN: usize = 128;
    const DYNAMIC_TOOL_NAMESPACE_MAX_LEN: usize = 64;
    const DYNAMIC_TOOL_NAMESPACE_DESCRIPTION_MAX_LEN: usize = 1024;
    const DYNAMIC_TOOL_IDENTIFIER_PATTERN: &str = "^[a-zA-Z0-9_-]+$";
    const RESERVED_RESPONSES_NAMESPACES: &[&str] = &[
        "api_tool",
        "browser",
        "computer",
        "container",
        "file_search",
        "functions",
        "image_gen",
        "multi_tool_use",
        "python",
        "python_user_visible",
        "submodel_delegator",
        "terminal",
        "tool_search",
        "web",
    ];

    fn escape_identifier_for_error(value: &str) -> String {
        value.escape_default().to_string()
    }

    fn validate_dynamic_tool_identifier(
        value: &str,
        label: &str,
        max_len: usize,
    ) -> Result<(), String> {
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(format!(
                "{label} must match {DYNAMIC_TOOL_IDENTIFIER_PATTERN} to match Responses API: {}",
                escape_identifier_for_error(value),
            ));
        }
        if value.chars().count() > max_len {
            return Err(format!(
                "{label} must be at most {max_len} characters to match Responses API: {}",
                escape_identifier_for_error(value),
            ));
        }
        Ok(())
    }

    fn validate_dynamic_tool<'a>(
        tool: &'a DynamicToolFunctionSpec,
        namespace: Option<&str>,
        seen: &mut HashSet<&'a str>,
    ) -> Result<(), String> {
        let name = tool.name.trim();
        if name.is_empty() {
            return Err("dynamic tool name must not be empty".to_string());
        }
        if name != tool.name {
            return Err(format!(
                "dynamic tool name has leading/trailing whitespace: {}",
                escape_identifier_for_error(&tool.name),
            ));
        }
        validate_dynamic_tool_identifier(name, "dynamic tool name", DYNAMIC_TOOL_NAME_MAX_LEN)?;
        if name == "mcp" || name.starts_with("mcp__") {
            return Err(format!("dynamic tool name is reserved: {name}"));
        }
        if !seen.insert(name) {
            if let Some(namespace) = namespace {
                return Err(format!(
                    "duplicate dynamic tool name in namespace {namespace}: {name}"
                ));
            }
            return Err(format!("duplicate dynamic tool name: {name}"));
        }
        if tool.defer_loading && namespace.is_none() {
            return Err(format!(
                "deferred dynamic tool must include a namespace: {name}"
            ));
        }

        if let Err(err) = datax_tools::parse_tool_input_schema(&tool.input_schema) {
            return Err(format!(
                "dynamic tool input schema is not supported for {name}: {err}"
            ));
        }
        Ok(())
    }

    let mut seen_tools = HashSet::new();
    let mut seen_namespaces = HashSet::new();
    for spec in tools {
        match spec {
            DynamicToolSpec::Function(tool) => {
                validate_dynamic_tool(tool, /*namespace*/ None, &mut seen_tools)?;
            }
            DynamicToolSpec::Namespace(namespace) => {
                let name = namespace.name.trim();
                if name.is_empty() {
                    return Err("dynamic tool namespace must not be empty".to_string());
                }
                if name != namespace.name {
                    return Err(format!(
                        "dynamic tool namespace has leading/trailing whitespace: {}",
                        escape_identifier_for_error(&namespace.name),
                    ));
                }
                validate_dynamic_tool_identifier(
                    name,
                    "dynamic tool namespace",
                    DYNAMIC_TOOL_NAMESPACE_MAX_LEN,
                )?;
                if namespace.description.chars().count()
                    > DYNAMIC_TOOL_NAMESPACE_DESCRIPTION_MAX_LEN
                {
                    return Err(format!(
                        "dynamic tool namespace description must be at most {DYNAMIC_TOOL_NAMESPACE_DESCRIPTION_MAX_LEN} characters"
                    ));
                }
                if name == "mcp" || name.starts_with("mcp__") {
                    return Err(format!("dynamic tool namespace is reserved: {name}"));
                }
                if RESERVED_RESPONSES_NAMESPACES.contains(&name) {
                    return Err(format!(
                        "dynamic tool namespace collides with a reserved Responses API namespace: {name}",
                    ));
                }
                if !seen_namespaces.insert(name) {
                    return Err(format!("duplicate dynamic tool namespace: {name}"));
                }
                if namespace.tools.is_empty() {
                    return Err(format!(
                        "dynamic tool namespace must contain at least one tool: {name}"
                    ));
                }
                let mut seen_namespace_tools = HashSet::new();
                for tool in &namespace.tools {
                    let DynamicToolNamespaceTool::Function(tool) = tool;
                    validate_dynamic_tool(tool, Some(name), &mut seen_namespace_tools)?;
                }
            }
        }
    }
    Ok(())
}

#[derive(Clone)]
pub(crate) struct ChatRequestProcessor {
    pub(super) auth_manager: Arc<AuthManager>,
    pub(super) chat_manager: Arc<ChatManager>,
    pub(super) outgoing: Arc<OutgoingMessageSender>,
    pub(super) arg0_paths: Arg0DispatchPaths,
    pub(super) config: Arc<Config>,
    pub(super) config_manager: ConfigManager,
    pub(super) chat_store: Arc<dyn ChatStore>,
    pub(super) pending_chat_unloads: Arc<Mutex<HashSet<ChatId>>>,
    pub(super) chat_state_manager: ChatStateManager,
    pub(super) chat_watch_manager: ChatWatchManager,
    pub(super) thread_list_state_permit: Arc<Semaphore>,
    pub(super) chat_goal_processor: ChatGoalRequestProcessor,
    pub(super) state_db: Option<StateDbHandle>,
    pub(super) log_db: Option<LogDbLayer>,
    pub(super) background_tasks: TaskTracker,
    pub(super) skills_watcher: Arc<SkillsWatcher>,
}

/// Outcome of trying to satisfy a resume request from an already loaded thread.
enum RunningThreadResumeResult {
    /// The request was delegated to the loaded thread.
    Handled,
    /// No loaded thread handled the request.
    ///
    /// The optional stored thread contains the history-bearing probe that cold
    /// resume can reuse instead of reading the rollout again.
    NotRunning(Option<Box<StoredChat>>),
}

impl ChatRequestProcessor {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        auth_manager: Arc<AuthManager>,
        chat_manager: Arc<ChatManager>,
        outgoing: Arc<OutgoingMessageSender>,
        arg0_paths: Arg0DispatchPaths,
        config: Arc<Config>,
        config_manager: ConfigManager,
        chat_store: Arc<dyn ChatStore>,
        pending_chat_unloads: Arc<Mutex<HashSet<ChatId>>>,
        chat_state_manager: ChatStateManager,
        chat_watch_manager: ChatWatchManager,
        thread_list_state_permit: Arc<Semaphore>,
        chat_goal_processor: ChatGoalRequestProcessor,
        state_db: Option<StateDbHandle>,
        log_db: Option<LogDbLayer>,
        skills_watcher: Arc<SkillsWatcher>,
    ) -> Self {
        Self {
            auth_manager,
            chat_manager,
            outgoing,
            arg0_paths,
            config,
            config_manager,
            chat_store,
            pending_chat_unloads,
            chat_state_manager,
            chat_watch_manager,
            thread_list_state_permit,
            chat_goal_processor,
            state_db,
            log_db,
            background_tasks: TaskTracker::new(),
            skills_watcher,
        }
    }

    pub(crate) async fn thread_start(
        &self,
        request_id: ConnectionRequestId,
        params: ChatStartParams,
        app_server_client_name: Option<String>,
        app_server_client_version: Option<String>,
        supports_openai_form_elicitation: bool,
        request_context: RequestContext,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_start_inner(
            request_id,
            params,
            app_server_client_name,
            app_server_client_version,
            supports_openai_form_elicitation,
            request_context,
        )
        .await
        .map(|()| None)
    }

    pub(crate) async fn thread_unsubscribe(
        &self,
        request_id: &ConnectionRequestId,
        params: ChatUnsubscribeParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_unsubscribe_response_inner(params, request_id.connection_id)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn chat_resume(
        &self,
        request_id: ConnectionRequestId,
        params: ChatResumeParams,
        app_server_client_name: Option<String>,
        app_server_client_version: Option<String>,
        supports_openai_form_elicitation: bool,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.chat_resume_inner(
            request_id,
            params,
            app_server_client_name,
            app_server_client_version,
            supports_openai_form_elicitation,
        )
        .await
        .map(|()| None)
    }

    pub(crate) async fn thread_fork(
        &self,
        request_id: ConnectionRequestId,
        params: ChatForkParams,
        app_server_client_name: Option<String>,
        app_server_client_version: Option<String>,
        supports_openai_form_elicitation: bool,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_fork_inner(
            request_id,
            params,
            app_server_client_name,
            app_server_client_version,
            supports_openai_form_elicitation,
        )
        .await
        .map(|()| None)
    }

    pub(crate) async fn thread_archive(
        &self,
        request_id: ConnectionRequestId,
        params: ChatArchiveParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        match self.thread_archive_inner(params).await {
            Ok((response, archived_chat_ids)) => {
                self.outgoing
                    .send_response(request_id.clone(), response)
                    .await;
                for chat_id in archived_chat_ids {
                    self.outgoing
                        .send_server_notification(ChatArchived(ChatArchivedNotification {
                            chat_id,
                        }))
                        .await;
                }
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) async fn thread_increment_elicitation(
        &self,
        params: ChatIncrementElicitationParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_increment_elicitation_inner(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn thread_decrement_elicitation(
        &self,
        params: ChatDecrementElicitationParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_decrement_elicitation_inner(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn thread_set_name(
        &self,
        request_id: ConnectionRequestId,
        params: ChatSetNameParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        match self.thread_set_name_response_inner(params).await {
            Ok((response, notification)) => {
                self.outgoing
                    .send_response(request_id.clone(), response)
                    .await;
                if let Some(notification) = notification {
                    self.outgoing
                        .send_server_notification(ChatNameUpdated(notification))
                        .await;
                }
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) async fn chat_metadata_update(
        &self,
        params: ChatMetadataUpdateParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.chat_metadata_update_response_inner(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn thread_memory_mode_set(
        &self,
        params: ChatMemoryModeSetParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_memory_mode_set_response_inner(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn memory_reset(
        &self,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.memory_reset_response_inner()
            .await
            .map(|response: MemoryResetResponse| Some(response.into()))
    }

    pub(crate) async fn thread_unarchive(
        &self,
        request_id: ConnectionRequestId,
        params: ChatUnarchiveParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        match self.thread_unarchive_inner(params).await {
            Ok((response, notification)) => {
                self.outgoing
                    .send_response(request_id.clone(), response)
                    .await;
                self.outgoing
                    .send_server_notification(ChatUnarchived(notification))
                    .await;
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) async fn thread_compact_start(
        &self,
        request_id: &ConnectionRequestId,
        params: ChatCompactStartParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_compact_start_inner(request_id, params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn thread_background_terminals_clean(
        &self,
        request_id: &ConnectionRequestId,
        params: ChatBackgroundTerminalsCleanParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_background_terminals_clean_inner(request_id, params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn thread_background_terminals_list(
        &self,
        params: ChatBackgroundTerminalsListParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_background_terminals_list_inner(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn thread_background_terminals_terminate(
        &self,
        params: ChatBackgroundTerminalsTerminateParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_background_terminals_terminate_inner(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn thread_rollback(
        &self,
        request_id: &ConnectionRequestId,
        params: ChatRollbackParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_rollback_inner(request_id, params)
            .await
            .map(|()| None)
    }

    pub(crate) async fn thread_list(
        &self,
        params: ChatListParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_list_response_inner(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn thread_search(
        &self,
        params: ChatSearchParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_search_response_inner(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn thread_loaded_list(
        &self,
        params: ChatLoadedListParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_loaded_list_response_inner(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn thread_read(
        &self,
        params: ChatReadParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_read_response_inner(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn thread_interactions_list(
        &self,
        params: ChatInteractionsListParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_interactions_list_response_inner(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn chat_interactions_items_list(
        &self,
        _params: ChatInteractionsMessagesListParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        Err(method_not_found(
            "chat/interactions/messages/list is not supported yet",
        ))
    }

    pub(crate) async fn thread_shell_command(
        &self,
        request_id: &ConnectionRequestId,
        params: ChatShellCommandParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_shell_command_inner(request_id, params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn thread_approve_guardian_denied_action(
        &self,
        request_id: &ConnectionRequestId,
        params: ChatApproveGuardianDeniedActionParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_approve_guardian_denied_action_inner(request_id, params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn conversation_summary(
        &self,
        params: GetConversationSummaryParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.get_chat_summary_response_inner(params)
            .await
            .map(|response| Some(response.into()))
    }

    async fn load_thread(
        &self,
        chat_id: &str,
    ) -> Result<(ChatId, Arc<DataxChat>), JSONRPCErrorError> {
        // Resolve the core conversation handle from a v2 thread id string.
        let chat_id = ChatId::from_string(chat_id)
            .map_err(|err| invalid_request(format!("invalid thread id: {err}")))?;

        let thread = self
            .chat_manager
            .get_chat(chat_id)
            .await
            .map_err(|_| invalid_request(format!("thread not found: {chat_id}")))?;

        Ok((chat_id, thread))
    }
    pub(super) async fn acquire_thread_list_state_permit(
        &self,
    ) -> Result<SemaphorePermit<'_>, JSONRPCErrorError> {
        self.thread_list_state_permit
            .acquire()
            .await
            .map_err(|err| {
                internal_error(format!("failed to acquire thread list state permit: {err}"))
            })
    }

    async fn set_app_server_client_info(
        chat: &DataxChat,
        app_server_client_name: Option<String>,
        app_server_client_version: Option<String>,
    ) -> Result<(), JSONRPCErrorError> {
        let mcp_elicitations_auto_deny = xcode_26_4_mcp_elicitations_auto_deny(
            app_server_client_name.as_deref(),
            app_server_client_version.as_deref(),
        );
        thread
            .set_app_server_client_info(
                app_server_client_name,
                app_server_client_version,
                mcp_elicitations_auto_deny,
            )
            .await
            .map_err(|err| internal_error(format!("failed to set app server client info: {err}")))
    }

    async fn finalize_thread_teardown(&self, chat_id: ChatId) {
        self.pending_chat_unloads.lock().await.remove(&chat_id);
        self.outgoing
            .cancel_requests_for_thread(chat_id, /*error*/ None)
            .await;
        self.chat_state_manager.remove_chat_state(chat_id).await;
        self.chat_watch_manager
            .remove_chat(&chat_id.to_string())
            .await;
    }

    async fn thread_unsubscribe_response_inner(
        &self,
        params: ChatUnsubscribeParams,
        connection_id: ConnectionId,
    ) -> Result<ChatUnsubscribeResponse, JSONRPCErrorError> {
        let chat_id = ChatId::from_string(&params.chat_id)
            .map_err(|err| invalid_request(format!("invalid thread id: {err}")))?;

        if self.chat_manager.get_chat(chat_id).await.is_err() {
            self.finalize_thread_teardown(chat_id).await;
            return Ok(ChatUnsubscribeResponse {
                status: ChatUnsubscribeStatus::NotLoaded,
            });
        };

        let was_subscribed = self
            .chat_state_manager
            .unsubscribe_connection_from_chat(chat_id, connection_id)
            .await;

        let status = if was_subscribed {
            ChatUnsubscribeStatus::Unsubscribed
        } else {
            ChatUnsubscribeStatus::NotSubscribed
        };
        Ok(ChatUnsubscribeResponse { status })
    }

    async fn prepare_thread_for_archive(&self, chat_id: ChatId) {
        self.prepare_thread_for_removal(chat_id, "archive").await;
    }

    pub(super) async fn prepare_thread_for_removal(&self, chat_id: ChatId, operation: &str) {
        let removed_conversation = self.chat_manager.remove_chat(&chat_id).await;
        if let Some(conversation) = removed_conversation {
            info!("thread {chat_id} was active; shutting down");
            match wait_for_chat_shutdown(&conversation).await {
                ChatShutdownResult::Complete => {}
                ChatShutdownResult::SubmitFailed => {
                    error!(
                        "failed to submit Shutdown to thread {chat_id}; proceeding with {operation}"
                    );
                }
                ChatShutdownResult::TimedOut => {
                    warn!("thread {chat_id} shutdown timed out; proceeding with {operation}");
                }
            }
        }
        self.finalize_thread_teardown(chat_id).await;
    }

    fn listener_task_context(&self) -> ListenerTaskContext {
        ListenerTaskContext {
            chat_manager: Arc::clone(&self.chat_manager),
            chat_state_manager: self.chat_state_manager.clone(),
            outgoing: Arc::clone(&self.outgoing),
            pending_chat_unloads: Arc::clone(&self.pending_chat_unloads),
            chat_watch_manager: self.chat_watch_manager.clone(),
            thread_list_state_permit: self.thread_list_state_permit.clone(),
            fallback_model_provider: self.config.model_provider_id.clone(),
            codex_home: self.config.codex_home.to_path_buf(),
            skills_watcher: Arc::clone(&self.skills_watcher),
        }
    }

    async fn ensure_conversation_listener(
        &self,
        conversation_id: ChatId,
        connection_id: ConnectionId,
        raw_events_enabled: bool,
    ) -> Result<EnsureConversationListenerResult, JSONRPCErrorError> {
        super::chat_lifecycle::ensure_conversation_listener(
            self.listener_task_context(),
            conversation_id,
            connection_id,
            raw_events_enabled,
        )
        .await
    }

    async fn ensure_listener_task_running(
        &self,
        conversation_id: ChatId,
        conversation: Arc<DataxChat>,
        chat_state: Arc<Mutex<ChatState>>,
    ) -> Result<(), JSONRPCErrorError> {
        super::chat_lifecycle::ensure_listener_task_running(
            self.listener_task_context(),
            conversation_id,
            conversation,
            chat_state,
        )
        .await
    }

    async fn thread_start_inner(
        &self,
        request_id: ConnectionRequestId,
        params: ChatStartParams,
        app_server_client_name: Option<String>,
        app_server_client_version: Option<String>,
        supports_openai_form_elicitation: bool,
        request_context: RequestContext,
    ) -> Result<(), JSONRPCErrorError> {
        let ChatStartParams {
            model,
            model_provider,
            service_tier,
            cwd,
            runtime_workspace_roots,
            approval_policy,
            approvals_reviewer,
            sandbox,
            permissions,
            config,
            service_name,
            base_instructions,
            developer_instructions,
            dynamic_tools,
            selected_capability_roots,
            mock_experimental_field: _mock_experimental_field,
            experimental_raw_events,
            personality,
            multi_agent_mode: _multi_agent_mode,
            ephemeral,
            session_start_source,
            chat_source,
            environments,
        } = params;
        if sandbox.is_some() && permissions.is_some() {
            return Err(invalid_request(
                "`permissions` cannot be combined with `sandbox`",
            ));
        }
        let environment_selections =
            resolve_turn_environment_selections(self.chat_manager.as_ref(), environments)?;
        let runtime_workspace_roots = runtime_workspace_roots.map(resolve_runtime_workspace_roots);
        let mut typesafe_overrides = self.build_thread_config_overrides(
            model,
            model_provider,
            service_tier,
            cwd,
            runtime_workspace_roots,
            approval_policy,
            approvals_reviewer,
            sandbox,
            permissions,
            base_instructions,
            developer_instructions,
            personality,
        );
        typesafe_overrides.ephemeral = ephemeral;
        let listener_task_context = ListenerTaskContext {
            chat_manager: Arc::clone(&self.chat_manager),
            chat_state_manager: self.chat_state_manager.clone(),
            outgoing: Arc::clone(&self.outgoing),
            pending_chat_unloads: Arc::clone(&self.pending_chat_unloads),
            chat_watch_manager: self.chat_watch_manager.clone(),
            thread_list_state_permit: self.thread_list_state_permit.clone(),
            fallback_model_provider: self.config.model_provider_id.clone(),
            codex_home: self.config.codex_home.to_path_buf(),
            skills_watcher: Arc::clone(&self.skills_watcher),
        };
        let request_trace = request_context.request_trace();
        let config_manager = self.config_manager.clone();
        let outgoing = Arc::clone(&listener_task_context.outgoing);
        let error_request_id = request_id.clone();
        let thread_start_task = async move {
            if let Err(error) = Self::thread_start_task(
                listener_task_context,
                config_manager,
                request_id,
                app_server_client_name,
                app_server_client_version,
                supports_openai_form_elicitation,
                config,
                typesafe_overrides,
                dynamic_tools,
                selected_capability_roots.unwrap_or_default(),
                session_start_source,
                chat_source.map(Into::into),
                environment_selections,
                service_name,
                experimental_raw_events,
                request_trace,
            )
            .await
            {
                outgoing.send_error(error_request_id, error).await;
            }
        };
        self.background_tasks
            .spawn(thread_start_task.instrument(request_context.span()));
        Ok(())
    }

    pub(crate) async fn drain_background_tasks(&self) {
        self.background_tasks.close();
        if tokio::time::timeout(Duration::from_secs(10), self.background_tasks.wait())
            .await
            .is_err()
        {
            warn!("timed out waiting for background tasks to shut down; proceeding");
        }
    }

    pub(crate) async fn clear_all_chat_listeners(&self) {
        self.chat_state_manager.clear_all_listeners().await;
    }

    pub(crate) async fn shutdown_chats(&self) {
        let report = self
            .chat_manager
            .shutdown_all_threads_bounded(Duration::from_secs(10))
            .await;
        for chat_id in report.submit_failed {
            warn!("failed to submit Shutdown to thread {chat_id}");
        }
        for chat_id in report.timed_out {
            warn!("timed out waiting for thread {chat_id} to shut down");
        }
    }

    async fn request_trace_context(
        &self,
        request_id: &ConnectionRequestId,
    ) -> Option<datax_protocol::protocol::W3cTraceContext> {
        self.outgoing.request_trace_context(request_id).await
    }

    async fn submit_core_op(
        &self,
        request_id: &ConnectionRequestId,
        chat: &DataxChat,
        op: Op,
    ) -> CodexResult<String> {
        thread
            .submit_with_trace(op, self.request_trace_context(request_id).await)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn thread_start_task(
        listener_task_context: ListenerTaskContext,
        config_manager: ConfigManager,
        request_id: ConnectionRequestId,
        app_server_client_name: Option<String>,
        app_server_client_version: Option<String>,
        supports_openai_form_elicitation: bool,
        config_overrides: Option<HashMap<String, serde_json::Value>>,
        typesafe_overrides: ConfigOverrides,
        dynamic_tools: Option<Vec<DynamicToolSpec>>,
        selected_capability_roots: Vec<SelectedCapabilityRoot>,
        session_start_source: Option<datax_app_server_protocol::ChatStartSource>,
        chat_source: Option<datax_protocol::protocol::ThreadSource>,
        environments: Option<Vec<TurnEnvironmentSelection>>,
        service_name: Option<String>,
        experimental_raw_events: bool,
        request_trace: Option<W3cTraceContext>,
    ) -> Result<(), JSONRPCErrorError> {
        let thread_start_started_at = std::time::Instant::now();
        let requested_cwd = typesafe_overrides.cwd.clone();
        let mut config = config_manager
            .load_with_overrides(config_overrides.clone(), typesafe_overrides.clone())
            .await
            .map_err(|err| config_load_error(&err))?;

        // The user may have requested WorkspaceWrite or DangerFullAccess via
        // the command line, though in the process of deriving the Config, it
        // could be downgraded to ReadOnly (perhaps there is no sandbox
        // available on Windows or the enterprise config disallows it). The cwd
        // should still be considered "trusted" in this case.
        let requested_permissions_trust_project =
            requested_permissions_trust_project(&typesafe_overrides, config.cwd.as_path());
        let effective_permissions_trust_project = permission_profile_trusts_project(
            &config.permissions.effective_permission_profile(),
            config.cwd.as_path(),
        );

        if requested_cwd.is_some()
            && config.active_project.trust_level.is_none()
            && (requested_permissions_trust_project || effective_permissions_trust_project)
        {
            let trust_target = resolve_root_git_project_for_trust(LOCAL_FS.as_ref(), &config.cwd)
                .await
                .unwrap_or_else(|| config.cwd.clone());
            let current_cli_overrides = config_manager.current_cli_overrides();
            let cli_overrides_with_trust;
            let cli_overrides_for_reload = if let Err(err) =
                datax_core::config::set_project_trust_level(
                    &listener_task_context.codex_home,
                    trust_target.as_path(),
                    TrustLevel::Trusted,
                ) {
                warn!(
                    "failed to persist trusted project state for {}; continuing with in-memory trust for this chat: {err}",
                    trust_target.display()
                );
                let mut project = toml::map::Map::new();
                project.insert(
                    "trust_level".to_string(),
                    TomlValue::String("trusted".to_string()),
                );
                let mut projects = toml::map::Map::new();
                projects.insert(
                    project_trust_key(trust_target.as_path()),
                    TomlValue::Table(project),
                );
                cli_overrides_with_trust = current_cli_overrides
                    .iter()
                    .cloned()
                    .chain(std::iter::once((
                        "projects".to_string(),
                        TomlValue::Table(projects),
                    )))
                    .collect::<Vec<_>>();
                cli_overrides_with_trust.as_slice()
            } else {
                current_cli_overrides.as_slice()
            };

            config = config_manager
                .load_with_cli_overrides(
                    cli_overrides_for_reload,
                    config_overrides,
                    typesafe_overrides,
                    /*fallback_cwd*/ None,
                )
                .await
                .map_err(|err| config_load_error(&err))?;
        }

        let environments = environments.unwrap_or_else(|| {
            listener_task_context
                .chat_manager
                .default_environment_selections(&config.cwd)
        });
        let dynamic_tools = dynamic_tools.unwrap_or_default();
        if !dynamic_tools.is_empty() {
            validate_dynamic_tools(&dynamic_tools).map_err(invalid_request)?;
        }
        // Count callable functions rather than top-level namespace containers.
        let dynamic_tool_count: usize = dynamic_tools
            .iter()
            .map(|tool| match tool {
                DynamicToolSpec::Function(_) => 1,
                DynamicToolSpec::Namespace(namespace) => namespace.tools.len(),
            })
            .sum();
        let mut thread_extension_init = ExtensionDataInit::new();
        if !selected_capability_roots.is_empty() {
            thread_extension_init.insert(selected_capability_roots);
            datax_mcp_extension::initialize_executor_plugin_thread_data(&mut thread_extension_init);
        }
        let create_chat_started_at = std::time::Instant::now();
        let NewChat {
            chat_id: chat_id,
            chat: thread,
            session_configured,
            ..
        } = listener_task_context
            .chat_manager
            .start_chat_with_options(StartChatOptions {
                config,
                initial_history: match session_start_source
                    .unwrap_or(datax_app_server_protocol::ChatStartSource::Startup)
                {
                    datax_app_server_protocol::ChatStartSource::Startup => InitialHistory::New,
                    datax_app_server_protocol::ChatStartSource::Clear => InitialHistory::Cleared,
                },
                session_source: None,
                thread_source: chat_source,
                dynamic_tools,
                metrics_service_name: service_name,
                parent_trace: request_trace,
                environments,
                thread_extension_init,
                supports_openai_form_elicitation,
            })
            .instrument(tracing::info_span!(
                "app_server.thread_start.create_chat",
                otel.name = "app_server.thread_start.create_chat",
                thread_start.dynamic_tool_count = dynamic_tool_count,
            ))
            .await
            .map_err(|err| match err {
                CodexErr::InvalidRequest(message) => invalid_request(message),
                err => internal_error(format!("error creating chat: {err}")),
            })?;
        let session_telemetry = thread.session_telemetry();
        session_telemetry.record_startup_phase(
            "thread_start_create_chat",
            create_chat_started_at.elapsed(),
            Some("ready"),
        );

        Self::set_app_server_client_info(
            thread.as_ref(),
            app_server_client_name,
            app_server_client_version,
        )
        .await?;

        let instruction_sources = thread.legacy_instruction_sources().await;
        let config_snapshot = thread
            .config_snapshot()
            .instrument(tracing::info_span!(
                "app_server.thread_start.config_snapshot",
                otel.name = "app_server.thread_start.config_snapshot",
            ))
            .await;
        let mut thread = build_thread_from_snapshot(
            chat_id,
            session_configured.session_id.to_string(),
            &config_snapshot,
            session_configured.rollout_path.clone(),
        );

        // Auto-attach a chat listener when starting a thread.
        log_listener_attach_result(
            super::chat_lifecycle::ensure_conversation_listener(
                listener_task_context.clone(),
                chat_id,
                request_id.connection_id,
                experimental_raw_events,
            )
            .instrument(tracing::info_span!(
                "app_server.thread_start.attach_listener",
                otel.name = "app_server.thread_start.attach_listener",
                thread_start.experimental_raw_events = experimental_raw_events,
            ))
            .await,
            chat_id,
            request_id.connection_id,
            "thread",
        );

        listener_task_context
            .chat_watch_manager
            .upsert_chat_silently(thread.clone())
            .instrument(tracing::info_span!(
                "app_server.thread_start.upsert_chat",
                otel.name = "app_server.thread_start.upsert_chat",
            ))
            .await;

        thread.status = resolve_chat_status(
            listener_task_context
                .chat_watch_manager
                .loaded_status_for_chat(&thread.id)
                .instrument(tracing::info_span!(
                    "app_server.thread_start.resolve_status",
                    otel.name = "app_server.thread_start.resolve_status",
                ))
                .await,
            /*has_in_progress_interaction*/ false,
        );

        let sandbox = thread_response_sandbox_policy(
            &config_snapshot.permission_profile,
            config_snapshot.cwd().as_path(),
        );
        let cwd = config_snapshot.cwd().clone();
        let active_permission_profile =
            thread_response_active_permission_profile(config_snapshot.active_permission_profile);

        let response = ChatStartResponse {
            chat: thread.clone(),
            model: config_snapshot.model,
            model_provider: config_snapshot.model_provider_id,
            service_tier: config_snapshot.service_tier,
            cwd,
            runtime_workspace_roots: config_snapshot.workspace_roots,
            instruction_sources,
            approval_policy: config_snapshot.approval_policy.into(),
            approvals_reviewer: config_snapshot.approvals_reviewer.into(),
            sandbox,
            active_permission_profile,
            reasoning_effort: config_snapshot.reasoning_effort,
            multi_agent_mode: MultiAgentMode::ExplicitRequestOnly,
        };
        let notif = thread_started_notification(thread);
        listener_task_context
            .outgoing
            .send_response(request_id, response)
            .instrument(tracing::info_span!(
                "app_server.thread_start.send_response",
                otel.name = "app_server.thread_start.send_response",
            ))
            .await;

        listener_task_context
            .outgoing
            .send_server_notification(ChatStarted(notif))
            .instrument(tracing::info_span!(
                "app_server.thread_start.notify_started",
                otel.name = "app_server.thread_start.notify_started",
            ))
            .await;
        session_telemetry.record_startup_phase(
            "thread_start_total",
            thread_start_started_at.elapsed(),
            Some("ready"),
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn build_thread_config_overrides(
        &self,
        model: Option<String>,
        model_provider: Option<String>,
        service_tier: Option<Option<String>>,
        cwd: Option<String>,
        runtime_workspace_roots: Option<Vec<AbsolutePathBuf>>,
        approval_policy: Option<datax_app_server_protocol::AskForApproval>,
        approvals_reviewer: Option<datax_app_server_protocol::ApprovalsReviewer>,
        sandbox: Option<SandboxMode>,
        permissions: Option<String>,
        base_instructions: Option<String>,
        developer_instructions: Option<String>,
        personality: Option<Personality>,
    ) -> ConfigOverrides {
        ConfigOverrides {
            model,
            model_provider,
            service_tier,
            cwd: cwd.map(PathBuf::from),
            workspace_roots: runtime_workspace_roots,
            default_permissions: permissions,
            approval_policy: approval_policy
                .map(datax_app_server_protocol::AskForApproval::to_core),
            approvals_reviewer: approvals_reviewer
                .map(datax_app_server_protocol::ApprovalsReviewer::to_core),
            sandbox_mode: sandbox.map(SandboxMode::to_core),
            codex_linux_sandbox_exe: self.arg0_paths.codex_linux_sandbox_exe.clone(),
            main_execve_wrapper_exe: self.arg0_paths.main_execve_wrapper_exe.clone(),
            base_instructions,
            developer_instructions,
            personality,
            ..Default::default()
        }
    }

    async fn thread_archive_inner(
        &self,
        params: ChatArchiveParams,
    ) -> Result<(ChatArchiveResponse, Vec<String>), JSONRPCErrorError> {
        let _thread_list_state_permit = self.acquire_thread_list_state_permit().await?;
        self.thread_archive_response(params).await
    }

    async fn thread_archive_response(
        &self,
        params: ChatArchiveParams,
    ) -> Result<(ChatArchiveResponse, Vec<String>), JSONRPCErrorError> {
        let chat_id = ChatId::from_string(&params.chat_id)
            .map_err(|err| invalid_request(format!("invalid session id: {err}")))?;

        let chat_ids = self.state_db_spawn_subtree_chat_ids(chat_id).await?;

        let mut archive_chat_ids = Vec::new();
        match self
            .chat_store
            .read_chat(StoreReadChatParams {
                chat_id: chat_id,
                include_archived: false,
                include_history: false,
            })
            .await
        {
            Ok(thread) => {
                if thread.archived_at.is_none() {
                    archive_chat_ids.push(chat_id);
                }
            }
            Err(err) => return Err(chat_store_archive_error("archive", err)),
        }
        for descendant_chat_id in chat_ids.into_iter().skip(1) {
            match self
                .chat_store
                .read_chat(StoreReadChatParams {
                    chat_id: descendant_chat_id,
                    include_archived: true,
                    include_history: false,
                })
                .await
            {
                Ok(thread) => {
                    if thread.archived_at.is_none() {
                        archive_chat_ids.push(descendant_chat_id);
                    }
                }
                Err(err) => {
                    warn!(
                        "failed to read spawned descendant thread {descendant_chat_id} while archiving {chat_id}: {err}"
                    );
                }
            }
        }

        let mut archived_chat_ids = Vec::new();
        let Some((parent_chat_id, descendant_chat_ids)) = archive_chat_ids.split_first() else {
            return Ok((ChatArchiveResponse {}, archived_chat_ids));
        };

        self.prepare_thread_for_archive(*parent_chat_id).await;
        match self
            .chat_store
            .archive_chat(StoreArchiveChatParams {
                chat_id: *parent_chat_id,
            })
            .await
        {
            Ok(()) => {
                archived_chat_ids.push(parent_chat_id.to_string());
            }
            Err(err) => return Err(chat_store_archive_error("archive", err)),
        }

        for descendant_chat_id in descendant_chat_ids.iter().rev().copied() {
            self.prepare_thread_for_archive(descendant_chat_id).await;
            match self
                .chat_store
                .archive_chat(StoreArchiveChatParams {
                    chat_id: descendant_chat_id,
                })
                .await
            {
                Ok(()) => {
                    archived_chat_ids.push(descendant_chat_id.to_string());
                }
                Err(err) => {
                    warn!(
                        "failed to archive spawned descendant thread {descendant_chat_id} while archiving {chat_id}: {err}"
                    );
                }
            }
        }

        Ok((ChatArchiveResponse {}, archived_chat_ids))
    }

    pub(super) async fn state_db_spawn_subtree_chat_ids(
        &self,
        chat_id: ChatId,
    ) -> Result<Vec<ChatId>, JSONRPCErrorError> {
        let mut chat_ids = vec![chat_id];
        let Some(state_db_ctx) = self.state_db.as_ref() else {
            return Ok(chat_ids);
        };
        let mut seen = HashSet::from([chat_id]);
        let descendants = state_db_ctx
            .list_thread_spawn_descendants(chat_id)
            .await
            .map_err(|err| {
                internal_error(format!(
                    "failed to list spawned descendants for thread id {chat_id}: {err}"
                ))
            })?;
        for descendant_id in descendants {
            if seen.insert(descendant_id) {
                chat_ids.push(descendant_id);
            }
        }
        Ok(chat_ids)
    }

    async fn thread_increment_elicitation_inner(
        &self,
        params: ChatIncrementElicitationParams,
    ) -> Result<ChatIncrementElicitationResponse, JSONRPCErrorError> {
        let (_, thread) = self.load_thread(&params.chat_id).await?;
        let count = thread
            .increment_out_of_band_elicitation_count()
            .await
            .map_err(|err| {
                internal_error(format!(
                    "failed to increment out-of-band elicitation counter: {err}"
                ))
            })?;
        Ok(ChatIncrementElicitationResponse {
            count,
            paused: count > 0,
        })
    }

    async fn thread_decrement_elicitation_inner(
        &self,
        params: ChatDecrementElicitationParams,
    ) -> Result<ChatDecrementElicitationResponse, JSONRPCErrorError> {
        let (_, thread) = self.load_thread(&params.chat_id).await?;
        let count = thread
            .decrement_out_of_band_elicitation_count()
            .await
            .map_err(|err| match err {
                CodexErr::InvalidRequest(message) => invalid_request(message),
                err => internal_error(format!(
                    "failed to decrement out-of-band elicitation counter: {err}"
                )),
            })?;
        Ok(ChatDecrementElicitationResponse {
            count,
            paused: count > 0,
        })
    }

    async fn thread_set_name_response_inner(
        &self,
        params: ChatSetNameParams,
    ) -> Result<(ChatSetNameResponse, Option<ChatNameUpdatedNotification>), JSONRPCErrorError> {
        let ChatSetNameParams { chat_id, name } = params;
        let chat_id = ChatId::from_string(&chat_id)
            .map_err(|err| invalid_request(format!("invalid thread id: {err}")))?;
        let Some(name) = datax_core::util::normalize_thread_name(&name) else {
            return Err(invalid_request("thread name must not be empty"));
        };

        let _thread_list_state_permit = self.acquire_thread_list_state_permit().await?;
        self.chat_manager
            .update_chat_metadata(
                chat_id,
                StoreChatMetadataPatch {
                    name: Some(Some(name.clone())),
                    ..Default::default()
                },
                /*include_archived*/ false,
            )
            .await
            .map_err(|err| core_thread_write_error("set thread name", err))?;

        Ok((
            ChatSetNameResponse {},
            Some(ChatNameUpdatedNotification {
                chat_id: chat_id.to_string(),
                thread_name: Some(name),
            }),
        ))
    }

    async fn thread_memory_mode_set_response_inner(
        &self,
        params: ChatMemoryModeSetParams,
    ) -> Result<ChatMemoryModeSetResponse, JSONRPCErrorError> {
        let ChatMemoryModeSetParams { chat_id, mode } = params;
        let chat_id = ChatId::from_string(&chat_id)
            .map_err(|err| invalid_request(format!("invalid thread id: {err}")))?;

        self.chat_manager
            .update_chat_metadata(
                chat_id,
                StoreChatMetadataPatch {
                    memory_mode: Some(mode.to_core()),
                    ..Default::default()
                },
                /*include_archived*/ false,
            )
            .await
            .map_err(|err| core_thread_write_error("set thread memory mode", err))?;

        Ok(ChatMemoryModeSetResponse {})
    }

    async fn memory_reset_response_inner(&self) -> Result<MemoryResetResponse, JSONRPCErrorError> {
        let state_db = self
            .state_db
            .clone()
            .ok_or_else(|| internal_error("sqlite state db unavailable for memory reset"))?;

        state_db
            .memories()
            .clear_memory_data()
            .await
            .map_err(|err| {
                internal_error(format!("failed to clear memory rows in memories db: {err}"))
            })?;

        clear_memory_roots_contents(&self.config.codex_home)
            .await
            .map_err(|err| {
                internal_error(format!(
                    "failed to clear memory directories under {}: {err}",
                    self.config.codex_home.display()
                ))
            })?;

        Ok(MemoryResetResponse {})
    }

    async fn chat_metadata_update_response_inner(
        &self,
        params: ChatMetadataUpdateParams,
    ) -> Result<ChatMetadataUpdateResponse, JSONRPCErrorError> {
        let ChatMetadataUpdateParams { chat_id, git_info } = params;

        let thread_uuid = ChatId::from_string(&chat_id)
            .map_err(|err| invalid_request(format!("invalid thread id: {err}")))?;

        let Some(ChatMetadataGitInfoUpdateParams {
            sha,
            branch,
            origin_url,
        }) = git_info
        else {
            return Err(invalid_request("gitInfo must include at least one field"));
        };

        if sha.is_none() && branch.is_none() && origin_url.is_none() {
            return Err(invalid_request("gitInfo must include at least one field"));
        }

        let git_sha = Self::normalize_chat_metadata_git_field(sha, "gitInfo.sha")?;
        let git_branch = Self::normalize_chat_metadata_git_field(branch, "gitInfo.branch")?;
        let git_origin_url =
            Self::normalize_chat_metadata_git_field(origin_url, "gitInfo.originUrl")?;

        let patch = StoreChatMetadataPatch {
            git_info: Some(StoreGitInfoPatch {
                sha: git_sha,
                branch: git_branch,
                origin_url: git_origin_url,
            }),
            ..Default::default()
        };

        let updated_thread = {
            let _thread_list_state_permit = self.acquire_thread_list_state_permit().await?;
            self.chat_manager
                .update_chat_metadata(thread_uuid, patch, /*include_archived*/ true)
                .await
                .map_err(|err| core_thread_write_error("update chat metadata", err))?
        };
        let (mut thread, _) = chat_from_stored_chat(
            updated_thread,
            self.config.model_provider_id.as_str(),
            &self.config.cwd,
        );
        if let Ok(loaded_thread) = self.chat_manager.get_chat(thread_uuid).await {
            thread.session_id = loaded_thread.session_configured().session_id.to_string();
        }
        self.attach_thread_name(thread_uuid, &mut thread).await;
        thread.status = resolve_chat_status(
            self.chat_watch_manager
                .loaded_status_for_chat(&thread.id)
                .await,
            /*has_in_progress_interaction*/ false,
        );

        Ok(ChatMetadataUpdateResponse { chat: thread })
    }

    fn normalize_chat_metadata_git_field(
        value: Option<Option<String>>,
        name: &str,
    ) -> Result<Option<Option<String>>, JSONRPCErrorError> {
        match value {
            Some(Some(value)) => {
                let value = value.trim().to_string();
                if value.is_empty() {
                    return Err(invalid_request(format!("{name} must not be empty")));
                }
                Ok(Some(Some(value)))
            }
            Some(None) => Ok(Some(None)),
            None => Ok(None),
        }
    }

    async fn thread_unarchive_inner(
        &self,
        params: ChatUnarchiveParams,
    ) -> Result<(ChatUnarchiveResponse, ChatUnarchivedNotification), JSONRPCErrorError> {
        let _thread_list_state_permit = self.acquire_thread_list_state_permit().await?;
        let (response, chat_id) = self.thread_unarchive_response(params).await?;
        Ok((response, ChatUnarchivedNotification { chat_id }))
    }

    async fn thread_unarchive_response(
        &self,
        params: ChatUnarchiveParams,
    ) -> Result<(ChatUnarchiveResponse, String), JSONRPCErrorError> {
        let chat_id = ChatId::from_string(&params.chat_id)
            .map_err(|err| invalid_request(format!("invalid session id: {err}")))?;

        let fallback_provider = self.config.model_provider_id.clone();
        let stored_chat = self
            .chat_store
            .unarchive_chat(StoreArchiveChatParams { chat_id: chat_id })
            .await
            .map_err(|err| chat_store_archive_error("unarchive", err))?;
        let (mut thread, _) =
            chat_from_stored_chat(stored_chat, fallback_provider.as_str(), &self.config.cwd);

        thread.status = resolve_chat_status(
            self.chat_watch_manager
                .loaded_status_for_chat(&thread.id)
                .await,
            /*has_in_progress_interaction*/ false,
        );
        self.attach_thread_name(chat_id, &mut thread).await;
        let chat_id = thread.id.clone();
        Ok((ChatUnarchiveResponse { chat: thread }, chat_id))
    }

    async fn thread_rollback_inner(
        &self,
        request_id: &ConnectionRequestId,
        params: ChatRollbackParams,
    ) -> Result<(), JSONRPCErrorError> {
        self.thread_rollback_start(request_id, params).await
    }

    async fn thread_rollback_start(
        &self,
        request_id: &ConnectionRequestId,
        params: ChatRollbackParams,
    ) -> Result<(), JSONRPCErrorError> {
        let ChatRollbackParams {
            chat_id,
            num_interactions,
        } = params;

        if num_interactions == 0 {
            return Err(invalid_request("numInteractions must be >= 1"));
        }

        let (chat_id, thread) = self.load_thread(&chat_id).await?;

        let request = request_id.clone();

        let rollback_already_in_progress = {
            let chat_state = self.chat_state_manager.chat_state(chat_id).await;
            let mut chat_state = chat_state.lock().await;
            if chat_state.pending_rollbacks.is_some() {
                true
            } else {
                chat_state.pending_rollbacks = Some(request.clone());
                false
            }
        };
        if rollback_already_in_progress {
            return Err(invalid_request(
                "rollback already in progress for this thread",
            ));
        }

        if let Err(err) = self
            .submit_core_op(
                request_id,
                thread.as_ref(),
                Op::ThreadRollback {
                    num_turns: num_interactions,
                },
            )
            .await
        {
            // No ThreadRollback event will arrive if an error occurs.
            // Clean up and reply immediately.
            let chat_state = self.chat_state_manager.chat_state(chat_id).await;
            chat_state.lock().await.pending_rollbacks = None;

            return Err(internal_error(format!("failed to start rollback: {err}")));
        }
        Ok(())
    }

    async fn thread_compact_start_inner(
        &self,
        request_id: &ConnectionRequestId,
        params: ChatCompactStartParams,
    ) -> Result<ChatCompactStartResponse, JSONRPCErrorError> {
        let ChatCompactStartParams { chat_id } = params;

        let (_, thread) = self.load_thread(&chat_id).await?;
        self.submit_core_op(request_id, thread.as_ref(), Op::Compact)
            .await
            .map_err(|err| internal_error(format!("failed to start compaction: {err}")))?;
        Ok(ChatCompactStartResponse {})
    }

    async fn thread_background_terminals_clean_inner(
        &self,
        request_id: &ConnectionRequestId,
        params: ChatBackgroundTerminalsCleanParams,
    ) -> Result<ChatBackgroundTerminalsCleanResponse, JSONRPCErrorError> {
        let ChatBackgroundTerminalsCleanParams { chat_id } = params;

        let (_, thread) = self.load_thread(&chat_id).await?;
        self.submit_core_op(request_id, thread.as_ref(), Op::CleanBackgroundTerminals)
            .await
            .map_err(|err| {
                internal_error(format!("failed to clean background terminals: {err}"))
            })?;
        Ok(ChatBackgroundTerminalsCleanResponse {})
    }

    async fn thread_background_terminals_list_inner(
        &self,
        params: ChatBackgroundTerminalsListParams,
    ) -> Result<ChatBackgroundTerminalsListResponse, JSONRPCErrorError> {
        let ChatBackgroundTerminalsListParams {
            chat_id,
            cursor,
            limit,
        } = params;

        let (_, thread) = self.load_thread(&chat_id).await?;
        let terminals = thread
            .list_background_terminals()
            .await
            .into_iter()
            .map(|terminal| {
                // TODO(anp): Migrate ChatBackgroundTerminal to PathUri.
                let cwd = terminal.cwd.to_abs_path().map_err(|err| {
                    internal_error(format!("background terminal has invalid cwd: {err}"))
                })?;
                Ok(ChatBackgroundTerminal {
                    message_id: terminal.item_id,
                    process_id: terminal.process_id,
                    command: terminal.command,
                    cwd,
                    os_pid: None,
                    cpu_percent: None,
                    rss_kb: None,
                })
            })
            .collect::<Result<Vec<_>, JSONRPCErrorError>>()?;

        let (data, next_cursor) = paginate_background_terminals(&terminals, cursor, limit)?;

        Ok(ChatBackgroundTerminalsListResponse { data, next_cursor })
    }

    async fn thread_background_terminals_terminate_inner(
        &self,
        params: ChatBackgroundTerminalsTerminateParams,
    ) -> Result<ChatBackgroundTerminalsTerminateResponse, JSONRPCErrorError> {
        let ChatBackgroundTerminalsTerminateParams {
            chat_id,
            process_id,
        } = params;
        let process_id = process_id.parse::<i32>().map_err(|err| {
            invalid_request(format!("invalid background terminal process id: {err}"))
        })?;

        let (_, thread) = self.load_thread(&chat_id).await?;
        let terminated = thread.terminate_background_terminal(process_id).await;
        Ok(ChatBackgroundTerminalsTerminateResponse { terminated })
    }

    async fn thread_shell_command_inner(
        &self,
        request_id: &ConnectionRequestId,
        params: ChatShellCommandParams,
    ) -> Result<ChatShellCommandResponse, JSONRPCErrorError> {
        let ChatShellCommandParams { chat_id, command } = params;
        let command = command.trim().to_string();
        if command.is_empty() {
            return Err(invalid_request("command must not be empty"));
        }
        // `chat/shellCommand` is app-server's local-host shell escape hatch,
        // not the normal turn-selected shell tool path.
        if self
            .chat_manager
            .environment_manager()
            .try_local_environment()
            .is_none()
        {
            return Err(internal_error("local environment is not configured"));
        }

        let (_, thread) = self.load_thread(&chat_id).await?;
        self.submit_core_op(
            request_id,
            thread.as_ref(),
            Op::RunUserShellCommand { command },
        )
        .await
        .map_err(|err| internal_error(format!("failed to start shell command: {err}")))?;
        Ok(ChatShellCommandResponse {})
    }

    async fn thread_approve_guardian_denied_action_inner(
        &self,
        request_id: &ConnectionRequestId,
        params: ChatApproveGuardianDeniedActionParams,
    ) -> Result<ChatApproveGuardianDeniedActionResponse, JSONRPCErrorError> {
        let ChatApproveGuardianDeniedActionParams { chat_id, event } = params;
        let event = serde_json::from_value(event)
            .map_err(|err| invalid_request(format!("invalid Guardian denial event: {err}")))?;
        let (_, thread) = self.load_thread(&chat_id).await?;

        self.submit_core_op(
            request_id,
            thread.as_ref(),
            Op::ApproveGuardianDeniedAction { event },
        )
        .await
        .map_err(|err| internal_error(format!("failed to approve Guardian denial: {err}")))?;
        Ok(ChatApproveGuardianDeniedActionResponse {})
    }

    async fn thread_list_response_inner(
        &self,
        params: ChatListParams,
    ) -> Result<ChatListResponse, JSONRPCErrorError> {
        let ChatListParams {
            cursor,
            limit,
            sort_key,
            sort_direction,
            model_providers,
            source_kinds,
            archived,
            cwd,
            use_state_db_only,
            search_term,
            parent_chat_id,
        } = params;
        let cwd_filters = normalize_thread_list_cwd_filters(cwd)?;
        let parent_chat_id = parent_chat_id
            .as_deref()
            .map(ChatId::from_string)
            .transpose()
            .map_err(|err| invalid_request(format!("invalid parent thread id: {err}")))?;

        let requested_page_size = limit
            .map(|value| value as usize)
            .unwrap_or(THREAD_LIST_DEFAULT_LIMIT)
            .clamp(1, THREAD_LIST_MAX_LIMIT);
        let store_sort_key = match sort_key.unwrap_or(ChatSortKey::CreatedAt) {
            ChatSortKey::CreatedAt => StoreChatSortKey::CreatedAt,
            ChatSortKey::UpdatedAt => StoreChatSortKey::UpdatedAt,
            ChatSortKey::RecencyAt => StoreChatSortKey::RecencyAt,
        };
        let sort_direction = sort_direction.unwrap_or(SortDirection::Desc);
        let (stored_chats, next_cursor) = self
            .list_chats_common(
                requested_page_size,
                cursor,
                store_sort_key,
                sort_direction,
                ThreadListFilters {
                    model_providers,
                    source_kinds,
                    archived: archived.unwrap_or(false),
                    cwd_filters,
                    search_term,
                    use_state_db_only,
                    parent_chat_id,
                },
            )
            .await?;
        let backwards_cursor = stored_chats.first().and_then(|thread| {
            thread_backwards_cursor_for_sort_key(thread, store_sort_key, sort_direction)
        });
        let mut threads = Vec::with_capacity(stored_chats.len());
        let mut status_ids = Vec::with_capacity(stored_chats.len());
        let fallback_provider = self.config.model_provider_id.clone();

        for stored_chat in stored_chats {
            let (thread, _) = chat_from_stored_chat(
                stored_chat,
                fallback_provider.as_str(),
                &self.config.cwd,
            );
            status_ids.push(thread.id.clone());
            threads.push(thread);
        }

        let statuses = self
            .chat_watch_manager
            .loaded_statuses_for_chats(status_ids)
            .await;

        let data: Vec<_> = threads
            .into_iter()
            .map(|mut thread| {
                if let Some(status) = statuses.get(&thread.id) {
                    thread.status = status.clone();
                }
                thread
            })
            .collect();
        Ok(ChatListResponse {
            data,
            next_cursor,
            backwards_cursor,
        })
    }

    async fn thread_search_response_inner(
        &self,
        params: ChatSearchParams,
    ) -> Result<ChatSearchResponse, JSONRPCErrorError> {
        let ChatSearchParams {
            cursor,
            limit,
            sort_key,
            sort_direction,
            source_kinds,
            archived,
            search_term,
        } = params;
        let search_term = search_term.trim().to_string();
        let search_term = (!search_term.is_empty())
            .then_some(search_term)
            .ok_or_else(|| invalid_request("chat/search requires a non-empty searchTerm"))?;
        let requested_page_size = limit
            .map(|value| value as usize)
            .unwrap_or(THREAD_LIST_DEFAULT_LIMIT)
            .clamp(1, THREAD_LIST_MAX_LIMIT);
        let store_sort_key = match sort_key.unwrap_or(ChatSortKey::CreatedAt) {
            ChatSortKey::CreatedAt => StoreChatSortKey::CreatedAt,
            ChatSortKey::UpdatedAt => StoreChatSortKey::UpdatedAt,
            ChatSortKey::RecencyAt => StoreChatSortKey::RecencyAt,
        };
        let store_sort_direction = sort_direction.unwrap_or(SortDirection::Desc);
        let (allowed_sources, source_kind_filter) = compute_source_filters(source_kinds);
        let mut cursor_obj = cursor;
        let mut last_cursor = cursor_obj.clone();
        let mut remaining = requested_page_size;
        let mut search_results = Vec::with_capacity(requested_page_size);
        let mut next_cursor = None;

        while remaining > 0 {
            let page = self
                .chat_store
                .search_chats(StoreSearchChatsParams {
                    page_size: remaining.min(THREAD_LIST_MAX_LIMIT),
                    cursor: cursor_obj.clone(),
                    sort_key: store_sort_key,
                    sort_direction: match store_sort_direction {
                        SortDirection::Asc => StoreSortDirection::Asc,
                        SortDirection::Desc => StoreSortDirection::Desc,
                    },
                    allowed_sources: allowed_sources.clone(),
                    archived: archived.unwrap_or(false),
                    search_term: search_term.clone(),
                })
                .await
                .map_err(chat_store_list_error)?;

            for result in page.items {
                let source = with_thread_spawn_agent_metadata(
                    result.chat.source.clone(),
                    result.chat.agent_nickname.clone(),
                    result.chat.agent_role.clone(),
                );
                if source_kind_filter
                    .as_ref()
                    .is_none_or(|filter| source_kind_matches(&source, filter))
                {
                    search_results.push(result);
                    if search_results.len() >= requested_page_size {
                        break;
                    }
                }
            }

            remaining = requested_page_size.saturating_sub(search_results.len());
            next_cursor = page.next_cursor;
            if remaining == 0 {
                break;
            }

            let Some(cursor_val) = next_cursor.clone() else {
                break;
            };
            if last_cursor.as_ref() == Some(&cursor_val) {
                next_cursor = None;
                break;
            }
            last_cursor = Some(cursor_val.clone());
            cursor_obj = Some(cursor_val);
        }

        let backwards_cursor = search_results.first().and_then(|result| {
            thread_backwards_cursor_for_sort_key(&result.chat, store_sort_key, store_sort_direction)
        });
        let fallback_provider = self.config.model_provider_id.clone();
        let mut results = Vec::with_capacity(search_results.len());
        let mut status_ids = Vec::with_capacity(search_results.len());
        for result in search_results {
            let (thread, _) =
                chat_from_stored_chat(result.chat, fallback_provider.as_str(), &self.config.cwd);
            status_ids.push(thread.id.clone());
            results.push((thread, result.snippet));
        }
        let statuses = self
            .chat_watch_manager
            .loaded_statuses_for_chats(status_ids)
            .await;
        let data = results
            .into_iter()
            .map(|(mut thread, snippet)| {
                if let Some(status) = statuses.get(&thread.id) {
                    thread.status = status.clone();
                }
                ChatSearchResult { thread, snippet }
            })
            .collect();

        Ok(ChatSearchResponse {
            data,
            next_cursor,
            backwards_cursor,
        })
    }

    async fn thread_loaded_list_response_inner(
        &self,
        params: ChatLoadedListParams,
    ) -> Result<ChatLoadedListResponse, JSONRPCErrorError> {
        let ChatLoadedListParams { cursor, limit } = params;
        let mut data: Vec<String> = self
            .chat_manager
            .list_chat_ids()
            .await
            .into_iter()
            .map(|chat_id| chat_id.to_string())
            .collect();

        if data.is_empty() {
            return Ok(ChatLoadedListResponse {
                data,
                next_cursor: None,
            });
        }

        data.sort();
        let total = data.len();
        let start = match cursor {
            Some(cursor) => {
                let cursor = match ChatId::from_string(&cursor) {
                    Ok(id) => id.to_string(),
                    Err(_) => return Err(invalid_request(format!("invalid cursor: {cursor}"))),
                };
                match data.binary_search(&cursor) {
                    Ok(idx) => idx + 1,
                    Err(idx) => idx,
                }
            }
            None => 0,
        };

        let effective_limit = limit.unwrap_or(total as u32).max(1) as usize;
        let end = start.saturating_add(effective_limit).min(total);
        let page = data[start..end].to_vec();
        let next_cursor = page.last().filter(|_| end < total).cloned();

        Ok(ChatLoadedListResponse {
            data: page,
            next_cursor,
        })
    }

    async fn thread_read_response_inner(
        &self,
        params: ChatReadParams,
    ) -> Result<ChatReadResponse, JSONRPCErrorError> {
        let ChatReadParams {
            chat_id,
            include_interactions,
        } = params;

        let thread_uuid = ChatId::from_string(&chat_id)
            .map_err(|err| invalid_request(format!("invalid thread id: {err}")))?;

        let thread = self
            .read_chat_view(thread_uuid, include_interactions)
            .await
            .map_err(thread_read_view_error)?;
        Ok(ChatReadResponse { chat: thread })
    }

    /// Builds the API view for `chat/read` from persisted metadata plus optional live state.
    async fn read_chat_view(
        &self,
        chat_id: ChatId,
        include_interactions: bool,
    ) -> Result<Chat, ThreadReadViewError> {
        let loaded_thread = self.chat_manager.get_chat(chat_id).await.ok();
        let mut thread = if include_interactions {
            if let Some(loaded_thread) = loaded_thread.as_ref() {
                // Loaded thread with interactions: use persisted metadata when it exists,
                // but reconstruct interactions from the live ChatStore history.
                let persisted_thread = self
                    .load_persisted_thread_for_read(chat_id, /*include_interactions*/ false)
                    .await?;
                self.load_live_chat_view(
                    chat_id,
                    include_interactions,
                    loaded_thread,
                    persisted_thread,
                )
                .await?
            } else if let Some(thread) = self
                .load_persisted_thread_for_read(chat_id, include_interactions)
                .await?
            {
                // Unloaded thread with interactions: load metadata and history together
                // from the ChatStore.
                thread
            } else {
                return Err(ThreadReadViewError::InvalidRequest(format!(
                    "thread not loaded: {chat_id}"
                )));
            }
        } else if let Some(thread) = self
            .load_persisted_thread_for_read(chat_id, include_interactions)
            .await?
        {
            // Persisted metadata-only read: no live thread state is needed.
            thread
        } else if let Some(loaded_thread) = loaded_thread.as_ref() {
            // Loaded metadata-only read before persistence is materialized: build
            // the response from the live thread snapshot.
            self.load_live_chat_view(
                chat_id,
                include_interactions,
                loaded_thread,
                /*persisted_thread*/ None,
            )
            .await?
        } else {
            return Err(ThreadReadViewError::InvalidRequest(format!(
                "thread not loaded: {chat_id}"
            )));
        };

        let has_live_in_progress_turn = if let Some(loaded_thread) = loaded_thread.as_ref() {
            matches!(loaded_thread.agent_status().await, AgentStatus::Running)
        } else {
            false
        };

        let chat_status = self
            .chat_watch_manager
            .loaded_status_for_chat(&thread.id)
            .await;

        set_chat_status_and_interrupt_stale_turns(
            &mut thread,
            chat_status,
            has_live_in_progress_turn,
        );
        Ok(thread)
    }

    async fn load_persisted_thread_for_read(
        &self,
        chat_id: ChatId,
        include_interactions: bool,
    ) -> Result<Option<Chat>, ThreadReadViewError> {
        let fallback_provider = self.config.model_provider_id.as_str();
        match self
            .chat_store
            .read_chat(StoreReadChatParams {
                chat_id: chat_id,
                include_archived: true,
                include_history: include_interactions,
            })
            .await
        {
            Ok(stored_chat) => {
                let (mut thread, history) =
                    chat_from_stored_chat(stored_chat, fallback_provider, &self.config.cwd);
                if include_interactions && let Some(history) = history {
                    thread.interactions = build_api_turns_from_rollout_items(&history.items);
                }
                Ok(Some(thread))
            }
            Err(ChatStoreError::InvalidRequest { message })
                if message == format!("no rollout found for thread id {chat_id}") =>
            {
                Ok(None)
            }
            Err(ChatStoreError::ChatNotFound {
                chat_id: missing_chat_id,
            }) if missing_chat_id == chat_id => Ok(None),
            Err(ChatStoreError::InvalidRequest { message }) => {
                Err(ThreadReadViewError::InvalidRequest(message))
            }
            Err(err) => Err(ThreadReadViewError::Internal(format!(
                "failed to read chat: {err}"
            ))),
        }
    }

    /// Builds a `chat/read` view from a loaded thread plus optional persisted metadata.
    async fn load_live_chat_view(
        &self,
        chat_id: ChatId,
        include_interactions: bool,
        loaded_thread: &DataxChat,
        persisted_thread: Option<Chat>,
    ) -> Result<Chat, ThreadReadViewError> {
        let config_snapshot = loaded_thread.config_snapshot().await;
        if include_interactions && config_snapshot.ephemeral {
            return Err(ThreadReadViewError::InvalidRequest(
                "ephemeral threads do not support includeTurns".to_string(),
            ));
        }
        let fallback_thread =
            build_thread_from_loaded_snapshot(chat_id, &config_snapshot, loaded_thread);
        let mut thread = if let Some(mut thread) = persisted_thread {
            if thread.path.is_none() {
                thread.path = fallback_thread.path.clone();
            }
            thread.session_id.clone_from(&fallback_thread.session_id);
            thread.ephemeral = fallback_thread.ephemeral;
            thread
        } else {
            fallback_thread
        };
        self.apply_thread_read_store_fields(
            chat_id,
            &mut thread,
            include_interactions,
            loaded_thread,
        )
        .await?;
        Ok(thread)
    }

    async fn apply_thread_read_store_fields(
        &self,
        chat_id: ChatId,
        chat: &mut Chat,
        include_interactions: bool,
        loaded_thread: &DataxChat,
    ) -> Result<(), ThreadReadViewError> {
        self.attach_thread_name(chat_id, thread).await;

        if include_interactions {
            let history = loaded_thread
                .load_history(/*include_archived*/ true)
                .await
                .map_err(|err| thread_read_history_load_error(chat_id, err))?;
            thread.interactions = build_api_turns_from_rollout_items(&history.items);
        }

        Ok(())
    }

    async fn thread_interactions_list_response_inner(
        &self,
        params: ChatInteractionsListParams,
    ) -> Result<ChatInteractionsListResponse, JSONRPCErrorError> {
        let ChatInteractionsListParams {
            chat_id,
            cursor,
            limit,
            sort_direction,
            messages_view,
        } = params;
        let thread_uuid = ChatId::from_string(&chat_id)
            .map_err(|err| invalid_request(format!("invalid thread id: {err}")))?;

        let messages = self
            .load_thread_interactions_list_history(thread_uuid)
            .await
            .map_err(thread_read_view_error)?;
        // This API optimizes network transfer by letting clients page through a
        // thread's interactions incrementally, but it still replays the entire rollout on
        // every request. Rollback and compaction events can change earlier interactions, so
        // the server has to rebuild the full turn list until turn metadata is indexed
        // separately.
        let loaded_thread = self.chat_manager.get_chat(thread_uuid).await.ok();
        let has_live_running_thread = match loaded_thread.as_ref() {
            Some(thread) => matches!(thread.agent_status().await, AgentStatus::Running),
            None => false,
        };
        let active_interaction = if loaded_thread.is_some() {
            // Persisted history may not yet include the currently running turn. The
            // app-server listener has already projected live turn events into ChatState,
            // so merge that in-memory snapshot before paginating.
            let chat_state = self.chat_state_manager.chat_state(thread_uuid).await;
            let state = chat_state.lock().await;
            state.active_interaction_snapshot()
        } else {
            None
        };
        build_chat_interactions_page_response(
            &messages,
            self.chat_watch_manager
                .loaded_status_for_chat(&thread_uuid.to_string())
                .await,
            has_live_running_thread,
            active_interaction,
            ThreadTurnsPageOptions {
                cursor: cursor.as_deref(),
                limit,
                sort_direction: sort_direction.unwrap_or(SortDirection::Desc),
                messages_view: messages_view.unwrap_or(InteractionMessagesView::Summary),
            },
        )
    }

    async fn load_thread_interactions_list_history(
        &self,
        chat_id: ChatId,
    ) -> Result<Vec<RolloutMessage>, ThreadReadViewError> {
        match self
            .chat_store
            .read_chat(StoreReadChatParams {
                chat_id: chat_id,
                include_archived: true,
                include_history: true,
            })
            .await
        {
            Ok(stored_chat) => {
                let history = stored_chat.history.ok_or_else(|| {
                    ThreadReadViewError::Internal(format!(
                        "chat store did not return history for thread {chat_id}"
                    ))
                })?;
                return Ok(history.items);
            }
            Err(ChatStoreError::InvalidRequest { message })
                if message == format!("no rollout found for thread id {chat_id}") => {}
            Err(ChatStoreError::ChatNotFound {
                chat_id: missing_chat_id,
            }) if missing_chat_id == chat_id => {}
            Err(ChatStoreError::InvalidRequest { message }) => {
                return Err(ThreadReadViewError::InvalidRequest(message));
            }
            Err(err) => {
                return Err(ThreadReadViewError::Internal(format!(
                    "failed to read chat: {err}"
                )));
            }
        }

        let thread = self.chat_manager.get_chat(chat_id).await.map_err(|_| {
            ThreadReadViewError::InvalidRequest(format!("thread not loaded: {chat_id}"))
        })?;
        let config_snapshot = thread.config_snapshot().await;
        if config_snapshot.ephemeral {
            return Err(ThreadReadViewError::InvalidRequest(
                "ephemeral threads do not support chat/interactions/list".to_string(),
            ));
        }

        thread
            .load_history(/*include_archived*/ true)
            .await
            .map(|history| history.items)
            .map_err(|err| thread_interactions_list_history_load_error(chat_id, err))
    }

    pub(crate) fn thread_created_receiver(&self) -> broadcast::Receiver<ChatId> {
        self.chat_manager.subscribe_thread_created()
    }

    pub(crate) async fn connection_initialized(
        &self,
        connection_id: ConnectionId,
        capabilities: ConnectionCapabilities,
    ) {
        self.chat_state_manager
            .connection_initialized(connection_id, capabilities)
            .await;
    }

    pub(crate) async fn connection_closed(&self, connection_id: ConnectionId) {
        let chat_ids = self
            .chat_state_manager
            .remove_connection(connection_id)
            .await;

        for chat_id in chat_ids {
            if self.chat_manager.get_chat(chat_id).await.is_err() {
                // Reconcile stale app-server bookkeeping when the thread has already been
                // removed from the core manager.
                self.finalize_thread_teardown(chat_id).await;
            }
        }
    }

    pub(crate) fn subscribe_running_assistant_turn_count(&self) -> watch::Receiver<usize> {
        self.chat_watch_manager.subscribe_running_interaction_count()
    }

    /// Best-effort: ensure initialized connections are subscribed to this thread.
    pub(crate) async fn try_attach_chat_listener(
        &self,
        chat_id: ChatId,
        connection_ids: Vec<ConnectionId>,
    ) {
        let mut raw_events_enabled = false;
        if let Ok(thread) = self.chat_manager.get_chat(chat_id).await {
            let config_snapshot = thread.config_snapshot().await;
            let loaded_thread = build_thread_from_snapshot(
                chat_id,
                thread.session_configured().session_id.to_string(),
                &config_snapshot,
                thread.rollout_path(),
            );
            self.chat_watch_manager.upsert_chat(loaded_thread).await;
            if let Some(parent_chat_id) = config_snapshot.parent_chat_id {
                raw_events_enabled = self
                    .chat_state_manager
                    .chat_state(parent_chat_id)
                    .await
                    .lock()
                    .await
                    .experimental_raw_events;
            }
        }

        for connection_id in connection_ids {
            log_listener_attach_result(
                self.ensure_conversation_listener(chat_id, connection_id, raw_events_enabled)
                    .await,
                chat_id,
                connection_id,
                "thread",
            );
        }
    }

    async fn chat_resume_inner(
        &self,
        request_id: ConnectionRequestId,
        params: ChatResumeParams,
        app_server_client_name: Option<String>,
        app_server_client_version: Option<String>,
        supports_openai_form_elicitation: bool,
    ) -> Result<(), JSONRPCErrorError> {
        if let Ok(chat_id) = ChatId::from_string(&params.chat_id)
            && self.pending_chat_unloads.lock().await.contains(&chat_id)
        {
            self.outgoing
                .send_error(
                    request_id,
                    invalid_request(format!(
                        "thread {chat_id} is closing; retry chat/resume after the thread is closed"
                    )),
                )
                .await;
            return Ok(());
        }

        if params.sandbox.is_some() && params.permissions.is_some() {
            self.outgoing
                .send_error(
                    request_id,
                    invalid_request("`permissions` cannot be combined with `sandbox`"),
                )
                .await;
            return Ok(());
        }
        let redact_resume_payloads =
            should_redact_chat_resume_payloads(app_server_client_name.as_deref());

        let _thread_list_state_permit = match self.acquire_thread_list_state_permit().await {
            Ok(permit) => permit,
            Err(error) => {
                self.outgoing.send_error(request_id, error).await;
                return Ok(());
            }
        };
        let stored_chat_from_running_probe = match self
            .resume_running_thread(
                &request_id,
                &params,
                app_server_client_name.clone(),
                app_server_client_version.clone(),
            )
            .await
        {
            Ok(RunningThreadResumeResult::Handled) => return Ok(()),
            Ok(RunningThreadResumeResult::NotRunning(stored_chat)) => stored_chat,
            Err(error) => {
                self.outgoing.send_error(request_id, error).await;
                return Ok(());
            }
        };

        let ChatResumeParams {
            chat_id,
            history,
            path,
            model,
            model_provider,
            service_tier,
            cwd,
            runtime_workspace_roots,
            approval_policy,
            approvals_reviewer,
            sandbox,
            permissions,
            config: mut request_overrides,
            base_instructions,
            developer_instructions,
            personality,
            exclude_interactions,
            initial_interactions_page,
        } = params;
        let include_interactions = !exclude_interactions;

        let resume_result = if let Some(history) = history {
            self.resume_chat_from_history(history.as_slice())
                .await
                .map(|thread_history| (thread_history, None))
        } else if let Some(stored_chat) = stored_chat_from_running_probe {
            self.stored_chat_to_initial_history(&stored_chat)
                .await
                .map(|thread_history| (thread_history, Some(*stored_chat)))
        } else {
            self.resume_chat_from_rollout(&chat_id, path.as_ref())
                .await
                .map(|(thread_history, stored_chat)| (thread_history, Some(stored_chat)))
        };
        let (thread_history, resume_source_thread) = match resume_result {
            Ok(value) => value,
            Err(error) => {
                self.outgoing.send_error(request_id, error).await;
                return Ok(());
            }
        };

        let history_cwd = thread_history.session_cwd();
        let runtime_workspace_roots = runtime_workspace_roots.map(resolve_runtime_workspace_roots);
        let mut typesafe_overrides = self.build_thread_config_overrides(
            model,
            model_provider,
            service_tier,
            cwd,
            runtime_workspace_roots,
            approval_policy,
            approvals_reviewer,
            sandbox,
            permissions,
            base_instructions,
            developer_instructions,
            personality,
        );
        self.load_and_apply_persisted_resume_metadata(
            &thread_history,
            &mut request_overrides,
            &mut typesafe_overrides,
        )
        .await;

        // Derive a Config using the same logic as new conversation, honoring overrides if provided.
        let config = match self
            .config_manager
            .load_for_cwd(request_overrides, typesafe_overrides, history_cwd)
            .await
        {
            Ok(config) => config,
            Err(err) => {
                let error = config_load_error(&err);
                self.outgoing.send_error(request_id, error).await;
                return Ok(());
            }
        };

        let response_history = thread_history.clone();

        match self
            .chat_manager
            .resume_chat_with_history(
                config,
                thread_history,
                self.auth_manager.clone(),
                self.request_trace_context(&request_id).await,
                supports_openai_form_elicitation,
            )
            .await
        {
            Ok(NewChat {
                chat_id: chat_id,
                chat: datax_chat,
                session_configured,
                ..
            }) => {
                if let Err(err) = Self::set_app_server_client_info(
                    datax_chat.as_ref(),
                    app_server_client_name,
                    app_server_client_version,
                )
                .await
                {
                    self.outgoing.send_error(request_id, err).await;
                    return Ok(());
                }
                let instruction_sources = datax_chat.legacy_instruction_sources().await;
                let SessionConfiguredEvent { rollout_path, .. } = session_configured;
                let Some(rollout_path) = rollout_path else {
                    let error =
                        internal_error(format!("rollout path missing for thread {chat_id}"));
                    self.outgoing.send_error(request_id, error).await;
                    return Ok(());
                };
                // Auto-attach a chat listener when resuming a thread.
                log_listener_attach_result(
                    self.ensure_conversation_listener(
                        chat_id,
                        request_id.connection_id,
                        /*raw_events_enabled*/ false,
                    )
                    .await,
                    chat_id,
                    request_id.connection_id,
                    "thread",
                );

                let mut thread = match self
                    .load_thread_from_resume_source_or_send_internal(
                        chat_id,
                        datax_chat.as_ref(),
                        &response_history,
                        rollout_path.as_path(),
                        resume_source_thread,
                        include_interactions,
                    )
                    .await
                {
                    Ok(thread) => thread,
                    Err(message) => {
                        self.outgoing
                            .send_error(request_id, internal_error(message))
                            .await;
                        return Ok(());
                    }
                };
                thread.chat_source = datax_chat
                    .config_snapshot()
                    .await
                    .thread_source
                    .map(Into::into);

                self.chat_watch_manager
                    .upsert_chat(thread.clone())
                    .await;

                let chat_status = self
                    .chat_watch_manager
                    .loaded_status_for_chat(&thread.id)
                    .await;

                set_chat_status_and_interrupt_stale_turns(
                    &mut thread,
                    chat_status,
                    /*has_live_in_progress_turn*/ false,
                );
                let config_snapshot = datax_chat.config_snapshot().await;
                let sandbox = thread_response_sandbox_policy(
                    &config_snapshot.permission_profile,
                    config_snapshot.cwd().as_path(),
                );
                let active_permission_profile = thread_response_active_permission_profile(
                    config_snapshot.active_permission_profile,
                );
                let token_usage_thread = include_interactions.then(|| thread.clone());
                let mut initial_interactions_page =
                    if let Some(params) = initial_interactions_page.as_ref() {
                        match build_chat_resume_initial_turns_page(
                            &response_history.get_rollout_items(),
                            thread.status.clone(),
                            /*has_live_running_thread*/ false,
                            /*active_interaction*/ None,
                            params,
                        ) {
                            Ok(page) => Some(page),
                            Err(error) => {
                                self.outgoing.send_error(request_id, error).await;
                                return Ok(());
                            }
                        }
                    } else {
                        None
                    };
                if redact_resume_payloads {
                    redact_chat_resume_payloads(&mut thread.interactions);
                    if let Some(initial_interactions_page) = initial_interactions_page.as_mut() {
                        redact_chat_resume_payloads(&mut initial_interactions_page.data);
                    }
                }

                let response = ChatResumeResponse {
                    chat: thread,
                    model: session_configured.model,
                    model_provider: session_configured.model_provider_id,
                    service_tier: session_configured.service_tier,
                    cwd: session_configured.cwd,
                    runtime_workspace_roots: config_snapshot.workspace_roots,
                    instruction_sources,
                    approval_policy: session_configured.approval_policy.into(),
                    approvals_reviewer: session_configured.approvals_reviewer.into(),
                    sandbox,
                    active_permission_profile,
                    reasoning_effort: session_configured.reasoning_effort,
                    multi_agent_mode: MultiAgentMode::ExplicitRequestOnly,
                    initial_interactions_page,
                };

                let connection_id = request_id.connection_id;
                self.outgoing.send_response(request_id, response).await;
                // `excludeTurns` is explicitly the cheap resume path, so avoid
                // rebuilding history only to attribute a replayed usage update.
                if let Some(token_usage_thread) = token_usage_thread {
                    let token_usage_interaction_id =
                        latest_token_usage_interaction_id_from_rollout_items(
                            &response_history.get_rollout_items(),
                            token_usage_thread.interactions.as_slice(),
                        );
                    // The client needs restored usage before it starts another turn.
                    // Sending after the response preserves JSON-RPC request ordering while
                    // still filling the status line before the next turn lifecycle begins.
                    send_thread_token_usage_update_to_connection(
                        &self.outgoing,
                        connection_id,
                        chat_id,
                        &token_usage_thread,
                        datax_chat.as_ref(),
                        token_usage_interaction_id,
                    )
                    .await;
                }
                self.chat_goal_processor
                    .emit_resume_goal_snapshot_and_continue(chat_id, datax_chat.as_ref())
                    .await;
            }
            Err(err) => {
                let error = internal_error(format!("error resuming chat: {err}"));
                self.outgoing.send_error(request_id, error).await;
            }
        }
        Ok(())
    }

    async fn load_and_apply_persisted_resume_metadata(
        &self,
        thread_history: &InitialHistory,
        request_overrides: &mut Option<HashMap<String, serde_json::Value>>,
        typesafe_overrides: &mut ConfigOverrides,
    ) -> Option<ThreadMetadata> {
        let InitialHistory::Resumed(resumed_history) = thread_history else {
            return None;
        };
        let state_db_ctx = self.state_db.clone()?;
        let persisted_metadata = state_db_ctx
            .get_chat(resumed_history.conversation_id)
            .await
            .ok()
            .flatten()?;
        merge_persisted_resume_metadata(request_overrides, typesafe_overrides, &persisted_metadata);
        Some(persisted_metadata)
    }

    #[tracing::instrument(level = "trace", skip_all)]
    async fn resume_running_thread(
        &self,
        request_id: &ConnectionRequestId,
        params: &ChatResumeParams,
        app_server_client_name: Option<String>,
        app_server_client_version: Option<String>,
    ) -> Result<RunningThreadResumeResult, JSONRPCErrorError> {
        let running_thread = if params.history.is_some() {
            if let Ok(existing_chat_id) = ChatId::from_string(&params.chat_id)
                && self.chat_manager.get_chat(existing_chat_id).await.is_ok()
            {
                return Err(invalid_request(format!(
                    "cannot resume thread {existing_chat_id} with history while it is already running"
                )));
            }
            None
        } else if let Ok(existing_chat_id) = ChatId::from_string(&params.chat_id)
            && let Ok(existing_thread) = self.chat_manager.get_chat(existing_chat_id).await
        {
            let source_thread = self
                .read_stored_chat_for_resume(
                    &params.chat_id,
                    /*path*/ None,
                    /*include_history*/ true,
                )
                .await?;
            Some((existing_chat_id, existing_thread, source_thread))
        } else {
            let source_thread = self
                .read_stored_chat_for_resume(
                    &params.chat_id,
                    params.path.as_ref(),
                    /*include_history*/ true,
                )
                .await?;
            let existing_chat_id = source_thread.chat_id;
            match self.chat_manager.get_chat(existing_chat_id).await {
                Ok(existing_thread) => Some((existing_chat_id, existing_thread, source_thread)),
                Err(_) => {
                    return Ok(RunningThreadResumeResult::NotRunning(Some(Box::new(
                        source_thread,
                    ))));
                }
            }
        };

        if let Some((existing_chat_id, existing_thread, source_thread)) = running_thread {
            let existing_thread_rollout_path = existing_thread.rollout_path();
            let active_path = existing_thread_rollout_path
                .as_ref()
                .or(source_thread.rollout_path.as_ref());
            if let (Some(requested_path), Some(active_path)) = (params.path.as_ref(), active_path)
                && !path_utils::paths_match_after_normalization(requested_path, active_path)
            {
                return Err(invalid_request(format!(
                    "cannot resume running thread {existing_chat_id} with stale path: requested `{}`, active `{}`",
                    requested_path.display(),
                    active_path.display()
                )));
            }
            let config_snapshot = existing_thread.config_snapshot().await;
            let mismatch_details = collect_resume_override_mismatches(params, &config_snapshot);
            if !mismatch_details.is_empty() {
                let has_subscribers = !self
                    .chat_state_manager
                    .subscribed_connection_ids(existing_chat_id)
                    .await
                    .is_empty();
                let loaded_status = self
                    .chat_watch_manager
                    .loaded_status_for_chat(&existing_chat_id.to_string())
                    .await;
                let is_running =
                    matches!(existing_thread.agent_status().await, AgentStatus::Running);

                if !has_subscribers && matches!(loaded_status, ChatStatus::Idle) && !is_running {
                    // A loaded idle thread is only a cache entry. Shut it down
                    // before removing it so cold resume cannot duplicate a
                    // thread that timed out during shutdown.
                    match wait_for_chat_shutdown(&existing_thread).await {
                        ChatShutdownResult::Complete => {
                            self.chat_manager.remove_chat(&existing_chat_id).await;
                            self.finalize_thread_teardown(existing_chat_id).await;
                            // Shutdown can flush newer rollout messages, so reload the
                            // stored thread before starting the replacement session.
                            return Ok(RunningThreadResumeResult::NotRunning(None));
                        }
                        ChatShutdownResult::SubmitFailed => {
                            warn!("failed to submit Shutdown to thread {existing_chat_id}");
                        }
                        ChatShutdownResult::TimedOut => {
                            warn!("thread {existing_chat_id} shutdown timed out");
                        }
                    }
                }

                // Preserve rejoin semantics when another client can still observe
                // the loaded thread or shutdown did not complete.
                tracing::warn!(
                    "chat/resume overrides ignored for loaded thread {}: {}",
                    existing_chat_id,
                    mismatch_details.join("; ")
                );
            }
            let redact_resume_payloads =
                should_redact_chat_resume_payloads(app_server_client_name.as_deref());
            let history_items = source_thread
                .history
                .as_ref()
                .map(|history| history.items.clone())
                .ok_or_else(|| {
                    internal_error(format!(
                        "thread {existing_chat_id} did not include persisted history"
                    ))
                })?;

            let chat_state = self
                .chat_state_manager
                .chat_state(existing_chat_id)
                .await;
            self.ensure_listener_task_running(
                existing_chat_id,
                existing_thread.clone(),
                chat_state.clone(),
            )
            .await?;
            Self::set_app_server_client_info(
                existing_thread.as_ref(),
                app_server_client_name,
                app_server_client_version,
            )
            .await?;

            let mut summary_source_thread = source_thread;
            summary_source_thread.history = None;
            let mut chat_summary = self.stored_chat_to_api_thread(
                summary_source_thread,
                config_snapshot.model_provider_id.as_str(),
                /*include_interactions*/ false,
            );
            chat_summary.session_id = existing_thread.session_configured().session_id.to_string();
            let instruction_sources = existing_thread.legacy_instruction_sources().await;

            let listener_command_tx = {
                let chat_state = chat_state.lock().await;
                chat_state.listener_command_tx()
            };
            let Some(listener_command_tx) = listener_command_tx else {
                return Err(internal_error(format!(
                    "failed to enqueue running chat resume for thread {existing_chat_id}: chat listener is not running"
                )));
            };

            let (emit_chat_goal_update, chat_goal_state_db) = self
                .chat_goal_processor
                .pending_resume_goal_state(existing_thread.as_ref())
                .await;

            let command = crate::chat_state::ChatListenerCommand::SendChatResumeResponse(
                Box::new(crate::chat_state::PendingChatResumeRequest {
                    request_id: request_id.clone(),
                    history_items,
                    config_snapshot,
                    instruction_sources,
                    chat_summary,
                    emit_chat_goal_update,
                    chat_goal_state_db,
                    include_interactions: !params.exclude_interactions,
                    initial_turns_page: params.initial_interactions_page.clone(),
                    redact_resume_payloads,
                }),
            );
            if listener_command_tx.send(command).is_err() {
                return Err(internal_error(format!(
                    "failed to enqueue running chat resume for thread {existing_chat_id}: chat listener command channel is closed"
                )));
            }
            return Ok(RunningThreadResumeResult::Handled);
        }
        Ok(RunningThreadResumeResult::NotRunning(None))
    }

    #[tracing::instrument(level = "trace", skip_all)]
    async fn resume_chat_from_history(
        &self,
        history: &[ResponseItem],
    ) -> Result<InitialHistory, JSONRPCErrorError> {
        if history.is_empty() {
            return Err(invalid_request("history must not be empty"));
        }
        Ok(InitialHistory::Forked(
            history
                .iter()
                .cloned()
                .map(RolloutMessage::ResponseItem)
                .collect(),
        ))
    }

    #[tracing::instrument(level = "trace", skip_all)]
    async fn resume_chat_from_rollout(
        &self,
        chat_id: &str,
        path: Option<&PathBuf>,
    ) -> Result<(InitialHistory, StoredChat), JSONRPCErrorError> {
        let stored_chat = self
            .read_stored_chat_for_resume(chat_id, path, /*include_history*/ true)
            .await?;
        let history = self
            .stored_chat_to_initial_history(&stored_chat)
            .await?;
        Ok((history, stored_chat))
    }

    async fn read_stored_chat_for_resume(
        &self,
        chat_id: &str,
        path: Option<&PathBuf>,
        include_history: bool,
    ) -> Result<StoredChat, JSONRPCErrorError> {
        let result = if let Some(path) = path {
            self.chat_store
                .read_chat_by_rollout_path(StoreReadChatByRolloutPathParams {
                    rollout_path: path.clone(),
                    include_archived: true,
                    include_history,
                })
                .await
        } else {
            let existing_chat_id = match ChatId::from_string(chat_id) {
                Ok(id) => id,
                Err(err) => {
                    return Err(invalid_request(format!("invalid session id: {err}")));
                }
            };
            let params = StoreReadChatParams {
                chat_id: existing_chat_id,
                include_archived: true,
                include_history,
            };
            self.chat_store.read_chat(params).await
        };

        let stored_chat = result.map_err(chat_store_resume_read_error)?;
        if stored_chat.archived_at.is_some() {
            let chat_id = stored_chat.chat_id;
            return Err(invalid_request(format!(
                "session {chat_id} is archived. Run `codex unarchive {chat_id}` to unarchive it first."
            )));
        }

        Ok(stored_chat)
    }

    #[tracing::instrument(level = "trace", skip_all)]
    async fn stored_chat_to_initial_history(
        &self,
        stored_chat: &StoredChat,
    ) -> Result<InitialHistory, JSONRPCErrorError> {
        let chat_id = stored_chat.chat_id;
        let history = stored_chat
            .history
            .as_ref()
            .map(|history| history.items.clone())
            .ok_or_else(|| {
                internal_error(format!(
                    "thread {chat_id} did not include persisted history"
                ))
            })?;
        Ok(InitialHistory::Resumed(ResumedHistory {
            conversation_id: chat_id,
            history,
            rollout_path: stored_chat.rollout_path.clone(),
        }))
    }

    fn stored_chat_to_api_thread(
        &self,
        stored_chat: StoredChat,
        fallback_provider: &str,
        include_interactions: bool,
    ) -> Chat {
        let (mut thread, history) =
            chat_from_stored_chat(stored_chat, fallback_provider, &self.config.cwd);
        if include_interactions && let Some(history) = history {
            populate_chat_interactions_from_history(
                &mut thread,
                &history.items,
                /*active_interaction*/ None,
            );
        }
        thread
    }

    async fn read_stored_chat_for_new_fork(
        &self,
        chat_id: ChatId,
        include_history: bool,
    ) -> Result<StoredChat, JSONRPCErrorError> {
        self.chat_store
            .read_chat(StoreReadChatParams {
                chat_id: chat_id,
                include_archived: true,
                include_history,
            })
            .await
            .map_err(chat_store_resume_read_error)
    }

    async fn load_thread_from_resume_source_or_send_internal(
        &self,
        chat_id: ChatId,
        chat: &DataxChat,
        thread_history: &InitialHistory,
        rollout_path: &Path,
        resume_source_thread: Option<StoredChat>,
        include_interactions: bool,
    ) -> std::result::Result<Chat, String> {
        let config_snapshot = thread.config_snapshot().await;
        let session_id = thread.session_configured().session_id.to_string();
        let thread = match thread_history {
            InitialHistory::Resumed(resumed) => {
                let fallback_provider = config_snapshot.model_provider_id.as_str();
                if let Some(stored_chat) = resume_source_thread {
                    let stored_chat =
                        if let Some(rollout_path) = stored_chat.rollout_path.clone() {
                            self.chat_store
                                .read_chat_by_rollout_path(StoreReadChatByRolloutPathParams {
                                    rollout_path,
                                    include_archived: true,
                                    include_history: false,
                                })
                                .await
                                .unwrap_or(StoredChat {
                                    history: None,
                                    ..stored_chat
                                })
                        } else {
                            self.chat_store
                                .read_chat(StoreReadChatParams {
                                    chat_id: stored_chat.chat_id,
                                    include_archived: true,
                                    include_history: false,
                                })
                                .await
                                .unwrap_or(StoredChat {
                                    history: None,
                                    ..stored_chat
                                })
                        };
                    Ok(
                        chat_from_stored_chat(stored_chat, fallback_provider, &self.config.cwd)
                            .0,
                    )
                } else {
                    match self
                        .chat_store
                        .read_chat(StoreReadChatParams {
                            chat_id: resumed.conversation_id,
                            include_archived: true,
                            include_history: false,
                        })
                        .await
                    {
                        Ok(stored_chat) => Ok(chat_from_stored_chat(
                            stored_chat,
                            fallback_provider,
                            &self.config.cwd,
                        )
                        .0),
                        Err(read_err) => {
                            Err(format!("failed to read thread from store: {read_err}"))
                        }
                    }
                }
            }
            InitialHistory::Forked(messages) => {
                let mut thread = build_thread_from_snapshot(
                    chat_id,
                    session_id.clone(),
                    &config_snapshot,
                    Some(rollout_path.into()),
                );
                thread.preview = preview_from_rollout_items(messages);
                Ok(thread)
            }
            InitialHistory::New | InitialHistory::Cleared => Err(format!(
                "failed to build resume response for thread {chat_id}: initial history missing"
            )),
        };
        let mut thread = thread?;
        thread.id = chat_id.to_string();
        thread.session_id = session_id;
        thread.path = Some(rollout_path.to_path_buf());
        if include_interactions {
            let history_items = thread_history.get_rollout_items();
            populate_chat_interactions_from_history(
                &mut thread,
                &history_items,
                /*active_interaction*/ None,
            );
        }
        self.attach_thread_name(chat_id, &mut thread).await;
        Ok(thread)
    }

    async fn attach_thread_name(&self, chat_id: ChatId, thread: &mut Chat) {
        if let Ok(stored_chat) = self
            .chat_store
            .read_chat(StoreReadChatParams {
                chat_id: chat_id,
                include_archived: true,
                include_history: false,
            })
            .await
            && let Some(title) = stored_chat.name.as_deref().map(str::trim)
            && !title.is_empty()
            && stored_chat.preview.trim() != title
        {
            set_thread_name_from_title(thread, title.to_string());
        }
    }

    async fn thread_fork_inner(
        &self,
        request_id: ConnectionRequestId,
        params: ChatForkParams,
        app_server_client_name: Option<String>,
        app_server_client_version: Option<String>,
        supports_openai_form_elicitation: bool,
    ) -> Result<(), JSONRPCErrorError> {
        let ChatForkParams {
            chat_id,
            path,
            model,
            model_provider,
            service_tier,
            cwd,
            runtime_workspace_roots,
            approval_policy,
            approvals_reviewer,
            sandbox,
            permissions,
            config: cli_overrides,
            base_instructions,
            developer_instructions,
            ephemeral,
            chat_source,
            exclude_interactions,
        } = params;
        let include_interactions = !exclude_interactions;
        if sandbox.is_some() && permissions.is_some() {
            return Err(invalid_request(
                "`permissions` cannot be combined with `sandbox`",
            ));
        }
        let source_thread = self
            .read_stored_chat_for_resume(&chat_id, path.as_ref(), /*include_history*/ true)
            .await?;
        let source_chat_id = source_thread.chat_id;
        let source_thread_name = source_thread
            .name
            .as_deref()
            .and_then(datax_core::util::normalize_thread_name);
        let history_items = source_thread
            .history
            .as_ref()
            .map(|history| history.items.clone())
            .ok_or_else(|| {
                internal_error(format!(
                    "thread {source_chat_id} did not include persisted history"
                ))
            })?;
        let history_cwd = Some(source_thread.cwd.clone());

        // Persist Windows sandbox mode.
        let mut cli_overrides = cli_overrides.unwrap_or_default();
        if cfg!(windows) {
            match WindowsSandboxLevel::from_config(&self.config) {
                WindowsSandboxLevel::Elevated => {
                    cli_overrides
                        .insert("windows.sandbox".to_string(), serde_json::json!("elevated"));
                }
                WindowsSandboxLevel::RestrictedToken => {
                    cli_overrides.insert(
                        "windows.sandbox".to_string(),
                        serde_json::json!("unelevated"),
                    );
                }
                WindowsSandboxLevel::Disabled => {}
            }
        }
        let request_overrides = if cli_overrides.is_empty() {
            None
        } else {
            Some(cli_overrides)
        };
        let runtime_workspace_roots = runtime_workspace_roots.map(resolve_runtime_workspace_roots);
        let mut typesafe_overrides = self.build_thread_config_overrides(
            model,
            model_provider,
            service_tier,
            cwd,
            runtime_workspace_roots,
            approval_policy,
            approvals_reviewer,
            sandbox,
            permissions,
            base_instructions,
            developer_instructions,
            /*personality*/ None,
        );
        typesafe_overrides.ephemeral = ephemeral.then_some(true);
        // Derive a Config using the same logic as new conversation, honoring overrides if provided.
        let config = self
            .config_manager
            .load_for_cwd(request_overrides, typesafe_overrides, history_cwd)
            .await
            .map_err(|err| config_load_error(&err))?;

        let fallback_model_provider = config.model_provider_id.clone();

        let NewChat {
            chat_id: chat_id,
            chat: forked_thread,
            session_configured,
            ..
        } = self
            .chat_manager
            .fork_chat_from_history(
                ForkSnapshot::Interrupted,
                config,
                InitialHistory::Resumed(ResumedHistory {
                    conversation_id: source_chat_id,
                    history: history_items.clone(),
                    rollout_path: source_thread.rollout_path.clone(),
                }),
                chat_source.map(Into::into),
                self.request_trace_context(&request_id).await,
                supports_openai_form_elicitation,
            )
            .await
            .map_err(|err| match err {
                CodexErr::Io(_) | CodexErr::Json(_) => {
                    invalid_request(format!("failed to load thread {source_chat_id}: {err}"))
                }
                CodexErr::InvalidRequest(message) => invalid_request(message),
                err => internal_error(format!("error forking chat: {err}")),
            })?;

        Self::set_app_server_client_info(
            forked_thread.as_ref(),
            app_server_client_name,
            app_server_client_version,
        )
        .await?;
        if session_configured.rollout_path.is_some()
            && let Some(name) = source_thread_name.clone()
        {
            self.chat_manager
                .update_chat_metadata(
                    chat_id,
                    StoreChatMetadataPatch {
                        name: Some(Some(name)),
                        ..Default::default()
                    },
                    /*include_archived*/ true,
                )
                .await
                .map_err(|err| core_thread_write_error("inherit source thread name", err))?;
        }

        let instruction_sources = forked_thread.legacy_instruction_sources().await;

        // Auto-attach a conversation listener when forking a thread.
        log_listener_attach_result(
            self.ensure_conversation_listener(
                chat_id,
                request_id.connection_id,
                /*raw_events_enabled*/ false,
            )
            .await,
            chat_id,
            request_id.connection_id,
            "thread",
        );

        // Persistent forks materialize their own rollout immediately. Ephemeral forks stay
        // pathless, so they rebuild their visible history from the copied source history instead.
        let mut thread = if session_configured.rollout_path.is_some() {
            let stored_chat = self
                .read_stored_chat_for_new_fork(chat_id, include_interactions)
                .await?;
            self.stored_chat_to_api_thread(
                stored_chat,
                fallback_model_provider.as_str(),
                include_interactions,
            )
        } else {
            let config_snapshot = forked_thread.config_snapshot().await;
            let mut thread = build_thread_from_snapshot(
                chat_id,
                session_configured.session_id.to_string(),
                &config_snapshot,
                /*path*/ None,
            );
            thread.preview = preview_from_rollout_items(&history_items);
            thread.forked_from_id = Some(source_chat_id.to_string());
            if include_interactions {
                populate_chat_interactions_from_history(
                    &mut thread,
                    &history_items,
                    /*active_interaction*/ None,
                );
            }
            thread
        };
        if let Some(name) = source_thread_name {
            set_thread_name_from_title(&mut thread, name);
        }
        thread.session_id = session_configured.session_id.to_string();
        thread.chat_source = forked_thread
            .config_snapshot()
            .await
            .thread_source
            .map(Into::into);

        self.chat_watch_manager
            .upsert_chat_silently(thread.clone())
            .await;

        thread.status = resolve_chat_status(
            self.chat_watch_manager
                .loaded_status_for_chat(&thread.id)
                .await,
            /*has_in_progress_interaction*/ false,
        );
        let config_snapshot = forked_thread.config_snapshot().await;
        let sandbox = thread_response_sandbox_policy(
            &config_snapshot.permission_profile,
            config_snapshot.cwd().as_path(),
        );
        let active_permission_profile =
            thread_response_active_permission_profile(config_snapshot.active_permission_profile);

        let response = ChatForkResponse {
            chat: thread.clone(),
            model: session_configured.model,
            model_provider: session_configured.model_provider_id,
            service_tier: session_configured.service_tier,
            cwd: session_configured.cwd,
            runtime_workspace_roots: config_snapshot.workspace_roots,
            instruction_sources,
            approval_policy: session_configured.approval_policy.into(),
            approvals_reviewer: session_configured.approvals_reviewer.into(),
            sandbox,
            active_permission_profile,
            reasoning_effort: session_configured.reasoning_effort,
            multi_agent_mode: MultiAgentMode::ExplicitRequestOnly,
        };

        let notif = thread_started_notification(thread);
        let connection_id = request_id.connection_id;
        let token_usage_thread = include_interactions.then(|| response.chat.clone());
        self.outgoing.send_response(request_id, response).await;
        // `excludeTurns` is the cheap fork path, so skip restored usage replay
        // instead of rebuilding history only to attribute a historical update.
        if let Some(token_usage_thread) = token_usage_thread {
            let token_usage_interaction_id = latest_token_usage_interaction_id_from_rollout_items(
                &history_items,
                token_usage_thread.interactions.as_slice(),
            );
            // Mirror the resume contract for forks: the new thread is usable as soon
            // as the response arrives, so restored usage must follow immediately.
            send_thread_token_usage_update_to_connection(
                &self.outgoing,
                connection_id,
                chat_id,
                &token_usage_thread,
                forked_thread.as_ref(),
                token_usage_interaction_id,
            )
            .await;
        }

        self.outgoing
            .send_server_notification(ChatStarted(notif))
            .await;
        Ok(())
    }

    async fn get_chat_summary_response_inner(
        &self,
        params: GetConversationSummaryParams,
    ) -> Result<GetConversationSummaryResponse, JSONRPCErrorError> {
        let fallback_provider = self.config.model_provider_id.as_str();
        let read_result = match params {
            GetConversationSummaryParams::ChatId { conversation_id } => self
                .chat_store
                .read_chat(StoreReadChatParams {
                    chat_id: conversation_id,
                    include_archived: true,
                    include_history: false,
                })
                .await
                .map_err(|err| conversation_summary_chat_id_read_error(conversation_id, err)),
            GetConversationSummaryParams::RolloutPath { rollout_path } => {
                let Some(local_chat_store) = self
                    .chat_store
                    .as_any()
                    .downcast_ref::<LocalChatStore>()
                else {
                    return Err(invalid_request(
                        "rollout path queries are only supported with the local chat store",
                    ));
                };

                local_chat_store
                    .read_chat_by_rollout_path(
                        rollout_path.clone(),
                        /*include_archived*/ true,
                        /*include_history*/ false,
                    )
                    .await
                    .map_err(|err| conversation_summary_rollout_path_read_error(&rollout_path, err))
            }
        };

        let stored_chat = read_result?;
        let summary = summary_from_stored_chat(stored_chat, fallback_provider);
        Ok(GetConversationSummaryResponse { summary })
    }

    async fn list_chats_common(
        &self,
        requested_page_size: usize,
        cursor: Option<String>,
        sort_key: StoreChatSortKey,
        sort_direction: SortDirection,
        filters: ThreadListFilters,
    ) -> Result<(Vec<StoredChat>, Option<String>), JSONRPCErrorError> {
        let ThreadListFilters {
            model_providers,
            source_kinds,
            archived,
            cwd_filters,
            search_term,
            use_state_db_only,
            parent_chat_id,
        } = filters;
        let mut cursor_obj = cursor;
        let mut last_cursor = cursor_obj.clone();
        let mut remaining = requested_page_size;
        let mut messages = Vec::with_capacity(requested_page_size);
        let mut next_cursor: Option<String> = None;

        let model_provider_filter = match model_providers {
            Some(providers) => {
                if providers.is_empty() {
                    None
                } else {
                    Some(providers)
                }
            }
            None if parent_chat_id.is_some() => None,
            None => Some(vec![self.config.model_provider_id.clone()]),
        };
        let (allowed_sources_vec, source_kind_filter) =
            if parent_chat_id.is_some() && source_kinds.is_none() {
                (Vec::new(), None)
            } else {
                compute_source_filters(source_kinds)
            };
        let allowed_sources = allowed_sources_vec.as_slice();
        let store_sort_direction = match sort_direction {
            SortDirection::Asc => StoreSortDirection::Asc,
            SortDirection::Desc => StoreSortDirection::Desc,
        };

        while remaining > 0 {
            let page_size = remaining.min(THREAD_LIST_MAX_LIMIT);
            let page = self
                .chat_store
                .list_chats(StoreListChatsParams {
                    page_size,
                    cursor: cursor_obj.clone(),
                    sort_key,
                    sort_direction: store_sort_direction,
                    allowed_sources: allowed_sources.to_vec(),
                    model_providers: model_provider_filter.clone(),
                    cwd_filters: cwd_filters.clone(),
                    archived,
                    search_term: search_term.clone(),
                    use_state_db_only,
                    parent_chat_id: parent_chat_id,
                })
                .await
                .map_err(chat_store_list_error)?;

            let mut filtered = Vec::with_capacity(page.items.len());
            for it in page.items {
                let source = with_thread_spawn_agent_metadata(
                    it.source.clone(),
                    it.agent_nickname.clone(),
                    it.agent_role.clone(),
                );
                if source_kind_filter
                    .as_ref()
                    .is_none_or(|filter| source_kind_matches(&source, filter))
                    && cwd_filters.as_ref().is_none_or(|expected_cwds| {
                        expected_cwds.iter().any(|expected_cwd| {
                            path_utils::paths_match_after_normalization(&it.cwd, expected_cwd)
                        })
                    })
                {
                    filtered.push(it);
                    if filtered.len() >= remaining {
                        break;
                    }
                }
            }
            messages.extend(filtered);
            remaining = requested_page_size.saturating_sub(messages.len());

            next_cursor = page.next_cursor;
            if remaining == 0 {
                break;
            }

            let Some(cursor_val) = next_cursor.clone() else {
                break;
            };
            // Break if our pagination would reuse the same cursor again; this avoids
            // an infinite loop when filtering drops everything on the page.
            if last_cursor.as_ref() == Some(&cursor_val) {
                next_cursor = None;
                break;
            }
            last_cursor = Some(cursor_val.clone());
            cursor_obj = Some(cursor_val);
        }

        Ok((messages, next_cursor))
    }
}

fn xcode_26_4_mcp_elicitations_auto_deny(
    client_name: Option<&str>,
    client_version: Option<&str>,
) -> bool {
    // Xcode 26.4 shipped before app-server MCP elicitation requests were
    // client-visible. Keep elicitations auto-denied for that client line.
    // TODO: Remove this compatibility hack once Xcode 26.4 ages out.
    client_name == Some("Xcode")
        && client_version.is_some_and(|version| version.starts_with("26.4"))
}

const THREAD_TURNS_DEFAULT_LIMIT: usize = 25;
const THREAD_TURNS_MAX_LIMIT: usize = 100;

fn thread_backwards_cursor_for_sort_key(
    chat: &StoredChat,
    sort_key: StoreChatSortKey,
    sort_direction: SortDirection,
) -> Option<String> {
    let timestamp = match sort_key {
        StoreChatSortKey::CreatedAt => thread.created_at,
        StoreChatSortKey::UpdatedAt => thread.updated_at,
        StoreChatSortKey::RecencyAt => thread.recency_at,
    };
    // The state DB stores unique millisecond timestamps. Offset the reverse cursor by one
    // millisecond so the opposite-direction query includes the page anchor.
    let timestamp = match sort_direction {
        SortDirection::Asc => timestamp.checked_add_signed(ChronoDuration::milliseconds(1))?,
        SortDirection::Desc => timestamp.checked_sub_signed(ChronoDuration::milliseconds(1))?,
    };
    Some(timestamp.to_rfc3339_opts(SecondsFormat::Millis, true))
}

struct ThreadTurnsPage {
    pub(super) interactions: Vec<Interaction>,
    pub(super) next_cursor: Option<String>,
    pub(super) backwards_cursor: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadTurnsCursor {
    interaction_id: String,
    include_anchor: bool,
}

fn paginate_chat_interactions(
    interactions: Vec<Interaction>,
    cursor: Option<&str>,
    limit: Option<u32>,
    sort_direction: SortDirection,
) -> Result<ThreadTurnsPage, JSONRPCErrorError> {
    if interactions.is_empty() {
        return Ok(ThreadTurnsPage {
            interactions: Vec::new(),
            next_cursor: None,
            backwards_cursor: None,
        });
    }

    let anchor = cursor.map(parse_chat_interactions_cursor).transpose()?;
    let page_size = limit
        .map(|value| value as usize)
        .unwrap_or(THREAD_TURNS_DEFAULT_LIMIT)
        .clamp(1, THREAD_TURNS_MAX_LIMIT);

    let anchor_index = anchor.as_ref().and_then(|anchor| {
        interactions
            .iter()
            .position(|turn| turn.id == anchor.interaction_id)
    });
    if anchor.is_some() && anchor_index.is_none() {
        return Err(invalid_request(
            "invalid cursor: anchor turn is no longer present",
        ));
    }

    let mut keyed_turns: Vec<_> = interactions.into_iter().enumerate().collect();
    match sort_direction {
        SortDirection::Asc => {
            if let (Some(anchor), Some(anchor_index)) = (anchor.as_ref(), anchor_index) {
                keyed_turns.retain(|(index, _)| {
                    if anchor.include_anchor {
                        *index >= anchor_index
                    } else {
                        *index > anchor_index
                    }
                });
            }
        }
        SortDirection::Desc => {
            keyed_turns.reverse();
            if let (Some(anchor), Some(anchor_index)) = (anchor.as_ref(), anchor_index) {
                keyed_turns.retain(|(index, _)| {
                    if anchor.include_anchor {
                        *index <= anchor_index
                    } else {
                        *index < anchor_index
                    }
                });
            }
        }
    }

    let more_turns_available = keyed_turns.len() > page_size;
    keyed_turns.truncate(page_size);
    let backwards_cursor = keyed_turns
        .first()
        .map(|(_, turn)| serialize_chat_interactions_cursor(&turn.id, /*include_anchor*/ true))
        .transpose()?;
    let next_cursor = if more_turns_available {
        keyed_turns
            .last()
            .map(|(_, turn)| serialize_chat_interactions_cursor(&turn.id, /*include_anchor*/ false))
            .transpose()?
    } else {
        None
    };
    let interactions = keyed_turns.into_iter().map(|(_, turn)| turn).collect();

    Ok(ThreadTurnsPage {
        interactions,
        next_cursor,
        backwards_cursor,
    })
}

fn serialize_chat_interactions_cursor(
    interaction_id: &str,
    include_anchor: bool,
) -> Result<String, JSONRPCErrorError> {
    serde_json::to_string(&ThreadTurnsCursor {
        interaction_id: interaction_id.to_string(),
        include_anchor,
    })
    .map_err(|err| internal_error(format!("failed to serialize cursor: {err}")))
}

fn parse_chat_interactions_cursor(cursor: &str) -> Result<ThreadTurnsCursor, JSONRPCErrorError> {
    serde_json::from_str(cursor).map_err(|_| invalid_request(format!("invalid cursor: {cursor}")))
}

struct ThreadTurnsPageOptions<'a> {
    cursor: Option<&'a str>,
    limit: Option<u32>,
    sort_direction: SortDirection,
    messages_view: InteractionMessagesView,
}

fn build_chat_interactions_page_response(
    messages: &[RolloutMessage],
    loaded_status: ChatStatus,
    has_live_running_thread: bool,
    active_interaction: Option<Interaction>,
    options: ThreadTurnsPageOptions<'_>,
) -> Result<ChatInteractionsListResponse, JSONRPCErrorError> {
    let mut interactions = reconstruct_chat_interactions_for_interactions_list(
        messages,
        loaded_status,
        has_live_running_thread,
        active_interaction,
    );
    apply_chat_interactions_messages_view(&mut interactions, options.messages_view);
    let page = paginate_chat_interactions(
        interactions,
        options.cursor,
        options.limit,
        options.sort_direction,
    )?;
    Ok(ChatInteractionsListResponse {
        data: page.interactions,
        next_cursor: page.next_cursor,
        backwards_cursor: page.backwards_cursor,
    })
}

pub(super) fn build_chat_resume_initial_turns_page(
    messages: &[RolloutMessage],
    loaded_status: ChatStatus,
    has_live_running_thread: bool,
    active_interaction: Option<Interaction>,
    params: &ChatResumeInitialInteractionsPageParams,
) -> Result<datax_app_server_protocol::InteractionsPage, JSONRPCErrorError> {
    build_chat_interactions_page_response(
        messages,
        loaded_status,
        has_live_running_thread,
        active_interaction,
        ThreadTurnsPageOptions {
            cursor: None,
            limit: params.limit,
            sort_direction: params.sort_direction.unwrap_or(SortDirection::Desc),
            messages_view: params
                .messages_view
                .unwrap_or(InteractionMessagesView::Summary),
        },
    )
    .map(Into::into)
}

fn apply_chat_interactions_messages_view(
    interactions: &mut [Interaction],
    messages_view: InteractionMessagesView,
) {
    for turn in interactions {
        match messages_view {
            InteractionMessagesView::NotLoaded => {
                turn.items.clear();
                turn.messages_view = InteractionMessagesView::NotLoaded;
            }
            InteractionMessagesView::Summary => {
                let first_user_message = turn
                    .items
                    .iter()
                    .find(|item| matches!(item, Message::UserMessage { .. }))
                    .cloned();
                let final_agent_message = turn
                    .items
                    .iter()
                    .rev()
                    .find(|item| matches!(item, Message::AgentMessage { .. }))
                    .cloned();
                turn.items = match (first_user_message, final_agent_message) {
                    (Some(user_message), Some(agent_message))
                        if user_message.id() != agent_message.id() =>
                    {
                        vec![user_message, agent_message]
                    }
                    (Some(user_message), _) => vec![user_message],
                    (None, Some(agent_message)) => vec![agent_message],
                    (None, None) => Vec::new(),
                };
                turn.messages_view = InteractionMessagesView::Summary;
            }
            InteractionMessagesView::Full => {
                turn.messages_view = InteractionMessagesView::Full;
            }
        }
    }
}

fn reconstruct_chat_interactions_for_interactions_list(
    messages: &[RolloutMessage],
    loaded_status: ChatStatus,
    has_live_running_thread: bool,
    active_interaction: Option<Interaction>,
) -> Vec<Interaction> {
    let has_live_in_progress_turn = has_live_running_thread
        || active_interaction
            .as_ref()
            .is_some_and(|turn| matches!(turn.status, InteractionStatus::InProgress));
    let mut interactions = build_api_turns_from_rollout_items(messages);
    normalize_chat_interactions_status(&mut interactions, loaded_status, has_live_in_progress_turn);
    if let Some(active_interaction) = active_interaction {
        merge_interaction_history_with_active_interaction(&mut interactions, active_interaction);
    }
    interactions
}

fn normalize_chat_interactions_status(
    interactions: &mut [Interaction],
    loaded_status: ChatStatus,
    has_live_in_progress_turn: bool,
) {
    let status = resolve_chat_status(loaded_status, has_live_in_progress_turn);
    if matches!(status, ChatStatus::Active { .. }) {
        return;
    }
    for turn in interactions {
        if matches!(turn.status, InteractionStatus::InProgress) {
            turn.status = InteractionStatus::Interrupted;
        }
    }
}

enum ThreadReadViewError {
    InvalidRequest(String),
    Unsupported(&'static str),
    Internal(String),
}

fn thread_read_view_error(err: ThreadReadViewError) -> JSONRPCErrorError {
    match err {
        ThreadReadViewError::InvalidRequest(message) => invalid_request(message),
        ThreadReadViewError::Unsupported(operation) => {
            unsupported_chat_store_operation(operation)
        }
        ThreadReadViewError::Internal(message) => internal_error(message),
    }
}

pub(super) fn unsupported_chat_store_operation(operation: &'static str) -> JSONRPCErrorError {
    method_not_found(format!("{operation} is not supported yet"))
}

fn chat_store_list_error(err: ChatStoreError) -> JSONRPCErrorError {
    match err {
        ChatStoreError::InvalidRequest { message } => invalid_request(message),
        ChatStoreError::Unsupported { operation } => {
            unsupported_chat_store_operation(operation)
        }
        err => internal_error(format!("failed to list threads: {err}")),
    }
}

fn chat_store_resume_read_error(err: ChatStoreError) -> JSONRPCErrorError {
    match err {
        ChatStoreError::InvalidRequest { message } => invalid_request(message),
        ChatStoreError::Unsupported { operation } => {
            unsupported_chat_store_operation(operation)
        }
        ChatStoreError::ChatNotFound { chat_id: chat_id } => {
            invalid_request(format!("no rollout found for thread id {chat_id}"))
        }
        err => internal_error(format!("failed to read chat: {err}")),
    }
}

fn thread_interactions_list_history_load_error(
    chat_id: ChatId,
    err: ChatStoreError,
) -> ThreadReadViewError {
    match err {
        ChatStoreError::InvalidRequest { message }
            if message.starts_with("failed to resolve rollout path `") =>
        {
            ThreadReadViewError::InvalidRequest(format!(
                "thread {chat_id} is not materialized yet; chat/interactions/list is unavailable before first user message"
            ))
        }
        ChatStoreError::InvalidRequest { message } => {
            ThreadReadViewError::InvalidRequest(message)
        }
        ChatStoreError::Unsupported { operation } => ThreadReadViewError::Unsupported(operation),
        err => ThreadReadViewError::Internal(format!(
            "failed to load thread history for thread {chat_id}: {err}"
        )),
    }
}

fn thread_read_history_load_error(chat_id: ChatId, err: ChatStoreError) -> ThreadReadViewError {
    match err {
        ChatStoreError::InvalidRequest { message }
            if message.starts_with("failed to resolve rollout path `") =>
        {
            ThreadReadViewError::InvalidRequest(format!(
                "thread {chat_id} is not materialized yet; includeTurns is unavailable before first user message"
            ))
        }
        ChatStoreError::ChatNotFound {
            chat_id: missing_chat_id,
        } if missing_chat_id == chat_id => ThreadReadViewError::InvalidRequest(format!(
            "thread {chat_id} is not materialized yet; includeTurns is unavailable before first user message"
        )),
        ChatStoreError::InvalidRequest { message } => {
            ThreadReadViewError::InvalidRequest(message)
        }
        ChatStoreError::Unsupported { operation } => ThreadReadViewError::Unsupported(operation),
        err => ThreadReadViewError::Internal(format!(
            "failed to load thread history for thread {chat_id}: {err}"
        )),
    }
}

fn conversation_summary_chat_id_read_error(
    conversation_id: ChatId,
    err: ChatStoreError,
) -> JSONRPCErrorError {
    let no_rollout_message = format!("no rollout found for thread id {conversation_id}");
    match err {
        ChatStoreError::InvalidRequest { message } if message == no_rollout_message => {
            conversation_summary_not_found_error(conversation_id)
        }
        ChatStoreError::Unsupported { operation } => {
            unsupported_chat_store_operation(operation)
        }
        ChatStoreError::ChatNotFound { chat_id: chat_id } if chat_id == conversation_id => {
            conversation_summary_not_found_error(conversation_id)
        }
        ChatStoreError::InvalidRequest { message } => invalid_request(message),
        err => internal_error(format!(
            "failed to load conversation summary for {conversation_id}: {err}"
        )),
    }
}

fn conversation_summary_not_found_error(conversation_id: ChatId) -> JSONRPCErrorError {
    invalid_request(format!(
        "no rollout found for conversation id {conversation_id}"
    ))
}

fn conversation_summary_rollout_path_read_error(
    path: &Path,
    err: ChatStoreError,
) -> JSONRPCErrorError {
    match err {
        ChatStoreError::InvalidRequest { message } => invalid_request(message),
        ChatStoreError::Unsupported { operation } => {
            unsupported_chat_store_operation(operation)
        }
        err => internal_error(format!(
            "failed to load conversation summary from {}: {}",
            path.display(),
            err
        )),
    }
}

pub(super) fn core_thread_write_error(operation: &str, err: CodexErr) -> JSONRPCErrorError {
    match err {
        CodexErr::ThreadNotFound(chat_id) => {
            invalid_request(format!("thread not found: {chat_id}"))
        }
        CodexErr::InvalidRequest(message) => invalid_request(message),
        CodexErr::UnsupportedOperation(message) => method_not_found(message),
        err => internal_error(format!("failed to {operation}: {err}")),
    }
}

fn chat_store_archive_error(operation: &str, err: ChatStoreError) -> JSONRPCErrorError {
    match err {
        ChatStoreError::InvalidRequest { message } => invalid_request(message),
        ChatStoreError::Unsupported {
            operation: unsupported_operation,
        } => unsupported_chat_store_operation(unsupported_operation),
        err => internal_error(format!("failed to {operation} session: {err}")),
    }
}

fn set_thread_name_from_title(chat: &mut Chat, title: String) {
    if title.trim().is_empty() || thread.preview.trim() == title.trim() {
        return;
    }
    thread.name = Some(title);
}

pub(crate) fn chat_from_stored_chat(
    chat: StoredChat,
    fallback_provider: &str,
    fallback_cwd: &AbsolutePathBuf,
) -> (Chat, Option<datax_thread_store::StoredChatHistory>) {
    let path = thread.rollout_path;
    let git_info = thread.git_info.map(|info| ApiGitInfo {
        sha: info.commit_hash.map(|sha| sha.0),
        branch: info.branch,
        origin_url: info.repository_url,
    });
    let cwd = AbsolutePathBuf::relative_to_current_dir(path_utils::normalize_for_native_workdir(
        thread.cwd,
    ))
    .unwrap_or_else(|err| {
        warn!("failed to normalize thread cwd while reading stored chat: {err}");
        fallback_cwd.clone()
    });
    let source = with_thread_spawn_agent_metadata(
        thread.source,
        thread.agent_nickname.clone(),
        thread.agent_role.clone(),
    );
    let history = thread.history;
    let chat_id = thread.chat_id.to_string();
    let thread = Chat {
        id: chat_id.clone(),
        session_id: chat_id,
        forked_from_id: thread.forked_from_id.map(|id| id.to_string()),
        parent_chat_id: thread.parent_chat_id.map(|id| id.to_string()),
        preview: thread.preview,
        ephemeral: false,
        model_provider: if thread.model_provider.is_empty() {
            fallback_provider.to_string()
        } else {
            thread.model_provider
        },
        created_at: thread.created_at.timestamp(),
        updated_at: thread.updated_at.timestamp(),
        recency_at: Some(thread.recency_at.timestamp()),
        status: ChatStatus::NotLoaded,
        path,
        cwd,
        cli_version: thread.cli_version,
        agent_nickname: source.get_nickname(),
        agent_role: source.get_agent_role(),
        source: source.into(),
        chat_source: thread.chat_source.map(Into::into),
        git_info,
        name: thread.name,
        interactions: Vec::new(),
    };
    (thread, history)
}

fn summary_from_stored_chat(chat: StoredChat, fallback_provider: &str) -> ConversationSummary {
    let path = thread.rollout_path.unwrap_or_default();
    let source = with_thread_spawn_agent_metadata(
        thread.source,
        thread.agent_nickname.clone(),
        thread.agent_role.clone(),
    );
    let git_info = thread.git_info.map(|git| ConversationGitInfo {
        sha: git.commit_hash.map(|sha| sha.0),
        branch: git.branch,
        origin_url: git.repository_url,
    });
    ConversationSummary {
        conversation_id: thread.chat_id,
        path,
        preview: thread.preview,
        // Preserve millisecond precision from the chat store so chat/list cursors
        // round-trip the same ordering key used by pagination queries.
        timestamp: Some(
            thread
                .created_at
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        ),
        updated_at: Some(
            thread
                .updated_at
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        ),
        model_provider: if thread.model_provider.is_empty() {
            fallback_provider.to_string()
        } else {
            thread.model_provider
        },
        cwd: thread.cwd,
        cli_version: thread.cli_version,
        source,
        git_info,
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
fn summary_from_state_db_metadata(
    conversation_id: ChatId,
    path: PathBuf,
    first_user_message: Option<String>,
    preview: Option<String>,
    timestamp: String,
    updated_at: String,
    model_provider: String,
    cwd: PathBuf,
    cli_version: String,
    source: String,
    _thread_source: Option<datax_protocol::protocol::ThreadSource>,
    agent_nickname: Option<String>,
    agent_role: Option<String>,
    git_sha: Option<String>,
    git_branch: Option<String>,
    git_origin_url: Option<String>,
) -> ConversationSummary {
    let preview = preview.or(first_user_message).unwrap_or_default();
    let source = serde_json::from_str(&source)
        .or_else(|_| serde_json::from_value(serde_json::Value::String(source.clone())))
        .unwrap_or(datax_protocol::protocol::SessionSource::Unknown);
    let source = with_thread_spawn_agent_metadata(source, agent_nickname, agent_role);
    let git_info = if git_sha.is_none() && git_branch.is_none() && git_origin_url.is_none() {
        None
    } else {
        Some(ConversationGitInfo {
            sha: git_sha,
            branch: git_branch,
            origin_url: git_origin_url,
        })
    };
    ConversationSummary {
        conversation_id,
        path,
        preview,
        timestamp: Some(timestamp),
        updated_at: Some(updated_at),
        model_provider,
        cwd,
        cli_version,
        source,
        git_info,
    }
}

#[cfg(test)]
fn summary_from_chat_metadata(metadata: &ThreadMetadata) -> ConversationSummary {
    summary_from_state_db_metadata(
        metadata.id,
        metadata.rollout_path.clone(),
        metadata.first_user_message.clone(),
        metadata.preview.clone(),
        metadata
            .created_at
            .to_rfc3339_opts(SecondsFormat::Secs, true),
        metadata
            .updated_at
            .to_rfc3339_opts(SecondsFormat::Secs, true),
        metadata.model_provider.clone(),
        metadata.cwd.clone(),
        metadata.cli_version.clone(),
        metadata.source.clone(),
        metadata.thread_source.clone(),
        metadata.agent_nickname.clone(),
        metadata.agent_role.clone(),
        metadata.git_sha.clone(),
        metadata.git_branch.clone(),
        metadata.git_origin_url.clone(),
    )
}

fn preview_from_rollout_items(messages: &[RolloutMessage]) -> String {
    messages
        .iter()
        .find_map(|item| match item {
            RolloutMessage::ResponseItem(item) => match datax_core::parse_turn_item(item) {
                Some(datax_protocol::items::InteractionMessage::UserMessage(user)) => Some(user.message()),
                _ => None,
            },
            _ => None,
        })
        .map(|preview| match preview.find(USER_MESSAGE_BEGIN) {
            Some(idx) => preview[idx + USER_MESSAGE_BEGIN.len()..].trim().to_string(),
            None => preview,
        })
        .unwrap_or_default()
}

fn requested_permissions_trust_project(overrides: &ConfigOverrides, cwd: &Path) -> bool {
    if matches!(
        overrides.sandbox_mode,
        Some(
            datax_protocol::config_types::SandboxMode::WorkspaceWrite
                | datax_protocol::config_types::SandboxMode::DangerFullAccess
        )
    ) {
        return true;
    }

    if matches!(
        overrides.default_permissions.as_deref(),
        Some(
            BUILT_IN_PERMISSION_PROFILE_WORKSPACE | BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS
        )
    ) {
        return true;
    }

    overrides
        .permission_profile
        .as_ref()
        .is_some_and(|profile| permission_profile_trusts_project(profile, cwd))
}

fn permission_profile_trusts_project(
    profile: &datax_protocol::models::PermissionProfile,
    cwd: &Path,
) -> bool {
    match profile {
        datax_protocol::models::PermissionProfile::Disabled
        | datax_protocol::models::PermissionProfile::External { .. } => true,
        datax_protocol::models::PermissionProfile::Managed { .. } => profile
            .file_system_sandbox_policy()
            .can_write_path_with_cwd(cwd, cwd),
    }
}

fn build_thread_from_snapshot(
    chat_id: ChatId,
    session_id: String,
    config_snapshot: &ThreadConfigSnapshot,
    path: Option<PathBuf>,
) -> Chat {
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    Chat {
        id: chat_id.to_string(),
        session_id,
        forked_from_id: None,
        parent_chat_id: config_snapshot.parent_chat_id.map(|id| id.to_string()),
        preview: String::new(),
        ephemeral: config_snapshot.ephemeral,
        model_provider: config_snapshot.model_provider_id.clone(),
        created_at: now,
        updated_at: now,
        recency_at: Some(now),
        status: ChatStatus::NotLoaded,
        path,
        cwd: config_snapshot.cwd().clone(),
        cli_version: env!("CARGO_PKG_VERSION").to_string(),
        agent_nickname: config_snapshot.session_source.get_nickname(),
        agent_role: config_snapshot.session_source.get_agent_role(),
        source: config_snapshot.session_source.clone().into(),
        chat_source: config_snapshot.thread_source.clone().map(Into::into),
        git_info: None,
        name: None,
        interactions: Vec::new(),
    }
}

fn paginate_background_terminals(
    terminals: &[ChatBackgroundTerminal],
    cursor: Option<String>,
    limit: Option<u32>,
) -> Result<(Vec<ChatBackgroundTerminal>, Option<String>), JSONRPCErrorError> {
    let start = match cursor {
        Some(cursor) => {
            let cursor = cursor
                .parse::<i32>()
                .map_err(|err| invalid_request(format!("invalid cursor: {err}")))?;
            terminals
                .iter()
                .position(|terminal| {
                    terminal
                        .process_id
                        .parse::<i32>()
                        .is_ok_and(|process_id| process_id > cursor)
                })
                .unwrap_or(terminals.len())
        }
        None => 0,
    };
    let effective_limit = limit.unwrap_or(terminals.len() as u32).max(1) as usize;
    let end = start.saturating_add(effective_limit).min(terminals.len());
    let next_cursor = (end < terminals.len()).then(|| terminals[end - 1].process_id.clone());
    Ok((terminals[start..end].to_vec(), next_cursor))
}

fn build_thread_from_loaded_snapshot(
    chat_id: ChatId,
    config_snapshot: &ThreadConfigSnapshot,
    loaded_thread: &DataxChat,
) -> Chat {
    build_thread_from_snapshot(
        chat_id,
        loaded_thread.session_configured().session_id.to_string(),
        config_snapshot,
        loaded_thread.rollout_path(),
    )
}

#[cfg(test)]
#[path = "chat_processor_tests.rs"]
mod chat_processor_tests;
