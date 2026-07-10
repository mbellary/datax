use chrono::DateTime;
use chrono::Utc;
use datax_protocol::models::PermissionProfile;
use datax_protocol::protocol::AskForApproval;
use datax_protocol::protocol::SessionMetaLine;
use datax_protocol::protocol::SessionSource;
use datax_rollout::RolloutRecorder;
use datax_rollout::find_archived_thread_path_by_id_str;
use datax_rollout::find_thread_name_by_id;
use datax_rollout::find_thread_path_by_id_str;
use datax_rollout::read_session_meta_line;
use datax_rollout::read_thread_item_from_rollout;
use datax_state::ThreadMetadata;

use super::LocalChatStore;
use super::helpers::distinct_chat_metadata_title;
use super::helpers::git_info_from_parts;
use super::helpers::permission_profile_from_metadata_value;
use super::helpers::rollout_path_is_archived;
use super::helpers::set_thread_name_from_title;
use super::helpers::stored_chat_from_rollout_item;
use super::live_writer;
use crate::ReadChatParams;
use crate::StoredChat;
use crate::StoredChatHistory;
use crate::ChatStoreError;
use crate::ChatStoreResult;

pub(super) async fn read_chat(
    store: &LocalChatStore,
    params: ReadChatParams,
) -> ChatStoreResult<StoredChat> {
    let chat_id = params.chat_id;
    if let Some(metadata) = read_sqlite_metadata(store, chat_id).await
        && (params.include_archived
            || (metadata.archived_at.is_none()
                && !rollout_path_is_archived(
                    store.config.codex_home.as_path(),
                    metadata.rollout_path.as_path(),
                )))
        && (!params.include_history
            || sqlite_rollout_path_can_load_history_for_thread(
                store,
                &metadata.rollout_path,
                chat_id,
            )
            .await)
    {
        let metadata_sandbox_policy = metadata.sandbox_policy.clone();
        let mut thread = stored_chat_from_sqlite_metadata(store, metadata).await;
        if !params.include_history
            && let Some(rollout_path) = thread.rollout_path.clone()
            && let Ok(mut rollout_thread) = read_chat_from_rollout_path(store, rollout_path).await
            && rollout_thread.chat_id == chat_id
            && (params.include_archived || rollout_thread.archived_at.is_none())
            && !rollout_thread.preview.is_empty()
        {
            rollout_thread.recency_at = thread.recency_at;
            if thread.name.is_some() {
                rollout_thread.name = thread.name;
            }
            rollout_thread.git_info = thread.git_info;
            rollout_thread.permission_profile = permission_profile_from_metadata_value(
                &metadata_sandbox_policy,
                rollout_thread.cwd.as_path(),
            );
            thread = rollout_thread;
        }
        attach_history_if_requested(&mut thread, params.include_history).await?;
        return Ok(thread);
    }

    let path = resolve_rollout_path(store, chat_id, params.include_archived)
        .await?
        .ok_or_else(|| ChatStoreError::InvalidRequest {
            message: format!("no rollout found for thread id {chat_id}"),
        })?;

    let mut thread = read_chat_from_rollout_path(store, path).await?;
    if !params.include_archived && thread.archived_at.is_some() {
        return Err(ChatStoreError::InvalidRequest {
            message: format!("thread {} is archived", thread.chat_id),
        });
    }
    attach_history_if_requested(&mut thread, params.include_history).await?;
    Ok(thread)
}

async fn sqlite_rollout_path_can_load_history_for_thread(
    store: &LocalChatStore,
    path: &std::path::Path,
    chat_id: datax_protocol::ChatId,
) -> bool {
    if datax_rollout::existing_rollout_path(path).await.is_none() {
        return false;
    }
    // SQLite metadata can outlive a moved/recreated rollout path. When history is
    // requested, verify the path still resolves to the requested thread before
    // trusting it as the source replay.
    read_chat_from_rollout_path(store, path.to_path_buf())
        .await
        .is_ok_and(|thread| thread.chat_id == chat_id)
}

pub(super) async fn read_chat_by_rollout_path(
    store: &LocalChatStore,
    rollout_path: std::path::PathBuf,
    include_archived: bool,
    include_history: bool,
) -> ChatStoreResult<StoredChat> {
    let path = resolve_requested_rollout_path(store, rollout_path).await?;
    let mut thread = read_chat_from_rollout_path(store, path).await?;
    if !include_archived && thread.archived_at.is_some() {
        return Err(ChatStoreError::InvalidRequest {
            message: format!("thread {} is archived", thread.chat_id),
        });
    }
    if let Some(metadata) = read_sqlite_metadata(store, thread.chat_id).await {
        thread.recency_at = metadata.recency_at;
        let existing_git_info = thread.git_info.take();
        let (fallback_sha, fallback_branch, fallback_origin_url) = match existing_git_info {
            Some(info) => (
                info.commit_hash.map(|sha| sha.0),
                info.branch,
                info.repository_url,
            ),
            None => (None, None, None),
        };
        thread.git_info = git_info_from_parts(
            metadata.git_sha.or(fallback_sha),
            metadata.git_branch.or(fallback_branch),
            metadata.git_origin_url.or(fallback_origin_url),
        );
    }
    attach_history_if_requested(&mut thread, include_history).await?;
    Ok(thread)
}

