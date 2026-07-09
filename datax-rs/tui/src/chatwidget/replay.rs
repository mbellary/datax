//! Chat replay rendering for `ChatWidget`.
//!
//! This module rehydrates turns and items into transcript state while avoiding
//! live-only side effects.

use super::*;

impl ChatWidget {
    /// Replay a subset of initial events into the UI to seed the transcript when
    /// resuming an existing session. This approximates the live event flow and
    /// is intentionally conservative: only safe-to-replay items are rendered to
    /// avoid triggering side effects. Event ids are passed as `None` to
    /// distinguish replayed events from live ones.
    pub(crate) fn replay_thread_turns(&mut self, turns: Vec<Interaction>, replay_kind: ReplayKind) {
        for turn in turns {
            let Interaction {
                id: interaction_id,
                messages_view: _,
                messages,
                status,
                error,
                started_at,
                completed_at,
                duration_ms,
            } = turn;
            if matches!(status, InteractionStatus::InProgress) {
                self.last_non_retry_error = None;
                self.on_task_started();
            }
            for item in messages {
                self.replay_thread_item(item, interaction_id.clone(), replay_kind);
            }
            if matches!(
                status,
                InteractionStatus::Completed
                    | InteractionStatus::Interrupted
                    | InteractionStatus::Failed
            ) {
                self.handle_turn_completed_notification(
                    InteractionCompletedNotification {
                        chat_id: self.chat_id.map(|id| id.to_string()).unwrap_or_default(),
                        turn: Interaction {
                            id: interaction_id,
                            messages_view:
                                datax_app_server_protocol::InteractionMessagesView::NotLoaded,
                            messages: Vec::new(),
                            status,
                            error,
                            started_at,
                            completed_at,
                            duration_ms,
                        },
                    },
                    Some(replay_kind),
                );
            }
        }
    }

    pub(crate) fn replay_thread_item(
        &mut self,
        item: Message,
        interaction_id: String,
        replay_kind: ReplayKind,
    ) {
        self.handle_thread_item(item, interaction_id, ThreadItemRenderSource::Replay(replay_kind));
    }

    pub(super) fn handle_thread_item(
        &mut self,
        item: Message,
        interaction_id: String,
        render_source: ThreadItemRenderSource,
    ) {
        let from_replay = render_source.is_replay();
        let replay_kind = render_source.replay_kind();
        match item {
            Message::UserMessage { content, .. } => {
                self.on_committed_user_message(&content, from_replay);
            }
            Message::AgentMessage {
                id,
                text,
                phase,
                memory_citation,
            } => {
                self.on_agent_message_item_completed(
                    AgentMessageItem {
                        id,
                        content: vec![AgentMessageContent::Text { text }],
                        phase,
                        memory_citation: memory_citation.map(|citation| {
                            datax_protocol::memory_citation::MemoryCitation {
                                entries: citation
                                    .entries
                                    .into_iter()
                                    .map(|entry| {
                                        datax_protocol::memory_citation::MemoryCitationEntry {
                                            path: entry.path,
                                            line_start: entry.line_start,
                                            line_end: entry.line_end,
                                            note: entry.note,
                                        }
                                    })
                                    .collect(),
                                rollout_ids: citation.chat_ids,
                            }
                        }),
                    },
                    from_replay,
                );
            }
            Message::Plan { text, .. } => self.on_plan_item_completed(text),
            Message::Reasoning {
                summary, content, ..
            } => {
                if from_replay {
                    for delta in summary {
                        self.on_agent_reasoning_delta(delta);
                    }
                    if self.config.show_raw_agent_reasoning {
                        for delta in content {
                            self.on_agent_reasoning_delta(delta);
                        }
                    }
                }
                self.on_agent_reasoning_final();
            }
            item @ Message::CommandExecution {
                status: datax_app_server_protocol::CommandExecutionStatus::InProgress,
                ..
            } => self.on_command_execution_started(item),
            item @ Message::CommandExecution { .. } => self.on_command_execution_completed(item),
            Message::FileChange {
                status: datax_app_server_protocol::PatchApplyStatus::InProgress,
                ..
            } => {}
            item @ Message::FileChange { .. } => self.on_file_change_completed(item),
            item @ Message::McpToolCall {
                status: datax_app_server_protocol::McpToolCallStatus::InProgress,
                ..
            } => self.on_mcp_tool_call_started(item),
            item @ Message::McpToolCall { .. } => self.on_mcp_tool_call_completed(item),
            Message::WebSearch { id, query, action } => {
                self.on_web_search_begin(id.clone());
                self.on_web_search_end(
                    id,
                    query,
                    action.unwrap_or(datax_app_server_protocol::WebSearchAction::Other),
                );
            }
            Message::ImageView { id: _, path } => {
                self.on_view_image_tool_call(path);
            }
            Message::ImageGeneration {
                id,
                status,
                revised_prompt,
                saved_path,
                ..
            } => {
                self.on_image_generation_end(id, status, revised_prompt, saved_path);
            }
            Message::EnteredReviewMode { review, .. } => {
                if from_replay {
                    self.enter_review_mode_with_hint(review, /*from_replay*/ true);
                }
            }
            Message::ExitedReviewMode { .. } => {
                self.exit_review_mode_after_item();
            }
            Message::ContextCompaction { .. } => {
                self.add_info_message("Context compacted".to_string(), /*hint*/ None);
            }
            Message::HookPrompt { .. } => {}
            Message::CollabAgentToolCall {
                id,
                tool,
                status,
                sender_chat_id,
                receiver_chat_ids,
                prompt,
                model,
                reasoning_effort,
                agents_states,
            } => self.on_collab_agent_tool_call(Message::CollabAgentToolCall {
                id,
                tool,
                status,
                sender_chat_id,
                receiver_chat_ids,
                prompt,
                model,
                reasoning_effort,
                agents_states,
            }),
            item @ Message::SubAgentActivity { .. } => self.on_sub_agent_activity(item),
            Message::DynamicToolCall { .. } => {}
            Message::Sleep { .. } => {}
        }

        if matches!(replay_kind, Some(ReplayKind::ThreadSnapshot)) && interaction_id.is_empty() {
            self.request_redraw();
        }
    }
}
