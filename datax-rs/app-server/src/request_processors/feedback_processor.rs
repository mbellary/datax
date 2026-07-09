use super::*;
#[cfg(target_os = "windows")]
use datax_feedback::WINDOWS_SANDBOX_LOG_ATTACHMENT_FILENAME;

const MAX_FEEDBACK_TREE_THREADS: usize = 8;

#[derive(Clone)]
pub(crate) struct FeedbackRequestProcessor {
    auth_manager: Arc<AuthManager>,
    chat_manager: Arc<ChatManager>,
    config: Arc<Config>,
    feedback: CodexFeedback,
    log_db: Option<LogDbLayer>,
    state_db: Option<StateDbHandle>,
}

impl FeedbackRequestProcessor {
    pub(crate) fn new(
        auth_manager: Arc<AuthManager>,
        chat_manager: Arc<ChatManager>,
        config: Arc<Config>,
        feedback: CodexFeedback,
        log_db: Option<LogDbLayer>,
        state_db: Option<StateDbHandle>,
    ) -> Self {
        Self {
            auth_manager,
            chat_manager,
            config,
            feedback,
            log_db,
            state_db,
        }
    }

    pub(crate) async fn feedback_upload(
        &self,
        params: FeedbackUploadParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.upload_feedback_response(params)
            .await
            .map(|response| Some(response.into()))
    }