async fn resolve_requested_rollout_path(
    store: &LocalChatStore,
    rollout_path: std::path::PathBuf,
) -> ChatStoreResult<std::path::PathBuf> {
    let path = if rollout_path.is_relative() {
        store.config.codex_home.join(rollout_path)
    } else {
        rollout_path
    };
    match tokio::fs::metadata(path.as_path()).await {
        Ok(metadata) if metadata.is_dir() => {
            return Err(ChatStoreError::InvalidRequest {
                message: format!(
                    "failed to resolve rollout path `{}`: path is a directory",
                    path.display()
                ),
            });
        }
        Ok(metadata) if !metadata.is_file() => {
            return Err(ChatStoreError::InvalidRequest {
                message: format!(
                    "failed to resolve rollout path `{}`: path is not a file",
                    path.display()
                ),
            });
        }
        _ => {}
    }
    let Some(path) = datax_rollout::existing_rollout_path(path.as_path()).await else {
        return Err(ChatStoreError::InvalidRequest {
            message: format!(
                "failed to resolve rollout path `{}`: file does not exist",
                path.display()
            ),
        });
    };
    std::fs::canonicalize(path.as_path()).map_err(|err| ChatStoreError::InvalidRequest {
        message: format!("failed to resolve rollout path `{}`: {err}", path.display()),
    })
}

async fn attach_history_if_requested(
    thread: &mut StoredChat,
    include_history: bool,
) -> ChatStoreResult<()> {
    if !include_history {
        return Ok(());
    }
    let chat_id = thread.chat_id;
    let Some(path) = thread.rollout_path.clone() else {
        return Err(ChatStoreError::Internal {
            message: format!("failed to load thread history for thread {chat_id}"),
        });
    };
    let items = load_history_items(&path).await?;
    thread.history = Some(StoredChatHistory { chat_id, items });
    Ok(())
}

async fn resolve_rollout_path(
    store: &LocalChatStore,
    chat_id: datax_protocol::ChatId,
    include_archived: bool,
) -> ChatStoreResult<Option<std::path::PathBuf>> {
    if let Ok(path) = live_writer::rollout_path(store, chat_id).await
        && datax_rollout::existing_rollout_path(path.as_path())
            .await
            .is_some()
        && (include_archived || !rollout_path_is_archived(store.config.codex_home.as_path(), &path))
    {
        return Ok(Some(path));
    }

    let state_db_ctx = store.state_db().await;
    if include_archived {
        match find_thread_path_by_id_str(
            store.config.codex_home.as_path(),
            &chat_id.to_string(),
            state_db_ctx.as_deref(),
        )
        .await
        .map_err(|err| ChatStoreError::InvalidRequest {
            message: format!("failed to locate thread id {chat_id}: {err}"),
        })? {
            Some(path) => Ok(Some(path)),
            None => find_archived_thread_path_by_id_str(
                store.config.codex_home.as_path(),
                &chat_id.to_string(),
                state_db_ctx.as_deref(),
            )
            .await
            .map_err(|err| ChatStoreError::InvalidRequest {
                message: format!("failed to locate archived thread id {chat_id}: {err}"),
            }),
        }
    } else {
        find_thread_path_by_id_str(
            store.config.codex_home.as_path(),
            &chat_id.to_string(),
            state_db_ctx.as_deref(),
        )
        .await
        .map_err(|err| ChatStoreError::InvalidRequest {
            message: format!("failed to locate thread id {chat_id}: {err}"),
        })
    }
}

