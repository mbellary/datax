use crate::app_command::AppCommand;
use datax_app_server_protocol::Message;
use datax_app_server_protocol::RequestId as AppServerRequestId;
use datax_app_server_protocol::ServerNotification;
use datax_app_server_protocol::ServerRequest;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ElicitationRequestKey {
    server_name: String,
    request_id: AppServerRequestId,
}

impl ElicitationRequestKey {
    fn new(server_name: String, request_id: AppServerRequestId) -> Self {
        Self {
            server_name,
            request_id,
        }
    }
}

#[derive(Debug, Default)]
// Tracks which interactive prompts are still unresolved in the thread-event buffer.
//
// Chat snapshots are replayed when switching threads/agents. Most events should replay
// verbatim, but interactive prompts (approvals, request_user_input, MCP elicitations) must
// only replay if they are still pending. This state is updated from:
// - inbound events (`note_event`)
// - outbound ops that resolve a prompt (`note_outbound_op`)
// - buffer eviction (`note_evicted_event`)
//
// We keep both fast lookup sets (for snapshot filtering by call_id/request key) and
// turn-indexed queues/vectors so turn completion or interruption can clear
// stale prompts tied to a turn. `request_user_input` removal is FIFO because
// the overlay answers queued prompts in FIFO order for a shared `interaction_id`.
pub(super) struct PendingInteractiveReplayState {
    exec_approval_call_ids: HashSet<String>,
    exec_approval_call_ids_by_interaction_id: HashMap<String, Vec<String>>,
    patch_approval_call_ids: HashSet<String>,
    patch_approval_call_ids_by_interaction_id: HashMap<String, Vec<String>>,
    elicitation_requests: HashSet<ElicitationRequestKey>,
    request_permissions_call_ids: HashSet<String>,
    request_permissions_call_ids_by_interaction_id: HashMap<String, Vec<String>>,
    request_user_input_call_ids: HashSet<String>,
    request_user_input_call_ids_by_interaction_id: HashMap<String, Vec<String>>,
    pending_requests_by_request_id: HashMap<AppServerRequestId, PendingInteractiveRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingInteractiveRequest {
    ExecApproval {
        interaction_id: String,
        approval_id: String,
    },
    PatchApproval {
        interaction_id: String,
        item_id: String,
    },
    Elicitation(ElicitationRequestKey),
    RequestPermissions {
        interaction_id: String,
        item_id: String,
    },
    RequestUserInput {
        interaction_id: String,
        item_id: String,
    },
}

impl PendingInteractiveReplayState {
    pub(super) fn op_can_change_state<T>(op: T) -> bool
    where
        T: Into<AppCommand>,
    {
        let op: AppCommand = op.into();
        matches!(
            &op,
            AppCommand::ExecApproval { .. }
                | AppCommand::PatchApproval { .. }
                | AppCommand::ResolveElicitation { .. }
                | AppCommand::RequestPermissionsResponse { .. }
                | AppCommand::UserInputAnswer { .. }
                | AppCommand::Shutdown
        )
    }