    async fn upload_feedback_response(
        &self,
        params: FeedbackUploadParams,
    ) -> Result<FeedbackUploadResponse, JSONRPCErrorError> {
        if !self.config.feedback_enabled {
            return Err(invalid_request(
                "sending feedback is disabled by configuration",
            ));
        }

        let FeedbackUploadParams {
            classification,
            reason,
            chat_id,
            include_logs,
            extra_log_files,
            tags,
        } = params;
        let mut upload_tags = tags.unwrap_or_default();

        let conversation_id = match chat_id.as_deref() {
            Some(chat_id) => match ChatId::from_string(chat_id) {
                Ok(conversation_id) => Some(conversation_id),
                Err(err) => return Err(invalid_request(format!("invalid thread id: {err}"))),
            },
            None => None,
        };

        if let Some(chatgpt_user_id) = self
            .auth_manager
            .auth_cached()
            .and_then(|auth| auth.get_chatgpt_user_id())
        {
            tracing::info!(target: "feedback_tags", chatgpt_user_id);
        }
        if let Some(account_id) = self
            .auth_manager
            .auth_cached()
            .and_then(|auth| auth.get_account_id())
        {
            tracing::info!(target: "feedback_tags", account_id);
        }
        let snapshot = self.feedback.snapshot(conversation_id);
        let chat_id = snapshot.chat_id.clone();
        let (feedback_chat_ids, sqlite_feedback_logs, state_db_ctx) = if include_logs {
            if let Some(log_db) = self.log_db.as_ref() {
                log_db.flush().await;
            }
            let state_db_ctx = self.state_db.clone();
            let feedback_chat_ids = match conversation_id {
                Some(conversation_id) => match self
                    .chat_manager
                    .list_agent_subtree_chat_ids(conversation_id)
                    .await
                {
                    Ok(chat_ids) => chat_ids,
                    Err(err) => {
                        warn!(
                            "failed to list feedback subtree for chat_id={conversation_id}: {err}"
                        );
                        let mut chat_ids = vec![conversation_id];
                        if let Some(state_db_ctx) = state_db_ctx.as_ref() {
                            for status in [
                                datax_state::DirectionalThreadSpawnEdgeStatus::Open,
                                datax_state::DirectionalThreadSpawnEdgeStatus::Closed,
                            ] {
                                match state_db_ctx
                                    .list_thread_spawn_descendants_with_status(
                                        conversation_id,
                                        status,
                                    )
                                    .await
                                {
                                    Ok(descendant_ids) => chat_ids.extend(descendant_ids),
                                    Err(err) => warn!(
                                        "failed to list persisted feedback subtree for chat_id={conversation_id}: {err}"
                                    ),
                                }
                            }
                        }
                        chat_ids
                    }
                },
                None => Vec::new(),
            };
            let mut feedback_chat_ids = feedback_chat_ids;
            let original_len = feedback_chat_ids.len();
            if let Some(conversation_id) = conversation_id {
                let mut descendant_chat_ids = feedback_chat_ids
                    .into_iter()
                    .filter(|chat_id| *chat_id != conversation_id)
                    .collect::<Vec<_>>();
                // Chat ids are UUIDv7, so lexicographic order tracks creation time.
                descendant_chat_ids.sort_unstable_by_key(ToString::to_string);
                if original_len > MAX_FEEDBACK_TREE_THREADS {
                    let keep_descendants = MAX_FEEDBACK_TREE_THREADS.saturating_sub(1);
                    let split_index = descendant_chat_ids.len().saturating_sub(keep_descendants);
                    descendant_chat_ids = descendant_chat_ids.split_off(split_index);
                    warn!(
                        "feedback log upload for chat_id={conversation_id:?} truncated from {original_len} threads to root plus {keep_descendants} most recent descendants"
                    );
                }
                feedback_chat_ids = Vec::with_capacity(descendant_chat_ids.len() + 1);
                feedback_chat_ids.push(conversation_id);
                feedback_chat_ids.extend(descendant_chat_ids);
            }
            let sqlite_feedback_logs = if let Some(state_db_ctx) = state_db_ctx.as_ref()
                && !feedback_chat_ids.is_empty()
            {
                let chat_id_texts = feedback_chat_ids
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                let chat_id_refs = chat_id_texts.iter().map(String::as_str).collect::<Vec<_>>();
                match state_db_ctx
                    .query_feedback_logs_for_threads(&chat_id_refs)
                    .await
                {
                    Ok(logs) if logs.is_empty() => None,
                    Ok(logs) => Some(logs),
                    Err(err) => {
                        let chat_ids = chat_id_texts.join(", ");
                        warn!(
                            "failed to query feedback logs from sqlite for chat_ids=[{chat_ids}]: {err}"
                        );
                        None
                    }
                }
            } else {
                None
            };
            (feedback_chat_ids, sqlite_feedback_logs, state_db_ctx)
        } else {
            (Vec::new(), None, None)
        };

        let mut attachment_paths = Vec::new();
        let mut seen_attachment_paths = HashSet::new();
        if include_logs {
            for feedback_chat_id in &feedback_chat_ids {
                let Some(rollout_path) = self
                    .resolve_rollout_path(*feedback_chat_id, state_db_ctx.as_ref())
                    .await
                else {
                    continue;
                };
                if seen_attachment_paths.insert(rollout_path.clone()) {
                    attachment_paths.push(FeedbackAttachmentPath {
                        path: rollout_path,
                        attachment_filename_override: None,
                    });
                }
            }
            if let Some(conversation_id) = conversation_id
                && let Ok(conversation) = self.chat_manager.get_chat(conversation_id).await
                && let Some(guardian_rollout_path) =
                    conversation.guardian_trunk_rollout_path().await
                && seen_attachment_paths.insert(guardian_rollout_path.clone())
            {
                attachment_paths.push(FeedbackAttachmentPath {
                    path: guardian_rollout_path,
                    attachment_filename_override: Some(auto_review_rollout_filename(
                        conversation_id,
                    )),
                });
            }
            if let Some(sandbox_log_attachment) =
                windows_sandbox_log_attachment(&self.config.codex_home)
                && seen_attachment_paths.insert(sandbox_log_attachment.path.clone())
            {
                attachment_paths.push(sandbox_log_attachment);
            }
        }
        if let Some(extra_log_files) = extra_log_files {
            for extra_log_file in extra_log_files {
                if seen_attachment_paths.insert(extra_log_file.clone()) {
                    attachment_paths.push(FeedbackAttachmentPath {
                        path: extra_log_file,
                        attachment_filename_override: None,
                    });
                }
            }
        }

        let mut extra_attachments = Vec::new();
        if include_logs
            && let Some(doctor_report) =
                super::feedback_doctor_report::doctor_feedback_report(&self.config).await
        {
            extra_attachments.push(doctor_report.attachment);
            for (key, value) in doctor_report.tags {
                upload_tags.entry(key).or_insert(value);
            }
        }

        let session_source = self.chat_manager.session_source();

        let upload_result = tokio::task::spawn_blocking(move || {
            let tags = (!upload_tags.is_empty()).then_some(&upload_tags);
            snapshot.upload_feedback(FeedbackUploadOptions {
                classification: &classification,
                reason: reason.as_deref(),
                tags,
                include_logs,
                extra_attachments: &extra_attachments,
                extra_attachment_paths: &attachment_paths,
                session_source: Some(session_source),
                logs_override: sqlite_feedback_logs,
            })
        })
        .await;

        let upload_result = match upload_result {
            Ok(result) => result,
            Err(join_err) => {
                return Err(internal_error(format!(
                    "failed to upload feedback: {join_err}"
                )));
            }
        };

        upload_result.map_err(|err| internal_error(format!("failed to upload feedback: {err}")))?;
        Ok(FeedbackUploadResponse { chat_id })
    }

