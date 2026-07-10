//! `chat/delete` request handling.

use super::chat_processor::core_thread_write_error;
use super::chat_processor::unsupported_chat_store_operation;
use super::*;

impl ChatRequestProcessor {
    pub(crate) async fn chat_delete(
        &self,
        request_id: ConnectionRequestId,
        params: ChatDeleteParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        let mut deleted_chat_ids = Vec::new();
        let result = {
            let _thread_list_state_permit = self.acquire_thread_list_state_permit().await?;
            self.chat_delete_response(params, &mut deleted_chat_ids)
                .await
        };
        match result {
            Ok(response) => {
                self.outgoing
                    .send_response(request_id.clone(), response)
                    .await;
                self.send_chat_deleted_notifications(deleted_chat_ids)
                    .await;
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    async fn chat_delete_response(
        &self,
        params: ChatDeleteParams,
        deleted_chat_ids: &mut Vec<String>,
    ) -> Result<ChatDeleteResponse, JSONRPCErrorError> {
        let chat_id = ChatId::from_string(&params.chat_id)
            .map_err(|err| invalid_request(format!("invalid thread id: {err}")))?;

        let mut chat_ids = self.state_db_spawn_subtree_chat_ids(chat_id).await?;
        let mut seen = chat_ids.iter().copied().collect::<HashSet<_>>();

        match self.chat_manager.list_agent_subtree_chat_ids(chat_id).await {
            Ok(live_chat_ids) => {
                for live_chat_id in live_chat_ids {
                    if seen.insert(live_chat_id) {
                        chat_ids.push(live_chat_id);
                    }
                }
            }
            Err(err) => return Err(core_thread_write_error("delete thread", err)),
        }

        self.validate_root_chat_delete(chat_id, chat_ids.len() > 1)
            .await?;
        for chat_id_to_delete in chat_ids.iter().copied() {
            self.prepare_thread_for_delete(chat_id_to_delete).await;
        }

        let mut delete_order: Vec<_> = chat_ids.iter().skip(1).rev().copied().collect();
        delete_order.push(chat_id);

        for chat_id_to_delete in delete_order.iter().copied() {
            match self
                .chat_store
                .delete_chat(StoreDeleteChatParams {
                    chat_id: chat_id_to_delete,
                })
                .await
            {
                Ok(()) => {}
                Err(ChatStoreError::ChatNotFound { .. }) => {
                    warn!(
                        "thread {chat_id_to_delete} was already missing while deleting {chat_id}"
                    );
                }
                Err(err) => {
                    return Err(chat_store_delete_error(err));
                }
            }
        }

        if let Some(state_db) = self.state_db.as_ref() {
            state_db
                .delete_chats_strict(chat_ids.as_slice())
                .await
                .map_err(|err| {
                    internal_error(format!(
                        "failed to delete app-server state for {chat_id}: {err}"
                    ))
                })?;
        }

        deleted_chat_ids.extend(delete_order.into_iter().map(|chat_id| chat_id.to_string()));
        Ok(ChatDeleteResponse {})
    }

    async fn send_chat_deleted_notifications(&self, deleted_chat_ids: Vec<String>) {
        for chat_id in deleted_chat_ids {
            self.outgoing
                .send_server_notification(ChatDeleted(ChatDeletedNotification { chat_id }))
                .await;
        }
    }

    async fn validate_root_chat_delete(
        &self,
        chat_id: ChatId,
        has_descendants: bool,
    ) -> Result<(), JSONRPCErrorError> {
        if let Ok(thread) = self.chat_manager.get_chat(chat_id).await {
            if !thread.config_snapshot().await.ephemeral {
                return Ok(());
            }
            return Err(invalid_request(format!(
                "thread is not persisted and cannot be deleted: {chat_id}"
            )));
        }
        match self
            .chat_store
            .read_chat(StoreReadChatParams {
                chat_id: chat_id,
                include_archived: true,
                include_history: false,
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(ChatStoreError::ChatNotFound { .. }) => {
                if has_descendants {
                    return Ok(());
                }
                let Some(state_db) = self.state_db.as_ref() else {
                    return Err(chat_store_delete_error(
                        ChatStoreError::ChatNotFound { chat_id: chat_id },
                    ));
                };
                if state_db
                    .get_chat(chat_id)
                    .await
                    .map_err(|err| {
                        internal_error(format!(
                            "failed to read app-server state for {chat_id}: {err}"
                        ))
                    })?
                    .is_some()
                {
                    Ok(())
                } else {
                    Err(chat_store_delete_error(
                        ChatStoreError::ChatNotFound { chat_id: chat_id },
                    ))
                }
            }
            Err(err) => Err(chat_store_delete_error(err)),
        }
    }

    async fn prepare_thread_for_delete(&self, chat_id: ChatId) {
        self.prepare_thread_for_removal(chat_id, "delete").await;
        if let Some(log_db) = self.log_db.as_ref() {
            log_db.flush().await;
        }
    }
}

fn chat_store_delete_error(err: ChatStoreError) -> JSONRPCErrorError {
    match err {
        ChatStoreError::ChatNotFound { chat_id: chat_id } => {
            invalid_request(format!("thread not found: {chat_id}"))
        }
        ChatStoreError::InvalidRequest { message } => invalid_request(message),
        ChatStoreError::Unsupported { operation } => {
            unsupported_chat_store_operation(operation)
        }
        err => internal_error(format!("failed to delete thread: {err}")),
    }
}