    pub(super) fn note_outbound_op<T>(&mut self, op: T)
    where
        T: Into<AppCommand>,
    {
        let op: AppCommand = op.into();
        match &op {
            AppCommand::ExecApproval { id, interaction_id, .. } => {
                self.exec_approval_call_ids.remove(id);
                if let Some(interaction_id) = interaction_id {
                    Self::remove_call_id_from_turn_map_entry(
                        &mut self.exec_approval_call_ids_by_interaction_id,
                        interaction_id,
                        id,
                    );
                }
                self.pending_requests_by_request_id
                    .retain(|_, pending| !matches!(pending, PendingInteractiveRequest::ExecApproval { approval_id, .. } if approval_id == id));
            }
            AppCommand::PatchApproval { id, .. } => {
                self.patch_approval_call_ids.remove(id);
                Self::remove_call_id_from_turn_map(
                    &mut self.patch_approval_call_ids_by_interaction_id,
                    id,
                );
                self.pending_requests_by_request_id
                    .retain(|_, pending| !matches!(pending, PendingInteractiveRequest::PatchApproval { item_id, .. } if item_id == id));
            }
            AppCommand::ResolveElicitation {
                server_name,
                request_id,
                ..
            } => {
                self.elicitation_requests
                    .remove(&ElicitationRequestKey::new(
                        server_name.to_string(),
                        request_id.clone(),
                    ));
                self.pending_requests_by_request_id.retain(
                    |_, pending| {
                        !matches!(pending, PendingInteractiveRequest::Elicitation(key) if key.server_name == *server_name && key.request_id == *request_id)
                    },
                );
            }
            AppCommand::RequestPermissionsResponse { id, .. } => {
                self.request_permissions_call_ids.remove(id);
                Self::remove_call_id_from_turn_map(
                    &mut self.request_permissions_call_ids_by_interaction_id,
                    id,
                );
                self.pending_requests_by_request_id.retain(
                    |_, pending| {
                        !matches!(pending, PendingInteractiveRequest::RequestPermissions { item_id, .. } if item_id == id)
                    },
                );
            }
            // `Op::UserInputAnswer` identifies the turn, not the prompt call_id. The UI
            // answers queued prompts for the same turn in FIFO order, so remove the oldest
            // queued call_id for that turn.
            AppCommand::UserInputAnswer { id, .. } => {
                let mut remove_turn_entry = false;
                if let Some(call_ids) = self.request_user_input_call_ids_by_interaction_id.get_mut(id) {
                    if !call_ids.is_empty() {
                        let call_id = call_ids.remove(0);
                        self.request_user_input_call_ids.remove(&call_id);
                        self.pending_requests_by_request_id.retain(
                            |_, pending| {
                                !matches!(pending, PendingInteractiveRequest::RequestUserInput { item_id, .. } if *item_id == call_id)
                            },
                        );
                    }
                    if call_ids.is_empty() {
                        remove_turn_entry = true;
                    }
                }
                if remove_turn_entry {
                    self.request_user_input_call_ids_by_interaction_id.remove(id);
                }
            }
            AppCommand::Shutdown => self.clear(),
            _ => {}
        }
    }

    pub(super) fn note_server_request(&mut self, request: &ServerRequest) {
        match request {
            ServerRequest::CommandExecutionRequestApproval { request_id, params } => {
                let approval_id = params
                    .approval_id
                    .clone()
                    .unwrap_or_else(|| params.message_id.clone());
                self.exec_approval_call_ids.insert(approval_id.clone());
                self.exec_approval_call_ids_by_interaction_id
                    .entry(params.interaction_id.clone())
                    .or_default()
                    .push(approval_id);
                self.pending_requests_by_request_id.insert(
                    request_id.clone(),
                    PendingInteractiveRequest::ExecApproval {
                        interaction_id: params.interaction_id.clone(),
                        approval_id: params
                            .approval_id
                            .clone()
                            .unwrap_or_else(|| params.message_id.clone()),
                    },
                );
            }
            ServerRequest::FileChangeRequestApproval { request_id, params } => {
                self.patch_approval_call_ids
                    .insert(params.message_id.clone());
                self.patch_approval_call_ids_by_interaction_id
                    .entry(params.interaction_id.clone())
                    .or_default()
                    .push(params.message_id.clone());
                self.pending_requests_by_request_id.insert(
                    request_id.clone(),
                    PendingInteractiveRequest::PatchApproval {
                        interaction_id: params.interaction_id.clone(),
                        item_id: params.message_id.clone(),
                    },
                );
            }
            ServerRequest::McpServerElicitationRequest { request_id, params } => {
                let key =
                    ElicitationRequestKey::new(params.server_name.clone(), request_id.clone());
                self.elicitation_requests.insert(key.clone());
                self.pending_requests_by_request_id.insert(
                    request_id.clone(),
                    PendingInteractiveRequest::Elicitation(key),
                );
            }
            ServerRequest::ToolRequestUserInput { request_id, params } => {
                self.request_user_input_call_ids
                    .insert(params.message_id.clone());
                self.request_user_input_call_ids_by_interaction_id
                    .entry(params.interaction_id.clone())
                    .or_default()
                    .push(params.message_id.clone());
                self.pending_requests_by_request_id.insert(
                    request_id.clone(),
                    PendingInteractiveRequest::RequestUserInput {
                        interaction_id: params.interaction_id.clone(),
                        item_id: params.message_id.clone(),
                    },
                );
            }
            ServerRequest::PermissionsRequestApproval { request_id, params } => {
                self.request_permissions_call_ids
                    .insert(params.message_id.clone());
                self.request_permissions_call_ids_by_interaction_id
                    .entry(params.interaction_id.clone())
                    .or_default()
                    .push(params.message_id.clone());
                self.pending_requests_by_request_id.insert(
                    request_id.clone(),
                    PendingInteractiveRequest::RequestPermissions {
                        interaction_id: params.interaction_id.clone(),
                        item_id: params.message_id.clone(),
                    },
                );
            }
            _ => {}
        }
    }