    async fn resolve_rollout_path(
        &self,
        conversation_id: ChatId,
        state_db_ctx: Option<&StateDbHandle>,
    ) -> Option<PathBuf> {
        if let Ok(conversation) = self.chat_manager.get_chat(conversation_id).await
            && let Some(rollout_path) = conversation.rollout_path()
        {
            return Some(rollout_path);
        }

        let state_db_ctx = state_db_ctx?;
        state_db_ctx
            .find_rollout_path_by_id(conversation_id, /*archived_only*/ None)
            .await
            .unwrap_or_else(|err| {
                warn!("failed to resolve rollout path for chat_id={conversation_id}: {err}");
                None
            })
    }
}

fn auto_review_rollout_filename(chat_id: ChatId) -> String {
    format!("auto-review-rollout-{chat_id}.jsonl")
}

#[cfg(target_os = "windows")]
fn windows_sandbox_log_attachment(codex_home: &Path) -> Option<FeedbackAttachmentPath> {
    let sandbox_log_path = datax_windows_sandbox::current_log_file_path_for_codex_home(codex_home);
    sandbox_log_path
        .is_file()
        .then_some(FeedbackAttachmentPath {
            path: sandbox_log_path,
            attachment_filename_override: Some(WINDOWS_SANDBOX_LOG_ATTACHMENT_FILENAME.to_string()),
        })
}

#[cfg(not(target_os = "windows"))]
fn windows_sandbox_log_attachment(_codex_home: &Path) -> Option<FeedbackAttachmentPath> {
    None
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn windows_sandbox_log_attachment_uses_current_log() {
        let codex_home = tempfile::tempdir().expect("create tempdir");
        let sandbox_dir = datax_windows_sandbox::sandbox_dir(codex_home.path());
        std::fs::create_dir_all(&sandbox_dir).expect("create sandbox dir");
        let sandbox_log_path =
            datax_windows_sandbox::current_log_file_path_for_codex_home(codex_home.path());
        std::fs::write(&sandbox_log_path, "sandbox log").expect("write sandbox log");

        let attachment = windows_sandbox_log_attachment(codex_home.path())
            .map(|attachment| (attachment.path, attachment.attachment_filename_override));

        assert_eq!(
            attachment,
            Some((
                sandbox_log_path,
                Some(WINDOWS_SANDBOX_LOG_ATTACHMENT_FILENAME.to_string())
            ))
        );
    }
}