async fn read_chat_from_rollout_path(
    store: &LocalChatStore,
    path: std::path::PathBuf,
) -> ChatStoreResult<StoredChat> {
    let Some(item) = read_thread_item_from_rollout(path.clone()).await else {
        return stored_chat_from_session_meta(store, path).await;
    };
    let archived = rollout_path_is_archived(store.config.codex_home.as_path(), path.as_path());
    let mut thread = stored_chat_from_rollout_item(
        item,
        archived,
        store.config.default_model_provider_id.as_str(),
    )
    .ok_or_else(|| ChatStoreError::Internal {
        message: format!("failed to read thread id from {}", path.display()),
    })?;
    thread.rollout_path = Some(datax_rollout::plain_rollout_path(path.as_path()));
    if let Ok(meta_line) = read_session_meta_line(path.as_path()).await {
        thread.forked_from_id = meta_line.meta.forked_from_id;
        thread.parent_chat_id = meta_line.meta.parent_chat_id;
        if let Some(model_provider) = meta_line
            .meta
            .model_provider
            .filter(|provider| !provider.is_empty())
        {
            thread.model_provider = model_provider;
        }
    }
    if let Ok(Some(title)) =
        find_thread_name_by_id(store.config.codex_home.as_path(), &thread.chat_id).await
    {
        set_thread_name_from_title(&mut thread, title);
    }
    Ok(thread)
}

async fn load_history_items(
    path: &std::path::Path,
) -> ChatStoreResult<Vec<datax_protocol::protocol::RolloutMessage>> {
    let (items, _, _) = RolloutRecorder::load_rollout_items(path)
        .await
        .map_err(|err| ChatStoreError::Internal {
            message: format!("failed to load thread history {}: {err}", path.display()),
        })?;
    Ok(items)
}

async fn read_sqlite_metadata(
    store: &LocalChatStore,
    chat_id: datax_protocol::ChatId,
) -> Option<ThreadMetadata> {
    let runtime = store.state_db().await?;
    runtime.get_chat(chat_id).await.ok().flatten()
}

async fn stored_chat_from_sqlite_metadata(
    store: &LocalChatStore,
    metadata: ThreadMetadata,
) -> StoredChat {
    let name = match distinct_chat_metadata_title(&metadata) {
        Some(title) => Some(title),
        None => find_thread_name_by_id(store.config.codex_home.as_path(), &metadata.id)
            .await
            .ok()
            .flatten()
            .filter(|title| !title.trim().is_empty()),
    };
    let session_meta = read_session_meta_line(metadata.rollout_path.as_path())
        .await
        .ok()
        .map(|meta_line| meta_line.meta);
    let rollout_path = datax_rollout::plain_rollout_path(metadata.rollout_path.as_path());
    let forked_from_id = session_meta.as_ref().and_then(|meta| meta.forked_from_id);
    let parent_chat_id = session_meta.as_ref().and_then(|meta| meta.parent_chat_id);
    let preview = metadata
        .preview
        .clone()
        .or_else(|| metadata.first_user_message.clone())
        .unwrap_or_default();
    let permission_profile =
        permission_profile_from_metadata_value(&metadata.sandbox_policy, metadata.cwd.as_path());
    StoredChat {
        chat_id: metadata.id,
        extra_config: None,
        rollout_path: Some(rollout_path),
        forked_from_id,
        parent_chat_id,
        preview,
        name,
        model_provider: if metadata.model_provider.is_empty() {
            store.config.default_model_provider_id.clone()
        } else {
            metadata.model_provider
        },
        model: metadata.model,
        reasoning_effort: metadata.reasoning_effort,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
        recency_at: metadata.recency_at,
        archived_at: metadata.archived_at,
        cwd: metadata.cwd,
        cli_version: metadata.cli_version,
        source: parse_session_source(&metadata.source),
        chat_source: metadata.chat_source,
        agent_nickname: metadata.agent_nickname,
        agent_role: metadata.agent_role,
        agent_path: metadata.agent_path,
        git_info: git_info_from_parts(
            metadata.git_sha,
            metadata.git_branch,
            metadata.git_origin_url,
        ),
        approval_mode: parse_or_default(&metadata.approval_mode, AskForApproval::OnRequest),
        permission_profile,
        token_usage: None,
        first_user_message: metadata.first_user_message,
        history: None,
    }
}

async fn stored_chat_from_session_meta(
    store: &LocalChatStore,
    path: std::path::PathBuf,
) -> ChatStoreResult<StoredChat> {
    let meta_line = read_session_meta_line(path.as_path())
        .await
        .map_err(|err| ChatStoreError::Internal {
            message: format!("failed to read thread {}: {err}", path.display()),
        })?;
    let archived = rollout_path_is_archived(store.config.codex_home.as_path(), path.as_path());
    Ok(stored_chat_from_meta_line(
        store, meta_line, path, archived,
    ))
}

