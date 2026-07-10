use datax_app_server_protocol::ChatTokenUsage;
use datax_app_server_protocol::CollabAgentState as ApiCollabAgentState;
use datax_app_server_protocol::CollabAgentStatus as ApiCollabAgentStatus;
use datax_app_server_protocol::CollabAgentTool;
use datax_app_server_protocol::CollabAgentToolCallStatus as ApiCollabAgentToolCallStatus;
use datax_app_server_protocol::CommandAction;
use datax_app_server_protocol::CommandExecutionSource;
use datax_app_server_protocol::CommandExecutionStatus as ApiCommandExecutionStatus;
use datax_app_server_protocol::ErrorNotification;
use datax_app_server_protocol::FileUpdateChange as ApiFileUpdateChange;
use datax_app_server_protocol::Interaction;
use datax_app_server_protocol::InteractionCompletedNotification;
use datax_app_server_protocol::InteractionError;
use datax_app_server_protocol::InteractionPlanStep;
use datax_app_server_protocol::InteractionPlanStepStatus;
use datax_app_server_protocol::InteractionPlanUpdatedNotification;
use datax_app_server_protocol::InteractionStartedNotification;
use datax_app_server_protocol::InteractionStatus;
use datax_app_server_protocol::McpToolCallError;
use datax_app_server_protocol::McpToolCallResult;
use datax_app_server_protocol::McpToolCallStatus as ApiMcpToolCallStatus;
use datax_app_server_protocol::Message;
use datax_app_server_protocol::MessageCompletedNotification;
use datax_app_server_protocol::MessageStartedNotification;
use datax_app_server_protocol::PatchApplyStatus as ApiPatchApplyStatus;
use datax_app_server_protocol::PatchChangeKind as ApiPatchChangeKind;
use datax_app_server_protocol::ServerNotification;
use datax_app_server_protocol::TokenUsageBreakdown;
use datax_app_server_protocol::WebSearchAction as ApiWebSearchAction;
use datax_protocol::SessionId;
use datax_protocol::ChatId;
use datax_protocol::models::PermissionProfile;
use datax_protocol::models::WebSearchAction;
use datax_protocol::protocol::AskForApproval;
use datax_protocol::protocol::SessionConfiguredEvent;
use datax_utils_absolute_path::test_support::PathBufExt;
use datax_utils_absolute_path::test_support::test_path_buf;
use pretty_assertions::assert_eq;
use serde_json::json;

use datax_exec::AgentMessageItem;
use datax_exec::CodexStatus;
use datax_exec::CollabAgentState;
use datax_exec::CollabAgentStatus;
use datax_exec::CollabTool;
use datax_exec::CollabToolCallItem;
use datax_exec::CollabToolCallStatus;
use datax_exec::CollectedThreadEvents;
use datax_exec::CommandExecutionItem;
use datax_exec::CommandExecutionStatus;
use datax_exec::ErrorItem;
use datax_exec::EventProcessorWithJsonOutput;
use datax_exec::ExecThreadItem;
use datax_exec::FileChangeItem;
use datax_exec::FileUpdateChange as ExecFileUpdateChange;
use datax_exec::MessageCompletedEvent;
use datax_exec::MessageStartedEvent;
use datax_exec::ItemUpdatedEvent;
use datax_exec::McpToolCallItem;
use datax_exec::McpToolCallItemError;
use datax_exec::McpToolCallItemResult;
use datax_exec::McpToolCallStatus;
use datax_exec::PatchApplyStatus;
use datax_exec::PatchChangeKind;
use datax_exec::ReasoningItem;
use datax_exec::ThreadErrorEvent;
use datax_exec::ThreadEvent;
use datax_exec::ThreadItemDetails;
use datax_exec::ThreadStartedEvent;
use datax_exec::TodoItem;
use datax_exec::TodoListItem;
use datax_exec::InteractionCompletedEvent;
use datax_exec::TurnFailedEvent;
use datax_exec::InteractionStartedEvent;
use datax_exec::Usage;
use datax_exec::WebSearchItem;

#[test]
fn map_todo_items_preserves_text_and_completion_state() {
    let items = EventProcessorWithJsonOutput::map_todo_items(&[
        InteractionPlanStep {
            step: "inspect bootstrap".to_string(),
            status: InteractionPlanStepStatus::InProgress,
        },
        InteractionPlanStep {
            step: "drop legacy notifications".to_string(),
            status: InteractionPlanStepStatus::Completed,
        },
    ]);

    assert_eq!(
        items,
        vec![
            TodoItem {
                text: "inspect bootstrap".to_string(),
                completed: false,
            },
            TodoItem {
                text: "drop legacy notifications".to_string(),
                completed: true,
            },
        ]
    );
}