    pub(super) fn note_server_notification(&mut self, notification: &ServerNotification) {
        match notification {
            ServerNotification::MessageStarted(notification) => match &notification.item {
                Message::CommandExecution { id, .. } => {
                    self.exec_approval_call_ids.remove(id);
                    Self::remove_call_id_from_turn_map(
                        &mut self.exec_approval_call_ids_by_interaction_id,
                        id,
                    );
                }
                Message::FileChange { id, .. } => {
                    self.patch_approval_call_ids.remove(id);
                    Self::remove_call_id_from_turn_map(
                        &mut self.patch_approval_call_ids_by_interaction_id,
                        id,
                    );
                }
                _ => {}
            },
            ServerNotification::InteractionCompleted(notification) => {
                self.clear_exec_approval_turn(&notification.interaction.id);
                self.clear_patch_approval_turn(&notification.interaction.id);
                self.clear_request_permissions_turn(&notification.interaction.id);
                self.clear_request_user_input_turn(&notification.interaction.id);
            }
            ServerNotification::ServerRequestResolved(notification) => {
                self.remove_request(&notification.request_id);
            }
            ServerNotification::ChatClosed(_) => self.clear(),
            _ => {}
        }
    }

    pub(super) fn note_evicted_server_request(&mut self, request: &ServerRequest) {
        match request {
            ServerRequest::CommandExecutionRequestApproval { params, .. } => {
                let approval_id = params
                    .approval_id
                    .clone()
                    .unwrap_or_else(|| params.message_id.clone());
                self.exec_approval_call_ids.remove(&approval_id);
                Self::remove_call_id_from_turn_map_entry(
                    &mut self.exec_approval_call_ids_by_interaction_id,
                    &params.interaction_id,
                    &approval_id,
                );
            }
            ServerRequest::FileChangeRequestApproval { params, .. } => {
                self.patch_approval_call_ids.remove(&params.message_id);
                Self::remove_call_id_from_turn_map_entry(
                    &mut self.patch_approval_call_ids_by_interaction_id,
                    &params.interaction_id,
                    &params.message_id,
                );
            }
            ServerRequest::McpServerElicitationRequest { request_id, params } => {
                self.elicitation_requests
                    .remove(&ElicitationRequestKey::new(
                        params.server_name.clone(),
                        request_id.clone(),
                    ));
            }
            ServerRequest::ToolRequestUserInput { params, .. } => {
                self.request_user_input_call_ids.remove(&params.message_id);
                let mut remove_turn_entry = false;
                if let Some(call_ids) = self
                    .request_user_input_call_ids_by_interaction_id
                    .get_mut(&params.interaction_id)
                {
                    call_ids.retain(|call_id| call_id != &params.message_id);
                    if call_ids.is_empty() {
                        remove_turn_entry = true;
                    }
                }
                if remove_turn_entry {
                    self.request_user_input_call_ids_by_interaction_id
                        .remove(&params.interaction_id);
                }
            }
            ServerRequest::PermissionsRequestApproval { params, .. } => {
                self.request_permissions_call_ids.remove(&params.message_id);
                let mut remove_turn_entry = false;
                if let Some(call_ids) = self
                    .request_permissions_call_ids_by_interaction_id
                    .get_mut(&params.interaction_id)
                {
                    call_ids.retain(|call_id| call_id != &params.message_id);
                    if call_ids.is_empty() {
                        remove_turn_entry = true;
                    }
                }
                if remove_turn_entry {
                    self.request_permissions_call_ids_by_interaction_id
                        .remove(&params.interaction_id);
                }
            }
            _ => {}
        }
        self.pending_requests_by_request_id
            .retain(|_, pending| !Self::request_matches_server_request(pending, request));
    }