fn stored_chat_from_meta_line(
    store: &LocalChatStore,
    meta_line: SessionMetaLine,
    path: std::path::PathBuf,
    archived: bool,
) -> StoredChat {
    let created_at = parse_rfc3339_non_optional(&meta_line.meta.timestamp).unwrap_or_else(Utc::now);
    let updated_at = std::fs::metadata(path.as_path())
        .ok()
        .and_then(|meta| meta.modified().ok())
        .map(DateTime::<Utc>::from)
        .unwrap_or(created_at);
    let rollout_path = datax_rollout::plain_rollout_path(path.as_path());
    StoredChat {
        chat_id: meta_line.meta.id,
        extra_config: None,
        rollout_path: Some(rollout_path),
        forked_from_id: meta_line.meta.forked_from_id,
        parent_chat_id: meta_line.meta.parent_chat_id,
        preview: String::new(),
        name: None,
        model_provider: meta_line
            .meta
            .model_provider
            .filter(|provider| !provider.is_empty())
            .unwrap_or_else(|| store.config.default_model_provider_id.clone()),
        model: None,
        reasoning_effort: None,
        created_at,
        updated_at,
        recency_at: updated_at,
        archived_at: archived.then_some(updated_at),
        cwd: meta_line.meta.cwd,
        cli_version: meta_line.meta.cli_version,
        source: meta_line.meta.source,
        chat_source: meta_line.meta.chat_source,
        agent_nickname: meta_line.meta.agent_nickname,
        agent_role: meta_line.meta.agent_role,
        agent_path: meta_line.meta.agent_path,
        git_info: meta_line.git,
        approval_mode: AskForApproval::OnRequest,
        permission_profile: PermissionProfile::read_only(),
        token_usage: None,
        first_user_message: None,
        history: None,
    }
}

fn parse_session_source(source: &str) -> SessionSource {
    serde_json::from_str(source)
        .or_else(|_| serde_json::from_value(serde_json::Value::String(source.to_string())))
        .unwrap_or(SessionSource::Unknown)
}

fn parse_or_default<T>(value: &str, default: T) -> T
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(value)
        .or_else(|_| serde_json::from_value(serde_json::Value::String(value.to_string())))
        .unwrap_or(default)
}