#[test]
fn session_configured_produces_thread_started_event() {
    let chat_id = ChatId::from_string("67e55044-10b1-426f-9247-bb680e5fe0c8")
        .expect("thread id should parse");
    let session_configured = SessionConfiguredEvent {
        session_id: SessionId::from(chat_id),
        chat_id,
        forked_from_id: None,
        parent_chat_id: None,
        chat_source: None,
        thread_name: None,
        model: "datax-mini-latest".to_string(),
        model_provider_id: "test-provider".to_string(),
        service_tier: None,
        approval_policy: AskForApproval::Never,
        approvals_reviewer: datax_protocol::config_types::ApprovalsReviewer::User,
        permission_profile: PermissionProfile::read_only(),
        active_permission_profile: None,
        cwd: test_path_buf("/tmp/project").abs(),
        reasoning_effort: None,
        initial_messages: None,
        network_proxy: None,
        rollout_path: None,
    };

    assert_eq!(
        EventProcessorWithJsonOutput::thread_started_event(&session_configured),
        ThreadEvent::ThreadStarted(ThreadStartedEvent {
            chat_id: "67e55044-10b1-426f-9247-bb680e5fe0c8".to_string(),
        })
    );
}

#[test]
fn turn_started_emits_turn_started_event() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let collected = processor.collect_thread_events(ServerNotification::InteractionStarted(
        InteractionStartedNotification {
            chat_id: "thread-1".to_string(),
            interaction: Interaction {
                id: "turn-1".to_string(),
                items_view: datax_app_server_protocol::InteractionMessagesView::Full,
                items: Vec::new(),
                status: InteractionStatus::InProgress,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
            },
        },
    ));

    assert_eq!(
        collected,
        CollectedThreadEvents {
            events: vec![ThreadEvent::InteractionStarted(InteractionStartedEvent {})],
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn command_execution_started_and_completed_translate_to_thread_events() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);
    let command_item = Message::CommandExecution {
        id: "cmd-1".to_string(),
        command: "ls".to_string(),
        cwd: test_path_buf("/tmp/project").abs().into(),
        process_id: Some("123".to_string()),
        source: CommandExecutionSource::UserShell,
        status: ApiCommandExecutionStatus::InProgress,
        command_actions: Vec::<CommandAction>::new(),
        aggregated_output: None,
        exit_code: None,
        duration_ms: None,
    };

    let started = processor.collect_thread_events(ServerNotification::MessageStarted(
        MessageStartedNotification {
            item: command_item,
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            started_at_ms: 0,
        },
    ));
    assert_eq!(
        started,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageStarted(MessageStartedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::CommandExecution(CommandExecutionItem {
                        command: "ls".to_string(),
                        aggregated_output: String::new(),
                        exit_code: None,
                        status: CommandExecutionStatus::InProgress,
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );

    let completed = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::CommandExecution {
                id: "cmd-1".to_string(),
                command: "ls".to_string(),
                cwd: test_path_buf("/tmp/project").abs().into(),
                process_id: Some("123".to_string()),
                source: CommandExecutionSource::UserShell,
                status: ApiCommandExecutionStatus::Completed,
                command_actions: Vec::<CommandAction>::new(),
                aggregated_output: Some("a.txt\n".to_string()),
                exit_code: Some(0),
                duration_ms: Some(3),
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));
    assert_eq!(
        completed,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageCompleted(MessageCompletedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::CommandExecution(CommandExecutionItem {
                        command: "ls".to_string(),
                        aggregated_output: "a.txt\n".to_string(),
                        exit_code: Some(0),
                        status: CommandExecutionStatus::Completed,
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn empty_reasoning_items_are_ignored() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let collected = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::Reasoning {
                id: "reasoning-1".to_string(),
                summary: Vec::new(),
                content: vec!["raw reasoning".to_string()],
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    assert_eq!(
        collected,
        CollectedThreadEvents {
            events: Vec::new(),
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn unsupported_items_do_not_consume_synthetic_ids() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let ignored = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::Plan {
                id: "plan-1".to_string(),
                text: "ignored plan".to_string(),
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    assert_eq!(
        ignored,
        CollectedThreadEvents {
            events: Vec::new(),
            status: CodexStatus::Running,
        }
    );

    let collected = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::AgentMessage {
                id: "message-1".to_string(),
                text: "hello".to_string(),
                phase: None,
                memory_citation: None,
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    assert_eq!(
        collected,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageCompleted(MessageCompletedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::AgentMessage(AgentMessageItem {
                        text: "hello".to_string(),
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn reasoning_items_emit_summary_not_raw_content() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let collected = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::Reasoning {
                id: "reasoning-1".to_string(),
                summary: vec!["safe summary".to_string()],
                content: vec!["raw reasoning".to_string()],
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    assert_eq!(
        collected,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageCompleted(MessageCompletedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::Reasoning(ReasoningItem {
                        text: "safe summary".to_string(),
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn web_search_completion_preserves_query_and_action() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let collected = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::WebSearch {
                id: "search-1".to_string(),
                query: "rust async await".to_string(),
                action: Some(ApiWebSearchAction::Search {
                    query: Some("rust async await".to_string()),
                    queries: None,
                }),
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    assert_eq!(
        collected,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageCompleted(MessageCompletedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::WebSearch(WebSearchItem {
                        id: "search-1".to_string(),
                        query: "rust async await".to_string(),
                        action: WebSearchAction::Search {
                            query: Some("rust async await".to_string()),
                            queries: None,
                        },
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn web_search_start_and_completion_reuse_item_id() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let started = processor.collect_thread_events(ServerNotification::MessageStarted(
        MessageStartedNotification {
            item: Message::WebSearch {
                id: "search-1".to_string(),
                query: String::new(),
                action: None,
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            started_at_ms: 0,
        },
    ));

    let completed = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::WebSearch {
                id: "search-1".to_string(),
                query: "rust async await".to_string(),
                action: Some(ApiWebSearchAction::Search {
                    query: Some("rust async await".to_string()),
                    queries: None,
                }),
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    assert_eq!(
        started,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageStarted(MessageStartedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::WebSearch(WebSearchItem {
                        id: "search-1".to_string(),
                        query: String::new(),
                        action: WebSearchAction::Other,
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
    assert_eq!(
        completed,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageCompleted(MessageCompletedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::WebSearch(WebSearchItem {
                        id: "search-1".to_string(),
                        query: "rust async await".to_string(),
                        action: WebSearchAction::Search {
                            query: Some("rust async await".to_string()),
                            queries: None,
                        },
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn mcp_tool_call_begin_and_end_emit_item_events() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let started = processor.collect_thread_events(ServerNotification::MessageStarted(
        MessageStartedNotification {
            item: Message::McpToolCall {
                id: "mcp-1".to_string(),
                server: "server_a".to_string(),
                tool: "tool_x".to_string(),
                status: ApiMcpToolCallStatus::InProgress,
                arguments: json!({ "key": "value" }),
                app_context: None,
                mcp_app_resource_uri: None,
                plugin_id: None,
                result: None,
                error: None,
                duration_ms: None,
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            started_at_ms: 0,
        },
    ));
    let completed = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::McpToolCall {
                id: "mcp-1".to_string(),
                server: "server_a".to_string(),
                tool: "tool_x".to_string(),
                status: ApiMcpToolCallStatus::Completed,
                arguments: json!({ "key": "value" }),
                app_context: None,
                mcp_app_resource_uri: None,
                plugin_id: None,
                result: Some(Box::new(McpToolCallResult {
                    content: Vec::new(),
                    structured_content: None,
                    meta: None,
                })),
                error: None,
                duration_ms: Some(1_000),
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    assert_eq!(
        started,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageStarted(MessageStartedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::McpToolCall(McpToolCallItem {
                        server: "server_a".to_string(),
                        tool: "tool_x".to_string(),
                        arguments: json!({ "key": "value" }),
                        result: None,
                        error: None,
                        status: McpToolCallStatus::InProgress,
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
    assert_eq!(
        completed,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageCompleted(MessageCompletedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::McpToolCall(McpToolCallItem {
                        server: "server_a".to_string(),
                        tool: "tool_x".to_string(),
                        arguments: json!({ "key": "value" }),
                        result: Some(McpToolCallItemResult {
                            content: Vec::new(),
                            meta: None,
                            structured_content: None,
                        }),
                        error: None,
                        status: McpToolCallStatus::Completed,
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn mcp_tool_call_failure_sets_failed_status() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let collected = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::McpToolCall {
                id: "mcp-2".to_string(),
                server: "server_b".to_string(),
                tool: "tool_y".to_string(),
                status: ApiMcpToolCallStatus::Failed,
                arguments: json!({ "param": 42 }),
                app_context: None,
                mcp_app_resource_uri: None,
                plugin_id: None,
                result: None,
                error: Some(McpToolCallError {
                    message: "tool exploded".to_string(),
                }),
                duration_ms: Some(5),
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    assert_eq!(
        collected,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageCompleted(MessageCompletedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::McpToolCall(McpToolCallItem {
                        server: "server_b".to_string(),
                        tool: "tool_y".to_string(),
                        arguments: json!({ "param": 42 }),
                        result: None,
                        error: Some(McpToolCallItemError {
                            message: "tool exploded".to_string(),
                        }),
                        status: McpToolCallStatus::Failed,
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn mcp_tool_call_defaults_arguments_and_preserves_structured_content() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let started = processor.collect_thread_events(ServerNotification::MessageStarted(
        MessageStartedNotification {
            item: Message::McpToolCall {
                id: "mcp-3".to_string(),
                server: "server_c".to_string(),
                tool: "tool_z".to_string(),
                status: ApiMcpToolCallStatus::InProgress,
                arguments: serde_json::Value::Null,
                app_context: None,
                mcp_app_resource_uri: None,
                plugin_id: None,
                result: None,
                error: None,
                duration_ms: None,
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            started_at_ms: 0,
        },
    ));
    let completed = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::McpToolCall {
                id: "mcp-3".to_string(),
                server: "server_c".to_string(),
                tool: "tool_z".to_string(),
                status: ApiMcpToolCallStatus::Completed,
                arguments: serde_json::Value::Null,
                app_context: None,
                mcp_app_resource_uri: None,
                plugin_id: None,
                result: Some(Box::new(McpToolCallResult {
                    content: vec![json!({
                        "type": "text",
                        "text": "done",
                    })],
                    structured_content: Some(json!({ "status": "ok" })),
                    meta: None,
                })),
                error: None,
                duration_ms: Some(10),
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    assert_eq!(
        started,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageStarted(MessageStartedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::McpToolCall(McpToolCallItem {
                        server: "server_c".to_string(),
                        tool: "tool_z".to_string(),
                        arguments: serde_json::Value::Null,
                        result: None,
                        error: None,
                        status: McpToolCallStatus::InProgress,
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
    assert_eq!(
        completed,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageCompleted(MessageCompletedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::McpToolCall(McpToolCallItem {
                        server: "server_c".to_string(),
                        tool: "tool_z".to_string(),
                        arguments: serde_json::Value::Null,
                        result: Some(McpToolCallItemResult {
                            content: vec![json!({
                                "type": "text",
                                "text": "done",
                            })],
                            meta: None,
                            structured_content: Some(json!({ "status": "ok" })),
                        }),
                        error: None,
                        status: McpToolCallStatus::Completed,
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn collab_spawn_begin_and_end_emit_item_events() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let started = processor.collect_thread_events(ServerNotification::MessageStarted(
        MessageStartedNotification {
            item: Message::CollabAgentToolCall {
                id: "collab-1".to_string(),
                tool: CollabAgentTool::SpawnAgent,
                status: ApiCollabAgentToolCallStatus::InProgress,
                sender_chat_id: "thread-parent".to_string(),
                receiver_chat_ids: Vec::new(),
                prompt: Some("draft a plan".to_string()),
                model: Some("gpt-5".to_string()),
                reasoning_effort: None,
                agents_states: std::collections::HashMap::new(),
            },
            chat_id: "thread-parent".to_string(),
            interaction_id: "turn-1".to_string(),
            started_at_ms: 0,
        },
    ));
    let completed = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::CollabAgentToolCall {
                id: "collab-1".to_string(),
                tool: CollabAgentTool::SpawnAgent,
                status: ApiCollabAgentToolCallStatus::Completed,
                sender_chat_id: "thread-parent".to_string(),
                receiver_chat_ids: vec!["thread-child".to_string()],
                prompt: Some("draft a plan".to_string()),
                model: Some("gpt-5".to_string()),
                reasoning_effort: None,
                agents_states: std::collections::HashMap::from([(
                    "thread-child".to_string(),
                    ApiCollabAgentState {
                        status: ApiCollabAgentStatus::Running,
                        message: None,
                    },
                )]),
            },
            chat_id: "thread-parent".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    assert_eq!(
        started,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageStarted(MessageStartedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::CollabToolCall(CollabToolCallItem {
                        tool: CollabTool::SpawnAgent,
                        sender_chat_id: "thread-parent".to_string(),
                        receiver_chat_ids: Vec::new(),
                        prompt: Some("draft a plan".to_string()),
                        agents_states: std::collections::HashMap::new(),
                        status: CollabToolCallStatus::InProgress,
                    },),
                },
            })],
            status: CodexStatus::Running,
        }
    );
    assert_eq!(
        completed,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageCompleted(MessageCompletedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::CollabToolCall(CollabToolCallItem {
                        tool: CollabTool::SpawnAgent,
                        sender_chat_id: "thread-parent".to_string(),
                        receiver_chat_ids: vec!["thread-child".to_string()],
                        prompt: Some("draft a plan".to_string()),
                        agents_states: std::collections::HashMap::from([(
                            "thread-child".to_string(),
                            CollabAgentState {
                                status: CollabAgentStatus::Running,
                                message: None,
                            },
                        )]),
                        status: CollabToolCallStatus::Completed,
                    },),
                },
            })],
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn file_change_completion_maps_change_kinds() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let collected = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::FileChange {
                id: "patch-1".to_string(),
                changes: vec![
                    ApiFileUpdateChange {
                        path: "a/added.txt".to_string(),
                        kind: ApiPatchChangeKind::Add,
                        diff: String::new(),
                    },
                    ApiFileUpdateChange {
                        path: "b/deleted.txt".to_string(),
                        kind: ApiPatchChangeKind::Delete,
                        diff: String::new(),
                    },
                    ApiFileUpdateChange {
                        path: "c/modified.txt".to_string(),
                        kind: ApiPatchChangeKind::Update { move_path: None },
                        diff: "@@ -1 +1 @@".to_string(),
                    },
                ],
                status: ApiPatchApplyStatus::Completed,
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    assert_eq!(
        collected,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageCompleted(MessageCompletedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::FileChange(FileChangeItem {
                        changes: vec![
                            ExecFileUpdateChange {
                                path: "a/added.txt".to_string(),
                                kind: PatchChangeKind::Add,
                            },
                            ExecFileUpdateChange {
                                path: "b/deleted.txt".to_string(),
                                kind: PatchChangeKind::Delete,
                            },
                            ExecFileUpdateChange {
                                path: "c/modified.txt".to_string(),
                                kind: PatchChangeKind::Update,
                            },
                        ],
                        status: PatchApplyStatus::Completed,
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn file_change_declined_maps_to_failed_status() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let collected = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::FileChange {
                id: "patch-2".to_string(),
                changes: vec![ApiFileUpdateChange {
                    path: "file.txt".to_string(),
                    kind: ApiPatchChangeKind::Update { move_path: None },
                    diff: "@@ -1 +1 @@".to_string(),
                }],
                status: ApiPatchApplyStatus::Declined,
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    assert_eq!(
        collected,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageCompleted(MessageCompletedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::FileChange(FileChangeItem {
                        changes: vec![ExecFileUpdateChange {
                            path: "file.txt".to_string(),
                            kind: PatchChangeKind::Update,
                        }],
                        status: PatchApplyStatus::Failed,
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn agent_message_item_updates_final_message() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let collected = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::AgentMessage {
                id: "msg-1".to_string(),
                text: "hello".to_string(),
                phase: None,
                memory_citation: None,
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    assert_eq!(
        collected,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageCompleted(MessageCompletedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::AgentMessage(AgentMessageItem {
                        text: "hello".to_string(),
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
    assert_eq!(processor.final_message(), Some("hello"));
}

#[test]
fn agent_message_item_started_is_ignored() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let collected = processor.collect_thread_events(ServerNotification::MessageStarted(
        MessageStartedNotification {
            item: Message::AgentMessage {
                id: "msg-1".to_string(),
                text: "hello".to_string(),
                phase: None,
                memory_citation: None,
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            started_at_ms: 0,
        },
    ));

    assert_eq!(
        collected,
        CollectedThreadEvents {
            events: Vec::new(),
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn reasoning_item_completed_uses_synthetic_id() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let collected = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::Reasoning {
                id: "rs-1".to_string(),
                summary: vec!["thinking...".to_string()],
                content: vec!["raw".to_string()],
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    assert_eq!(
        collected,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageCompleted(MessageCompletedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::Reasoning(ReasoningItem {
                        text: "thinking...".to_string(),
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn warning_event_produces_error_item() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let collected = processor.collect_warning(
        "Heads up: Long conversations and multiple compactions can cause the model to be less accurate. Start a new conversation when possible to keep conversations small and targeted.".to_string(),
    );

    assert_eq!(
        collected,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageCompleted(MessageCompletedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::Error(ErrorItem {
                        message: "Heads up: Long conversations and multiple compactions can cause the model to be less accurate. Start a new conversation when possible to keep conversations small and targeted.".to_string(),
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn plan_update_emits_started_then_updated_then_completed() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let started = processor.collect_thread_events(ServerNotification::InteractionPlanUpdated(
        InteractionPlanUpdatedNotification {
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            explanation: None,
            plan: vec![
                InteractionPlanStep {
                    step: "step one".to_string(),
                    status: InteractionPlanStepStatus::Pending,
                },
                InteractionPlanStep {
                    step: "step two".to_string(),
                    status: InteractionPlanStepStatus::InProgress,
                },
            ],
        },
    ));
    assert_eq!(
        started,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageStarted(MessageStartedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::TodoList(TodoListItem {
                        items: vec![
                            TodoItem {
                                text: "step one".to_string(),
                                completed: false,
                            },
                            TodoItem {
                                text: "step two".to_string(),
                                completed: false,
                            },
                        ],
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );

    let updated = processor.collect_thread_events(ServerNotification::InteractionPlanUpdated(
        InteractionPlanUpdatedNotification {
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            explanation: None,
            plan: vec![
                InteractionPlanStep {
                    step: "step one".to_string(),
                    status: InteractionPlanStepStatus::Completed,
                },
                InteractionPlanStep {
                    step: "step two".to_string(),
                    status: InteractionPlanStepStatus::InProgress,
                },
            ],
        },
    ));
    assert_eq!(
        updated,
        CollectedThreadEvents {
            events: vec![ThreadEvent::ItemUpdated(ItemUpdatedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::TodoList(TodoListItem {
                        items: vec![
                            TodoItem {
                                text: "step one".to_string(),
                                completed: true,
                            },
                            TodoItem {
                                text: "step two".to_string(),
                                completed: false,
                            },
                        ],
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );

    let completed = processor.collect_thread_events(ServerNotification::InteractionCompleted(
        InteractionCompletedNotification {
            chat_id: "thread-1".to_string(),
            interaction: Interaction {
                id: "turn-1".to_string(),
                items_view: datax_app_server_protocol::InteractionMessagesView::Full,
                items: Vec::new(),
                status: InteractionStatus::Completed,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
            },
        },
    ));
    assert_eq!(
        completed,
        CollectedThreadEvents {
            events: vec![
                ThreadEvent::MessageCompleted(MessageCompletedEvent {
                    item: ExecThreadItem {
                        id: "item_0".to_string(),
                        details: ThreadItemDetails::TodoList(TodoListItem {
                            items: vec![
                                TodoItem {
                                    text: "step one".to_string(),
                                    completed: true,
                                },
                                TodoItem {
                                    text: "step two".to_string(),
                                    completed: false,
                                },
                            ],
                        }),
                    },
                }),
                ThreadEvent::InteractionCompleted(InteractionCompletedEvent {
                    usage: Usage::default(),
                }),
            ],
            status: CodexStatus::InitiateShutdown,
        }
    );
}

#[test]
fn plan_update_after_completion_starts_new_todo_list_with_new_id() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let _ = processor.collect_thread_events(ServerNotification::InteractionPlanUpdated(
        InteractionPlanUpdatedNotification {
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            explanation: None,
            plan: vec![InteractionPlanStep {
                step: "only".to_string(),
                status: InteractionPlanStepStatus::Pending,
            }],
        },
    ));
    let _ = processor.collect_thread_events(ServerNotification::InteractionCompleted(
        InteractionCompletedNotification {
            chat_id: "thread-1".to_string(),
            interaction: Interaction {
                id: "turn-1".to_string(),
                items_view: datax_app_server_protocol::InteractionMessagesView::Full,
                items: Vec::new(),
                status: InteractionStatus::Completed,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
            },
        },
    ));

    let restarted = processor.collect_thread_events(ServerNotification::InteractionPlanUpdated(
        InteractionPlanUpdatedNotification {
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-2".to_string(),
            explanation: None,
            plan: vec![InteractionPlanStep {
                step: "again".to_string(),
                status: InteractionPlanStepStatus::Pending,
            }],
        },
    ));

    assert_eq!(
        restarted,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageStarted(MessageStartedEvent {
                item: ExecThreadItem {
                    id: "item_1".to_string(),
                    details: ThreadItemDetails::TodoList(TodoListItem {
                        items: vec![TodoItem {
                            text: "again".to_string(),
                            completed: false,
                        }],
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );
}

#[test]
fn token_usage_update_is_emitted_on_turn_completion() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let usage_update = processor.collect_thread_events(ServerNotification::ChatTokenUsageUpdated(
        datax_app_server_protocol::ChatTokenUsageUpdatedNotification {
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            token_usage: ChatTokenUsage {
                total: TokenUsageBreakdown {
                    total_tokens: 42,
                    input_tokens: 10,
                    cached_input_tokens: 3,
                    output_tokens: 29,
                    reasoning_output_tokens: 7,
                },
                last: TokenUsageBreakdown {
                    total_tokens: 42,
                    input_tokens: 10,
                    cached_input_tokens: 3,
                    output_tokens: 29,
                    reasoning_output_tokens: 7,
                },
                model_context_window: Some(128_000),
            },
        },
    ));
    assert_eq!(
        usage_update,
        CollectedThreadEvents {
            events: Vec::new(),
            status: CodexStatus::Running,
        }
    );

    let completed = processor.collect_thread_events(ServerNotification::InteractionCompleted(
        InteractionCompletedNotification {
            chat_id: "thread-1".to_string(),
            interaction: Interaction {
                id: "turn-1".to_string(),
                items_view: datax_app_server_protocol::InteractionMessagesView::Full,
                items: Vec::new(),
                status: InteractionStatus::Completed,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
            },
        },
    ));
    assert_eq!(
        completed,
        CollectedThreadEvents {
            events: vec![ThreadEvent::InteractionCompleted(InteractionCompletedEvent {
                usage: Usage {
                    input_tokens: 10,
                    cached_input_tokens: 3,
                    output_tokens: 29,
                    reasoning_output_tokens: 7,
                },
            })],
            status: CodexStatus::InitiateShutdown,
        }
    );
}

#[test]
fn turn_completion_recovers_final_message_from_turn_items() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let completed = processor.collect_thread_events(ServerNotification::InteractionCompleted(
        InteractionCompletedNotification {
            chat_id: "thread-1".to_string(),
            interaction: Interaction {
                id: "turn-1".to_string(),
                items_view: datax_app_server_protocol::InteractionMessagesView::Full,
                items: vec![Message::AgentMessage {
                    id: "msg-1".to_string(),
                    text: "final answer".to_string(),
                    phase: None,
                    memory_citation: None,
                }],
                status: InteractionStatus::Completed,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
            },
        },
    ));

    assert_eq!(
        completed,
        CollectedThreadEvents {
            events: vec![ThreadEvent::InteractionCompleted(InteractionCompletedEvent {
                usage: Usage::default(),
            })],
            status: CodexStatus::InitiateShutdown,
        }
    );
    assert_eq!(processor.final_message(), Some("final answer"));
}

#[test]
fn turn_completion_reconciles_started_items_from_turn_items() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let started = processor.collect_thread_events(ServerNotification::MessageStarted(
        MessageStartedNotification {
            item: Message::CommandExecution {
                id: "cmd-1".to_string(),
                command: "ls".to_string(),
                cwd: test_path_buf("/tmp/project").abs().into(),
                process_id: Some("123".to_string()),
                source: CommandExecutionSource::UserShell,
                status: ApiCommandExecutionStatus::InProgress,
                command_actions: Vec::<CommandAction>::new(),
                aggregated_output: None,
                exit_code: None,
                duration_ms: None,
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            started_at_ms: 0,
        },
    ));
    assert_eq!(
        started,
        CollectedThreadEvents {
            events: vec![ThreadEvent::MessageStarted(MessageStartedEvent {
                item: ExecThreadItem {
                    id: "item_0".to_string(),
                    details: ThreadItemDetails::CommandExecution(CommandExecutionItem {
                        command: "ls".to_string(),
                        aggregated_output: String::new(),
                        exit_code: None,
                        status: CommandExecutionStatus::InProgress,
                    }),
                },
            })],
            status: CodexStatus::Running,
        }
    );

    let completed = processor.collect_thread_events(ServerNotification::InteractionCompleted(
        InteractionCompletedNotification {
            chat_id: "thread-1".to_string(),
            interaction: Interaction {
                id: "turn-1".to_string(),
                items_view: datax_app_server_protocol::InteractionMessagesView::Full,
                items: vec![Message::CommandExecution {
                    id: "cmd-1".to_string(),
                    command: "ls".to_string(),
                    cwd: test_path_buf("/tmp/project").abs().into(),
                    process_id: Some("123".to_string()),
                    source: CommandExecutionSource::UserShell,
                    status: ApiCommandExecutionStatus::Completed,
                    command_actions: Vec::<CommandAction>::new(),
                    aggregated_output: Some("a.txt\n".to_string()),
                    exit_code: Some(0),
                    duration_ms: Some(3),
                }],
                status: InteractionStatus::Completed,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
            },
        },
    ));

    assert_eq!(
        completed,
        CollectedThreadEvents {
            events: vec![
                ThreadEvent::MessageCompleted(MessageCompletedEvent {
                    item: ExecThreadItem {
                        id: "item_0".to_string(),
                        details: ThreadItemDetails::CommandExecution(CommandExecutionItem {
                            command: "ls".to_string(),
                            aggregated_output: "a.txt\n".to_string(),
                            exit_code: Some(0),
                            status: CommandExecutionStatus::Completed,
                        }),
                    },
                }),
                ThreadEvent::InteractionCompleted(InteractionCompletedEvent {
                    usage: Usage::default(),
                }),
            ],
            status: CodexStatus::InitiateShutdown,
        }
    );
}

#[test]
fn turn_completion_overwrites_stale_final_message_from_turn_items() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);
    let _ = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::AgentMessage {
                id: "msg-stale".to_string(),
                text: "stale answer".to_string(),
                phase: None,
                memory_citation: None,
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    let completed = processor.collect_thread_events(ServerNotification::InteractionCompleted(
        InteractionCompletedNotification {
            chat_id: "thread-1".to_string(),
            interaction: Interaction {
                id: "turn-1".to_string(),
                items_view: datax_app_server_protocol::InteractionMessagesView::Full,
                items: vec![Message::AgentMessage {
                    id: "msg-1".to_string(),
                    text: "final answer".to_string(),
                    phase: None,
                    memory_citation: None,
                }],
                status: InteractionStatus::Completed,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
            },
        },
    ));

    assert_eq!(
        completed,
        CollectedThreadEvents {
            events: vec![ThreadEvent::InteractionCompleted(InteractionCompletedEvent {
                usage: Usage::default(),
            })],
            status: CodexStatus::InitiateShutdown,
        }
    );
    assert_eq!(processor.final_message(), Some("final answer"));
}

#[test]
fn turn_completion_preserves_streamed_final_message_when_turn_items_are_empty() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);
    let _ = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::AgentMessage {
                id: "msg-streamed".to_string(),
                text: "streamed answer".to_string(),
                phase: None,
                memory_citation: None,
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    let completed = processor.collect_thread_events(ServerNotification::InteractionCompleted(
        InteractionCompletedNotification {
            chat_id: "thread-1".to_string(),
            interaction: Interaction {
                id: "turn-1".to_string(),
                items_view: datax_app_server_protocol::InteractionMessagesView::Full,
                items: Vec::new(),
                status: InteractionStatus::Completed,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
            },
        },
    ));

    assert_eq!(
        completed,
        CollectedThreadEvents {
            events: vec![ThreadEvent::InteractionCompleted(InteractionCompletedEvent {
                usage: Usage::default(),
            })],
            status: CodexStatus::InitiateShutdown,
        }
    );
    assert_eq!(processor.final_message(), Some("streamed answer"));
}

#[test]
fn failed_turn_clears_stale_final_message() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let collected = processor.collect_thread_events(ServerNotification::MessageCompleted(
        MessageCompletedNotification {
            item: Message::AgentMessage {
                id: "msg-1".to_string(),
                text: "partial answer".to_string(),
                phase: None,
                memory_citation: None,
            },
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            completed_at_ms: 0,
        },
    ));

    assert_eq!(collected.status, CodexStatus::Running);
    assert_eq!(processor.final_message(), Some("partial answer"));

    let collected = processor.collect_thread_events(ServerNotification::InteractionCompleted(
        InteractionCompletedNotification {
            chat_id: "thread-1".to_string(),
            interaction: Interaction {
                id: "turn-1".to_string(),
                items_view: datax_app_server_protocol::InteractionMessagesView::Full,
                items: Vec::new(),
                status: InteractionStatus::Failed,
                error: Some(InteractionError {
                    message: "turn failed".to_string(),
                    additional_details: None,
                    codex_error_info: None,
                }),
                started_at: None,
                completed_at: None,
                duration_ms: None,
            },
        },
    ));

    assert_eq!(collected.status, CodexStatus::InitiateShutdown);
    assert_eq!(processor.final_message(), None);
}

#[test]
fn turn_completion_falls_back_to_final_plan_text() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let completed = processor.collect_thread_events(ServerNotification::InteractionCompleted(
        InteractionCompletedNotification {
            chat_id: "thread-1".to_string(),
            interaction: Interaction {
                id: "turn-1".to_string(),
                items_view: datax_app_server_protocol::InteractionMessagesView::Full,
                items: vec![Message::Plan {
                    id: "plan-1".to_string(),
                    text: "ship the typed adapter".to_string(),
                }],
                status: InteractionStatus::Completed,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
            },
        },
    ));

    assert_eq!(
        completed,
        CollectedThreadEvents {
            events: vec![ThreadEvent::InteractionCompleted(InteractionCompletedEvent {
                usage: Usage::default(),
            })],
            status: CodexStatus::InitiateShutdown,
        }
    );
    assert_eq!(processor.final_message(), Some("ship the typed adapter"));
}

#[test]
fn turn_failure_prefers_structured_error_message() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let error = processor.collect_thread_events(ServerNotification::Error(ErrorNotification {
        error: InteractionError {
            message: "backend failed".to_string(),
            codex_error_info: None,
            additional_details: Some("request id abc".to_string()),
        },
        will_retry: false,
        chat_id: "thread-1".to_string(),
        interaction_id: "turn-1".to_string(),
    }));
    assert_eq!(
        error,
        CollectedThreadEvents {
            events: vec![ThreadEvent::Error(ThreadErrorEvent {
                message: "backend failed (request id abc)".to_string(),
            })],
            status: CodexStatus::Running,
        }
    );

    let failed = processor.collect_thread_events(ServerNotification::InteractionCompleted(
        InteractionCompletedNotification {
            chat_id: "thread-1".to_string(),
            interaction: Interaction {
                id: "turn-1".to_string(),
                items_view: datax_app_server_protocol::InteractionMessagesView::Full,
                items: Vec::new(),
                status: InteractionStatus::Failed,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
            },
        },
    ));
    assert_eq!(
        failed,
        CollectedThreadEvents {
            events: vec![ThreadEvent::TurnFailed(TurnFailedEvent {
                error: ThreadErrorEvent {
                    message: "backend failed (request id abc)".to_string(),
                },
            })],
            status: CodexStatus::InitiateShutdown,
        }
    );
}

#[test]
fn model_reroute_surfaces_as_error_item() {
    let mut processor = EventProcessorWithJsonOutput::new(/*last_message_path*/ None);

    let collected = processor.collect_thread_events(ServerNotification::ModelRerouted(
        datax_app_server_protocol::ModelReroutedNotification {
            chat_id: "thread-1".to_string(),
            interaction_id: "turn-1".to_string(),
            from_model: "gpt-5".to_string(),
            to_model: "gpt-5-mini".to_string(),
            reason: datax_app_server_protocol::ModelRerouteReason::HighRiskCyberActivity,
        },
    ));

    assert_eq!(collected.status, CodexStatus::Running);
    assert_eq!(collected.events.len(), 1);
    let ThreadEvent::MessageCompleted(MessageCompletedEvent { item }) = &collected.events[0] else {
        panic!("expected MessageCompleted");
    };
    assert_eq!(item.id, "item_0");
    assert_eq!(
        item.details,
        ThreadItemDetails::Error(ErrorItem {
            message: "model rerouted: gpt-5 -> gpt-5-mini (HighRiskCyberActivity)".to_string(),
        })
    );
}
