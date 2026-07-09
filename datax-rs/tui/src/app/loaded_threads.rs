//! Discovers subagent threads that belong to a primary thread by walking spawn-tree edges.
//!
//! When the TUI resumes or switches to an existing thread, it needs to populate
//! `AgentNavigationState` and `ChatWidget` metadata for every subagent that was spawned during
//! that thread's lifetime. The app server exposes a flat list of currently loaded threads via
//! `thread/loaded/list`, but the TUI must figure out which of those are descendants of the
//! primary thread.
//!
//! This module provides the pure, synchronous tree-walk that turns that flat list into the filtered
//! set of descendants. It intentionally has no async, no I/O, and no side effects so it can be
//! unit-tested in isolation.
//!
//! The walk starts from `primary_chat_id` and repeatedly follows
//! `SessionSource::SubAgent(ThreadSpawn { parent_chat_id, .. })` edges until no new children are
//! found. The primary thread itself is never included in the output.

use datax_app_server_protocol::Chat;
use datax_app_server_protocol::SessionSource;
use datax_protocol::ChatId;
use datax_protocol::protocol::SubAgentSource;
use std::collections::HashMap;
use std::collections::HashSet;

/// A subagent thread discovered by the spawn-tree walk, carrying just enough metadata for the
/// TUI to register it in the navigation cache and rendering metadata map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoadedSubagentThread {
    pub(crate) chat_id: ChatId,
    pub(crate) agent_nickname: Option<String>,
    pub(crate) agent_role: Option<String>,
    pub(crate) agent_path: Option<String>,
}

/// Walks the spawn tree rooted at `primary_chat_id` and returns every descendant subagent.
///
/// The walk is breadth-first over `SessionSource::SubAgent(ThreadSpawn { parent_chat_id })` edges.
/// Threads whose `source` is not a `ThreadSpawn`, or whose `parent_chat_id` does not chain back
/// to `primary_chat_id`, are excluded. The primary thread itself is never included.
///
/// Results are sorted by stringified thread id for deterministic output in tests and in the
/// navigation cache. Callers should not rely on this ordering for anything semantic; it exists
/// purely to make snapshot assertions stable.
///
/// If two threads claim the same parent, both are included. Cycles in the parent chain are not
/// possible because `ChatId`s are server-assigned UUIDs and the server enforces acyclicity, but
/// the `included` set guards against re-visiting regardless.
pub(crate) fn find_loaded_subagent_threads_for_primary(
    threads: Vec<Chat>,
    primary_chat_id: ChatId,
) -> Vec<LoadedSubagentThread> {
    let mut threads_by_id = HashMap::new();
    for thread in threads {
        let Ok(chat_id) = ChatId::from_string(&thread.id) else {
            continue;
        };
        threads_by_id.insert(chat_id, thread);
    }

    let mut included = HashSet::new();
    let mut pending = vec![primary_chat_id];
    while let Some(parent_chat_id) = pending.pop() {
        for (chat_id, thread) in &threads_by_id {
            if included.contains(chat_id) {
                continue;
            }

            let Some(source_parent_chat_id) = thread_spawn_parent_chat_id(&thread.source)
            else {
                continue;
            };

            if source_parent_chat_id != parent_chat_id {
                continue;
            }

            included.insert(*chat_id);
            pending.push(*chat_id);
        }
    }

    let mut loaded_threads: Vec<LoadedSubagentThread> = included
        .into_iter()
        .filter_map(|chat_id| {
            threads_by_id
                .remove(&chat_id)
                .map(|thread| LoadedSubagentThread {
                    chat_id,
                    agent_nickname: thread.agent_nickname,
                    agent_role: thread.agent_role,
                    agent_path: thread_spawn_agent_path(&thread.source),
                })
        })
        .collect();
    loaded_threads.sort_by_key(|thread| thread.chat_id.to_string());
    loaded_threads
}

fn thread_spawn_agent_path(source: &SessionSource) -> Option<String> {
    match source {
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn { agent_path, .. }) => {
            agent_path.clone().map(String::from)
        }
        _ => None,
    }
}

