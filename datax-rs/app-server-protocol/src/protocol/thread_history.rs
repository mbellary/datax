use crate::protocol::item_builders::build_command_execution_begin_item;
use crate::protocol::item_builders::build_command_execution_end_item;
use crate::protocol::item_builders::build_file_change_approval_request_item;
use crate::protocol::item_builders::build_file_change_begin_item;
use crate::protocol::item_builders::build_file_change_end_item;
use crate::protocol::item_builders::build_item_from_guardian_event;
use crate::protocol::v2::CollabAgentState;
use crate::protocol::v2::CollabAgentTool;
use crate::protocol::v2::CollabAgentToolCallStatus;
use crate::protocol::v2::CommandExecutionStatus;
use crate::protocol::v2::DynamicToolCallOutputContentItem;
use crate::protocol::v2::DynamicToolCallStatus;
use crate::protocol::v2::Interaction;
use crate::protocol::v2::InteractionError as V2TurnError;
use crate::protocol::v2::InteractionError;
use crate::protocol::v2::InteractionMessagesView;
use crate::protocol::v2::InteractionStatus;
use crate::protocol::v2::McpToolCallAppContext;
use crate::protocol::v2::McpToolCallError;
use crate::protocol::v2::McpToolCallResult;
use crate::protocol::v2::McpToolCallStatus;
use crate::protocol::v2::Message;
use crate::protocol::v2::UserInput;
use crate::protocol::v2::WebSearchAction;
use datax_protocol::items::parse_hook_prompt_message;
use datax_protocol::models::MessagePhase;
use datax_protocol::protocol::AgentReasoningEvent;
use datax_protocol::protocol::AgentReasoningRawContentEvent;
use datax_protocol::protocol::AgentStatus;
use datax_protocol::protocol::ApplyPatchApprovalRequestEvent;
use datax_protocol::protocol::CompactedItem;
use datax_protocol::protocol::ContextCompactedEvent;
use datax_protocol::protocol::DynamicToolCallResponseEvent;
use datax_protocol::protocol::ErrorEvent;
use datax_protocol::protocol::EventMsg;
use datax_protocol::protocol::ExecCommandBeginEvent;
use datax_protocol::protocol::ExecCommandEndEvent;
use datax_protocol::protocol::GuardianAssessmentEvent;
use datax_protocol::protocol::GuardianAssessmentStatus;
use datax_protocol::protocol::ImageGenerationBeginEvent;
use datax_protocol::protocol::ImageGenerationEndEvent;
use datax_protocol::protocol::ItemCompletedEvent;
use datax_protocol::protocol::ItemStartedEvent;
use datax_protocol::protocol::McpToolCallBeginEvent;
use datax_protocol::protocol::McpToolCallEndEvent;
use datax_protocol::protocol::PatchApplyBeginEvent;
use datax_protocol::protocol::PatchApplyEndEvent;
use datax_protocol::protocol::ReviewOutputEvent;
use datax_protocol::protocol::RolloutItem;
use datax_protocol::protocol::ThreadRolledBackEvent;
use datax_protocol::protocol::InteractionAbortedEvent;
use datax_protocol::protocol::InteractionCompleteEvent;
use datax_protocol::protocol::InteractionStartedEvent;
use datax_protocol::protocol::UserMessageEvent;
use datax_protocol::protocol::ViewImageToolCallEvent;
use datax_protocol::protocol::WebSearchBeginEvent;
use datax_protocol::protocol::WebSearchEndEvent;
use std::collections::HashMap;
use tracing::warn;
use uuid::Uuid;

#[cfg(test)]
use crate::protocol::v2::CommandAction;
#[cfg(test)]
use crate::protocol::v2::FileUpdateChange;
#[cfg(test)]
use crate::protocol::v2::PatchApplyStatus;
#[cfg(test)]
use crate::protocol::v2::PatchChangeKind;
#[cfg(test)]
use datax_protocol::protocol::ExecCommandStatus as CoreExecCommandStatus;
#[cfg(test)]
use datax_protocol::protocol::PatchApplyStatus as CorePatchApplyStatus;

/// Convert persisted [`RolloutItem`] entries into a sequence of [`Interaction`] values.
///
/// When available, this uses `TurnContext.interaction_id` as the canonical turn id so
/// resumed/rebuilt thread history preserves the original turn identifiers.
pub fn build_turns_from_rollout_items(messages: &[RolloutItem]) -> Vec<Interaction> {
    let mut builder = ChatHistoryBuilder::new();
    for item in messages {
        builder.handle_rollout_item(item);
    }
    builder.finish()
}

/// A materialized `Message` snapshot that changed while handling one input.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatHistoryMessageChange {
    pub interaction_id: String,
    pub item: Message,
}

/// Lightweight turn metadata snapshot for projectors that track turn status without
/// re-reading the full item list.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatHistoryInteractionChange {
    pub interaction_id: String,
    pub status: InteractionStatus,
    pub error: Option<InteractionError>,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub duration_ms: Option<i64>,
}

/// Incremental changes produced by opt-in `ChatHistoryBuilder` handlers.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ChatHistoryChangeSet {
    pub changed_items: Vec<ChatHistoryMessageChange>,
    pub changed_turns: Vec<ChatHistoryInteractionChange>,
    pub removed_interaction_ids: Vec<String>,
}

impl ChatHistoryChangeSet {
    pub fn is_empty(&self) -> bool {
        self.changed_items.is_empty()
            && self.changed_turns.is_empty()
            && self.removed_interaction_ids.is_empty()
    }
}

impl ChatHistoryInteractionChange {
    fn from_pending_turn(turn: &PendingTurn) -> Self {
        Self {
            interaction_id: turn.id.clone(),
            status: turn.status.clone(),
            error: turn.error.clone(),
            started_at: turn.started_at,
            completed_at: turn.completed_at,
            duration_ms: turn.duration_ms,
        }
    }

    fn from_turn(turn: &Interaction) -> Self {
        Self {
            interaction_id: turn.id.clone(),
            status: turn.status.clone(),
            error: turn.error.clone(),
            started_at: turn.started_at,
            completed_at: turn.completed_at,
            duration_ms: turn.duration_ms,
        }
    }
}

/// Coalesces per-rollout-item changes into an end-of-batch view. It preserves
/// first-change order while replacing repeated message/turn snapshots with their
/// latest value, and drops accumulated changes for interactions removed by rollback.
#[derive(Default)]
struct ThreadHistoryChangeAccumulator {
    changed_items: Vec<Option<ChatHistoryMessageChange>>,
    changed_item_indexes: HashMap<(String, String), usize>,
    changed_turns: Vec<Option<ChatHistoryInteractionChange>>,
    changed_turn_indexes: HashMap<String, usize>,
    removed_interaction_ids: Vec<String>,
    removed_turn_indexes: HashMap<String, usize>,
}

impl ThreadHistoryChangeAccumulator {
    fn push(&mut self, changes: ChatHistoryChangeSet) {
        for interaction_id in changes.removed_interaction_ids {
            self.push_removed_interaction_id(interaction_id);
        }
        for item_change in changes.changed_items {
            self.push_item_change(item_change);
        }
        for turn_change in changes.changed_turns {
            self.push_turn_change(turn_change);
        }
    }

    fn finish(self) -> ChatHistoryChangeSet {
        ChatHistoryChangeSet {
            changed_items: self.changed_items.into_iter().flatten().collect(),
            changed_turns: self.changed_turns.into_iter().flatten().collect(),
            removed_interaction_ids: self.removed_interaction_ids,
        }
    }

    fn push_item_change(&mut self, change: ChatHistoryMessageChange) {
        let key = (change.interaction_id.clone(), change.item.id().to_string());
        if let Some(index) = self.changed_item_indexes.get(&key).copied() {
            self.changed_items[index] = Some(change);
            return;
        }

        self.changed_item_indexes
            .insert(key, self.changed_items.len());
        self.changed_items.push(Some(change));
    }

    fn push_turn_change(&mut self, change: ChatHistoryInteractionChange) {
        if let Some(index) = self
            .changed_turn_indexes
            .get(&change.interaction_id)
            .copied()
        {
            self.changed_turns[index] = Some(change);
            return;
        }

        self.changed_turn_indexes
            .insert(change.interaction_id.clone(), self.changed_turns.len());
        self.changed_turns.push(Some(change));
    }

    fn push_removed_interaction_id(&mut self, interaction_id: String) {
        if !self.removed_turn_indexes.contains_key(&interaction_id) {
            self.removed_turn_indexes
                .insert(interaction_id.clone(), self.removed_interaction_ids.len());
            self.removed_interaction_ids.push(interaction_id.clone());
        }

        if let Some(index) = self.changed_turn_indexes.remove(&interaction_id) {
            self.changed_turns[index] = None;
        }

        let removed_item_keys: Vec<(String, String)> = self
            .changed_item_indexes
            .keys()
            .filter(|(item_interaction_id, _)| item_interaction_id == &interaction_id)
            .cloned()
            .collect();
        for key in removed_item_keys {
            if let Some(index) = self.changed_item_indexes.remove(&key) {
                self.changed_items[index] = None;
            }
        }
    }
}

pub struct ChatHistoryBuilder {
    interactions: Vec<Interaction>,
    current_turn: Option<PendingTurn>,
    next_item_index: i64,
    current_rollout_index: usize,
    next_rollout_index: usize,
    active_change_set: Option<ChatHistoryChangeSet>,
}

