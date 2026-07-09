use std::collections::HashMap;
use std::collections::HashSet;

use datax_install_context::InstallContext;
use datax_protocol::ChatId;
use datax_rollout::RolloutConfig;
use datax_rollout::find_thread_names_by_ids;
use datax_rollout::first_rollout_content_match_snippet;
use datax_rollout::parse_cursor;
use datax_rollout::search_rollout_matches;

use super::LocalChatStore;
use super::helpers::distinct_chat_metadata_title;
use super::helpers::set_thread_name_from_title;
use super::helpers::stored_chat_from_rollout_item;
use super::list_chats::list_rollout_threads;
use crate::ListChatsParams;
use crate::SearchChatsParams;
use crate::SortDirection;
use crate::StoredChatSearchResult;
use crate::ChatSearchPage;
use crate::ChatSortKey;
use crate::ChatStoreError;
use crate::ChatStoreResult;

#[cfg(test)]
#[path = "search_chats_tests.rs"]
mod tests;

struct ThreadSearchItem {
    item: datax_rollout::ThreadItem,
    snippet: String,
}

pub(super) async fn search_chats(
    store: &LocalChatStore,
    params: SearchChatsParams,
) -> ChatStoreResult<ChatSearchPage> {
    let search_term = params.search_term.as_str();
    if search_term.is_empty() {
        return Err(ChatStoreError::InvalidRequest {
            message: "thread/search requires search_term".to_string(),
        });
    }
    let cursor = params
        .cursor
        .as_deref()
        .map(|cursor| {
            parse_cursor(cursor).ok_or_else(|| ChatStoreError::InvalidRequest {
                message: format!("invalid cursor: {cursor}"),
            })
        })
        .transpose()?;
    let sort_key = match params.sort_key {
        ChatSortKey::CreatedAt => datax_rollout::ChatSortKey::CreatedAt,
        ChatSortKey::UpdatedAt => datax_rollout::ChatSortKey::UpdatedAt,
        ChatSortKey::RecencyAt => datax_rollout::ChatSortKey::RecencyAt,
    };
    let sort_direction = match params.sort_direction {
        SortDirection::Asc => datax_rollout::SortDirection::Asc,
        SortDirection::Desc => datax_rollout::SortDirection::Desc,
    };
    let state_db = store.state_db().await;
    let rollout_config = RolloutConfig {
        codex_home: store.config.codex_home.clone(),
        sqlite_home: store.config.sqlite_home.clone(),
        cwd: store.config.codex_home.clone(),
        model_provider_id: store.config.default_model_provider_id.clone(),
        generate_memories: false,
    };
    let rg_command = InstallContext::current().rg_command();
    let matching_rollouts = search_rollout_matches(
        rg_command.as_path(),
        store.config.codex_home.as_path(),
        params.archived,
        search_term,
    )
    .await
    .map_err(|err| ChatStoreError::Internal {
        message: format!("failed to search rollout contents: {err}"),
    })?;
    if matching_rollouts.is_empty() {
        return Ok(ChatSearchPage {
            items: Vec::new(),
            next_cursor: None,
        });
    }
    let mut matching_items = Vec::new();
    let mut page_cursor = cursor;
    let scan_page_size = params.page_size.saturating_mul(8).clamp(256, 2048);
    let scan_params = ListChatsParams {
        page_size: scan_page_size,
        cursor: None,
        sort_key: params.sort_key,
        sort_direction: params.sort_direction,
        allowed_sources: params.allowed_sources.clone(),
        model_providers: None,
        cwd_filters: None,
        archived: params.archived,
        search_term: None,
        parent_chat_id: None,
        use_state_db_only: state_db.is_some(),
    };
    let mut remaining_rollouts = matching_rollouts;

    loop {
        let page = list_rollout_threads(
            state_db.clone(),
            &rollout_config,
            store.config.default_model_provider_id.as_str(),
            &scan_params,
            page_cursor.as_ref(),
            sort_key,
            sort_direction,
        )
        .await?;
        for item in page.items {
            let logical_path = datax_rollout::plain_rollout_path(item.path.as_path());
            let Some(snippet) = (match remaining_rollouts.remove(logical_path.as_path()) {
                Some(Some(snippet)) => Some(snippet),
                Some(None) => first_rollout_content_match_snippet(item.path.as_path(), search_term)
                    .await
                    .map_err(|err| ChatStoreError::Internal {
                        message: format!("failed to read rollout search match: {err}"),
                    })?,
                None => None,
            }) else {
                continue;
            };
            matching_items.push(ThreadSearchItem { item, snippet });
            if matching_items.len() > params.page_size {
                break;
            }
        }
        page_cursor = page.next_cursor;
        if matching_items.len() > params.page_size
            || remaining_rollouts.is_empty()
            || page_cursor.is_none()
        {
            break;
        }
    }

    let more_matches_available = matching_items.len() > params.page_size;
    matching_items.truncate(params.page_size);
    let next_cursor = if more_matches_available {
        matching_items
            .last()
            .and_then(|item| cursor_from_thread_search_item(item, params.sort_key))
    } else {
        None
    }
    .as_ref()
    .and_then(|cursor| serde_json::to_value(cursor).ok())
    .and_then(|value| value.as_str().map(str::to_owned));

    let mut items = matching_items
        .into_iter()
        .filter_map(|item| {
            stored_chat_from_rollout_item(
                item.item,
                params.archived,
                store.config.default_model_provider_id.as_str(),
            )
            .map(|chat| StoredChatSearchResult {
                chat,
                snippet: item.snippet,
            })
        })
        .collect::<Vec<_>>();
    set_thread_search_result_names(store, &mut items).await;

    Ok(ChatSearchPage { items, next_cursor })
}

