use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;

use chrono::Utc;
use datax_protocol::ChatId;
use datax_protocol::models::PermissionProfile;
use datax_protocol::protocol::AskForApproval;
use datax_protocol::protocol::RolloutMessage;
use datax_protocol::protocol::SessionMeta;
use datax_protocol::protocol::SessionMetaLine;
use datax_protocol::protocol::ThreadMemoryMode;
use datax_rollout::persisted_rollout_items;

use crate::AppendChatMessagesParams;
use crate::ArchiveChatParams;
use crate::CreateChatParams;
use crate::DeleteChatParams;
use crate::ListChatsParams;
use crate::LoadChatHistoryParams;
use crate::ReadChatByRolloutPathParams;
use crate::ReadChatParams;
use crate::ResumeChatParams;
use crate::StoredChat;
use crate::StoredChatHistory;
use crate::ChatMetadataPatch;
use crate::ChatPage;
use crate::ChatStore;
use crate::ChatStoreError;
use crate::ChatStoreFuture;
use crate::ChatStoreResult;
use crate::UpdateChatMetadataParams;

static IN_MEMORY_THREAD_STORES: OnceLock<Mutex<HashMap<String, Arc<InMemoryChatStore>>>> =
    OnceLock::new();

fn stores() -> &'static Mutex<HashMap<String, Arc<InMemoryChatStore>>> {
    IN_MEMORY_THREAD_STORES.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ListMessagesParams;
    use crate::ListInteractionsParams;
    use crate::SortDirection;
    use crate::StoredInteractionMessagesView;
    use crate::ChatPersistenceMetadata;
    use crate::ChatSortKey;
    use datax_protocol::models::BaseInstructions;
    use datax_protocol::protocol::SessionSource;

    #[tokio::test]
    async fn default_turn_pagination_methods_return_unsupported() {
        let store = InMemoryChatStore::default();
        let chat_id = ChatId::default();

        let turns_err = store
            .list_interactions(ListInteractionsParams {
                chat_id,
                include_archived: true,
                cursor: None,
                page_size: 10,
                sort_direction: SortDirection::Asc,
                messages_view: StoredInteractionMessagesView::Summary,
            })
            .await
            .expect_err("default list_interactions should be unsupported");
        assert!(matches!(
            turns_err,
            ChatStoreError::Unsupported {
                operation: "list_interactions"
            }
        ));

        let items_err = store
            .list_messages(ListMessagesParams {
                chat_id,
                interaction_id: None,
                include_archived: true,
                cursor: None,
                page_size: 10,
                sort_direction: SortDirection::Asc,
            })
            .await
            .expect_err("default list_messages should be unsupported");
        assert!(matches!(
            items_err,
            ChatStoreError::Unsupported {
                operation: "list_messages"
            }
        ));
    }

    #[tokio::test]
    async fn list_chats_filters_by_parent_chat_id() {
        let store = InMemoryChatStore::default();
        let parent_chat_id = ChatId::default();
        let child_chat_id =
            ChatId::from_string("00000000-0000-0000-0000-000000000001").expect("valid thread id");
        let unrelated_chat_id =
            ChatId::from_string("00000000-0000-0000-0000-000000000002").expect("valid thread id");

        for (chat_id, parent_chat_id) in [
            (child_chat_id, Some(parent_chat_id)),
            (unrelated_chat_id, None),
        ] {
            store
                .create_chat(CreateChatParams {
                    session_id: chat_id.into(),
                    chat_id,
                    extra_config: None,
                    forked_from_id: None,
                    parent_chat_id,
                    source: SessionSource::Exec,
                    thread_source: None,
                    base_instructions: BaseInstructions::default(),
                    dynamic_tools: Vec::new(),
                    multi_agent_version: None,
                    metadata: ChatPersistenceMetadata {
                        cwd: None,
                        model_provider: "test-provider".to_string(),
                        memory_mode: ThreadMemoryMode::Enabled,
                    },
                })
                .await
                .expect("create thread");
        }

        let page = ChatStore::list_chats(
            &store,
            ListChatsParams {
                page_size: 10,
                cursor: None,
                sort_key: ChatSortKey::CreatedAt,
                sort_direction: SortDirection::Desc,
                allowed_sources: Vec::new(),
                model_providers: None,
                cwd_filters: None,
                archived: false,
                search_term: None,
                parent_chat_id: Some(parent_chat_id),
                use_state_db_only: false,
            },
        )
        .await
        .expect("list child threads");

        assert_eq!(
            page.items
                .into_iter()
                .map(|item| item.chat_id)
                .collect::<Vec<_>>(),
            vec![child_chat_id]
        );
    }
}