impl Default for ChatHistoryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatHistoryBuilder {
    pub fn new() -> Self {
        Self {
            interactions: Vec::new(),
            current_turn: None,
            next_item_index: 1,
            current_rollout_index: 0,
            next_rollout_index: 0,
            active_change_set: None,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn finish(mut self) -> Vec<Interaction> {
        self.finish_current_turn();
        self.interactions
    }

    pub fn active_turn_snapshot(&self) -> Option<Interaction> {
        self.current_turn
            .as_ref()
            .map(Interaction::from)
            .or_else(|| self.interactions.last().cloned())
    }

    pub fn turn_snapshot(&self, interaction_id: &str) -> Option<Interaction> {
        self.current_turn
            .as_ref()
            .filter(|turn| turn.id == interaction_id)
            .map(Interaction::from)
            .or_else(|| {
                self.interactions
                    .iter()
                    .find(|turn| turn.id == interaction_id)
                    .cloned()
            })
    }

    /// Returns the index of the active turn snapshot within the finished turn list.
    ///
    /// When a turn is still open, this is the index it will occupy after
    /// `finish`. When no turn is open, it is the index of the last finished turn.
    pub fn active_turn_position(&self) -> Option<usize> {
        if self.current_turn.is_some() {
            Some(self.interactions.len())
        } else if self.interactions.is_empty() {
            None
        } else {
            Some(self.interactions.len() - 1)
        }
    }

    pub fn has_active_turn(&self) -> bool {
        self.current_turn.is_some()
    }

    pub fn active_interaction_id_if_explicit(&self) -> Option<String> {
        self.current_turn
            .as_ref()
            .filter(|turn| turn.opened_explicitly)
            .map(|turn| turn.id.clone())
    }

    pub fn active_turn_start_index(&self) -> Option<usize> {
        self.current_turn
            .as_ref()
            .map(|turn| turn.rollout_start_index)
    }

    /// Shared reducer for persisted rollout replay and in-memory current-turn
    /// tracking used by running thread resume/rejoin.
    ///
    /// This function should handle all EventMsg variants that can be persisted in a rollout file.
    /// See `should_persist_event_msg` in `datax-rs/core/rollout/policy.rs`.
    pub fn handle_event(&mut self, event: &EventMsg) {
        match event {
            EventMsg::UserMessage(payload) => self.handle_user_message(payload),
            EventMsg::AgentMessage(payload) => self.handle_agent_message(
                payload.message.clone(),
                payload.phase.clone(),
                payload.memory_citation.clone().map(Into::into),
            ),
            EventMsg::AgentReasoning(payload) => self.handle_agent_reasoning(payload),
            EventMsg::AgentReasoningRawContent(payload) => {
                self.handle_agent_reasoning_raw_content(payload)
            }
            EventMsg::WebSearchBegin(payload) => self.handle_web_search_begin(payload),
            EventMsg::WebSearchEnd(payload) => self.handle_web_search_end(payload),
            EventMsg::ExecCommandBegin(payload) => self.handle_exec_command_begin(payload),
            EventMsg::ExecCommandEnd(payload) => self.handle_exec_command_end(payload),
            EventMsg::GuardianAssessment(payload) => self.handle_guardian_assessment(payload),
            EventMsg::ApplyPatchApprovalRequest(payload) => {
                self.handle_apply_patch_approval_request(payload)
            }
            EventMsg::PatchApplyBegin(payload) => self.handle_patch_apply_begin(payload),
            EventMsg::PatchApplyEnd(payload) => self.handle_patch_apply_end(payload),
            EventMsg::DynamicToolCallRequest(payload) => {
                self.handle_dynamic_tool_call_request(payload)
            }
            EventMsg::DynamicToolCallResponse(payload) => {
                self.handle_dynamic_tool_call_response(payload)
            }
            EventMsg::McpToolCallBegin(payload) => self.handle_mcp_tool_call_begin(payload),
            EventMsg::McpToolCallEnd(payload) => self.handle_mcp_tool_call_end(payload),
            EventMsg::ViewImageToolCall(payload) => self.handle_view_image_tool_call(payload),
            EventMsg::ImageGenerationBegin(payload) => self.handle_image_generation_begin(payload),
            EventMsg::ImageGenerationEnd(payload) => self.handle_image_generation_end(payload),
            EventMsg::CollabAgentSpawnBegin(payload) => {
                self.handle_collab_agent_spawn_begin(payload)
            }
            EventMsg::CollabAgentSpawnEnd(payload) => self.handle_collab_agent_spawn_end(payload),
            EventMsg::CollabAgentInteractionBegin(payload) => {
                self.handle_collab_agent_interaction_begin(payload)
            }
            EventMsg::CollabAgentInteractionEnd(payload) => {
                self.handle_collab_agent_interaction_end(payload)
            }
            EventMsg::SubAgentActivity(payload) => self.handle_sub_agent_activity(payload),
            EventMsg::CollabWaitingBegin(payload) => self.handle_collab_waiting_begin(payload),
            EventMsg::CollabWaitingEnd(payload) => self.handle_collab_waiting_end(payload),
            EventMsg::CollabCloseBegin(payload) => self.handle_collab_close_begin(payload),
            EventMsg::CollabCloseEnd(payload) => self.handle_collab_close_end(payload),
            EventMsg::CollabResumeBegin(payload) => self.handle_collab_resume_begin(payload),
            EventMsg::CollabResumeEnd(payload) => self.handle_collab_resume_end(payload),
            EventMsg::ContextCompacted(payload) => self.handle_context_compacted(payload),
            EventMsg::EnteredReviewMode(payload) => self.handle_entered_review_mode(payload),
            EventMsg::ExitedReviewMode(payload) => self.handle_exited_review_mode(payload),
            EventMsg::ItemStarted(payload) => self.handle_item_started(payload),
            EventMsg::ItemCompleted(payload) => self.handle_item_completed(payload),
            EventMsg::HookStarted(_) | EventMsg::HookCompleted(_) => {}
            EventMsg::Error(payload) => self.handle_error(payload),
            EventMsg::TokenCount(_) => {}
            EventMsg::ThreadRolledBack(payload) => self.handle_thread_rollback(payload),
            EventMsg::InteractionAborted(payload) => self.handle_turn_aborted(payload),
            EventMsg::InteractionStarted(payload) => self.handle_turn_started(payload),
            EventMsg::InteractionComplete(payload) => self.handle_turn_complete(payload),
            _ => {}
        }
    }

    pub fn handle_rollout_item(&mut self, item: &RolloutItem) {
        self.current_rollout_index = self.next_rollout_index;
        self.next_rollout_index += 1;
        match item {
            RolloutItem::EventMsg(event) => self.handle_event(event),
            RolloutItem::Compacted(payload) => self.handle_compacted(payload),
            RolloutItem::ResponseItem(item) => self.handle_response_item(item),
            RolloutItem::InterAgentCommunication(_)
            | RolloutItem::TurnContext(_)
            | RolloutItem::SessionMeta(_) => {}
        }
    }

    /// Handles one event and returns the materialized messages or turn metadata
    /// changed by that event.
    pub fn handle_event_with_changes(&mut self, event: &EventMsg) -> ChatHistoryChangeSet {
        self.collect_changes(|builder| builder.handle_event(event))
    }

    /// Handles a rollout item and returns the materialized messages or turn metadata
    /// changed by that one append.
    pub fn handle_rollout_item_with_changes(&mut self, item: &RolloutItem) -> ChatHistoryChangeSet {
        self.collect_changes(|builder| builder.handle_rollout_item(item))
    }

    /// Handles rollout messages in order and returns a coalesced end-of-batch
    /// change set. Multiple changes to the same item or turn are deduplicated
    /// so only the latest snapshot is emitted.
    pub fn handle_rollout_items_with_changes(
        &mut self,
        messages: &[RolloutItem],
    ) -> ChatHistoryChangeSet {
        let mut accumulator = ThreadHistoryChangeAccumulator::default();
        for item in messages {
            accumulator.push(self.handle_rollout_item_with_changes(item));
        }
        accumulator.finish()
    }

    fn collect_changes(&mut self, handle: impl FnOnce(&mut Self)) -> ChatHistoryChangeSet {
        debug_assert!(self.active_change_set.is_none());
        self.active_change_set = Some(ChatHistoryChangeSet::default());
        handle(self);
        self.active_change_set.take().unwrap_or_default()
    }

    fn handle_response_item(&mut self, item: &datax_protocol::models::ResponseItem) {
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

        self.push_item_in_current_turn(Message::HookPrompt {
            id: hook_prompt.id,
            fragments: hook_prompt
                .fragments
                .into_iter()
                .map(crate::protocol::v2::HookPromptFragment::from)
                .collect(),
        });
    }

    fn handle_user_message(&mut self, payload: &UserMessageEvent) {
        // User messages should stay in explicitly opened interactions. For backward
        // compatibility with older streams that did not open interactions explicitly,
        // close any implicit/inactive turn and start a fresh one for this input.
        if let Some(turn) = self.current_turn.as_ref()
            && !turn.opened_explicitly
            && !(turn.saw_compaction && turn.messages.is_empty())
        {
            self.finish_current_turn();
        }
        let id = self.next_item_id();
        let content = self.build_user_inputs(payload);
        self.push_item_in_current_turn(Message::UserMessage {
            id,
            client_id: payload.client_id.clone(),
            content,
        });
    }

    fn handle_agent_message(
        &mut self,
        text: String,
        phase: Option<MessagePhase>,
        memory_citation: Option<crate::protocol::v2::MemoryCitation>,
    ) {
        if text.is_empty() {
            return;
        }

        let id = self.next_item_id();
        self.push_item_in_current_turn(Message::AgentMessage {
            id,
            text,
            phase,
            memory_citation,
        });
    }

    fn handle_agent_reasoning(&mut self, payload: &AgentReasoningEvent) {
        if payload.text.is_empty() {
            return;
        }

        // If the last item is a reasoning item, add the new text to the summary.
        let existing_item_change = {
            let tracking_changes = self.is_tracking_changes();
            let turn = self.ensure_turn();
            if let Some(Message::Reasoning { summary, .. }) = turn.messages.last_mut() {
                summary.push(payload.text.clone());
                let changed_item = if tracking_changes {
                    turn.messages
                        .last()
                        .cloned()
                        .map(|item| (turn.id.clone(), item))
                } else {
                    None
                };
                Some(changed_item)
            } else {
                None
            }
        };
        if let Some(changed_item) = existing_item_change {
            if let Some((interaction_id, item)) = changed_item {
                self.record_changed_item(interaction_id, item);
            }
            return;
        }

        // Otherwise, create a new reasoning item.
        let id = self.next_item_id();
        self.push_item_in_current_turn(Message::Reasoning {
            id,
            summary: vec![payload.text.clone()],
            content: Vec::new(),
        });
    }

    fn handle_agent_reasoning_raw_content(&mut self, payload: &AgentReasoningRawContentEvent) {
        if payload.text.is_empty() {
            return;
        }

        // If the last item is a reasoning item, add the new text to the content.
        let existing_item_change = {
            let tracking_changes = self.is_tracking_changes();
            let turn = self.ensure_turn();
            if let Some(Message::Reasoning { content, .. }) = turn.messages.last_mut() {
                content.push(payload.text.clone());
                let changed_item = if tracking_changes {
                    turn.messages
                        .last()
                        .cloned()
                        .map(|item| (turn.id.clone(), item))
                } else {
                    None
                };
                Some(changed_item)
            } else {
                None
            }
        };
        if let Some(changed_item) = existing_item_change {
            if let Some((interaction_id, item)) = changed_item {
                self.record_changed_item(interaction_id, item);
            }
            return;
        }

        // Otherwise, create a new reasoning item.
        let id = self.next_item_id();
        self.push_item_in_current_turn(Message::Reasoning {
            id,
            summary: Vec::new(),
            content: vec![payload.text.clone()],
        });
    }

    fn handle_item_started(&mut self, payload: &ItemStartedEvent) {
        match &payload.item {
            datax_protocol::items::TurnItem::Plan(plan) => {
                if plan.text.is_empty() {
                    return;
                }
                self.upsert_item_in_interaction_id(&payload.interaction_id, Message::from(payload.item.clone()));
            }
            datax_protocol::items::TurnItem::Sleep(_) => {
                self.upsert_item_in_interaction_id(&payload.interaction_id, Message::from(payload.item.clone()));
            }
            datax_protocol::items::TurnItem::UserMessage(_)
            | datax_protocol::items::TurnItem::HookPrompt(_)
            | datax_protocol::items::TurnItem::AgentMessage(_)
            | datax_protocol::items::TurnItem::Reasoning(_)
            | datax_protocol::items::TurnItem::WebSearch(_)
            | datax_protocol::items::TurnItem::ImageView(_)
            | datax_protocol::items::TurnItem::ImageGeneration(_)
            | datax_protocol::items::TurnItem::FileChange(_)
            | datax_protocol::items::TurnItem::McpToolCall(_)
            | datax_protocol::items::TurnItem::ContextCompaction(_) => {}
        }
    }

    fn handle_item_completed(&mut self, payload: &ItemCompletedEvent) {
        match &payload.item {
            datax_protocol::items::TurnItem::Plan(plan) => {
                if plan.text.is_empty() {
                    return;
                }
                self.upsert_item_in_interaction_id(&payload.interaction_id, Message::from(payload.item.clone()));
            }
            datax_protocol::items::TurnItem::Sleep(_) => {
                self.upsert_item_in_interaction_id(&payload.interaction_id, Message::from(payload.item.clone()));
            }
            datax_protocol::items::TurnItem::UserMessage(_)
            | datax_protocol::items::TurnItem::HookPrompt(_)
            | datax_protocol::items::TurnItem::AgentMessage(_)
            | datax_protocol::items::TurnItem::Reasoning(_)
            | datax_protocol::items::TurnItem::WebSearch(_)
            | datax_protocol::items::TurnItem::ImageView(_)
            | datax_protocol::items::TurnItem::ImageGeneration(_)
            | datax_protocol::items::TurnItem::FileChange(_)
            | datax_protocol::items::TurnItem::McpToolCall(_)
            | datax_protocol::items::TurnItem::ContextCompaction(_) => {}
        }
    }

    fn handle_web_search_begin(&mut self, payload: &WebSearchBeginEvent) {
        let item = Message::WebSearch {
            id: payload.call_id.clone(),
            query: String::new(),
            action: None,
        };
        self.upsert_item_in_current_turn(item);
    }

    fn handle_web_search_end(&mut self, payload: &WebSearchEndEvent) {
        let item = Message::WebSearch {
            id: payload.call_id.clone(),
            query: payload.query.clone(),
            action: Some(WebSearchAction::from(payload.action.clone())),
        };
        self.upsert_item_in_current_turn(item);
    }

    fn handle_exec_command_begin(&mut self, payload: &ExecCommandBeginEvent) {
        let item = build_command_execution_begin_item(payload);
        self.upsert_item_in_interaction_id(&payload.interaction_id, item);
    }

    fn handle_exec_command_end(&mut self, payload: &ExecCommandEndEvent) {
        let item = build_command_execution_end_item(payload);
        // Command completions can arrive out of order. Unified exec may return
        // while a PTY is still running, then emit ExecCommandEnd later from a
        // background exit watcher when that process finally exits. By then, a
        // newer user turn may already have started. Route by event interaction_id so
        // replay preserves the original turn association.
        self.upsert_item_in_interaction_id(&payload.interaction_id, item);
    }

    fn handle_guardian_assessment(&mut self, payload: &GuardianAssessmentEvent) {
        let status = match payload.status {
            GuardianAssessmentStatus::InProgress => CommandExecutionStatus::InProgress,
            GuardianAssessmentStatus::Denied | GuardianAssessmentStatus::Aborted => {
                CommandExecutionStatus::Declined
            }
            GuardianAssessmentStatus::TimedOut => CommandExecutionStatus::Failed,
            GuardianAssessmentStatus::Approved => return,
        };
        let Some(item) = build_item_from_guardian_event(payload, status) else {
            return;
        };
        if payload.interaction_id.is_empty() {
            self.upsert_item_in_current_turn(item);
        } else {
            self.upsert_item_in_interaction_id(&payload.interaction_id, item);
        }
    }

    fn handle_apply_patch_approval_request(&mut self, payload: &ApplyPatchApprovalRequestEvent) {
        let item = build_file_change_approval_request_item(payload);
        if payload.interaction_id.is_empty() {
            self.upsert_item_in_current_turn(item);
        } else {
            self.upsert_item_in_interaction_id(&payload.interaction_id, item);
        }
    }

    fn handle_patch_apply_begin(&mut self, payload: &PatchApplyBeginEvent) {
        let item = build_file_change_begin_item(payload);
        if payload.interaction_id.is_empty() {
            self.upsert_item_in_current_turn(item);
        } else {
            self.upsert_item_in_interaction_id(&payload.interaction_id, item);
        }
    }

    fn handle_patch_apply_end(&mut self, payload: &PatchApplyEndEvent) {
        let item = build_file_change_end_item(payload);
        if payload.interaction_id.is_empty() {
            self.upsert_item_in_current_turn(item);
        } else {
            self.upsert_item_in_interaction_id(&payload.interaction_id, item);
        }
    }

    fn handle_dynamic_tool_call_request(
        &mut self,
        payload: &datax_protocol::dynamic_tools::DynamicToolCallRequest,
    ) {
        let item = Message::DynamicToolCall {
            id: payload.call_id.clone(),
            namespace: payload.namespace.clone(),
            tool: payload.tool.clone(),
            arguments: payload.arguments.clone(),
            status: DynamicToolCallStatus::InProgress,
            content_items: None,
            success: None,
            duration_ms: None,
        };
        if payload.interaction_id.is_empty() {
            self.upsert_item_in_current_turn(item);
        } else {
            self.upsert_item_in_interaction_id(&payload.interaction_id, item);
        }
    }

    fn handle_dynamic_tool_call_response(&mut self, payload: &DynamicToolCallResponseEvent) {
        let status = if payload.success {
            DynamicToolCallStatus::Completed
        } else {
            DynamicToolCallStatus::Failed
        };
        let duration_ms = i64::try_from(payload.duration.as_millis()).ok();
        let item = Message::DynamicToolCall {
            id: payload.call_id.clone(),
            namespace: payload.namespace.clone(),
            tool: payload.tool.clone(),
            arguments: payload.arguments.clone(),
            status,
            content_items: Some(convert_dynamic_tool_content_items(&payload.content_items)),
            success: Some(payload.success),
            duration_ms,
        };
        if payload.interaction_id.is_empty() {
            self.upsert_item_in_current_turn(item);
        } else {
            self.upsert_item_in_interaction_id(&payload.interaction_id, item);
        }
    }

    fn handle_mcp_tool_call_begin(&mut self, payload: &McpToolCallBeginEvent) {
        let item = Message::McpToolCall {
            id: payload.call_id.clone(),
            server: payload.invocation.server.clone(),
            tool: payload.invocation.tool.clone(),
            status: McpToolCallStatus::InProgress,
            arguments: payload
                .invocation
                .arguments
                .clone()
                .unwrap_or(serde_json::Value::Null),
            app_context: payload
                .connector_id
                .clone()
                .map(|connector_id| McpToolCallAppContext {
                    connector_id,
                    link_id: payload.link_id.clone(),
                    resource_uri: payload.mcp_app_resource_uri.clone(),
                }),
            mcp_app_resource_uri: payload.mcp_app_resource_uri.clone(),
            plugin_id: payload.plugin_id.clone(),
            result: None,
            error: None,
            duration_ms: None,
        };
        self.upsert_item_in_current_turn(item);
    }

    fn handle_mcp_tool_call_end(&mut self, payload: &McpToolCallEndEvent) {
        let status = if payload.is_success() {
            McpToolCallStatus::Completed
        } else {
            McpToolCallStatus::Failed
        };
        let duration_ms = i64::try_from(payload.duration.as_millis()).ok();
        let (result, error) = match &payload.result {
            Ok(value) => (
                Some(Box::new(McpToolCallResult {
                    content: value.content.clone(),
                    structured_content: value.structured_content.clone(),
                    meta: value.meta.clone(),
                })),
                None,
            ),
            Err(message) => (
                None,
                Some(McpToolCallError {
                    message: message.clone(),
                }),
            ),
        };
        let item = Message::McpToolCall {
            id: payload.call_id.clone(),
            server: payload.invocation.server.clone(),
            tool: payload.invocation.tool.clone(),
            status,
            arguments: payload
                .invocation
                .arguments
                .clone()
                .unwrap_or(serde_json::Value::Null),
            app_context: payload
                .connector_id
                .clone()
                .map(|connector_id| McpToolCallAppContext {
                    connector_id,
                    link_id: payload.link_id.clone(),
                    resource_uri: payload.mcp_app_resource_uri.clone(),
                }),
            mcp_app_resource_uri: payload.mcp_app_resource_uri.clone(),
            plugin_id: payload.plugin_id.clone(),
            result,
            error,
            duration_ms,
        };
        self.upsert_item_in_current_turn(item);
    }

    fn handle_view_image_tool_call(&mut self, payload: &ViewImageToolCallEvent) {
        let item = Message::ImageView {
            id: payload.call_id.clone(),
            path: payload.path.clone(),
        };
        self.upsert_item_in_current_turn(item);
    }

    fn handle_image_generation_begin(&mut self, payload: &ImageGenerationBeginEvent) {
        let item = Message::ImageGeneration {
            id: payload.call_id.clone(),
            status: String::new(),
            revised_prompt: None,
            result: String::new(),
            saved_path: None,
        };
        self.upsert_item_in_current_turn(item);
    }

    fn handle_image_generation_end(&mut self, payload: &ImageGenerationEndEvent) {
        let item = Message::ImageGeneration {
            id: payload.call_id.clone(),
            status: payload.status.clone(),
            revised_prompt: payload.revised_prompt.clone(),
            result: payload.result.clone(),
            saved_path: payload.saved_path.clone(),
        };
        self.upsert_item_in_current_turn(item);
    }

    fn handle_collab_agent_spawn_begin(
        &mut self,
        payload: &datax_protocol::protocol::CollabAgentSpawnBeginEvent,
    ) {
        let item = Message::CollabAgentToolCall {
            id: payload.call_id.clone(),
            tool: CollabAgentTool::SpawnAgent,
            status: CollabAgentToolCallStatus::InProgress,
            sender_chat_id: payload.sender_chat_id.to_string(),
            receiver_chat_ids: Vec::new(),
            prompt: Some(payload.prompt.clone()),
            model: Some(payload.model.clone()),
            reasoning_effort: Some(payload.reasoning_effort.clone()),
            agents_states: HashMap::new(),
        };
        self.upsert_item_in_current_turn(item);
    }

    fn handle_collab_agent_spawn_end(
        &mut self,
        payload: &datax_protocol::protocol::CollabAgentSpawnEndEvent,
    ) {
        let has_receiver = payload.new_chat_id.is_some();
        let status = match &payload.status {
            AgentStatus::Errored(_) | AgentStatus::NotFound => CollabAgentToolCallStatus::Failed,
            _ if has_receiver => CollabAgentToolCallStatus::Completed,
            _ => CollabAgentToolCallStatus::Failed,
        };
        let (receiver_chat_ids, agents_states) = match &payload.new_chat_id {
            Some(id) => {
                let receiver_id = id.to_string();
                let received_status = CollabAgentState::from(payload.status.clone());
                (
                    vec![receiver_id.clone()],
                    [(receiver_id, received_status)].into_iter().collect(),
                )
            }
            None => (Vec::new(), HashMap::new()),
        };
        self.upsert_item_in_current_turn(Message::CollabAgentToolCall {
            id: payload.call_id.clone(),
            tool: CollabAgentTool::SpawnAgent,
            status,
            sender_chat_id: payload.sender_chat_id.to_string(),
            receiver_chat_ids,
            prompt: Some(payload.prompt.clone()),
            model: Some(payload.model.clone()),
            reasoning_effort: Some(payload.reasoning_effort.clone()),
            agents_states,
        });
    }

    fn handle_collab_agent_interaction_begin(
        &mut self,
        payload: &datax_protocol::protocol::CollabAgentInteractionBeginEvent,
    ) {
        let item = Message::CollabAgentToolCall {
            id: payload.call_id.clone(),
            tool: CollabAgentTool::SendInput,
            status: CollabAgentToolCallStatus::InProgress,
            sender_chat_id: payload.sender_chat_id.to_string(),
            receiver_chat_ids: vec![payload.receiver_chat_id.to_string()],
            prompt: Some(payload.prompt.clone()),
            model: None,
            reasoning_effort: None,
            agents_states: HashMap::new(),
        };
        self.upsert_item_in_current_turn(item);
    }

    fn handle_collab_agent_interaction_end(
        &mut self,
        payload: &datax_protocol::protocol::CollabAgentInteractionEndEvent,
    ) {
        let status = match &payload.status {
            AgentStatus::Errored(_) | AgentStatus::NotFound => CollabAgentToolCallStatus::Failed,
            _ => CollabAgentToolCallStatus::Completed,
        };
        let receiver_id = payload.receiver_chat_id.to_string();
        let received_status = CollabAgentState::from(payload.status.clone());
        self.upsert_item_in_current_turn(Message::CollabAgentToolCall {
            id: payload.call_id.clone(),
            tool: CollabAgentTool::SendInput,
            status,
            sender_chat_id: payload.sender_chat_id.to_string(),
            receiver_chat_ids: vec![receiver_id.clone()],
            prompt: Some(payload.prompt.clone()),
            model: None,
            reasoning_effort: None,
            agents_states: [(receiver_id, received_status)].into_iter().collect(),
        });
    }

    fn handle_sub_agent_activity(
        &mut self,
        payload: &datax_protocol::protocol::SubAgentActivityEvent,
    ) {
        self.upsert_item_in_current_turn(Message::SubAgentActivity {
            id: payload.event_id.clone(),
            kind: payload.kind.into(),
            agent_chat_id: payload.agent_chat_id.to_string(),
            agent_path: String::from(payload.agent_path.clone()),
        });
    }

    fn handle_collab_waiting_begin(
        &mut self,
        payload: &datax_protocol::protocol::CollabWaitingBeginEvent,
    ) {
        let item = Message::CollabAgentToolCall {
            id: payload.call_id.clone(),
            tool: CollabAgentTool::Wait,
            status: CollabAgentToolCallStatus::InProgress,
            sender_chat_id: payload.sender_chat_id.to_string(),
            receiver_chat_ids: payload
                .receiver_chat_ids
                .iter()
                .map(ToString::to_string)
                .collect(),
            prompt: None,
            model: None,
            reasoning_effort: None,
            agents_states: HashMap::new(),
        };
        self.upsert_item_in_current_turn(item);
    }

    fn handle_collab_waiting_end(
        &mut self,
        payload: &datax_protocol::protocol::CollabWaitingEndEvent,
    ) {
        let status = if payload
            .statuses
            .values()
            .any(|status| matches!(status, AgentStatus::Errored(_) | AgentStatus::NotFound))
        {
            CollabAgentToolCallStatus::Failed
        } else {
            CollabAgentToolCallStatus::Completed
        };
        let mut receiver_chat_ids: Vec<String> =
            payload.statuses.keys().map(ToString::to_string).collect();
        receiver_chat_ids.sort();
        let agents_states = payload
            .statuses
            .iter()
            .map(|(id, status)| (id.to_string(), CollabAgentState::from(status.clone())))
            .collect();
        self.upsert_item_in_current_turn(Message::CollabAgentToolCall {
            id: payload.call_id.clone(),
            tool: CollabAgentTool::Wait,
            status,
            sender_chat_id: payload.sender_chat_id.to_string(),
            receiver_chat_ids,
            prompt: None,
            model: None,
            reasoning_effort: None,
            agents_states,
        });
    }

    fn handle_collab_close_begin(
        &mut self,
        payload: &datax_protocol::protocol::CollabCloseBeginEvent,
    ) {
        let item = Message::CollabAgentToolCall {
            id: payload.call_id.clone(),
            tool: CollabAgentTool::CloseAgent,
            status: CollabAgentToolCallStatus::InProgress,
            sender_chat_id: payload.sender_chat_id.to_string(),
            receiver_chat_ids: vec![payload.receiver_chat_id.to_string()],
            prompt: None,
            model: None,
            reasoning_effort: None,
            agents_states: HashMap::new(),
        };
        self.upsert_item_in_current_turn(item);
    }

    fn handle_collab_close_end(&mut self, payload: &datax_protocol::protocol::CollabCloseEndEvent) {
        let status = match &payload.status {
            AgentStatus::Errored(_) | AgentStatus::NotFound => CollabAgentToolCallStatus::Failed,
            _ => CollabAgentToolCallStatus::Completed,
        };
        let receiver_id = payload.receiver_chat_id.to_string();
        let agents_states = [(
            receiver_id.clone(),
            CollabAgentState::from(payload.status.clone()),
        )]
        .into_iter()
        .collect();
        self.upsert_item_in_current_turn(Message::CollabAgentToolCall {
            id: payload.call_id.clone(),
            tool: CollabAgentTool::CloseAgent,
            status,
            sender_chat_id: payload.sender_chat_id.to_string(),
            receiver_chat_ids: vec![receiver_id],
            prompt: None,
            model: None,
            reasoning_effort: None,
            agents_states,
        });
    }

    fn handle_collab_resume_begin(
        &mut self,
        payload: &datax_protocol::protocol::CollabResumeBeginEvent,
    ) {
        let item = Message::CollabAgentToolCall {
            id: payload.call_id.clone(),
            tool: CollabAgentTool::ResumeAgent,
            status: CollabAgentToolCallStatus::InProgress,
            sender_chat_id: payload.sender_chat_id.to_string(),
            receiver_chat_ids: vec![payload.receiver_chat_id.to_string()],
            prompt: None,
            model: None,
            reasoning_effort: None,
            agents_states: HashMap::new(),
        };
        self.upsert_item_in_current_turn(item);
    }

    fn handle_collab_resume_end(
        &mut self,
        payload: &datax_protocol::protocol::CollabResumeEndEvent,
    ) {
        let status = match &payload.status {
            AgentStatus::Errored(_) | AgentStatus::NotFound => CollabAgentToolCallStatus::Failed,
            _ => CollabAgentToolCallStatus::Completed,
        };
        let receiver_id = payload.receiver_chat_id.to_string();
        let agents_states = [(
            receiver_id.clone(),
            CollabAgentState::from(payload.status.clone()),
        )]
        .into_iter()
        .collect();
        self.upsert_item_in_current_turn(Message::CollabAgentToolCall {
            id: payload.call_id.clone(),
            tool: CollabAgentTool::ResumeAgent,
            status,
            sender_chat_id: payload.sender_chat_id.to_string(),
            receiver_chat_ids: vec![receiver_id],
            prompt: None,
            model: None,
            reasoning_effort: None,
            agents_states,
        });
    }

    fn handle_context_compacted(&mut self, _payload: &ContextCompactedEvent) {
        let id = self.next_item_id();
        self.push_item_in_current_turn(Message::ContextCompaction { id });
    }

    fn handle_entered_review_mode(&mut self, payload: &datax_protocol::protocol::ReviewRequest) {
        let review = payload
            .user_facing_hint
            .clone()
            .unwrap_or_else(|| "Review requested.".to_string());
        let id = self.next_item_id();
        self.push_item_in_current_turn(Message::EnteredReviewMode { id, review });
    }

    fn handle_exited_review_mode(
        &mut self,
        payload: &datax_protocol::protocol::ExitedReviewModeEvent,
    ) {
        let review = payload
            .review_output
            .as_ref()
            .map(render_review_output_text)
            .unwrap_or_else(|| REVIEW_FALLBACK_MESSAGE.to_string());
        let id = self.next_item_id();
        self.push_item_in_current_turn(Message::ExitedReviewMode { id, review });
    }

    fn handle_error(&mut self, payload: &ErrorEvent) {
        if !payload.affects_turn_status() {
            return;
        }
        let tracking_changes = self.is_tracking_changes();
        let changed_turn = if let Some(turn) = self.current_turn.as_mut() {
            turn.status = InteractionStatus::Failed;
            turn.error = Some(V2TurnError {
                message: payload.message.clone(),
                codex_error_info: payload.codex_error_info.clone().map(Into::into),
                additional_details: None,
            });
            tracking_changes.then(|| ChatHistoryInteractionChange::from_pending_turn(turn))
        } else {
            None
        };
        if let Some(changed_turn) = changed_turn {
            self.record_changed_turn(changed_turn);
        }
    }

    fn handle_turn_aborted(&mut self, payload: &InteractionAbortedEvent) {
        let apply_abort = |turn: &mut PendingTurn| {
            turn.status = InteractionStatus::Interrupted;
            turn.completed_at = payload.completed_at;
            turn.duration_ms = payload.duration_ms;
            ChatHistoryInteractionChange::from_pending_turn(turn)
        };
        if let Some(interaction_id) = payload.interaction_id.as_deref() {
            // Prefer an exact ID match so we interrupt the turn explicitly targeted by the event.
            if let Some(turn) = self
                .current_turn
                .as_mut()
                .filter(|turn| turn.id == interaction_id)
            {
                let changed_turn = apply_abort(turn);
                self.record_changed_turn(changed_turn);
                return;
            }

            if let Some(turn) = self
                .interactions
                .iter_mut()
                .find(|turn| turn.id == interaction_id)
            {
                turn.status = InteractionStatus::Interrupted;
                turn.completed_at = payload.completed_at;
                turn.duration_ms = payload.duration_ms;
                let changed_turn = ChatHistoryInteractionChange::from_turn(turn);
                self.record_changed_turn(changed_turn);
                return;
            }
        }

        // If the event has no ID (or refers to an unknown turn), fall back to the active turn.
        if let Some(turn) = self.current_turn.as_mut() {
            let changed_turn = apply_abort(turn);
            self.record_changed_turn(changed_turn);
        }
    }

    fn handle_turn_started(&mut self, payload: &InteractionStartedEvent) {
        self.finish_current_turn();
        let turn = self
            .new_turn(Some(payload.interaction_id.clone()))
            .with_status(InteractionStatus::InProgress)
            .with_started_at(payload.started_at)
            .opened_explicitly();
        self.record_changed_pending_turn(&turn);
        self.current_turn = Some(turn);
    }

    fn handle_turn_complete(&mut self, payload: &InteractionCompleteEvent) {
        let mark_completed = |turn: &mut PendingTurn| {
            if matches!(
                turn.status,
                InteractionStatus::Completed | InteractionStatus::InProgress
            ) {
                turn.status = InteractionStatus::Completed;
            }
            turn.completed_at = payload.completed_at;
            turn.duration_ms = payload.duration_ms;
            ChatHistoryInteractionChange::from_pending_turn(turn)
        };

        // Prefer an exact ID match from the active turn and then close it.
        if let Some(current_turn) = self
            .current_turn
            .as_mut()
            .filter(|turn| turn.id == payload.interaction_id)
        {
            let changed_turn = mark_completed(current_turn);
            self.record_changed_turn(changed_turn);
            self.finish_current_turn();
            return;
        }

        if let Some(turn) = self
            .interactions
            .iter_mut()
            .find(|turn| turn.id == payload.interaction_id)
        {
            if matches!(
                turn.status,
                InteractionStatus::Completed | InteractionStatus::InProgress
            ) {
                turn.status = InteractionStatus::Completed;
            }
            turn.completed_at = payload.completed_at;
            turn.duration_ms = payload.duration_ms;
            let changed_turn = ChatHistoryInteractionChange::from_turn(turn);
            self.record_changed_turn(changed_turn);
            return;
        }

        // If the completion event cannot be matched, apply it to the active turn.
        if let Some(current_turn) = self.current_turn.as_mut() {
            let changed_turn = mark_completed(current_turn);
            self.record_changed_turn(changed_turn);
            self.finish_current_turn();
        }
    }

    /// Marks the current turn as containing a persisted compaction marker.
    ///
    /// This keeps compaction-only legacy interactions from being dropped by
    /// `finish_current_turn` when they have no renderable messages and were not
    /// explicitly opened.
    fn handle_compacted(&mut self, _payload: &CompactedItem) {
        self.ensure_turn().saw_compaction = true;
    }

    fn handle_thread_rollback(&mut self, payload: &ThreadRolledBackEvent) {
        self.finish_current_turn();

        let n = usize::try_from(payload.num_turns).unwrap_or(usize::MAX);
        let removed_interaction_ids = if n >= self.interactions.len() {
            self.interactions
                .iter()
                .map(|turn| turn.id.clone())
                .collect()
        } else if n == 0 {
            Vec::new()
        } else {
            self.interactions[self.interactions.len() - n..]
                .iter()
                .map(|turn| turn.id.clone())
                .collect()
        };
        self.record_removed_interaction_ids(removed_interaction_ids);

        if n >= self.interactions.len() {
            self.interactions.clear();
        } else {
            self.interactions
                .truncate(self.interactions.len().saturating_sub(n));
        }

        let item_count: usize = self.interactions.iter().map(|t| t.messages.len()).sum();
        self.next_item_index = i64::try_from(item_count.saturating_add(1)).unwrap_or(i64::MAX);
    }

    fn finish_current_turn(&mut self) {
        if let Some(turn) = self.current_turn.take() {
            if turn.messages.is_empty() && !turn.opened_explicitly && !turn.saw_compaction {
                return;
            }
            self.interactions.push(Interaction::from(turn));
        }
    }

    fn new_turn(&mut self, id: Option<String>) -> PendingTurn {
        let id = id.unwrap_or_else(|| {
            if self.next_rollout_index == 0 {
                Uuid::now_v7().to_string()
            } else {
                format!("rollout-{}", self.current_rollout_index)
            }
        });
        PendingTurn {
            id,
            messages: Vec::new(),
            error: None,
            status: InteractionStatus::Completed,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            opened_explicitly: false,
            saw_compaction: false,
            rollout_start_index: self.current_rollout_index,
        }
    }

    fn ensure_turn(&mut self) -> &mut PendingTurn {
        if self.current_turn.is_none() {
            let turn = self.new_turn(/*id*/ None);
            self.record_changed_pending_turn(&turn);
            self.current_turn = Some(turn);
        }

        if let Some(turn) = self.current_turn.as_mut() {
            return turn;
        }

        unreachable!("current turn must exist after initialization");
    }

    fn push_item_in_current_turn(&mut self, item: Message) {
        let tracking_changes = self.is_tracking_changes();
        let changed_item = {
            let turn = self.ensure_turn();
            let changed_item = tracking_changes.then(|| (turn.id.clone(), item.clone()));
            turn.messages.push(item);
            changed_item
        };
        if let Some((interaction_id, item)) = changed_item {
            self.record_changed_item(interaction_id, item);
        }
    }

    fn upsert_item_in_interaction_id(&mut self, interaction_id: &str, item: Message) {
        let tracking_changes = self.is_tracking_changes();
        if let Some(turn) = self.current_turn.as_mut()
            && turn.id == interaction_id
        {
            let changed_item = {
                let item = upsert_turn_item(&mut turn.messages, item);
                tracking_changes.then(|| (turn.id.clone(), item.clone()))
            };
            if let Some((interaction_id, item)) = changed_item {
                self.record_changed_item(interaction_id, item);
            }
            return;
        }

        if let Some(turn) = self
            .interactions
            .iter_mut()
            .find(|turn| turn.id == interaction_id)
        {
            let changed_item = {
                let item = upsert_turn_item(&mut turn.messages, item);
                tracking_changes.then(|| (turn.id.clone(), item.clone()))
            };
            if let Some((interaction_id, item)) = changed_item {
                self.record_changed_item(interaction_id, item);
            }
            return;
        }

        warn!(
            message_id = item.id(),
            "dropping turn-scoped item for unknown turn id `{interaction_id}`"
        );
    }

    fn upsert_item_in_current_turn(&mut self, item: Message) {
        let tracking_changes = self.is_tracking_changes();
        let changed_item = {
            let turn = self.ensure_turn();
            let item = upsert_turn_item(&mut turn.messages, item);
            tracking_changes.then(|| (turn.id.clone(), item.clone()))
        };
        if let Some((interaction_id, item)) = changed_item {
            self.record_changed_item(interaction_id, item);
        }
    }

    fn is_tracking_changes(&self) -> bool {
        self.active_change_set.is_some()
    }

    fn record_changed_item(&mut self, interaction_id: String, item: Message) {
        if let Some(change_set) = self.active_change_set.as_mut() {
            change_set.changed_items.push(ChatHistoryMessageChange {
                interaction_id,
                item,
            });
        }
    }

    fn record_changed_pending_turn(&mut self, turn: &PendingTurn) {
        if self.is_tracking_changes() {
            self.record_changed_turn(ChatHistoryInteractionChange::from_pending_turn(turn));
        }
    }

    fn record_changed_turn(&mut self, turn: ChatHistoryInteractionChange) {
        if let Some(change_set) = self.active_change_set.as_mut() {
            change_set.changed_turns.push(turn);
        }
    }

    fn record_removed_interaction_ids(&mut self, removed_interaction_ids: Vec<String>) {
        if let Some(change_set) = self.active_change_set.as_mut() {
            change_set.removed_interaction_ids.extend(removed_interaction_ids);
        }
    }

    fn next_item_id(&mut self) -> String {
        let id = format!("item-{}", self.next_item_index);
        self.next_item_index += 1;
        id
    }

    fn build_user_inputs(&self, payload: &UserMessageEvent) -> Vec<UserInput> {
        let mut content = Vec::new();
        if !payload.message.trim().is_empty() {
            content.push(UserInput::Text {
                text: payload.message.clone(),
                text_elements: payload
                    .text_elements
                    .iter()
                    .cloned()
                    .map(Into::into)
                    .collect(),
            });
        }
        if let Some(images) = &payload.images {
            for (idx, image) in images.iter().enumerate() {
                content.push(UserInput::Image {
                    url: image.clone(),
                    detail: payload.image_details.get(idx).copied().flatten(),
                });
            }
        }
        for (idx, path) in payload.local_images.iter().enumerate() {
            content.push(UserInput::LocalImage {
                path: path.clone(),
                detail: payload.local_image_details.get(idx).copied().flatten(),
            });
        }
        content
    }
}

const REVIEW_FALLBACK_MESSAGE: &str = "Reviewer failed to output a response.";

fn render_review_output_text(output: &ReviewOutputEvent) -> String {
    let explanation = output.overall_explanation.trim();
    if explanation.is_empty() {
        REVIEW_FALLBACK_MESSAGE.to_string()
    } else {
        explanation.to_string()
    }
}

fn convert_dynamic_tool_content_items(
    messages: &[datax_protocol::dynamic_tools::DynamicToolCallOutputContentItem],
) -> Vec<DynamicToolCallOutputContentItem> {
    messages
        .iter()
        .cloned()
        .map(|item| match item {
            datax_protocol::dynamic_tools::DynamicToolCallOutputContentItem::InputText { text } => {
                DynamicToolCallOutputContentItem::InputText { text }
            }
            datax_protocol::dynamic_tools::DynamicToolCallOutputContentItem::InputImage {
                image_url,
            } => DynamicToolCallOutputContentItem::InputImage { image_url },
        })
        .collect()
}

fn upsert_turn_item(messages: &mut Vec<Message>, item: Message) -> &Message {
    if let Some(existing_item_index) = messages
        .iter()
        .position(|existing_item| existing_item.id() == item.id())
    {
        messages[existing_item_index] = item;
        return &messages[existing_item_index];
    }
    let inserted_item_index = messages.len();
    messages.push(item);
    &messages[inserted_item_index]
}

struct PendingTurn {
    id: String,
    messages: Vec<Message>,
    error: Option<InteractionError>,
    status: InteractionStatus,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    duration_ms: Option<i64>,
    /// True when this turn originated from an explicit `turn_started`/`turn_complete`
    /// boundary, so we preserve it even if it has no renderable messages.
    opened_explicitly: bool,
    /// True when this turn includes a persisted `RolloutItem::Compacted`, which
    /// should keep the turn from being dropped even without normal messages.
    saw_compaction: bool,
    /// Index of the rollout item that opened this turn during replay.
    rollout_start_index: usize,
}

impl PendingTurn {
    fn opened_explicitly(mut self) -> Self {
        self.opened_explicitly = true;
        self
    }

    fn with_status(mut self, status: InteractionStatus) -> Self {
        self.status = status;
        self
    }

    fn with_started_at(mut self, started_at: Option<i64>) -> Self {
        self.started_at = started_at;
        self
    }
}

impl From<PendingTurn> for Interaction {
    fn from(value: PendingTurn) -> Self {
        Self {
            id: value.id,
            messages: value.messages,
            messages_view: InteractionMessagesView::Full,
            error: value.error,
            status: value.status,
            started_at: value.started_at,
            completed_at: value.completed_at,
            duration_ms: value.duration_ms,
        }
    }
}

impl From<&PendingTurn> for Interaction {
    fn from(value: &PendingTurn) -> Self {
        Self {
            id: value.id.clone(),
            messages: value.messages.clone(),
            messages_view: InteractionMessagesView::Full,
            error: value.error.clone(),
            status: value.status.clone(),
            started_at: value.started_at,
            completed_at: value.completed_at,
            duration_ms: value.duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::v2::CommandExecutionSource;
    use datax_protocol::ChatId;
    use datax_protocol::dynamic_tools::DynamicToolCallOutputContentItem as CoreDynamicToolCallOutputContentItem;
    use datax_protocol::items::HookPromptFragment as CoreHookPromptFragment;
    use datax_protocol::items::SleepItem as CoreSleepItem;
    use datax_protocol::items::TurnItem as CoreTurnItem;
    use datax_protocol::items::UserMessageItem as CoreUserMessageItem;
    use datax_protocol::items::build_hook_prompt_message;
    use datax_protocol::mcp::CallToolResult;
    use datax_protocol::models::ImageDetail;
    use datax_protocol::models::MessagePhase as CoreMessagePhase;
    use datax_protocol::models::WebSearchAction as CoreWebSearchAction;
    use datax_protocol::parse_command::ParsedCommand;
    use datax_protocol::protocol::AgentMessageEvent;
    use datax_protocol::protocol::AgentReasoningEvent;
    use datax_protocol::protocol::AgentReasoningRawContentEvent;
    use datax_protocol::protocol::ApplyPatchApprovalRequestEvent;
    use datax_protocol::protocol::CodexErrorInfo;
    use datax_protocol::protocol::CompactedItem;
    use datax_protocol::protocol::DynamicToolCallResponseEvent;
    use datax_protocol::protocol::ExecCommandEndEvent;
    use datax_protocol::protocol::ExecCommandSource;
    use datax_protocol::protocol::ItemStartedEvent;
    use datax_protocol::protocol::McpInvocation;
    use datax_protocol::protocol::McpToolCallEndEvent;
    use datax_protocol::protocol::PatchApplyBeginEvent;
    use datax_protocol::protocol::ThreadRolledBackEvent;
    use datax_protocol::protocol::InteractionAbortReason;
    use datax_protocol::protocol::InteractionAbortedEvent;
    use datax_protocol::protocol::InteractionCompleteEvent;
    use datax_protocol::protocol::InteractionStartedEvent;
    use datax_protocol::protocol::UserMessageEvent;
    use datax_protocol::protocol::WebSearchBeginEvent;
    use datax_protocol::protocol::WebSearchEndEvent;
    use datax_utils_absolute_path::test_support::PathBufExt;
    use datax_utils_absolute_path::test_support::test_path_buf;
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn builds_multiple_turns_with_reasoning_items() {
        let events = vec![
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "First turn".into(),
                images: Some(vec!["https://example.com/one.png".into()]),
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::AgentMessage(AgentMessageEvent {
                message: "Hi there".into(),
                phase: None,
                memory_citation: None,
            }),
            EventMsg::AgentReasoning(AgentReasoningEvent {
                text: "thinking".into(),
            }),
            EventMsg::AgentReasoningRawContent(AgentReasoningRawContentEvent {
                text: "full reasoning".into(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "Second turn".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::AgentMessage(AgentMessageEvent {
                message: "Reply two".into(),
                phase: None,
                memory_citation: None,
            }),
        ];

        let mut builder = ChatHistoryBuilder::new();
        for event in &events {
            builder.handle_event(event);
        }
        let interactions = builder.finish();
        assert_eq!(interactions.len(), 2);

        let first = &interactions[0];
        assert!(Uuid::parse_str(&first.id).is_ok());
        assert_eq!(first.status, InteractionStatus::Completed);
        assert_eq!(first.messages.len(), 3);
        assert_eq!(
            first.messages[0],
            Message::UserMessage {
                id: "item-1".into(),
                client_id: None,
                content: vec![
                    UserInput::Text {
                        text: "First turn".into(),
                        text_elements: Vec::new(),
                    },
                    UserInput::Image {
                        url: "https://example.com/one.png".into(),
                        detail: None,
                    }
                ],
            }
        );
        assert_eq!(
            first.messages[1],
            Message::AgentMessage {
                id: "item-2".into(),
                text: "Hi there".into(),
                phase: None,
                memory_citation: None,
            }
        );
        assert_eq!(
            first.messages[2],
            Message::Reasoning {
                id: "item-3".into(),
                summary: vec!["thinking".into()],
                content: vec!["full reasoning".into()],
            }
        );

        let second = &interactions[1];
        assert!(Uuid::parse_str(&second.id).is_ok());
        assert_ne!(first.id, second.id);
        assert_eq!(second.messages.len(), 2);
        assert_eq!(
            second.messages[0],
            Message::UserMessage {
                id: "item-4".into(),
                client_id: None,
                content: vec![UserInput::Text {
                    text: "Second turn".into(),
                    text_elements: Vec::new(),
                }],
            }
        );
        assert_eq!(
            second.messages[1],
            Message::AgentMessage {
                id: "item-5".into(),
                text: "Reply two".into(),
                phase: None,
                memory_citation: None,
            }
        );
    }

    #[test]
    fn rebuilds_user_message_image_details_from_legacy_events() {
        let local_path = PathBuf::from("/tmp/local.png");
        let events = vec![RolloutItem::EventMsg(EventMsg::UserMessage(
            UserMessageEvent {
                client_id: None,
                message: "inspect these".into(),
                images: Some(vec!["https://example.com/image.png".into()]),
                image_details: vec![Some(ImageDetail::Original)],
                local_images: vec![local_path.clone()],
                local_image_details: vec![Some(ImageDetail::Original)],
                text_elements: Vec::new(),
            },
        ))];

        let interactions = build_turns_from_rollout_items(&events);

        assert_eq!(interactions.len(), 1);
        assert_eq!(
            interactions[0].messages[0],
            Message::UserMessage {
                id: "item-1".into(),
                client_id: None,
                content: vec![
                    UserInput::Text {
                        text: "inspect these".into(),
                        text_elements: Vec::new(),
                    },
                    UserInput::Image {
                        url: "https://example.com/image.png".into(),
                        detail: Some(ImageDetail::Original),
                    },
                    UserInput::LocalImage {
                        path: local_path,
                        detail: Some(ImageDetail::Original),
                    },
                ],
            }
        );
    }

    #[test]
    fn ignores_user_message_item_lifecycle_events() {
        let interaction_id = "turn-1";
        let chat_id = ChatId::new();
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: interaction_id.to_string(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "hello".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::ItemStarted(ItemStartedEvent {
                chat_id: chat_id.clone(),
                interaction_id: interaction_id.to_string(),
                item: CoreTurnItem::UserMessage(CoreUserMessageItem {
                    id: "user-item-id".to_string(),
                    client_id: None,
                    content: Vec::new(),
                }),
                started_at_ms: 0,
            }),
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: interaction_id.to_string(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].messages.len(), 1);
        assert_eq!(
            interactions[0].messages[0],
            Message::UserMessage {
                id: "item-1".into(),
                client_id: None,
                content: vec![UserInput::Text {
                    text: "hello".into(),
                    text_elements: Vec::new(),
                }],
            }
        );
    }

    #[test]
    fn rebuilds_sleep_item_from_persisted_completion() {
        let interaction_id = "turn-1";
        let chat_id = ChatId::new();
        let sleep_item = CoreTurnItem::Sleep(CoreSleepItem {
            id: "sleep-1".to_string(),
            duration_ms: 1_000,
        });
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: interaction_id.to_string(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::ItemCompleted(ItemCompletedEvent {
                chat_id: chat_id.clone(),
                interaction_id: interaction_id.to_string(),
                item: sleep_item,
                completed_at_ms: 1_000,
            }),
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: interaction_id.to_string(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);

        assert_eq!(interactions.len(), 1);
        assert_eq!(
            interactions[0].messages,
            vec![Message::Sleep {
                id: "sleep-1".to_string(),
                duration_ms: 1_000,
            }]
        );
    }

    #[test]
    fn preserves_user_message_client_id_from_legacy_event() {
        let interaction_id = "turn-1";
        let chat_id = ChatId::new();
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: interaction_id.to_string(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::ItemStarted(ItemStartedEvent {
                chat_id: chat_id.clone(),
                interaction_id: interaction_id.to_string(),
                item: CoreTurnItem::UserMessage(CoreUserMessageItem {
                    id: "user-item-id".to_string(),
                    client_id: Some("client-message-1".to_string()),
                    content: vec![datax_protocol::user_input::UserInput::Text {
                        text: "hello".into(),
                        text_elements: Vec::new(),
                    }],
                }),
                started_at_ms: 0,
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: Some("client-message-1".to_string()),
                message: "hello".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: interaction_id.to_string(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(
            interactions[0].messages,
            vec![Message::UserMessage {
                id: "item-1".into(),
                client_id: Some("client-message-1".to_string()),
                content: vec![UserInput::Text {
                    text: "hello".into(),
                    text_elements: Vec::new(),
                }],
            }]
        );
    }

    #[test]
    fn preserves_agent_message_phase_in_history() {
        let events = vec![EventMsg::AgentMessage(AgentMessageEvent {
            message: "Final reply".into(),
            phase: Some(CoreMessagePhase::FinalAnswer),
            memory_citation: None,
        })];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(
            interactions[0].messages[0],
            Message::AgentMessage {
                id: "item-1".into(),
                text: "Final reply".into(),
                phase: Some(MessagePhase::FinalAnswer),
                memory_citation: None,
            }
        );
    }

    #[test]
    fn replays_image_generation_end_events_into_turn_history() {
        let messages = vec![
            RolloutItem::EventMsg(EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-image".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            })),
            RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "generate an image".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            })),
            RolloutItem::EventMsg(EventMsg::ImageGenerationEnd(ImageGenerationEndEvent {
                call_id: "ig_123".into(),
                status: "completed".into(),
                revised_prompt: Some("final prompt".into()),
                result: "Zm9v".into(),
                saved_path: Some(test_path_buf("/tmp/ig_123.png").abs()),
            })),
            RolloutItem::EventMsg(EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-image".into(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            })),
        ];

        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(
            interactions[0],
            Interaction {
                id: "turn-image".into(),
                status: InteractionStatus::Completed,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                messages_view: InteractionMessagesView::Full,
                messages: vec![
                    Message::UserMessage {
                        id: "item-1".into(),
                        client_id: None,
                        content: vec![UserInput::Text {
                            text: "generate an image".into(),
                            text_elements: Vec::new(),
                        }],
                    },
                    Message::ImageGeneration {
                        id: "ig_123".into(),
                        status: "completed".into(),
                        revised_prompt: Some("final prompt".into()),
                        result: "Zm9v".into(),
                        saved_path: Some(test_path_buf("/tmp/ig_123.png").abs()),
                    },
                ],
            }
        );
    }

    #[test]
    fn splits_reasoning_when_interleaved() {
        let events = vec![
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "Interaction start".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::AgentReasoning(AgentReasoningEvent {
                text: "first summary".into(),
            }),
            EventMsg::AgentReasoningRawContent(AgentReasoningRawContentEvent {
                text: "first content".into(),
            }),
            EventMsg::AgentMessage(AgentMessageEvent {
                message: "interlude".into(),
                phase: None,
                memory_citation: None,
            }),
            EventMsg::AgentReasoning(AgentReasoningEvent {
                text: "second summary".into(),
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        let turn = &interactions[0];
        assert_eq!(turn.messages.len(), 4);

        assert_eq!(
            turn.messages[1],
            Message::Reasoning {
                id: "item-2".into(),
                summary: vec!["first summary".into()],
                content: vec!["first content".into()],
            }
        );
        assert_eq!(
            turn.messages[3],
            Message::Reasoning {
                id: "item-4".into(),
                summary: vec!["second summary".into()],
                content: Vec::new(),
            }
        );
    }

    #[test]
    fn marks_turn_as_interrupted_when_aborted() {
        let events = vec![
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "Please do the thing".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::AgentMessage(AgentMessageEvent {
                message: "Working...".into(),
                phase: None,
                memory_citation: None,
            }),
            EventMsg::InteractionAborted(InteractionAbortedEvent {
                interaction_id: Some("turn-1".into()),
                reason: InteractionAbortReason::Replaced,
                completed_at: None,
                duration_ms: None,
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "Let's try again".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::AgentMessage(AgentMessageEvent {
                message: "Second attempt complete.".into(),
                phase: None,
                memory_citation: None,
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 2);

        let first_turn = &interactions[0];
        assert_eq!(first_turn.status, InteractionStatus::Interrupted);
        assert_eq!(first_turn.messages.len(), 2);
        assert_eq!(
            first_turn.messages[0],
            Message::UserMessage {
                id: "item-1".into(),
                client_id: None,
                content: vec![UserInput::Text {
                    text: "Please do the thing".into(),
                    text_elements: Vec::new(),
                }],
            }
        );
        assert_eq!(
            first_turn.messages[1],
            Message::AgentMessage {
                id: "item-2".into(),
                text: "Working...".into(),
                phase: None,
                memory_citation: None,
            }
        );

        let second_turn = &interactions[1];
        assert_eq!(second_turn.status, InteractionStatus::Completed);
        assert_eq!(second_turn.messages.len(), 2);
        assert_eq!(
            second_turn.messages[0],
            Message::UserMessage {
                id: "item-3".into(),
                client_id: None,
                content: vec![UserInput::Text {
                    text: "Let's try again".into(),
                    text_elements: Vec::new(),
                }],
            }
        );
        assert_eq!(
            second_turn.messages[1],
            Message::AgentMessage {
                id: "item-4".into(),
                text: "Second attempt complete.".into(),
                phase: None,
                memory_citation: None,
            }
        );
    }

    #[test]
    fn drops_last_turns_on_thread_rollback() {
        let events = vec![
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "First".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::AgentMessage(AgentMessageEvent {
                message: "A1".into(),
                phase: None,
                memory_citation: None,
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "Second".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::AgentMessage(AgentMessageEvent {
                message: "A2".into(),
                phase: None,
                memory_citation: None,
            }),
            EventMsg::ThreadRolledBack(ThreadRolledBackEvent { num_turns: 1 }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "Third".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::AgentMessage(AgentMessageEvent {
                message: "A3".into(),
                phase: None,
                memory_citation: None,
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 2);
        assert_eq!(interactions[0].id, "rollout-0");
        assert_eq!(interactions[1].id, "rollout-5");
        assert_ne!(interactions[0].id, interactions[1].id);
        assert_eq!(interactions[0].status, InteractionStatus::Completed);
        assert_eq!(interactions[1].status, InteractionStatus::Completed);
        assert_eq!(
            interactions[0].messages,
            vec![
                Message::UserMessage {
                    id: "item-1".into(),
                    client_id: None,
                    content: vec![UserInput::Text {
                        text: "First".into(),
                        text_elements: Vec::new(),
                    }],
                },
                Message::AgentMessage {
                    id: "item-2".into(),
                    text: "A1".into(),
                    phase: None,
                    memory_citation: None,
                },
            ]
        );
        assert_eq!(
            interactions[1].messages,
            vec![
                Message::UserMessage {
                    id: "item-3".into(),
                    client_id: None,
                    content: vec![UserInput::Text {
                        text: "Third".into(),
                        text_elements: Vec::new(),
                    }],
                },
                Message::AgentMessage {
                    id: "item-4".into(),
                    text: "A3".into(),
                    phase: None,
                    memory_citation: None,
                },
            ]
        );
    }

    #[test]
    fn thread_rollback_clears_all_turns_when_num_turns_exceeds_history() {
        let events = vec![
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "One".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::AgentMessage(AgentMessageEvent {
                message: "A1".into(),
                phase: None,
                memory_citation: None,
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "Two".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::AgentMessage(AgentMessageEvent {
                message: "A2".into(),
                phase: None,
                memory_citation: None,
            }),
            EventMsg::ThreadRolledBack(ThreadRolledBackEvent { num_turns: 99 }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions, Vec::<Interaction>::new());
    }

    #[test]
    fn uses_explicit_turn_boundaries_for_mid_turn_steering() {
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-a".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "Start".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "Steer".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-a".into(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].id, "turn-a");
        assert_eq!(
            interactions[0].messages,
            vec![
                Message::UserMessage {
                    id: "item-1".into(),
                    client_id: None,
                    content: vec![UserInput::Text {
                        text: "Start".into(),
                        text_elements: Vec::new(),
                    }],
                },
                Message::UserMessage {
                    id: "item-2".into(),
                    client_id: None,
                    content: vec![UserInput::Text {
                        text: "Steer".into(),
                        text_elements: Vec::new(),
                    }],
                },
            ]
        );
    }

    #[test]
    fn reconstructs_tool_items_from_persisted_completion_events() {
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-1".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "run tools".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::WebSearchEnd(WebSearchEndEvent {
                call_id: "search-1".into(),
                query: "codex".into(),
                action: CoreWebSearchAction::Search {
                    query: Some("codex".into()),
                    queries: None,
                },
            }),
            EventMsg::ExecCommandEnd(ExecCommandEndEvent {
                call_id: "exec-1".into(),
                process_id: Some("pid-1".into()),
                interaction_id: "turn-1".into(),
                completed_at_ms: 0,
                command: vec!["echo".into(), "hello world".into()],
                cwd: test_path_buf("/tmp").abs().into(),
                parsed_cmd: vec![ParsedCommand::Unknown {
                    cmd: "echo hello world".into(),
                }],
                source: ExecCommandSource::Agent,
                interaction_input: None,
                stdout: String::new(),
                stderr: String::new(),
                aggregated_output: "hello world\n".into(),
                exit_code: 0,
                duration: Duration::from_millis(12),
                formatted_output: String::new(),
                status: CoreExecCommandStatus::Completed,
            }),
            EventMsg::McpToolCallEnd(McpToolCallEndEvent {
                call_id: "mcp-1".into(),
                invocation: McpInvocation {
                    server: "docs".into(),
                    tool: "lookup".into(),
                    arguments: Some(serde_json::json!({"id":"123"})),
                },
                connector_id: None,
                mcp_app_resource_uri: None,
                link_id: None,
                plugin_id: None,
                duration: Duration::from_millis(8),
                result: Err("boom".into()),
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].messages.len(), 4);
        assert_eq!(
            interactions[0].messages[1],
            Message::WebSearch {
                id: "search-1".into(),
                query: "codex".into(),
                action: Some(WebSearchAction::Search {
                    query: Some("codex".into()),
                    queries: None,
                }),
            }
        );
        assert_eq!(
            interactions[0].messages[2],
            Message::CommandExecution {
                id: "exec-1".into(),
                command: "echo 'hello world'".into(),
                cwd: test_path_buf("/tmp").abs().into(),
                process_id: Some("pid-1".into()),
                source: CommandExecutionSource::Agent,
                status: CommandExecutionStatus::Completed,
                command_actions: vec![CommandAction::Unknown {
                    command: "echo hello world".into(),
                }],
                aggregated_output: Some("hello world\n".into()),
                exit_code: Some(0),
                duration_ms: Some(12),
            }
        );
        assert_eq!(
            interactions[0].messages[3],
            Message::McpToolCall {
                id: "mcp-1".into(),
                server: "docs".into(),
                tool: "lookup".into(),
                status: McpToolCallStatus::Failed,
                arguments: serde_json::json!({"id":"123"}),
                app_context: None,
                mcp_app_resource_uri: None,
                plugin_id: None,
                result: None,
                error: Some(McpToolCallError {
                    message: "boom".into(),
                }),
                duration_ms: Some(8),
            }
        );
    }

    #[test]
    fn reconstructs_mcp_tool_result_meta_from_persisted_completion_events() {
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-1".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::McpToolCallEnd(McpToolCallEndEvent {
                call_id: "mcp-1".into(),
                invocation: McpInvocation {
                    server: "docs".into(),
                    tool: "lookup".into(),
                    arguments: Some(serde_json::json!({"id":"123"})),
                },
                connector_id: Some("calendar".into()),
                mcp_app_resource_uri: Some("ui://widget/lookup.html".into()),
                link_id: Some("link_calendar".into()),
                plugin_id: Some("sample@test".into()),
                duration: Duration::from_millis(8),
                result: Ok(CallToolResult {
                    content: vec![serde_json::json!({
                        "type": "text",
                        "text": "result"
                    })],
                    structured_content: Some(serde_json::json!({"id":"123"})),
                    is_error: Some(false),
                    meta: Some(serde_json::json!({
                        "ui/resourceUri": "ui://widget/lookup.html"
                    })),
                }),
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(
            interactions[0].messages[0],
            Message::McpToolCall {
                id: "mcp-1".into(),
                server: "docs".into(),
                tool: "lookup".into(),
                status: McpToolCallStatus::Completed,
                arguments: serde_json::json!({"id":"123"}),
                app_context: Some(McpToolCallAppContext {
                    connector_id: "calendar".into(),
                    link_id: Some("link_calendar".into()),
                    resource_uri: Some("ui://widget/lookup.html".into()),
                }),
                mcp_app_resource_uri: Some("ui://widget/lookup.html".into()),
                plugin_id: Some("sample@test".into()),
                result: Some(Box::new(McpToolCallResult {
                    content: vec![serde_json::json!({
                        "type": "text",
                        "text": "result"
                    })],
                    structured_content: Some(serde_json::json!({"id":"123"})),
                    meta: Some(serde_json::json!({
                        "ui/resourceUri": "ui://widget/lookup.html"
                    })),
                })),
                error: None,
                duration_ms: Some(8),
            }
        );
    }

    #[test]
    fn reconstructs_dynamic_tool_items_from_request_and_response_events() {
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-1".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "run dynamic tool".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::DynamicToolCallRequest(
                datax_protocol::dynamic_tools::DynamicToolCallRequest {
                    call_id: "dyn-1".into(),
                    interaction_id: "turn-1".into(),
                    started_at_ms: 0,
                    namespace: Some("codex_app".into()),
                    tool: "lookup_ticket".into(),
                    arguments: serde_json::json!({"id":"ABC-123"}),
                },
            ),
            EventMsg::DynamicToolCallResponse(DynamicToolCallResponseEvent {
                call_id: "dyn-1".into(),
                interaction_id: "turn-1".into(),
                completed_at_ms: 0,
                namespace: Some("codex_app".into()),
                tool: "lookup_ticket".into(),
                arguments: serde_json::json!({"id":"ABC-123"}),
                content_items: vec![CoreDynamicToolCallOutputContentItem::InputText {
                    text: "Ticket is open".into(),
                }],
                success: true,
                error: None,
                duration: Duration::from_millis(42),
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].messages.len(), 2);
        assert_eq!(
            interactions[0].messages[1],
            Message::DynamicToolCall {
                id: "dyn-1".into(),
                namespace: Some("codex_app".into()),
                tool: "lookup_ticket".into(),
                arguments: serde_json::json!({"id":"ABC-123"}),
                status: DynamicToolCallStatus::Completed,
                content_items: Some(vec![DynamicToolCallOutputContentItem::InputText {
                    text: "Ticket is open".into(),
                }]),
                success: Some(true),
                duration_ms: Some(42),
            }
        );
    }

    #[test]
    fn reconstructs_declined_exec_and_patch_items() {
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-1".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "run tools".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::ExecCommandEnd(ExecCommandEndEvent {
                call_id: "exec-declined".into(),
                process_id: Some("pid-2".into()),
                interaction_id: "turn-1".into(),
                completed_at_ms: 0,
                command: vec!["ls".into()],
                cwd: test_path_buf("/tmp").abs().into(),
                parsed_cmd: vec![ParsedCommand::Unknown { cmd: "ls".into() }],
                source: ExecCommandSource::Agent,
                interaction_input: None,
                stdout: String::new(),
                stderr: "exec command rejected by user".into(),
                aggregated_output: "exec command rejected by user".into(),
                exit_code: -1,
                duration: Duration::ZERO,
                formatted_output: String::new(),
                status: CoreExecCommandStatus::Declined,
            }),
            EventMsg::PatchApplyEnd(PatchApplyEndEvent {
                call_id: "patch-declined".into(),
                interaction_id: "turn-1".into(),
                stdout: String::new(),
                stderr: "patch rejected by user".into(),
                success: false,
                changes: [(
                    PathBuf::from("README.md"),
                    datax_protocol::protocol::FileChange::Add {
                        content: "hello\n".into(),
                    },
                )]
                .into_iter()
                .collect(),
                status: CorePatchApplyStatus::Declined,
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].messages.len(), 3);
        assert_eq!(
            interactions[0].messages[1],
            Message::CommandExecution {
                id: "exec-declined".into(),
                command: "ls".into(),
                cwd: test_path_buf("/tmp").abs().into(),
                process_id: Some("pid-2".into()),
                source: CommandExecutionSource::Agent,
                status: CommandExecutionStatus::Declined,
                command_actions: vec![CommandAction::Unknown {
                    command: "ls".into(),
                }],
                aggregated_output: Some("exec command rejected by user".into()),
                exit_code: Some(-1),
                duration_ms: Some(0),
            }
        );
        assert_eq!(
            interactions[0].messages[2],
            Message::FileChange {
                id: "patch-declined".into(),
                changes: vec![FileUpdateChange {
                    path: "README.md".into(),
                    kind: PatchChangeKind::Add,
                    diff: "hello\n".into(),
                }],
                status: PatchApplyStatus::Declined,
            }
        );
    }

    #[test]
    fn reconstructs_declined_guardian_command_item() {
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-1".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "review this command".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::GuardianAssessment(GuardianAssessmentEvent {
                id: "review-guardian-exec".into(),
                target_item_id: Some("guardian-exec".into()),
                interaction_id: "turn-1".into(),
                started_at_ms: 1_000,
                completed_at_ms: None,
                status: GuardianAssessmentStatus::InProgress,
                risk_level: None,
                user_authorization: None,
                rationale: None,
                decision_source: None,
                action: serde_json::from_value(serde_json::json!({
                    "type": "command",
                    "source": "shell",
                    "command": "rm -rf /tmp/guardian",
                    "cwd": test_path_buf("/tmp"),
                }))
                .expect("guardian action"),
            }),
            EventMsg::GuardianAssessment(GuardianAssessmentEvent {
                id: "review-guardian-exec".into(),
                target_item_id: Some("guardian-exec".into()),
                interaction_id: "turn-1".into(),
                started_at_ms: 1_000,
                completed_at_ms: Some(1_042),
                status: GuardianAssessmentStatus::Denied,
                risk_level: Some(datax_protocol::protocol::GuardianRiskLevel::High),
                user_authorization: Some(datax_protocol::protocol::GuardianUserAuthorization::Low),
                rationale: Some("Would delete user data.".into()),
                decision_source: Some(
                    datax_protocol::protocol::GuardianAssessmentDecisionSource::Agent,
                ),
                action: serde_json::from_value(serde_json::json!({
                    "type": "command",
                    "source": "shell",
                    "command": "rm -rf /tmp/guardian",
                    "cwd": test_path_buf("/tmp"),
                }))
                .expect("guardian action"),
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].messages.len(), 2);
        assert_eq!(
            interactions[0].messages[1],
            Message::CommandExecution {
                id: "guardian-exec".into(),
                command: "rm -rf /tmp/guardian".into(),
                cwd: test_path_buf("/tmp").abs().into(),
                process_id: None,
                source: CommandExecutionSource::Agent,
                status: CommandExecutionStatus::Declined,
                command_actions: vec![CommandAction::Unknown {
                    command: "rm -rf /tmp/guardian".into(),
                }],
                aggregated_output: None,
                exit_code: None,
                duration_ms: None,
            }
        );
    }

    #[test]
    fn reconstructs_in_progress_guardian_execve_item() {
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-1".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "run a subcommand".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::GuardianAssessment(GuardianAssessmentEvent {
                id: "review-guardian-execve".into(),
                target_item_id: Some("guardian-execve".into()),
                interaction_id: "turn-1".into(),
                started_at_ms: 2_000,
                completed_at_ms: None,
                status: GuardianAssessmentStatus::InProgress,
                risk_level: None,
                user_authorization: None,
                rationale: None,
                decision_source: None,
                action: serde_json::from_value(serde_json::json!({
                    "type": "execve",
                    "source": "shell",
                    "program": "/bin/rm",
                    "argv": ["/usr/bin/rm", "-f", "/tmp/file.sqlite"],
                    "cwd": test_path_buf("/tmp"),
                }))
                .expect("guardian action"),
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].messages.len(), 2);
        assert_eq!(
            interactions[0].messages[1],
            Message::CommandExecution {
                id: "guardian-execve".into(),
                command: "/bin/rm -f /tmp/file.sqlite".into(),
                cwd: test_path_buf("/tmp").abs().into(),
                process_id: None,
                source: CommandExecutionSource::Agent,
                status: CommandExecutionStatus::InProgress,
                command_actions: vec![CommandAction::Unknown {
                    command: "/bin/rm -f /tmp/file.sqlite".into(),
                }],
                aggregated_output: None,
                exit_code: None,
                duration_ms: None,
            }
        );
    }

    #[test]
    fn assigns_late_exec_completion_to_original_turn() {
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-a".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "first".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-a".into(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-b".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "second".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::ExecCommandEnd(ExecCommandEndEvent {
                call_id: "exec-late".into(),
                process_id: Some("pid-42".into()),
                interaction_id: "turn-a".into(),
                completed_at_ms: 0,
                command: vec!["echo".into(), "done".into()],
                cwd: test_path_buf("/tmp").abs().into(),
                parsed_cmd: vec![ParsedCommand::Unknown {
                    cmd: "echo done".into(),
                }],
                source: ExecCommandSource::Agent,
                interaction_input: None,
                stdout: "done\n".into(),
                stderr: String::new(),
                aggregated_output: "done\n".into(),
                exit_code: 0,
                duration: Duration::from_millis(5),
                formatted_output: "done\n".into(),
                status: CoreExecCommandStatus::Completed,
            }),
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-b".into(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 2);
        assert_eq!(interactions[0].id, "turn-a");
        assert_eq!(interactions[1].id, "turn-b");
        assert_eq!(interactions[0].messages.len(), 2);
        assert_eq!(interactions[1].messages.len(), 1);
        assert_eq!(
            interactions[0].messages[1],
            Message::CommandExecution {
                id: "exec-late".into(),
                command: "echo done".into(),
                cwd: test_path_buf("/tmp").abs().into(),
                process_id: Some("pid-42".into()),
                source: CommandExecutionSource::Agent,
                status: CommandExecutionStatus::Completed,
                command_actions: vec![CommandAction::Unknown {
                    command: "echo done".into(),
                }],
                aggregated_output: Some("done\n".into()),
                exit_code: Some(0),
                duration_ms: Some(5),
            }
        );
    }

    #[test]
    fn drops_late_turn_scoped_item_for_unknown_interaction_id() {
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-a".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "first".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-a".into(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-b".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "second".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::ExecCommandEnd(ExecCommandEndEvent {
                call_id: "exec-unknown-turn".into(),
                process_id: Some("pid-42".into()),
                interaction_id: "turn-missing".into(),
                completed_at_ms: 0,
                command: vec!["echo".into(), "done".into()],
                cwd: test_path_buf("/tmp").abs().into(),
                parsed_cmd: vec![ParsedCommand::Unknown {
                    cmd: "echo done".into(),
                }],
                source: ExecCommandSource::Agent,
                interaction_input: None,
                stdout: "done\n".into(),
                stderr: String::new(),
                aggregated_output: "done\n".into(),
                exit_code: 0,
                duration: Duration::from_millis(5),
                formatted_output: "done\n".into(),
                status: CoreExecCommandStatus::Completed,
            }),
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-b".into(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
        ];

        let mut builder = ChatHistoryBuilder::new();
        for event in &events {
            builder.handle_event(event);
        }
        let interactions = builder.finish();
        assert_eq!(interactions.len(), 2);
        assert_eq!(interactions[0].id, "turn-a");
        assert_eq!(interactions[1].id, "turn-b");
        assert_eq!(interactions[0].messages.len(), 1);
        assert_eq!(interactions[1].messages.len(), 1);
        assert_eq!(
            interactions[1].messages[0],
            Message::UserMessage {
                id: "item-2".into(),
                client_id: None,
                content: vec![UserInput::Text {
                    text: "second".into(),
                    text_elements: Vec::new(),
                }],
            }
        );
    }

    #[test]
    fn patch_apply_begin_updates_active_turn_snapshot_with_file_change() {
        let interaction_id = "turn-1";
        let mut builder = ChatHistoryBuilder::new();
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: interaction_id.to_string(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "apply patch".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::PatchApplyBegin(PatchApplyBeginEvent {
                call_id: "patch-call".into(),
                interaction_id: interaction_id.to_string(),
                auto_approved: false,
                changes: [(
                    PathBuf::from("README.md"),
                    datax_protocol::protocol::FileChange::Add {
                        content: "hello\n".into(),
                    },
                )]
                .into_iter()
                .collect(),
            }),
        ];

        for event in &events {
            builder.handle_event(event);
        }

        let snapshot = builder
            .active_turn_snapshot()
            .expect("active turn snapshot");
        assert_eq!(snapshot.id, interaction_id);
        assert_eq!(snapshot.status, InteractionStatus::InProgress);
        assert_eq!(
            snapshot.messages,
            vec![
                Message::UserMessage {
                    id: "item-1".into(),
                    client_id: None,
                    content: vec![UserInput::Text {
                        text: "apply patch".into(),
                        text_elements: Vec::new(),
                    }],
                },
                Message::FileChange {
                    id: "patch-call".into(),
                    changes: vec![FileUpdateChange {
                        path: "README.md".into(),
                        kind: PatchChangeKind::Add,
                        diff: "hello\n".into(),
                    }],
                    status: PatchApplyStatus::InProgress,
                },
            ]
        );
    }

    #[test]
    fn apply_patch_approval_request_updates_active_turn_snapshot_with_file_change() {
        let interaction_id = "turn-1";
        let mut builder = ChatHistoryBuilder::new();
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: interaction_id.to_string(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "apply patch".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::ApplyPatchApprovalRequest(ApplyPatchApprovalRequestEvent {
                call_id: "patch-call".into(),
                interaction_id: interaction_id.to_string(),
                started_at_ms: 0,
                changes: [(
                    PathBuf::from("README.md"),
                    datax_protocol::protocol::FileChange::Add {
                        content: "hello\n".into(),
                    },
                )]
                .into_iter()
                .collect(),
                reason: None,
                grant_root: None,
            }),
        ];

        for event in &events {
            builder.handle_event(event);
        }

        let snapshot = builder
            .active_turn_snapshot()
            .expect("active turn snapshot");
        assert_eq!(snapshot.id, interaction_id);
        assert_eq!(snapshot.status, InteractionStatus::InProgress);
        assert_eq!(
            snapshot.messages,
            vec![
                Message::UserMessage {
                    id: "item-1".into(),
                    client_id: None,
                    content: vec![UserInput::Text {
                        text: "apply patch".into(),
                        text_elements: Vec::new(),
                    }],
                },
                Message::FileChange {
                    id: "patch-call".into(),
                    changes: vec![FileUpdateChange {
                        path: "README.md".into(),
                        kind: PatchChangeKind::Add,
                        diff: "hello\n".into(),
                    }],
                    status: PatchApplyStatus::InProgress,
                },
            ]
        );
    }

    #[test]
    fn late_turn_complete_does_not_close_active_turn() {
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-a".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "first".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-a".into(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-b".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "second".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-a".into(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
            EventMsg::AgentMessage(AgentMessageEvent {
                message: "still in b".into(),
                phase: None,
                memory_citation: None,
            }),
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-b".into(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 2);
        assert_eq!(interactions[0].id, "turn-a");
        assert_eq!(interactions[1].id, "turn-b");
        assert_eq!(interactions[1].messages.len(), 2);
    }

    #[test]
    fn late_turn_aborted_does_not_interrupt_active_turn() {
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-a".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "first".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-a".into(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-b".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "second".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::InteractionAborted(InteractionAbortedEvent {
                interaction_id: Some("turn-a".into()),
                reason: InteractionAbortReason::Replaced,
                completed_at: None,
                duration_ms: None,
            }),
            EventMsg::AgentMessage(AgentMessageEvent {
                message: "still in b".into(),
                phase: None,
                memory_citation: None,
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 2);
        assert_eq!(interactions[0].id, "turn-a");
        assert_eq!(interactions[1].id, "turn-b");
        assert_eq!(interactions[1].status, InteractionStatus::InProgress);
        assert_eq!(interactions[1].messages.len(), 2);
    }

    #[test]
    fn preserves_compaction_only_turn() {
        let messages = vec![
            RolloutItem::EventMsg(EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-compact".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            })),
            RolloutItem::Compacted(CompactedItem {
                message: String::new(),
                replacement_history: None,
                window_number: None,
                first_window_id: None,
                previous_window_id: None,
                window_id: None,
            }),
            RolloutItem::EventMsg(EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-compact".into(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            })),
        ];

        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(
            interactions,
            vec![Interaction {
                id: "turn-compact".into(),
                status: InteractionStatus::Completed,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                messages_view: InteractionMessagesView::Full,
                messages: Vec::new(),
            }]
        );
    }

    #[test]
    fn reconstructs_collab_resume_end_item() {
        let events = vec![
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "resume agent".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::CollabResumeEnd(datax_protocol::protocol::CollabResumeEndEvent {
                call_id: "resume-1".into(),
                completed_at_ms: 0,
                sender_chat_id: ChatId::try_from("00000000-0000-0000-0000-000000000001")
                    .expect("valid sender thread id"),
                receiver_chat_id: ChatId::try_from("00000000-0000-0000-0000-000000000002")
                    .expect("valid receiver thread id"),
                receiver_agent_nickname: None,
                receiver_agent_role: None,
                status: AgentStatus::Completed(None),
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].messages.len(), 2);
        assert_eq!(
            interactions[0].messages[1],
            Message::CollabAgentToolCall {
                id: "resume-1".into(),
                tool: CollabAgentTool::ResumeAgent,
                status: CollabAgentToolCallStatus::Completed,
                sender_chat_id: "00000000-0000-0000-0000-000000000001".into(),
                receiver_chat_ids: vec!["00000000-0000-0000-0000-000000000002".into()],
                prompt: None,
                model: None,
                reasoning_effort: None,
                agents_states: [(
                    "00000000-0000-0000-0000-000000000002".into(),
                    CollabAgentState {
                        status: crate::protocol::v2::CollabAgentStatus::Completed,
                        message: None,
                    },
                )]
                .into_iter()
                .collect(),
            }
        );
    }

    #[test]
    fn reconstructs_collab_spawn_end_item_with_model_metadata() {
        let sender_chat_id = ChatId::try_from("00000000-0000-0000-0000-000000000001")
            .expect("valid sender thread id");
        let spawned_chat_id = ChatId::try_from("00000000-0000-0000-0000-000000000002")
            .expect("valid receiver thread id");
        let events = vec![
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "spawn agent".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::CollabAgentSpawnEnd(datax_protocol::protocol::CollabAgentSpawnEndEvent {
                call_id: "spawn-1".into(),
                completed_at_ms: 0,
                sender_chat_id,
                new_chat_id: Some(spawned_chat_id),
                new_agent_nickname: Some("Scout".into()),
                new_agent_role: Some("explorer".into()),
                prompt: "inspect the repo".into(),
                model: "gpt-5.4-mini".into(),
                reasoning_effort: datax_protocol::openai_models::ReasoningEffort::Medium,
                status: AgentStatus::Running,
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].messages.len(), 2);
        assert_eq!(
            interactions[0].messages[1],
            Message::CollabAgentToolCall {
                id: "spawn-1".into(),
                tool: CollabAgentTool::SpawnAgent,
                status: CollabAgentToolCallStatus::Completed,
                sender_chat_id: "00000000-0000-0000-0000-000000000001".into(),
                receiver_chat_ids: vec!["00000000-0000-0000-0000-000000000002".into()],
                prompt: Some("inspect the repo".into()),
                model: Some("gpt-5.4-mini".into()),
                reasoning_effort: Some(datax_protocol::openai_models::ReasoningEffort::Medium),
                agents_states: [(
                    "00000000-0000-0000-0000-000000000002".into(),
                    CollabAgentState {
                        status: crate::protocol::v2::CollabAgentStatus::Running,
                        message: None,
                    },
                )]
                .into_iter()
                .collect(),
            }
        );
    }

    #[test]
    fn reconstructs_interrupted_send_input_as_completed_collab_call() {
        // `send_input(interrupt=true)` first stops the child's active turn, then redirects it with
        // new input. The transient interrupted status should remain visible in agent state, but the
        // collab tool call itself is still a successful redirect rather than a failed operation.
        let sender = ChatId::try_from("00000000-0000-0000-0000-000000000001")
            .expect("valid sender thread id");
        let receiver = ChatId::try_from("00000000-0000-0000-0000-000000000002")
            .expect("valid receiver thread id");
        let events = vec![
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "redirect".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::CollabAgentInteractionBegin(
                datax_protocol::protocol::CollabAgentInteractionBeginEvent {
                    call_id: "send-1".into(),
                    started_at_ms: 0,
                    sender_chat_id: sender,
                    receiver_chat_id: receiver,
                    prompt: "new task".into(),
                },
            ),
            EventMsg::CollabAgentInteractionEnd(
                datax_protocol::protocol::CollabAgentInteractionEndEvent {
                    call_id: "send-1".into(),
                    completed_at_ms: 0,
                    sender_chat_id: sender,
                    receiver_chat_id: receiver,
                    receiver_agent_nickname: None,
                    receiver_agent_role: None,
                    prompt: "new task".into(),
                    status: AgentStatus::Interrupted,
                },
            ),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].messages.len(), 2);
        assert_eq!(
            interactions[0].messages[1],
            Message::CollabAgentToolCall {
                id: "send-1".into(),
                tool: CollabAgentTool::SendInput,
                status: CollabAgentToolCallStatus::Completed,
                sender_chat_id: sender.to_string(),
                receiver_chat_ids: vec![receiver.to_string()],
                prompt: Some("new task".into()),
                model: None,
                reasoning_effort: None,
                agents_states: [(
                    receiver.to_string(),
                    CollabAgentState {
                        status: crate::protocol::v2::CollabAgentStatus::Interrupted,
                        message: None,
                    },
                )]
                .into_iter()
                .collect(),
            }
        );
    }

    #[test]
    fn rollback_failed_error_does_not_mark_turn_failed() {
        let events = vec![
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "hello".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::AgentMessage(AgentMessageEvent {
                message: "done".into(),
                phase: None,
                memory_citation: None,
            }),
            EventMsg::Error(ErrorEvent {
                message: "rollback failed".into(),
                codex_error_info: Some(CodexErrorInfo::ThreadRollbackFailed),
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].status, InteractionStatus::Completed);
        assert_eq!(interactions[0].error, None);
    }

    #[test]
    fn out_of_turn_error_does_not_create_or_fail_a_turn() {
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-a".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "hello".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-a".into(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
            EventMsg::Error(ErrorEvent {
                message: "request-level failure".into(),
                codex_error_info: Some(CodexErrorInfo::BadRequest),
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(
            interactions[0],
            Interaction {
                id: "turn-a".into(),
                status: InteractionStatus::Completed,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                messages_view: InteractionMessagesView::Full,
                messages: vec![Message::UserMessage {
                    id: "item-1".into(),
                    client_id: None,
                    content: vec![UserInput::Text {
                        text: "hello".into(),
                        text_elements: Vec::new(),
                    }],
                }],
            }
        );
    }

    #[test]
    fn error_then_turn_complete_preserves_failed_status() {
        let events = vec![
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-a".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
            EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "hello".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
            EventMsg::Error(ErrorEvent {
                message: "stream failure".into(),
                codex_error_info: Some(CodexErrorInfo::ResponseStreamDisconnected {
                    http_status_code: Some(502),
                }),
            }),
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-a".into(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
        ];

        let messages = events
            .into_iter()
            .map(RolloutItem::EventMsg)
            .collect::<Vec<_>>();
        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].id, "turn-a");
        assert_eq!(interactions[0].status, InteractionStatus::Failed);
        assert_eq!(
            interactions[0].error,
            Some(InteractionError {
                message: "stream failure".into(),
                codex_error_info: Some(
                    crate::protocol::v2::CodexErrorInfo::ResponseStreamDisconnected {
                        http_status_code: Some(502),
                    }
                ),
                additional_details: None,
            })
        );
    }

    #[test]
    fn rebuilds_hook_prompt_items_from_rollout_response_items() {
        let hook_prompt = build_hook_prompt_message(&[
            CoreHookPromptFragment::from_single_hook("Retry with tests.", "hook-run-1"),
            CoreHookPromptFragment::from_single_hook("Then summarize cleanly.", "hook-run-2"),
        ])
        .expect("hook prompt message");
        let messages = vec![
            RolloutItem::EventMsg(EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-a".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            })),
            RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "hello".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            })),
            RolloutItem::ResponseItem(hook_prompt),
            RolloutItem::EventMsg(EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-a".into(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            })),
        ];

        let interactions = build_turns_from_rollout_items(&messages);

        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].messages.len(), 2);
        assert_eq!(
            interactions[0].messages[1],
            Message::HookPrompt {
                id: interactions[0].messages[1].id().to_string(),
                fragments: vec![
                    crate::protocol::v2::HookPromptFragment {
                        text: "Retry with tests.".into(),
                        hook_run_id: "hook-run-1".into(),
                    },
                    crate::protocol::v2::HookPromptFragment {
                        text: "Then summarize cleanly.".into(),
                        hook_run_id: "hook-run-2".into(),
                    },
                ],
            }
        );
    }

    #[test]
    fn ignores_plain_user_response_items_in_rollout_replay() {
        let messages = vec![
            RolloutItem::EventMsg(EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-a".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            })),
            RolloutItem::ResponseItem(datax_protocol::models::ResponseItem::Message {
                id: Some("msg-1".into()),
                role: "user".into(),
                content: vec![datax_protocol::models::ContentItem::InputText {
                    text: "plain text".into(),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            }),
            RolloutItem::EventMsg(EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-a".into(),
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            })),
        ];

        let interactions = build_turns_from_rollout_items(&messages);
        assert_eq!(interactions.len(), 1);
        assert!(interactions[0].messages.is_empty());
    }

    #[test]
    fn changed_rollout_item_reports_new_item_snapshot() {
        let mut builder = ChatHistoryBuilder::new();

        let changes = builder.handle_rollout_item_with_changes(&RolloutItem::EventMsg(
            EventMsg::UserMessage(UserMessageEvent {
                client_id: Some("client-message-1".into()),
                message: "hello".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            }),
        ));

        assert_eq!(
            changes,
            ChatHistoryChangeSet {
                changed_items: vec![ChatHistoryMessageChange {
                    interaction_id: "rollout-0".into(),
                    item: Message::UserMessage {
                        id: "item-1".into(),
                        client_id: Some("client-message-1".into()),
                        content: vec![UserInput::Text {
                            text: "hello".into(),
                            text_elements: Vec::new(),
                        }],
                    },
                }],
                changed_turns: vec![ChatHistoryInteractionChange {
                    interaction_id: "rollout-0".into(),
                    status: InteractionStatus::Completed,
                    error: None,
                    started_at: None,
                    completed_at: None,
                    duration_ms: None,
                }],
                removed_interaction_ids: Vec::new(),
            }
        );
    }

    #[test]
    fn changed_rollout_item_reports_updated_existing_item_snapshot() {
        let mut builder = ChatHistoryBuilder::new();
        builder.handle_rollout_item_with_changes(&RolloutItem::EventMsg(EventMsg::WebSearchBegin(
            WebSearchBeginEvent {
                call_id: "search-1".into(),
            },
        )));

        let changes = builder.handle_rollout_item_with_changes(&RolloutItem::EventMsg(
            EventMsg::WebSearchEnd(WebSearchEndEvent {
                call_id: "search-1".into(),
                query: "codex".into(),
                action: CoreWebSearchAction::Search {
                    query: Some("codex".into()),
                    queries: None,
                },
            }),
        ));

        assert_eq!(
            changes,
            ChatHistoryChangeSet {
                changed_items: vec![ChatHistoryMessageChange {
                    interaction_id: "rollout-0".into(),
                    item: Message::WebSearch {
                        id: "search-1".into(),
                        query: "codex".into(),
                        action: Some(WebSearchAction::Search {
                            query: Some("codex".into()),
                            queries: None,
                        }),
                    },
                }],
                changed_turns: Vec::new(),
                removed_interaction_ids: Vec::new(),
            }
        );
    }

    #[test]
    fn changed_rollout_item_reports_streaming_item_mutation() {
        let mut builder = ChatHistoryBuilder::new();
        builder.handle_rollout_item_with_changes(&RolloutItem::EventMsg(EventMsg::AgentReasoning(
            AgentReasoningEvent {
                text: "summary".into(),
            },
        )));

        let changes = builder.handle_rollout_item_with_changes(&RolloutItem::EventMsg(
            EventMsg::AgentReasoningRawContent(AgentReasoningRawContentEvent {
                text: "raw content".into(),
            }),
        ));

        assert_eq!(
            changes,
            ChatHistoryChangeSet {
                changed_items: vec![ChatHistoryMessageChange {
                    interaction_id: "rollout-0".into(),
                    item: Message::Reasoning {
                        id: "item-1".into(),
                        summary: vec!["summary".into()],
                        content: vec!["raw content".into()],
                    },
                }],
                changed_turns: Vec::new(),
                removed_interaction_ids: Vec::new(),
            }
        );
    }

    #[test]
    fn changed_rollout_item_reports_turn_completion_metadata() {
        let mut builder = ChatHistoryBuilder::new();

        let start_changes = builder.handle_rollout_item_with_changes(&RolloutItem::EventMsg(
            EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-a".into(),
                trace_id: None,
                started_at: Some(10),
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            }),
        ));
        assert_eq!(
            start_changes,
            ChatHistoryChangeSet {
                changed_items: Vec::new(),
                changed_turns: vec![ChatHistoryInteractionChange {
                    interaction_id: "turn-a".into(),
                    status: InteractionStatus::InProgress,
                    error: None,
                    started_at: Some(10),
                    completed_at: None,
                    duration_ms: None,
                }],
                removed_interaction_ids: Vec::new(),
            }
        );

        builder.handle_rollout_item_with_changes(&RolloutItem::EventMsg(EventMsg::UserMessage(
            UserMessageEvent {
                client_id: None,
                message: "hello".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            },
        )));
        let complete_changes = builder.handle_rollout_item_with_changes(&RolloutItem::EventMsg(
            EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-a".into(),
                last_agent_message: None,
                completed_at: Some(20),
                duration_ms: Some(123),
                time_to_first_token_ms: None,
            }),
        ));

        assert_eq!(
            complete_changes,
            ChatHistoryChangeSet {
                changed_items: Vec::new(),
                changed_turns: vec![ChatHistoryInteractionChange {
                    interaction_id: "turn-a".into(),
                    status: InteractionStatus::Completed,
                    error: None,
                    started_at: Some(10),
                    completed_at: Some(20),
                    duration_ms: Some(123),
                }],
                removed_interaction_ids: Vec::new(),
            }
        );
    }

    #[test]
    fn changed_rollout_items_dedupe_updated_item_snapshots() {
        let mut builder = ChatHistoryBuilder::new();
        let changes = builder.handle_rollout_items_with_changes(&[
            RolloutItem::EventMsg(EventMsg::WebSearchBegin(WebSearchBeginEvent {
                call_id: "search-1".into(),
            })),
            RolloutItem::EventMsg(EventMsg::WebSearchEnd(WebSearchEndEvent {
                call_id: "search-1".into(),
                query: "codex".into(),
                action: CoreWebSearchAction::Search {
                    query: Some("codex".into()),
                    queries: None,
                },
            })),
        ]);

        assert_eq!(
            changes,
            ChatHistoryChangeSet {
                changed_items: vec![ChatHistoryMessageChange {
                    interaction_id: "rollout-0".into(),
                    item: Message::WebSearch {
                        id: "search-1".into(),
                        query: "codex".into(),
                        action: Some(WebSearchAction::Search {
                            query: Some("codex".into()),
                            queries: None,
                        }),
                    },
                }],
                changed_turns: vec![ChatHistoryInteractionChange {
                    interaction_id: "rollout-0".into(),
                    status: InteractionStatus::Completed,
                    error: None,
                    started_at: None,
                    completed_at: None,
                    duration_ms: None,
                }],
                removed_interaction_ids: Vec::new(),
            }
        );
    }

    #[test]
    fn changed_rollout_items_dedupe_turn_metadata_snapshots() {
        let mut builder = ChatHistoryBuilder::new();
        let changes = builder.handle_rollout_items_with_changes(&[
            RolloutItem::EventMsg(EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-a".into(),
                trace_id: None,
                started_at: Some(10),
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            })),
            RolloutItem::EventMsg(EventMsg::InteractionComplete(InteractionCompleteEvent {
                interaction_id: "turn-a".into(),
                last_agent_message: None,
                completed_at: Some(20),
                duration_ms: Some(123),
                time_to_first_token_ms: None,
            })),
        ]);

        assert_eq!(
            changes,
            ChatHistoryChangeSet {
                changed_items: Vec::new(),
                changed_turns: vec![ChatHistoryInteractionChange {
                    interaction_id: "turn-a".into(),
                    status: InteractionStatus::Completed,
                    error: None,
                    started_at: Some(10),
                    completed_at: Some(20),
                    duration_ms: Some(123),
                }],
                removed_interaction_ids: Vec::new(),
            }
        );
    }

    #[test]
    fn changed_rollout_items_drop_prior_changes_for_removed_turns() {
        let mut builder = ChatHistoryBuilder::new();
        let changes = builder.handle_rollout_items_with_changes(&[
            RolloutItem::EventMsg(EventMsg::InteractionStarted(InteractionStartedEvent {
                interaction_id: "turn-a".into(),
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: Default::default(),
            })),
            RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "hello".into(),
                images: None,
                text_elements: Vec::new(),
                local_images: Vec::new(),
                ..Default::default()
            })),
            RolloutItem::EventMsg(EventMsg::ThreadRolledBack(ThreadRolledBackEvent {
                num_turns: 1,
            })),
        ]);

        assert_eq!(
            changes,
            ChatHistoryChangeSet {
                changed_items: Vec::new(),
                changed_turns: Vec::new(),
                removed_interaction_ids: vec!["turn-a".into()],
            }
        );
    }
}