    pub(super) fn should_replay_snapshot_request(&self, request: &ServerRequest) -> bool {
        match request {
            ServerRequest::CommandExecutionRequestApproval { params, .. } => self
                .exec_approval_call_ids
                .contains(params.approval_id.as_ref().unwrap_or(&params.message_id)),
            ServerRequest::FileChangeRequestApproval { params, .. } => {
                self.patch_approval_call_ids.contains(&params.message_id)
            }
            ServerRequest::McpServerElicitationRequest { request_id, params } => self
                .elicitation_requests
                .contains(&ElicitationRequestKey::new(
                    params.server_name.clone(),
                    request_id.clone(),
                )),
            ServerRequest::ToolRequestUserInput { params, .. } => self
                .request_user_input_call_ids
                .contains(&params.message_id),
            ServerRequest::PermissionsRequestApproval { params, .. } => self
                .request_permissions_call_ids
                .contains(&params.message_id),
            _ => true,
        }
    }

    pub(super) fn has_pending_thread_approvals(&self) -> bool {
        !self.exec_approval_call_ids.is_empty()
            || !self.patch_approval_call_ids.is_empty()
            || !self.elicitation_requests.is_empty()
            || !self.request_permissions_call_ids.is_empty()
    }

    pub(super) fn has_pending_thread_user_input(&self) -> bool {
        !self.request_user_input_call_ids.is_empty()
    }

    fn clear_request_user_input_turn(&mut self, interaction_id: &str) {
        if let Some(call_ids) = self.request_user_input_call_ids_by_interaction_id.remove(interaction_id) {
            for call_id in call_ids {
                self.request_user_input_call_ids.remove(&call_id);
            }
        }
        self.pending_requests_by_request_id.retain(
            |_, pending| {
                !matches!(pending, PendingInteractiveRequest::RequestUserInput { interaction_id: pending_interaction_id, .. } if pending_interaction_id == interaction_id)
            },
        );
    }

    fn clear_request_permissions_turn(&mut self, interaction_id: &str) {
        if let Some(call_ids) = self.request_permissions_call_ids_by_interaction_id.remove(interaction_id) {
            for call_id in call_ids {
                self.request_permissions_call_ids.remove(&call_id);
            }
        }
        self.pending_requests_by_request_id.retain(
            |_, pending| {
                !matches!(pending, PendingInteractiveRequest::RequestPermissions { interaction_id: pending_interaction_id, .. } if pending_interaction_id == interaction_id)
            },
        );
    }

    fn clear_exec_approval_turn(&mut self, interaction_id: &str) {
        if let Some(call_ids) = self.exec_approval_call_ids_by_interaction_id.remove(interaction_id) {
            for call_id in call_ids {
                self.exec_approval_call_ids.remove(&call_id);
            }
        }
        self.pending_requests_by_request_id.retain(
            |_, pending| {
                !matches!(pending, PendingInteractiveRequest::ExecApproval { interaction_id: pending_interaction_id, .. } if pending_interaction_id == interaction_id)
            },
        );
    }

    fn clear_patch_approval_turn(&mut self, interaction_id: &str) {
        if let Some(call_ids) = self.patch_approval_call_ids_by_interaction_id.remove(interaction_id) {
            for call_id in call_ids {
                self.patch_approval_call_ids.remove(&call_id);
            }
        }
        self.pending_requests_by_request_id.retain(
            |_, pending| {
                !matches!(pending, PendingInteractiveRequest::PatchApproval { interaction_id: pending_interaction_id, .. } if pending_interaction_id == interaction_id)
            },
        );
    }

    fn remove_call_id_from_turn_map(
        call_ids_by_interaction_id: &mut HashMap<String, Vec<String>>,
        call_id: &str,
    ) {
        call_ids_by_interaction_id.retain(|_, call_ids| {
            call_ids.retain(|queued_call_id| queued_call_id != call_id);
            !call_ids.is_empty()
        });
    }

    fn remove_call_id_from_turn_map_entry(
        call_ids_by_interaction_id: &mut HashMap<String, Vec<String>>,
        interaction_id: &str,
        call_id: &str,
    ) {
        let mut remove_turn_entry = false;
        if let Some(call_ids) = call_ids_by_interaction_id.get_mut(interaction_id) {
            call_ids.retain(|queued_call_id| queued_call_id != call_id);
            if call_ids.is_empty() {
                remove_turn_entry = true;
            }
        }
        if remove_turn_entry {
            call_ids_by_interaction_id.remove(interaction_id);
        }
    }