fn thread_spawn_parent_chat_id(source: &SessionSource) -> Option<ChatId> {
    match source {
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_chat_id, ..
        }) => Some(*parent_chat_id),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::LoadedSubagentThread;
    use super::find_loaded_subagent_threads_for_primary;
    use datax_app_server_protocol::Chat;
    use datax_app_server_protocol::ChatStatus;
    use datax_app_server_protocol::SessionSource;
    use datax_protocol::ChatId;
    use datax_utils_absolute_path::test_support::PathBufExt;
    use datax_utils_absolute_path::test_support::test_path_buf;
    use pretty_assertions::assert_eq;

    fn test_thread(chat_id: ChatId, source: SessionSource) -> Chat {
        Chat {
            id: chat_id.to_string(),
            session_id: chat_id.to_string(),
            forked_from_id: None,
            parent_chat_id: None,
            preview: String::new(),
            ephemeral: false,
            model_provider: "openai".to_string(),
            created_at: 0,
            updated_at: 0,
            recency_at: Some(0),
            status: ChatStatus::Idle,
            path: None,
            cwd: test_path_buf("/tmp").abs(),
            cli_version: "0.0.0".to_string(),
            source,
            chat_source: None,
            agent_nickname: None,
            agent_role: None,
            git_info: None,
            name: None,
            interactions: Vec::new(),
        }
    }

    fn thread_spawn_source(
        parent_chat_id: ChatId,
        depth: i32,
        agent_nickname: &str,
        agent_role: &str,
    ) -> SessionSource {
        serde_json::from_value(serde_json::json!({
            "subAgent": {
                "thread_spawn": {
                    "parent_chat_id": parent_chat_id.to_string(),
                    "depth": depth,
                    "agent_nickname": agent_nickname,
                    "agent_role": agent_role,
                }
            }
        }))
        .expect("valid subagent source")
    }

    #[test]
    fn finds_loaded_subagent_tree_for_primary_thread() {
        let primary_chat_id =
            ChatId::from_string("00000000-0000-0000-0000-000000000001").expect("valid thread");
        let child_chat_id =
            ChatId::from_string("00000000-0000-0000-0000-000000000002").expect("valid thread");
        let grandchild_chat_id =
            ChatId::from_string("00000000-0000-0000-0000-000000000003").expect("valid thread");
        let unrelated_parent_id =
            ChatId::from_string("00000000-0000-0000-0000-000000000004").expect("valid thread");
        let unrelated_child_id =
            ChatId::from_string("00000000-0000-0000-0000-000000000005").expect("valid thread");

        let mut child = test_thread(
            child_chat_id,
            thread_spawn_source(primary_chat_id, /*depth*/ 1, "Scout", "explorer"),
        );
        child.agent_nickname = Some("Scout".to_string());
        child.agent_role = Some("explorer".to_string());

        let mut grandchild = test_thread(
            grandchild_chat_id,
            thread_spawn_source(child_chat_id, /*depth*/ 2, "Atlas", "worker"),
        );
        grandchild.agent_nickname = Some("Atlas".to_string());
        grandchild.agent_role = Some("worker".to_string());

        let unrelated_child = test_thread(
            unrelated_child_id,
            thread_spawn_source(unrelated_parent_id, /*depth*/ 1, "Other", "researcher"),
        );

        let loaded = find_loaded_subagent_threads_for_primary(
            vec![
                test_thread(primary_chat_id, SessionSource::Cli),
                child,
                grandchild,
                unrelated_child,
            ],
            primary_chat_id,
        );

        assert_eq!(
            loaded,
            vec![
                LoadedSubagentThread {
                    chat_id: child_chat_id,
                    agent_nickname: Some("Scout".to_string()),
                    agent_role: Some("explorer".to_string()),
                    agent_path: None,
                },
                LoadedSubagentThread {
                    chat_id: grandchild_chat_id,
                    agent_nickname: Some("Atlas".to_string()),
                    agent_role: Some("worker".to_string()),
                    agent_path: None,
                },
            ]
        );
    }
}