fn cursor_from_thread_search_item(
    item: &ThreadSearchItem,
    sort_key: ChatSortKey,
) -> Option<datax_rollout::Cursor> {
    let timestamp = match sort_key {
        ChatSortKey::CreatedAt => item.item.created_at.as_deref()?,
        ChatSortKey::UpdatedAt => item
            .item
            .updated_at
            .as_deref()
            .or(item.item.created_at.as_deref())?,
        ChatSortKey::RecencyAt => item
            .item
            .recency_at
            .as_deref()
            .or(item.item.updated_at.as_deref())
            .or(item.item.created_at.as_deref())?,
    };
    match sort_key {
        ChatSortKey::RecencyAt => parse_cursor(&format!("{timestamp}|{}", item.item.chat_id?)),
        ChatSortKey::CreatedAt | ChatSortKey::UpdatedAt => parse_cursor(timestamp),
    }
}

async fn set_thread_search_result_names(
    store: &LocalChatStore,
    items: &mut [StoredChatSearchResult],
) {
    let chat_ids = items
        .iter()
        .map(|item| item.chat.chat_id)
        .collect::<HashSet<_>>();
    let mut names = HashMap::<ChatId, String>::with_capacity(chat_ids.len());
    if let Some(state_db_ctx) = store.state_db().await {
        for &chat_id in &chat_ids {
            let Ok(Some(metadata)) = state_db_ctx.get_thread(chat_id).await else {
                continue;
            };
            if let Some(title) = distinct_chat_metadata_title(&metadata) {
                names.insert(chat_id, title);
            }
        }
    }
    if names.len() < chat_ids.len()
        && let Ok(legacy_names) =
            find_thread_names_by_ids(store.config.codex_home.as_path(), &chat_ids).await
    {
        for (chat_id, title) in legacy_names {
            names.entry(chat_id).or_insert(title);
        }
    }
    for item in items {
        if let Some(title) = names.get(&item.chat.chat_id).cloned() {
            set_thread_name_from_title(&mut item.chat, title);
        }
    }
}