fn stores_guard() -> MutexGuard<'static, HashMap<String, Arc<InMemoryChatStore>>> {
    match stores().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Recorded call counts for [`InMemoryChatStore`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InMemoryChatStoreCalls {
    pub create_chat: usize,
    pub resume_chat: usize,
    pub append_items: usize,
    pub persist_chat: usize,
    pub flush_chat: usize,
    pub shutdown_chat: usize,
    pub discard_chat: usize,
    pub load_history: usize,
    pub read_chat: usize,
    pub read_chat_with_history: usize,
    pub read_chat_by_rollout_path: usize,
    pub list_chats: usize,
    pub update_chat_metadata: usize,
    pub archive_chat: usize,
    pub unarchive_chat: usize,
    pub delete_chat: usize,
}

/// In-memory [`ChatStore`] implementation for tests and debug configs.
///
/// Test and debug configs can select this store by id, letting tests exercise
/// config-driven non-local persistence without requiring the real remote gRPC
/// service.
#[derive(Default)]
pub struct InMemoryChatStore {
    state: tokio::sync::Mutex<InMemoryChatStoreState>,
}

#[derive(Default)]
struct InMemoryChatStoreState {
    calls: InMemoryChatStoreCalls,
    created_threads: HashMap<ChatId, CreateChatParams>,
    histories: HashMap<ChatId, Vec<RolloutMessage>>,
    metadata_updates: HashMap<ChatId, ChatMetadataPatch>,
    names: HashMap<ChatId, Option<String>>,
    rollout_paths: HashMap<PathBuf, ChatId>,
}

impl InMemoryChatStore {
    /// Returns the store associated with `id`, creating it if needed.
    pub fn for_id(id: impl Into<String>) -> Arc<Self> {
        let id = id.into();
        let mut stores = stores_guard();
        stores
            .entry(id)
            .or_insert_with(|| Arc::new(Self::default()))
            .clone()
    }

    /// Removes a shared in-memory store for `id`.
    pub fn remove_id(id: &str) -> Option<Arc<Self>> {
        stores_guard().remove(id)
    }

    /// Returns the calls observed by this store.
    pub async fn calls(&self) -> InMemoryChatStoreCalls {
        self.state.lock().await.calls.clone()
    }

    async fn create_chat(&self, params: CreateChatParams) -> ChatStoreResult<()> {
        let mut state = self.state.lock().await;
        state.calls.create_chat += 1;
        let session_meta = SessionMeta {
            session_id: params.session_id,
            id: params.chat_id,
            forked_from_id: params.forked_from_id,
            parent_chat_id: params.parent_chat_id,
            cwd: params.metadata.cwd.clone().unwrap_or_default(),
            agent_nickname: params.source.get_nickname(),
            agent_role: params.source.get_agent_role(),
            agent_path: params.source.get_agent_path().map(Into::into),
            source: params.source.clone(),
            thread_source: params.thread_source.clone(),
            model_provider: Some(params.metadata.model_provider.clone()),
            base_instructions: Some(params.base_instructions.clone()),
            dynamic_tools: (!params.dynamic_tools.is_empty()).then(|| params.dynamic_tools.clone()),
            memory_mode: matches!(params.metadata.memory_mode, ThreadMemoryMode::Disabled)
                .then_some("disabled".to_string()),
            multi_agent_version: params.multi_agent_version,
            ..SessionMeta::default()
        };
        state
            .histories
            .entry(params.chat_id)
            .or_default()
            .push(RolloutMessage::SessionMeta(SessionMetaLine {
                meta: session_meta,
                git: None,
            }));
        state.created_threads.insert(params.chat_id, params);
        Ok(())
    }

    async fn resume_chat(&self, params: ResumeChatParams) -> ChatStoreResult<()> {
        let mut state = self.state.lock().await;
        state.calls.resume_chat += 1;
        if let Some(history) = params.history {
            state.histories.insert(params.chat_id, history);
        } else {
            state.histories.entry(params.chat_id).or_default();
        }
        if let Some(rollout_path) = params.rollout_path {
            state.rollout_paths.insert(rollout_path, params.chat_id);
        }
        Ok(())
    }

    async fn append_items(&self, params: AppendChatMessagesParams) -> ChatStoreResult<()> {
        let canonical_items = persisted_rollout_items(params.items.as_slice());
        if canonical_items.is_empty() {
            return Ok(());
        }
        let mut state = self.state.lock().await;
        state.calls.append_items += 1;
        state
            .histories
            .entry(params.chat_id)
            .or_default()
            .extend(canonical_items);
        Ok(())
    }

    async fn load_history(
        &self,
        params: LoadChatHistoryParams,
    ) -> ChatStoreResult<StoredChatHistory> {
        let mut state = self.state.lock().await;
        state.calls.load_history += 1;
        let items = state.histories.get(&params.chat_id).cloned().ok_or(
            ChatStoreError::ChatNotFound {
                chat_id: params.chat_id,
            },
        )?;
        Ok(StoredChatHistory {
            chat_id: params.chat_id,
            items,
        })
    }

    async fn read_chat(&self, params: ReadChatParams) -> ChatStoreResult<StoredChat> {
        let mut state = self.state.lock().await;
        state.calls.read_chat += 1;
        if params.include_history {
            state.calls.read_chat_with_history += 1;
        }
        stored_chat_from_state(&state, params.chat_id, params.include_history)
    }

    async fn read_chat_by_rollout_path(
        &self,
        params: ReadChatByRolloutPathParams,
    ) -> ChatStoreResult<StoredChat> {
        let mut state = self.state.lock().await;
        state.calls.read_chat_by_rollout_path += 1;
        let Some(chat_id) = state.rollout_paths.get(&params.rollout_path).copied() else {
            return Err(ChatStoreError::InvalidRequest {
                message: format!(
                    "in-memory chat store does not know rollout path {}",
                    params.rollout_path.display()
                ),
            });
        };
        stored_chat_from_state(&state, chat_id, params.include_history)
    }

    async fn list_chats(&self) -> ChatStoreResult<ChatPage> {
        let mut state = self.state.lock().await;
        state.calls.list_chats += 1;
        let mut items = state
            .created_threads
            .keys()
            .map(|chat_id| {
                stored_chat_from_state(&state, *chat_id, /*include_history*/ false)
            })
            .collect::<ChatStoreResult<Vec<_>>>()?;
        items.sort_by_key(|item| item.chat_id.to_string());
        Ok(ChatPage {
            items,
            next_cursor: None,
        })
    }

    async fn update_chat_metadata(
        &self,
        params: UpdateChatMetadataParams,
    ) -> ChatStoreResult<StoredChat> {
        let mut state = self.state.lock().await;
        state.calls.update_chat_metadata += 1;
        if let Some(name) = params.patch.name.clone() {
            state.names.insert(params.chat_id, name);
        }
        state
            .metadata_updates
            .entry(params.chat_id)
            .or_default()
            .merge(params.patch);
        stored_chat_from_state(&state, params.chat_id, /*include_history*/ false)
    }

    async fn delete_chat(&self, params: DeleteChatParams) -> ChatStoreResult<()> {
        let mut state = self.state.lock().await;
        state.calls.delete_chat += 1;
        let existed = state.histories.remove(&params.chat_id).is_some();
        state.created_threads.remove(&params.chat_id);
        state.names.remove(&params.chat_id);
        state.metadata_updates.remove(&params.chat_id);
        state
            .rollout_paths
            .retain(|_, chat_id| *chat_id != params.chat_id);
        if existed {
            Ok(())
        } else {
            Err(ChatStoreError::ChatNotFound {
                chat_id: params.chat_id,
            })
        }
    }
}

impl ChatStore for InMemoryChatStore {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn create_chat(&self, params: CreateChatParams) -> ChatStoreFuture<'_, ()> {
        Box::pin(InMemoryChatStore::create_chat(self, params))
    }

    fn resume_chat(&self, params: ResumeChatParams) -> ChatStoreFuture<'_, ()> {
        Box::pin(InMemoryChatStore::resume_chat(self, params))
    }

    fn append_items(&self, params: AppendChatMessagesParams) -> ChatStoreFuture<'_, ()> {
        Box::pin(InMemoryChatStore::append_items(self, params))
    }

    fn persist_chat(&self, _chat_id: ChatId) -> ChatStoreFuture<'_, ()> {
        Box::pin(async move {
            self.state.lock().await.calls.persist_chat += 1;
            Ok(())
        })
    }

    fn flush_chat(&self, _chat_id: ChatId) -> ChatStoreFuture<'_, ()> {
        Box::pin(async move {
            self.state.lock().await.calls.flush_chat += 1;
            Ok(())
        })
    }

    fn shutdown_chat(&self, _chat_id: ChatId) -> ChatStoreFuture<'_, ()> {
        Box::pin(async move {
            self.state.lock().await.calls.shutdown_chat += 1;
            Ok(())
        })
    }

    fn discard_chat(&self, _chat_id: ChatId) -> ChatStoreFuture<'_, ()> {
        Box::pin(async move {
            self.state.lock().await.calls.discard_chat += 1;
            Ok(())
        })
    }

    fn load_history(
        &self,
        params: LoadChatHistoryParams,
    ) -> ChatStoreFuture<'_, StoredChatHistory> {
        Box::pin(InMemoryChatStore::load_history(self, params))
    }

    fn read_chat(&self, params: ReadChatParams) -> ChatStoreFuture<'_, StoredChat> {
        Box::pin(InMemoryChatStore::read_chat(self, params))
    }

    fn read_chat_by_rollout_path(
        &self,
        params: ReadChatByRolloutPathParams,
    ) -> ChatStoreFuture<'_, StoredChat> {
        Box::pin(InMemoryChatStore::read_chat_by_rollout_path(
            self, params,
        ))
    }

    fn list_chats(&self, params: ListChatsParams) -> ChatStoreFuture<'_, ChatPage> {
        Box::pin(async move {
            let mut page = InMemoryChatStore::list_chats(self).await?;
            if let Some(parent_chat_id) = params.parent_chat_id {
                page.items
                    .retain(|thread| thread.parent_chat_id == Some(parent_chat_id));
            }
            Ok(page)
        })
    }

    fn update_chat_metadata(
        &self,
        params: UpdateChatMetadataParams,
    ) -> ChatStoreFuture<'_, StoredChat> {
        Box::pin(InMemoryChatStore::update_chat_metadata(self, params))
    }

    fn archive_chat(&self, _params: ArchiveChatParams) -> ChatStoreFuture<'_, ()> {
        Box::pin(async move {
            self.state.lock().await.calls.archive_chat += 1;
            Ok(())
        })
    }

    fn unarchive_chat(&self, params: ArchiveChatParams) -> ChatStoreFuture<'_, StoredChat> {
        Box::pin(async move {
            let mut state = self.state.lock().await;
            state.calls.unarchive_chat += 1;
            stored_chat_from_state(&state, params.chat_id, /*include_history*/ false)
        })
    }

    fn delete_chat(&self, params: DeleteChatParams) -> ChatStoreFuture<'_, ()> {
        Box::pin(InMemoryChatStore::delete_chat(self, params))
    }
}

