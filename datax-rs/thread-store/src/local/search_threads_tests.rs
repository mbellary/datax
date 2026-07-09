use datax_protocol::ChatId;
use datax_rollout::ThreadItem;
use pretty_assertions::assert_eq;

use super::ThreadSearchItem;
use super::cursor_from_thread_search_item;
use crate::ThreadSortKey;

#[test]
fn recency_cursor_includes_chat_id_tie_breaker() {
    let chat_id = ChatId::from_string("00000000-0000-0000-0000-000000000123")
        .expect("thread ID should parse");
    let item = ThreadSearchItem {
        item: ThreadItem {
            chat_id: Some(chat_id),
            recency_at: Some("2026-01-27T12:34:56Z".to_string()),
            ..Default::default()
        },
        snippet: String::new(),
    };

    let cursor = cursor_from_thread_search_item(&item, ThreadSortKey::RecencyAt)
        .expect("cursor should build");

    assert_eq!(
        serde_json::to_string(&cursor).expect("cursor should serialize"),
        format!("\"2026-01-27T12:34:56Z|{chat_id}\"")
    );
}