    fn clear(&mut self) {
        self.exec_approval_call_ids.clear();
        self.exec_approval_call_ids_by_interaction_id.clear();
        self.patch_approval_call_ids.clear();
        self.patch_approval_call_ids_by_interaction_id.clear();
        self.elicitation_requests.clear();
        self.request_permissions_call_ids.clear();
        self.request_permissions_call_ids_by_interaction_id.clear();
        self.request_user_input_call_ids.clear();
        self.request_user_input_call_ids_by_interaction_id.clear();
        self.pending_requests_by_request_id.clear();
    }

    fn remove_request(&mut self, request_id: &AppServerRequestId) {
        let Some(pending) = self.pending_requests_by_request_id.remove(request_id) else {
            return;
        };
        match pending {
            PendingInteractiveRequest::ExecApproval {
                interaction_id,
                approval_id,
            } => {
                self.exec_approval_call_ids.remove(&approval_id);
                Self::remove_call_id_from_turn_map_entry(
                    &mut self.exec_approval_call_ids_by_interaction_id,
                    &interaction_id,
                    &approval_id,
                );
            }
            PendingInteractiveRequest::PatchApproval { interaction_id, item_id } => {
                self.patch_approval_call_ids.remove(&item_id);
                Self::remove_call_id_from_turn_map_entry(
                    &mut self.patch_approval_call_ids_by_interaction_id,
                    &interaction_id,
                    &item_id,
                );
            }
            PendingInteractiveRequest::Elicitation(key) => {
                self.elicitation_requests.remove(&key);
            }
            PendingInteractiveRequest::RequestPermissions { interaction_id, item_id } => {
                self.request_permissions_call_ids.remove(&item_id);
                Self::remove_call_id_from_turn_map_entry(
                    &mut self.request_permissions_call_ids_by_interaction_id,
                    &interaction_id,
                    &item_id,
                );
            }
            PendingInteractiveRequest::RequestUserInput { interaction_id, item_id } => {
                self.request_user_input_call_ids.remove(&item_id);
                Self::remove_call_id_from_turn_map_entry(
                    &mut self.request_user_input_call_ids_by_interaction_id,
                    &interaction_id,
                    &item_id,
                );
            }
        }
    }