fn stored_chat_from_state(
    state: &InMemoryChatStoreState,
    chat_id: ChatId,
    include_history: bool,
) -> ChatStoreResult<StoredChat> {
    let created = state
        .created_threads
        .get(&chat_id)
        .ok_or(ChatStoreError::ChatNotFound { chat_id })?;
    let history_items = state.histories.get(&chat_id).cloned().unwrap_or_default();
    let history = include_history.then(|| StoredChatHistory {
        chat_id,
        items: history_items.clone(),
    });
    let name = state.names.get(&chat_id).cloned().flatten();
    let metadata = state.metadata_updates.get(&chat_id);
    let rollout_path = state
        .rollout_paths
        .iter()
        .find_map(|(path, mapped_chat_id)| {
            (*mapped_chat_id == chat_id).then(|| path.clone())
        });

    Ok(StoredChat {
        chat_id,
        extra_config: created.extra_config.clone(),
        rollout_path: metadata
            .and_then(|metadata| metadata.rollout_path.clone())
            .or(rollout_path),
        forked_from_id: created.forked_from_id,
        parent_chat_id: created.parent_chat_id,
        preview: metadata
            .and_then(|metadata| metadata.preview.clone())
            .unwrap_or_default(),
        name,
        model_provider: metadata
            .and_then(|metadata| metadata.model_provider.clone())
            .unwrap_or_else(|| "test".to_string()),
        model: metadata.and_then(|metadata| metadata.model.clone()),
        reasoning_effort: metadata.and_then(|metadata| metadata.reasoning_effort.clone()),
        created_at: metadata
            .and_then(|metadata| metadata.created_at)
            .unwrap_or_else(Utc::now),
        updated_at: metadata
            .and_then(|metadata| metadata.updated_at)
            .unwrap_or_else(Utc::now),
        recency_at: metadata
            .and_then(|metadata| metadata.advance_recency_at.or(metadata.updated_at))
            .unwrap_or_else(Utc::now),
        archived_at: None,
        cwd: metadata
            .and_then(|metadata| metadata.cwd.clone())
            .unwrap_or_default(),
        cli_version: metadata
            .and_then(|metadata| metadata.cli_version.clone())
            .unwrap_or_else(|| "test".to_string()),
        source: metadata
            .and_then(|metadata| metadata.source.clone())
            .unwrap_or_else(|| created.source.clone()),
        thread_source: metadata
            .and_then(|metadata| metadata.thread_source.clone())
            .unwrap_or_else(|| created.thread_source.clone()),
        agent_nickname: metadata.and_then(|metadata| metadata.agent_nickname.clone().flatten()),
        agent_role: metadata.and_then(|metadata| metadata.agent_role.clone().flatten()),
        agent_path: metadata.and_then(|metadata| metadata.agent_path.clone().flatten()),
        git_info: metadata.and_then(git_info_from_patch),
        approval_mode: metadata
            .and_then(|metadata| metadata.approval_mode)
            .unwrap_or(AskForApproval::Never),
        permission_profile: metadata
            .and_then(|metadata| metadata.permission_profile.clone())
            .unwrap_or_else(PermissionProfile::read_only),
        token_usage: metadata.and_then(|metadata| metadata.token_usage.clone()),
        first_user_message: metadata.and_then(|metadata| metadata.first_user_message.clone()),
        history,
    })
}

fn git_info_from_patch(patch: &ChatMetadataPatch) -> Option<datax_protocol::protocol::GitInfo> {
    let git_info = patch.git_info.as_ref()?;
    let sha = git_info.sha.clone().flatten();
    let branch = git_info.branch.clone().flatten();
    let origin_url = git_info.origin_url.clone().flatten();
    if sha.is_none() && branch.is_none() && origin_url.is_none() {
        return None;
    }
    Some(datax_protocol::protocol::GitInfo {
        commit_hash: sha.as_deref().map(datax_git_utils::GitSha::new),
        branch,
        repository_url: origin_url,
    })
}