fn parse_rfc3339_non_optional(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::PathBuf;

    use chrono::Utc;
    use datax_protocol::ChatId;
    use datax_protocol::protocol::SandboxPolicy;
    use datax_protocol::protocol::SessionSource;
    use datax_state::ThreadMetadataBuilder;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::*;
    use crate::ChatStore;
    use crate::local::LocalChatStore;
    use crate::local::test_support::test_config;
    use crate::local::test_support::write_archived_session_file;
    use crate::local::test_support::write_session_file;
    use crate::local::test_support::write_session_file_with_fork;

    #[tokio::test]
    async fn read_chat_returns_active_rollout_summary() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let uuid = Uuid::from_u128(205);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let active_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: false,
                include_history: true,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.chat_id, chat_id);
        assert_eq!(thread.rollout_path, Some(active_path));
        assert_eq!(thread.archived_at, None);
        assert_eq!(thread.preview, "Hello from user");
        assert_eq!(
            thread.history.expect("history should load").chat_id,
            chat_id
        );
    }

    #[tokio::test]
    async fn read_chat_returns_rollout_path_summary() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let uuid = Uuid::from_u128(211);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let active_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        let relative_path = active_path
            .strip_prefix(home.path())
            .expect("path should be under codex home")
            .to_path_buf();

        let thread = store
            .read_chat_by_rollout_path(
                relative_path,
                /*include_archived*/ false,
                /*include_history*/ false,
            )
            .await
            .expect("read thread by rollout path");

        assert_eq!(thread.chat_id, chat_id);
        assert_eq!(
            thread.rollout_path,
            Some(std::fs::canonicalize(active_path).expect("canonical path"))
        );
        assert_eq!(thread.preview, "Hello from user");
    }

    #[tokio::test]
    async fn read_chat_by_rollout_path_prefers_sqlite_git_info() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let uuid = Uuid::from_u128(223);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let active_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = LocalChatStore::new(config.clone(), Some(runtime.clone()));
        let mut builder = ThreadMetadataBuilder::new(
            chat_id,
            active_path.clone(),
            Utc::now(),
            SessionSource::Cli,
        );
        builder.model_provider = Some(config.default_model_provider_id.clone());
        builder.git_branch = Some("sqlite-branch".to_string());
        let recency_at = chrono::DateTime::parse_from_rfc3339("2026-01-03T12:00:00Z")
            .expect("timestamp should parse")
            .with_timezone(&Utc);
        builder.recency_at = Some(recency_at);
        runtime
            .upsert_thread(&builder.build(config.default_model_provider_id.as_str()))
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_chat_by_rollout_path(
                active_path,
                /*include_archived*/ false,
                /*include_history*/ false,
            )
            .await
            .expect("read thread by rollout path");

        let git_info = thread.git_info.expect("git info should be present");
        assert_eq!(thread.recency_at, recency_at);
        assert_eq!(git_info.branch.as_deref(), Some("sqlite-branch"));
        assert_eq!(
            git_info.commit_hash.as_ref().map(|sha| sha.0.as_str()),
            Some("abcdef")
        );
        assert_eq!(
            git_info.repository_url.as_deref(),
            Some("https://example.com/repo.git")
        );
    }

    #[tokio::test]
    async fn read_chat_returns_archived_rollout_when_requested() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let uuid = Uuid::from_u128(207);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let archived_path = write_archived_session_file(home.path(), "2025-01-03T12-00-00", uuid)
            .expect("archived session file");

        let active_only_err = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect_err("active-only read should fail for archived rollout");
        let ChatStoreError::InvalidRequest { message } = active_only_err else {
            panic!("expected invalid request error");
        };
        assert_eq!(
            message,
            format!("no rollout found for thread id {chat_id}")
        );

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: true,
                include_history: false,
            })
            .await
            .expect("read archived thread");

        assert_eq!(thread.chat_id, chat_id);
        assert_eq!(thread.rollout_path, Some(archived_path));
        assert!(thread.archived_at.is_some());
        assert_eq!(thread.preview, "Archived user message");
        assert!(thread.history.is_none());
    }

    #[tokio::test]
    async fn read_chat_prefers_active_rollout_over_archived() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let uuid = Uuid::from_u128(208);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let active_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        write_archived_session_file(home.path(), "2025-01-03T12-00-00", uuid)
            .expect("archived session file");

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: true,
                include_history: false,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.rollout_path, Some(active_path));
        assert_eq!(thread.archived_at, None);
        assert_eq!(thread.preview, "Hello from user");
    }

    #[tokio::test]
    async fn read_chat_returns_forked_from_id() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let uuid = Uuid::from_u128(209);
        let parent_uuid = Uuid::from_u128(210);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let parent_chat_id =
            ChatId::from_string(&parent_uuid.to_string()).expect("valid parent thread id");
        write_session_file_with_fork(
            home.path(),
            home.path().join("sessions/2025/01/03"),
            "2025-01-03T12-00-00",
            uuid,
            "Forked user message",
            Some("test-provider"),
            Some(parent_uuid),
        )
        .expect("forked session file");

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.forked_from_id, Some(parent_chat_id));
    }

    #[tokio::test]
    async fn read_chat_applies_sqlite_thread_name() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let uuid = Uuid::from_u128(212);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = LocalChatStore::new(config.clone(), Some(runtime.clone()));
        let mut builder =
            ThreadMetadataBuilder::new(chat_id, rollout_path, Utc::now(), SessionSource::Cli);
        builder.model_provider = Some(config.default_model_provider_id.clone());
        builder.cwd = home.path().to_path_buf();
        builder.cli_version = Some("test_version".to_string());
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.title = "Saved title".to_string();
        metadata.first_user_message = Some("Hello from user".to_string());
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.name, Some("Saved title".to_string()));
    }

    #[tokio::test]
    async fn read_chat_returns_permission_profile_from_sqlite_metadata() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let uuid = Uuid::from_u128(225);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = LocalChatStore::new(config.clone(), Some(runtime.clone()));
        let mut builder =
            ThreadMetadataBuilder::new(chat_id, rollout_path, Utc::now(), SessionSource::Cli);
        builder.model_provider = Some(config.default_model_provider_id.clone());
        builder.cwd = home.path().to_path_buf();
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        let permission_profile: PermissionProfile = PermissionProfile::Disabled;
        metadata.sandbox_policy =
            serde_json::to_string(&permission_profile).expect("serialize profile");
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.preview, "Hello from user");
        assert_eq!(thread.permission_profile, PermissionProfile::Disabled);
    }

    #[tokio::test]
    async fn read_chat_accepts_legacy_sandbox_policy_metadata() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let uuid = Uuid::from_u128(226);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = LocalChatStore::new(config.clone(), Some(runtime.clone()));
        let mut builder =
            ThreadMetadataBuilder::new(chat_id, rollout_path, Utc::now(), SessionSource::Cli);
        builder.model_provider = Some(config.default_model_provider_id.clone());
        builder.cwd = home.path().to_path_buf();
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.sandbox_policy = "danger-full-access".to_string();
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: false,
                include_history: true,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.permission_profile, PermissionProfile::Disabled);
    }

    #[tokio::test]
    async fn read_chat_preserves_rollout_cwd_when_sqlite_metadata_exists() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = LocalChatStore::new(config.clone(), Some(runtime.clone()));
        let uuid = Uuid::from_u128(224);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let day_dir = home.path().join("sessions/2025/01/03");
        std::fs::create_dir_all(&day_dir).expect("sessions dir");
        let rollout_path = day_dir.join(format!("rollout-2025-01-03T12-00-00-{uuid}.jsonl"));
        let mut file = std::fs::File::create(&rollout_path).expect("session file");
        let rollout_cwd = PathBuf::from("/");
        let meta = serde_json::json!({
            "timestamp": "2025-01-03T12:00:00Z",
            "type": "session_meta",
            "payload": {
                "session_id": uuid,
                "id": uuid,
                "timestamp": "2025-01-03T12:00:00Z",
                "cwd": rollout_cwd,
                "originator": "test_originator",
                "cli_version": "test_version",
                "source": "cli",
                "model_provider": "rollout-provider"
            },
        });
        writeln!(file, "{meta}").expect("write session meta");
        let user_event = serde_json::json!({
            "timestamp": "2025-01-03T12:00:00Z",
            "type": "event_msg",
            "payload": {
                "type": "user_message",
                "message": "Hello from rollout",
                "kind": "plain",
            },
        });
        writeln!(file, "{user_event}").expect("write user event");

        let mut builder = ThreadMetadataBuilder::new(
            chat_id,
            rollout_path.clone(),
            Utc::now(),
            SessionSource::Cli,
        );
        builder.model_provider = Some(config.default_model_provider_id.clone());
        builder.cwd = home.path().join("sqlite-workspace");
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.title = "Saved title".to_string();
        metadata.first_user_message = Some("Hello from sqlite".to_string());
        metadata.sandbox_policy = "workspace-write".to_string();
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.chat_id, chat_id);
        assert_eq!(thread.rollout_path, Some(rollout_path));
        assert_eq!(thread.preview, "Hello from rollout");
        assert_eq!(thread.name, Some("Saved title".to_string()));
        assert_eq!(thread.model_provider, "rollout-provider");
        assert_eq!(thread.cwd, rollout_cwd);
        let legacy_policy = SandboxPolicy::WorkspaceWrite {
            writable_roots: Vec::new(),
            network_access: false,
            exclude_tmpdir_env_var: false,
            exclude_slash_tmp: false,
        };
        assert_eq!(
            thread.permission_profile,
            PermissionProfile::from_legacy_sandbox_policy_for_cwd(
                &legacy_policy,
                rollout_cwd.as_path()
            )
        );
    }

    #[tokio::test]
    async fn read_chat_uses_legacy_thread_name_when_sqlite_title_is_missing() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let uuid = Uuid::from_u128(213);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        datax_rollout::append_thread_name(home.path(), chat_id, "Legacy title")
            .await
            .expect("append legacy thread name");

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.name, Some("Legacy title".to_string()));
    }

    #[tokio::test]
    async fn read_chat_uses_sqlite_metadata_for_rollout_without_user_preview() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let uuid = Uuid::from_u128(217);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let day_dir = home.path().join("sessions/2025/01/03");
        std::fs::create_dir_all(&day_dir).expect("sessions dir");
        let rollout_path = day_dir.join(format!("rollout-2025-01-03T12-00-00-{uuid}.jsonl"));
        let mut file = std::fs::File::create(&rollout_path).expect("session file");
        let meta = serde_json::json!({
            "timestamp": "2025-01-03T12-00-00",
            "type": "session_meta",
            "payload": {
                "session_id": uuid,
                "id": uuid,
                "timestamp": "2025-01-03T12-00-00",
                "cwd": home.path(),
                "originator": "test_originator",
                "cli_version": "test_version",
                "source": "cli",
                "model_provider": "rollout-provider"
            },
        });
        writeln!(file, "{meta}").expect("write session meta");

        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = LocalChatStore::new(config.clone(), Some(runtime.clone()));
        let mut builder = ThreadMetadataBuilder::new(
            chat_id,
            rollout_path.clone(),
            Utc::now(),
            SessionSource::Cli,
        );
        builder.model_provider = Some("sqlite-provider".to_string());
        builder.cwd = home.path().join("workspace");
        builder.cli_version = Some("sqlite-cli".to_string());
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.title = "Command-only thread".to_string();
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: false,
                include_history: true,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.chat_id, chat_id);
        assert_eq!(thread.rollout_path, Some(rollout_path));
        assert_eq!(thread.preview, "");
        assert_eq!(thread.name.as_deref(), Some("Command-only thread"));
        assert_eq!(thread.model_provider, "sqlite-provider");
        assert_eq!(thread.cwd, home.path().join("workspace"));
        assert_eq!(thread.cli_version, "sqlite-cli");
        let history = thread.history.expect("history should load");
        assert_eq!(history.chat_id, chat_id);
        assert_eq!(history.items.len(), 1);
    }

    #[tokio::test]
    async fn read_chat_falls_back_to_rollout_search_when_sqlite_path_is_stale() {
        let home = TempDir::new().expect("temp dir");
        let external = TempDir::new().expect("external temp dir");
        let config = test_config(home.path());
        let uuid = Uuid::from_u128(220);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        let stale_path = external.path().join("missing-rollout.jsonl");
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = LocalChatStore::new(config.clone(), Some(runtime.clone()));
        let mut builder = ThreadMetadataBuilder::new(
            chat_id,
            stale_path.clone(),
            Utc::now(),
            SessionSource::Cli,
        );
        builder.model_provider = Some("stale-sqlite-provider".to_string());
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.first_user_message = Some("stale sqlite preview".to_string());
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: true,
                include_history: true,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.chat_id, chat_id);
        assert_eq!(thread.rollout_path, Some(rollout_path));
        assert_eq!(thread.preview, "Hello from user");
        assert_eq!(thread.model_provider, config.default_model_provider_id);
        let history = thread.history.expect("history should load");
        assert_eq!(history.chat_id, chat_id);
        assert_eq!(history.items.len(), 2);
    }

    #[tokio::test]
    async fn read_chat_falls_back_when_sqlite_path_points_to_another_thread() {
        let home = TempDir::new().expect("temp dir");
        let external = TempDir::new().expect("external temp dir");
        let config = test_config(home.path());
        let uuid = Uuid::from_u128(221);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path =
            write_session_file(home.path(), "2025-01-03T12-00-00", uuid).expect("session file");
        let other_uuid = Uuid::from_u128(222);
        let stale_path = write_session_file(external.path(), "2025-01-04T12-00-00", other_uuid)
            .expect("other session file");
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = LocalChatStore::new(config.clone(), Some(runtime.clone()));
        let mut builder =
            ThreadMetadataBuilder::new(chat_id, stale_path, Utc::now(), SessionSource::Cli);
        builder.model_provider = Some("wrong-sqlite-provider".to_string());
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.first_user_message = Some("wrong sqlite preview".to_string());
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: true,
                include_history: true,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.chat_id, chat_id);
        assert_eq!(thread.rollout_path, Some(rollout_path));
        assert_eq!(thread.preview, "Hello from user");
        assert_eq!(thread.model_provider, config.default_model_provider_id);
        let history = thread.history.expect("history should load");
        assert_eq!(history.chat_id, chat_id);
        assert_eq!(history.items.len(), 2);
    }

    #[tokio::test]
    async fn read_chat_uses_session_meta_for_rollout_without_user_preview_or_sqlite_metadata() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let uuid = Uuid::from_u128(218);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let day_dir = home.path().join("sessions/2025/01/03");
        std::fs::create_dir_all(&day_dir).expect("sessions dir");
        let rollout_path = day_dir.join(format!("rollout-2025-01-03T12-00-00-{uuid}.jsonl"));
        let mut file = std::fs::File::create(&rollout_path).expect("session file");
        let meta = serde_json::json!({
            "timestamp": "2025-01-03T12:00:00Z",
            "type": "session_meta",
            "payload": {
                "session_id": uuid,
                "id": uuid,
                "timestamp": "2025-01-03T12:00:00Z",
                "cwd": home.path(),
                "originator": "test_originator",
                "cli_version": "test_version",
                "source": "cli",
                "model_provider": "rollout-provider"
            },
        });
        writeln!(file, "{meta}").expect("write session meta");

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: false,
                include_history: true,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.chat_id, chat_id);
        assert_eq!(thread.rollout_path, Some(rollout_path));
        assert_eq!(thread.preview, "");
        assert_eq!(thread.name, None);
        assert_eq!(thread.model_provider, "rollout-provider");
        assert_eq!(
            thread.created_at,
            parse_rfc3339_non_optional("2025-01-03T12:00:00Z").unwrap()
        );
        assert!(thread.updated_at >= thread.created_at);
        assert_eq!(thread.archived_at, None);
        assert_eq!(thread.cwd, home.path());
        assert_eq!(thread.cli_version, "test_version");
        assert_eq!(thread.source, SessionSource::Cli);
        let history = thread.history.expect("history should load");
        assert_eq!(history.chat_id, chat_id);
        assert_eq!(history.items.len(), 1);
    }

    #[tokio::test]
    async fn read_chat_falls_back_to_sqlite_summary() {
        let home = TempDir::new().expect("temp dir");
        let external = TempDir::new().expect("external temp dir");
        let config = test_config(home.path());
        let uuid = Uuid::from_u128(214);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path = external
            .path()
            .join(format!("rollout-2025-01-03T12-00-00-{uuid}.jsonl"));
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = LocalChatStore::new(config.clone(), Some(runtime.clone()));
        let mut builder = ThreadMetadataBuilder::new(
            chat_id,
            rollout_path.clone(),
            Utc::now(),
            SessionSource::Exec,
        );
        builder.model_provider = Some("sqlite-provider".to_string());
        builder.cwd = external.path().join("workspace");
        builder.cli_version = Some("sqlite-cli".to_string());
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.preview = Some("optimize the benchmark".to_string());
        metadata.first_user_message = Some("next normal prompt".to_string());
        metadata.title = "next normal prompt".to_string();
        metadata.model = Some("sqlite-model".to_string());
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect("read thread");

        assert_eq!(thread.chat_id, chat_id);
        assert_eq!(thread.rollout_path, Some(rollout_path));
        assert_eq!(thread.preview, "optimize the benchmark");
        assert_eq!(
            thread.first_user_message.as_deref(),
            Some("next normal prompt")
        );
        assert_eq!(thread.name, None);
        assert_eq!(thread.model_provider, "sqlite-provider");
        assert_eq!(thread.model.as_deref(), Some("sqlite-model"));
        assert_eq!(thread.cwd, external.path().join("workspace"));
        assert_eq!(thread.cli_version, "sqlite-cli");
        assert_eq!(thread.source, SessionSource::Exec);
        assert_eq!(thread.archived_at, None);
        assert!(thread.history.is_none());
    }

    #[tokio::test]
    async fn read_chat_sqlite_fallback_respects_include_archived() {
        let home = TempDir::new().expect("temp dir");
        let external = TempDir::new().expect("external temp dir");
        let config = test_config(home.path());
        let uuid = Uuid::from_u128(216);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let rollout_path = external
            .path()
            .join(format!("rollout-2025-01-03T12-00-00-{uuid}.jsonl"));
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let mut builder =
            ThreadMetadataBuilder::new(chat_id, rollout_path, Utc::now(), SessionSource::Cli);
        let store = LocalChatStore::new(config.clone(), Some(runtime.clone()));
        builder.archived_at = Some(Utc::now());
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.first_user_message = Some("Archived SQLite preview".to_string());
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let active_only_err = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect_err("active-only read should fail for archived metadata");
        let ChatStoreError::InvalidRequest { message } = active_only_err else {
            panic!("expected invalid request error");
        };
        assert_eq!(
            message,
            format!("no rollout found for thread id {chat_id}")
        );

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: true,
                include_history: false,
            })
            .await
            .expect("read archived thread");

        assert_eq!(thread.chat_id, chat_id);
        assert_eq!(thread.preview, "Archived SQLite preview");
        assert!(thread.archived_at.is_some());
    }

    #[tokio::test]
    async fn read_chat_sqlite_fallback_loads_archived_history() {
        let home = TempDir::new().expect("temp dir");
        let config = test_config(home.path());
        let uuid = Uuid::from_u128(219);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");
        let archived_path = write_archived_session_file(home.path(), "2025-01-03T12-00-00", uuid)
            .expect("archived session file");
        let runtime = datax_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.default_model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let store = LocalChatStore::new(config.clone(), Some(runtime.clone()));
        let mut builder = ThreadMetadataBuilder::new(
            chat_id,
            archived_path.clone(),
            Utc::now(),
            SessionSource::Cli,
        );
        builder.archived_at = Some(Utc::now());
        let mut metadata = builder.build(config.default_model_provider_id.as_str());
        metadata.first_user_message = Some("Archived SQLite preview".to_string());
        runtime
            .upsert_thread(&metadata)
            .await
            .expect("state db upsert should succeed");

        let thread = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: true,
                include_history: true,
            })
            .await
            .expect("read archived thread");

        assert_eq!(thread.chat_id, chat_id);
        assert_eq!(thread.rollout_path, Some(archived_path));
        assert_eq!(thread.preview, "Archived SQLite preview");
        assert!(thread.archived_at.is_some());
        let history = thread.history.expect("history should load");
        assert_eq!(history.chat_id, chat_id);
        assert_eq!(history.items.len(), 2);
    }

    #[tokio::test]
    async fn read_chat_fails_without_rollout() {
        let home = TempDir::new().expect("temp dir");
        let store = LocalChatStore::new(test_config(home.path()), /*state_db*/ None);
        let uuid = Uuid::from_u128(206);
        let chat_id = ChatId::from_string(&uuid.to_string()).expect("valid thread id");

        let err = store
            .read_chat(ReadChatParams {
                chat_id,
                include_archived: false,
                include_history: false,
            })
            .await
            .expect_err("read should fail without rollout");

        let ChatStoreError::InvalidRequest { message } = err else {
            panic!("expected invalid request error");
        };
        assert_eq!(
            message,
            format!("no rollout found for thread id {chat_id}")
        );
    }
}