    fn request_matches_server_request(
        pending: &PendingInteractiveRequest,
        request: &ServerRequest,
    ) -> bool {
        match (pending, request) {
            (
                PendingInteractiveRequest::ExecApproval {
                    interaction_id,
                    approval_id,
                },
                ServerRequest::CommandExecutionRequestApproval { params, .. },
            ) => {
                interaction_id == &params.interaction_id
                    && approval_id == params.approval_id.as_ref().unwrap_or(&params.message_id)
            }
            (
                PendingInteractiveRequest::PatchApproval { interaction_id, item_id },
                ServerRequest::FileChangeRequestApproval { params, .. },
            ) => interaction_id == &params.interaction_id && item_id == &params.message_id,
            (
                PendingInteractiveRequest::Elicitation(key),
                ServerRequest::McpServerElicitationRequest { request_id, params },
            ) => key.server_name == params.server_name && key.request_id == *request_id,
            (
                PendingInteractiveRequest::RequestPermissions { interaction_id, item_id },
                ServerRequest::PermissionsRequestApproval { params, .. },
            ) => interaction_id == &params.interaction_id && item_id == &params.message_id,
            (
                PendingInteractiveRequest::RequestUserInput { interaction_id, item_id },
                ServerRequest::ToolRequestUserInput { params, .. },
            ) => interaction_id == &params.interaction_id && item_id == &params.message_id,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::ThreadBufferedEvent;
    use super::super::ThreadEventStore;
    use crate::app_command::AppCommand as Op;
    use datax_app_server_protocol::ChatClosedNotification;
    use datax_app_server_protocol::CommandExecutionApprovalDecision;
    use datax_app_server_protocol::CommandExecutionRequestApprovalParams;
    use datax_app_server_protocol::FileChangeRequestApprovalParams;
    use datax_app_server_protocol::Interaction;
    use datax_app_server_protocol::InteractionCompletedNotification;
    use datax_app_server_protocol::InteractionStatus;
    use datax_app_server_protocol::McpElicitationObjectType;
    use datax_app_server_protocol::McpElicitationSchema;
    use datax_app_server_protocol::McpServerElicitationAction;
    use datax_app_server_protocol::McpServerElicitationRequest;
    use datax_app_server_protocol::McpServerElicitationRequestParams;
    use datax_app_server_protocol::RequestId as AppServerRequestId;
    use datax_app_server_protocol::ServerNotification;
    use datax_app_server_protocol::ServerRequest;
    use datax_app_server_protocol::ServerRequestResolvedNotification;
    use datax_app_server_protocol::ToolRequestUserInputParams;
    use datax_app_server_protocol::ToolRequestUserInputResponse;
    use datax_utils_absolute_path::test_support::PathBufExt;
    use datax_utils_absolute_path::test_support::test_path_buf;
    use pretty_assertions::assert_eq;
    use std::collections::BTreeMap;
    use std::collections::HashMap;

    fn request_user_input_request(call_id: &str, interaction_id: &str) -> ServerRequest {
        ServerRequest::ToolRequestUserInput {
            request_id: AppServerRequestId::Integer(1),
            params: ToolRequestUserInputParams {
                chat_id: "thread-1".to_string(),
                interaction_id: interaction_id.to_string(),
                message_id: call_id.to_string(),
                questions: Vec::new(),
                auto_resolution_ms: None,
            },
        }
    }

    fn exec_approval_request(
        call_id: &str,
        approval_id: Option<&str>,
        interaction_id: &str,
    ) -> ServerRequest {
        ServerRequest::CommandExecutionRequestApproval {
            request_id: AppServerRequestId::Integer(2),
            params: CommandExecutionRequestApprovalParams {
                chat_id: "thread-1".to_string(),
                interaction_id: interaction_id.to_string(),
                message_id: call_id.to_string(),
                started_at_ms: 0,
                approval_id: approval_id.map(str::to_string),
                environment_id: None,
                reason: None,
                network_approval_context: None,
                command: Some("echo hi".to_string()),
                cwd: Some(test_path_buf("/tmp").abs().into()),
                command_actions: None,
                additional_permissions: None,
                proposed_execpolicy_amendment: None,
                proposed_network_policy_amendments: None,
                available_decisions: None,
            },
        }
    }

    fn patch_approval_request(call_id: &str, interaction_id: &str) -> ServerRequest {
        ServerRequest::FileChangeRequestApproval {
            request_id: AppServerRequestId::Integer(3),
            params: FileChangeRequestApprovalParams {
                chat_id: "thread-1".to_string(),
                interaction_id: interaction_id.to_string(),
                message_id: call_id.to_string(),
                started_at_ms: 0,
                reason: None,
                grant_root: None,
            },
        }
    }

    fn elicitation_request(
        server_name: &str,
        request_id: &str,
        interaction_id: &str,
    ) -> ServerRequest {
        ServerRequest::McpServerElicitationRequest {
            request_id: AppServerRequestId::String(request_id.to_string()),
            params: McpServerElicitationRequestParams {
                chat_id: "thread-1".to_string(),
                interaction_id: Some(interaction_id.to_string()),
                server_name: server_name.to_string(),
                request: McpServerElicitationRequest::Form {
                    meta: None,
                    message: "Please confirm".to_string(),
                    requested_schema: McpElicitationSchema {
                        schema_uri: None,
                        type_: McpElicitationObjectType::Object,
                        properties: BTreeMap::new(),
                        required: None,
                    },
                },
            },
        }
    }

    fn turn_completed(interaction_id: &str) -> ServerNotification {
        ServerNotification::InteractionCompleted(InteractionCompletedNotification {
            chat_id: "thread-1".to_string(),
            interaction: Interaction {
                id: interaction_id.to_string(),
                messages_view: datax_app_server_protocol::InteractionMessagesView::Full,
                messages: Vec::new(),
                status: InteractionStatus::Completed,
                error: None,
                started_at: None,
                completed_at: Some(0),
                duration_ms: Some(1),
            },
        })
    }

    fn thread_closed() -> ServerNotification {
        ServerNotification::ChatClosed(ChatClosedNotification {
            chat_id: "thread-1".to_string(),
        })
    }

    fn request_resolved(request_id: AppServerRequestId) -> ServerNotification {
        ServerNotification::ServerRequestResolved(ServerRequestResolvedNotification {
            chat_id: "thread-1".to_string(),
            request_id,
        })
    }

    #[test]
    fn thread_event_snapshot_keeps_pending_request_user_input() {
        let mut store = ThreadEventStore::new(/*capacity*/ 8);
        let request = request_user_input_request("call-1", "turn-1");

        store.push_request(request);

        let snapshot = store.snapshot();
        assert_eq!(snapshot.events.len(), 1);
        assert!(matches!(
            snapshot.events.first(),
            Some(ThreadBufferedEvent::Request(ServerRequest::ToolRequestUserInput { params, .. }))
                if params.message_id == "call-1"
        ));
    }

    #[test]
    fn thread_event_snapshot_drops_resolved_request_user_input_after_user_answer() {
        let mut store = ThreadEventStore::new(/*capacity*/ 8);
        store.push_request(request_user_input_request("call-1", "turn-1"));

        store.note_outbound_op(&Op::UserInputAnswer {
            id: "turn-1".to_string(),
            response: ToolRequestUserInputResponse {
                answers: HashMap::new(),
            },
        });

        let snapshot = store.snapshot();
        assert!(
            snapshot.events.is_empty(),
            "resolved request_user_input prompt should not replay on thread switch"
        );
    }

    #[test]
    fn thread_event_snapshot_drops_resolved_request_user_input_after_server_resolution() {
        let mut store = ThreadEventStore::new(/*capacity*/ 8);
        store.push_request(request_user_input_request("call-1", "turn-1"));

        store.push_notification(request_resolved(AppServerRequestId::Integer(1)));

        let snapshot = store.snapshot();
        assert!(
            snapshot.events.iter().all(|event| {
                !matches!(
                    event,
                    ThreadBufferedEvent::Request(ServerRequest::ToolRequestUserInput { .. })
                )
            }),
            "server-resolved request_user_input prompt should not replay on thread switch"
        );
    }

    #[test]
    fn thread_event_snapshot_drops_resolved_exec_approval_after_outbound_approval_id() {
        let mut store = ThreadEventStore::new(/*capacity*/ 8);
        store.push_request(exec_approval_request(
            "call-1",
            Some("approval-1"),
            "turn-1",
        ));

        store.note_outbound_op(&Op::ExecApproval {
            id: "approval-1".to_string(),
            interaction_id: Some("turn-1".to_string()),
            decision: CommandExecutionApprovalDecision::Accept,
        });

        let snapshot = store.snapshot();
        assert!(
            snapshot.events.is_empty(),
            "resolved exec approval prompt should not replay on thread switch"
        );
    }

    #[test]
    fn thread_event_snapshot_drops_resolved_exec_approval_after_server_resolution() {
        let mut store = ThreadEventStore::new(/*capacity*/ 8);
        store.push_request(exec_approval_request(
            "call-1",
            Some("approval-1"),
            "turn-1",
        ));

        store.push_notification(request_resolved(AppServerRequestId::Integer(2)));

        let snapshot = store.snapshot();
        assert!(
            snapshot.events.iter().all(|event| {
                !matches!(
                    event,
                    ThreadBufferedEvent::Request(
                        ServerRequest::CommandExecutionRequestApproval { .. }
                    )
                )
            }),
            "server-resolved exec approval prompt should not replay on thread switch"
        );
    }

    #[test]
    fn thread_event_snapshot_drops_answered_request_user_input_for_multi_prompt_turn() {
        let mut store = ThreadEventStore::new(/*capacity*/ 8);
        store.push_request(request_user_input_request("call-1", "turn-1"));

        store.note_outbound_op(&Op::UserInputAnswer {
            id: "turn-1".to_string(),
            response: ToolRequestUserInputResponse {
                answers: HashMap::new(),
            },
        });

        store.push_request(request_user_input_request("call-2", "turn-1"));

        let snapshot = store.snapshot();
        assert_eq!(snapshot.events.len(), 1);
        assert!(matches!(
            snapshot.events.first(),
            Some(ThreadBufferedEvent::Request(ServerRequest::ToolRequestUserInput { params, .. }))
                if params.message_id == "call-2"
        ));
    }

    #[test]
    fn thread_event_snapshot_keeps_newer_request_user_input_pending_when_same_turn_has_queue() {
        let mut store = ThreadEventStore::new(/*capacity*/ 8);
        store.push_request(request_user_input_request("call-1", "turn-1"));
        store.push_request(request_user_input_request("call-2", "turn-1"));

        store.note_outbound_op(&Op::UserInputAnswer {
            id: "turn-1".to_string(),
            response: ToolRequestUserInputResponse {
                answers: HashMap::new(),
            },
        });

        let snapshot = store.snapshot();
        assert_eq!(snapshot.events.len(), 1);
        assert!(matches!(
            snapshot.events.first(),
            Some(ThreadBufferedEvent::Request(ServerRequest::ToolRequestUserInput { params, .. }))
                if params.message_id == "call-2"
        ));
    }

    #[test]
    fn thread_event_snapshot_drops_resolved_patch_approval_after_outbound_approval() {
        let mut store = ThreadEventStore::new(/*capacity*/ 8);
        store.push_request(patch_approval_request("call-1", "turn-1"));

        store.note_outbound_op(&Op::PatchApproval {
            id: "call-1".to_string(),
            decision: datax_app_server_protocol::FileChangeApprovalDecision::Accept,
        });

        let snapshot = store.snapshot();
        assert!(
            snapshot.events.is_empty(),
            "resolved patch approval prompt should not replay on thread switch"
        );
    }

    #[test]
    fn thread_event_snapshot_drops_pending_approvals_when_turn_completes() {
        let mut store = ThreadEventStore::new(/*capacity*/ 8);
        store.push_request(exec_approval_request(
            "exec-call-1",
            Some("approval-1"),
            "turn-1",
        ));
        store.push_request(patch_approval_request("patch-call-1", "turn-1"));
        store.push_notification(turn_completed("turn-1"));

        let snapshot = store.snapshot();
        assert!(snapshot.events.iter().all(|event| {
            !matches!(
                event,
                ThreadBufferedEvent::Request(ServerRequest::CommandExecutionRequestApproval { .. })
                    | ThreadBufferedEvent::Request(ServerRequest::FileChangeRequestApproval { .. })
            )
        }));
    }

    #[test]
    fn thread_event_snapshot_drops_resolved_elicitation_after_outbound_resolution() {
        let mut store = ThreadEventStore::new(/*capacity*/ 8);
        let request_id = AppServerRequestId::String("request-1".to_string());
        store.push_request(elicitation_request("server-1", "request-1", "turn-1"));

        store.note_outbound_op(&Op::ResolveElicitation {
            server_name: "server-1".to_string(),
            request_id,
            decision: McpServerElicitationAction::Accept,
            content: None,
            meta: None,
        });

        let snapshot = store.snapshot();
        assert!(
            snapshot.events.is_empty(),
            "resolved elicitation prompt should not replay on thread switch"
        );
    }

    #[test]
    fn thread_event_store_reports_pending_thread_approvals() {
        let mut store = ThreadEventStore::new(/*capacity*/ 8);
        assert_eq!(store.has_pending_thread_approvals(), false);

        store.push_request(exec_approval_request(
            "call-1", /*approval_id*/ None, "turn-1",
        ));

        assert_eq!(store.has_pending_thread_approvals(), true);

        store.note_outbound_op(&Op::ExecApproval {
            id: "call-1".to_string(),
            interaction_id: Some("turn-1".to_string()),
            decision: CommandExecutionApprovalDecision::Accept,
        });

        assert_eq!(store.has_pending_thread_approvals(), false);
    }

    #[test]
    fn request_user_input_does_not_count_as_pending_thread_approval() {
        let mut store = ThreadEventStore::new(/*capacity*/ 8);
        store.push_request(request_user_input_request("call-1", "turn-1"));

        assert_eq!(store.has_pending_thread_approvals(), false);
    }

    #[test]
    fn thread_event_snapshot_drops_pending_requests_when_thread_closes() {
        let mut store = ThreadEventStore::new(/*capacity*/ 8);
        store.push_request(exec_approval_request(
            "call-1", /*approval_id*/ None, "turn-1",
        ));
        store.push_notification(thread_closed());

        assert!(store.snapshot().events.iter().all(|event| {
            !matches!(
                event,
                ThreadBufferedEvent::Request(ServerRequest::CommandExecutionRequestApproval { .. })
            )
        }));
    }
}
