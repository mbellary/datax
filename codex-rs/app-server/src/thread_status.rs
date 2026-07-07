#[cfg(test)]
use crate::outgoing_message::OutgoingEnvelope;
#[cfg(test)]
use crate::outgoing_message::OutgoingMessage;
use crate::outgoing_message::OutgoingMessageSender;
use datax_app_server_protocol::Chat;
use datax_app_server_protocol::ChatActiveFlag;
use datax_app_server_protocol::ChatStatus;
use datax_app_server_protocol::ChatStatusChangedNotification;
use datax_app_server_protocol::ServerNotification;
use datax_protocol::ThreadId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
#[cfg(test)]
use tokio::sync::mpsc;
use tokio::sync::watch;

#[derive(Clone)]
pub(crate) struct ThreadWatchManager {
    state: Arc<Mutex<ThreadWatchState>>,
    outgoing: Option<Arc<OutgoingMessageSender>>,
    running_turn_count_tx: watch::Sender<usize>,
}

pub(crate) struct ThreadWatchActiveGuard {
    manager: ThreadWatchManager,
    chat_id: String,
    guard_type: ThreadWatchActiveGuardType,
    handle: tokio::runtime::Handle,
}

impl ThreadWatchActiveGuard {
    fn new(
        manager: ThreadWatchManager,
        chat_id: String,
        guard_type: ThreadWatchActiveGuardType,
    ) -> Self {
        Self {
            manager,
            chat_id,
            guard_type,
            handle: tokio::runtime::Handle::current(),
        }
    }
}

impl Drop for ThreadWatchActiveGuard {
    fn drop(&mut self) {
        let manager = self.manager.clone();
        let chat_id = self.chat_id.clone();
        let guard_type = self.guard_type;
        self.handle.spawn(async move {
            manager
                .note_active_guard_released(chat_id, guard_type)
                .await;
        });
    }
}

#[derive(Clone, Copy)]
enum ThreadWatchActiveGuardType {
    Permission,
    UserInput,
}

impl Default for ThreadWatchManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadWatchManager {
    pub(crate) fn new() -> Self {
        let (running_turn_count_tx, _running_turn_count_rx) = watch::channel(0);
        Self {
            state: Arc::new(Mutex::new(ThreadWatchState::default())),
            outgoing: None,
            running_turn_count_tx,
        }
    }

    pub(crate) fn new_with_outgoing(outgoing: Arc<OutgoingMessageSender>) -> Self {
        let (running_turn_count_tx, _running_turn_count_rx) = watch::channel(0);
        Self {
            state: Arc::new(Mutex::new(ThreadWatchState::default())),
            outgoing: Some(outgoing),
            running_turn_count_tx,
        }
    }

    pub(crate) async fn upsert_thread(&self, thread: Chat) {
        self.mutate_and_publish(move |state| {
            state.upsert_thread(thread.id, /*emit_notification*/ true)
        })
        .await;
    }

    pub(crate) async fn upsert_thread_silently(&self, thread: Chat) {
        self.mutate_and_publish(move |state| {
            state.upsert_thread(thread.id, /*emit_notification*/ false)
        })
        .await;
    }

    pub(crate) async fn remove_thread(&self, chat_id: &str) {
        let chat_id = chat_id.to_string();
        self.mutate_and_publish(move |state| state.remove_thread(&chat_id))
            .await;
    }

    pub(crate) async fn loaded_status_for_thread(&self, chat_id: &str) -> ChatStatus {
        self.state.lock().await.loaded_status_for_thread(chat_id)
    }

    pub(crate) async fn loaded_statuses_for_threads(
        &self,
        chat_ids: Vec<String>,
    ) -> HashMap<String, ChatStatus> {
        let state = self.state.lock().await;
        chat_ids
            .into_iter()
            .map(|chat_id| {
                let status = state.loaded_status_for_thread(&chat_id);
                (chat_id, status)
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) async fn running_turn_count(&self) -> usize {
        self.state
            .lock()
            .await
            .runtime_by_thread_id
            .values()
            .filter(|runtime| runtime.running)
            .count()
    }

    pub(crate) fn subscribe_running_turn_count(&self) -> watch::Receiver<usize> {
        self.running_turn_count_tx.subscribe()
    }

    pub(crate) async fn note_turn_started(&self, chat_id: &str) {
        self.update_runtime_for_thread(chat_id, |runtime| {
            runtime.is_loaded = true;
            runtime.running = true;
            runtime.has_system_error = false;
        })
        .await;
    }

    pub(crate) async fn note_turn_completed(&self, chat_id: &str, _failed: bool) {
        self.clear_active_state(chat_id).await;
    }

    pub(crate) async fn note_turn_interrupted(&self, chat_id: &str) {
        self.clear_active_state(chat_id).await;
    }

    pub(crate) async fn note_thread_shutdown(&self, chat_id: &str) {
        self.update_runtime_for_thread(chat_id, |runtime| {
            runtime.running = false;
            runtime.pending_permission_requests = 0;
            runtime.pending_user_input_requests = 0;
            runtime.is_loaded = false;
        })
        .await;
    }

    pub(crate) async fn note_system_error(&self, chat_id: &str) {
        self.update_runtime_for_thread(chat_id, |runtime| {
            runtime.running = false;
            runtime.pending_permission_requests = 0;
            runtime.pending_user_input_requests = 0;
            runtime.has_system_error = true;
        })
        .await;
    }

    async fn clear_active_state(&self, chat_id: &str) {
        self.update_runtime_for_thread(chat_id, move |runtime| {
            runtime.running = false;
            runtime.pending_permission_requests = 0;
            runtime.pending_user_input_requests = 0;
        })
        .await;
    }

    pub(crate) async fn note_permission_requested(&self, chat_id: &str) -> ThreadWatchActiveGuard {
        self.note_pending_request(chat_id, ThreadWatchActiveGuardType::Permission)
            .await
    }

    pub(crate) async fn note_user_input_requested(&self, chat_id: &str) -> ThreadWatchActiveGuard {
        self.note_pending_request(chat_id, ThreadWatchActiveGuardType::UserInput)
            .await
    }

    async fn note_pending_request(
        &self,
        chat_id: &str,
        guard_type: ThreadWatchActiveGuardType,
    ) -> ThreadWatchActiveGuard {
        self.update_runtime_for_thread(chat_id, move |runtime| {
            runtime.is_loaded = true;
            let counter = Self::pending_counter(runtime, guard_type);
            *counter = counter.saturating_add(1);
        })
        .await;
        ThreadWatchActiveGuard::new(self.clone(), chat_id.to_string(), guard_type)
    }

    async fn mutate_and_publish<F>(&self, mutate: F)
    where
        F: FnOnce(&mut ThreadWatchState) -> Option<ChatStatusChangedNotification>,
    {
        let (notification, running_turn_count) = {
            let mut state = self.state.lock().await;
            let notification = mutate(&mut state);
            let running_turn_count = state
                .runtime_by_thread_id
                .values()
                .filter(|runtime| runtime.running)
                .count();
            (notification, running_turn_count)
        };
        let _ = self.running_turn_count_tx.send(running_turn_count);

        if let Some(notification) = notification
            && let Some(outgoing) = &self.outgoing
        {
            outgoing
                .send_server_notification(ChatStatusChanged(notification))
                .await;
        }
    }

    pub(crate) async fn subscribe(&self, chat_id: ThreadId) -> Option<watch::Receiver<ChatStatus>> {
        Some(self.state.lock().await.subscribe(chat_id.to_string()))
    }

    async fn note_active_guard_released(
        &self,
        chat_id: String,
        guard_type: ThreadWatchActiveGuardType,
    ) {
        self.update_runtime_for_thread(&chat_id, move |runtime| {
            let counter = Self::pending_counter(runtime, guard_type);
            *counter = counter.saturating_sub(1);
        })
        .await;
    }

    async fn update_runtime_for_thread<F>(&self, chat_id: &str, update: F)
    where
        F: FnOnce(&mut RuntimeFacts),
    {
        let chat_id = chat_id.to_string();
        self.mutate_and_publish(move |state| state.update_runtime(&chat_id, update))
            .await;
    }

    fn pending_counter(
        runtime: &mut RuntimeFacts,
        guard_type: ThreadWatchActiveGuardType,
    ) -> &mut u32 {
        match guard_type {
            ThreadWatchActiveGuardType::Permission => &mut runtime.pending_permission_requests,
            ThreadWatchActiveGuardType::UserInput => &mut runtime.pending_user_input_requests,
        }
    }
}

pub(crate) fn resolve_thread_status(status: ChatStatus, has_in_progress_turn: bool) -> ChatStatus {
    // Running-turn events can arrive before the watch runtime state is observed by
    // the listener loop. In that window we prefer to reflect a real active turn as
    // `Active` instead of `Idle`/`NotLoaded`.
    if has_in_progress_turn && matches!(status, ChatStatus::Idle | ChatStatus::NotLoaded) {
        return ChatStatus::Active {
            active_flags: Vec::new(),
        };
    }

    status
}

#[derive(Default)]
struct ThreadWatchState {
    runtime_by_thread_id: HashMap<String, RuntimeFacts>,
    status_watcher_by_thread_id: HashMap<String, watch::Sender<ChatStatus>>,
}

impl ThreadWatchState {
    fn upsert_thread(
        &mut self,
        chat_id: String,
        emit_notification: bool,
    ) -> Option<ChatStatusChangedNotification> {
        let previous_status = self.status_for(&chat_id);
        let runtime = self
            .runtime_by_thread_id
            .entry(chat_id.clone())
            .or_default();
        runtime.is_loaded = true;
        self.update_status_watcher_for_thread(&chat_id);
        if emit_notification {
            self.status_changed_notification(chat_id, previous_status)
        } else {
            None
        }
    }

    fn remove_thread(&mut self, chat_id: &str) -> Option<ChatStatusChangedNotification> {
        let previous_status = self.status_for(chat_id);
        self.runtime_by_thread_id.remove(chat_id);
        self.update_status_watcher(chat_id, &ChatStatus::NotLoaded);
        if previous_status.is_some() && previous_status != Some(ChatStatus::NotLoaded) {
            Some(ChatStatusChangedNotification {
                chat_id: chat_id.to_string(),
                status: ChatStatus::NotLoaded,
            })
        } else {
            None
        }
    }

    fn update_runtime<F>(
        &mut self,
        chat_id: &str,
        mutate: F,
    ) -> Option<ChatStatusChangedNotification>
    where
        F: FnOnce(&mut RuntimeFacts),
    {
        let previous_status = self.status_for(chat_id);
        let runtime = self
            .runtime_by_thread_id
            .entry(chat_id.to_string())
            .or_default();
        runtime.is_loaded = true;
        mutate(runtime);
        self.update_status_watcher_for_thread(chat_id);
        self.status_changed_notification(chat_id.to_string(), previous_status)
    }

    fn status_for(&self, chat_id: &str) -> Option<ChatStatus> {
        self.runtime_by_thread_id
            .get(chat_id)
            .map(loaded_thread_status)
    }

    fn loaded_status_for_thread(&self, chat_id: &str) -> ChatStatus {
        self.status_for(chat_id).unwrap_or(ChatStatus::NotLoaded)
    }

    fn subscribe(&mut self, chat_id: String) -> watch::Receiver<ChatStatus> {
        let status = self.loaded_status_for_thread(&chat_id);
        let sender = self
            .status_watcher_by_thread_id
            .entry(chat_id)
            .or_insert_with(|| watch::channel(status.clone()).0);
        sender.subscribe()
    }

    fn update_status_watcher_for_thread(&mut self, chat_id: &str) {
        let status = self.loaded_status_for_thread(chat_id);
        self.update_status_watcher(chat_id, &status);
    }

    fn update_status_watcher(&mut self, chat_id: &str, status: &ChatStatus) {
        let remove_watcher = if let Some(sender) = self.status_watcher_by_thread_id.get(chat_id) {
            let status = status.clone();
            let _ = sender.send_if_modified(|current| {
                if *current == status {
                    false
                } else {
                    *current = status;
                    true
                }
            });
            sender.receiver_count() == 0
        } else {
            false
        };
        if remove_watcher {
            self.status_watcher_by_thread_id.remove(chat_id);
        }
    }

    fn status_changed_notification(
        &self,
        chat_id: String,
        previous_status: Option<ChatStatus>,
    ) -> Option<ChatStatusChangedNotification> {
        let status = self.status_for(&chat_id)?;

        if previous_status.as_ref() == Some(&status) {
            return None;
        }

        Some(ChatStatusChangedNotification { chat_id, status })
    }
}

#[derive(Clone, Default)]
struct RuntimeFacts {
    is_loaded: bool,
    running: bool,
    pending_permission_requests: u32,
    pending_user_input_requests: u32,
    has_system_error: bool,
}

fn loaded_thread_status(runtime: &RuntimeFacts) -> ChatStatus {
    if !runtime.is_loaded {
        return ChatStatus::NotLoaded;
    }

    let mut active_flags = Vec::new();
    if runtime.pending_permission_requests > 0 {
        active_flags.push(ChatActiveFlag::WaitingOnApproval);
    }
    if runtime.pending_user_input_requests > 0 {
        active_flags.push(ChatActiveFlag::WaitingOnUserInput);
    }

    if runtime.running || !active_flags.is_empty() {
        return ChatStatus::Active { active_flags };
    }

    if runtime.has_system_error {
        return ChatStatus::SystemError;
    }

    ChatStatus::Idle
}

#[cfg(test)]
mod tests {
    use super::*;
    use datax_utils_absolute_path::test_support::PathBufExt;
    use datax_utils_absolute_path::test_support::test_path_buf;
    use pretty_assertions::assert_eq;
    use tokio::time::Duration;
    use tokio::time::timeout;

    const INTERACTIVE_THREAD_ID: &str = "00000000-0000-0000-0000-000000000001";
    const NON_INTERACTIVE_THREAD_ID: &str = "00000000-0000-0000-0000-000000000002";

    #[tokio::test]
    async fn loaded_status_defaults_to_not_loaded_for_untracked_threads() {
        let manager = ThreadWatchManager::new();

        assert_eq!(
            manager
                .loaded_status_for_thread("00000000-0000-0000-0000-000000000003")
                .await,
            ChatStatus::NotLoaded,
        );
    }

    #[tokio::test]
    async fn tracks_non_interactive_thread_status() {
        let manager = ThreadWatchManager::new();
        manager
            .upsert_thread(test_thread(
                NON_INTERACTIVE_THREAD_ID,
                datax_app_server_protocol::SessionSource::AppServer,
            ))
            .await;

        manager.note_turn_started(NON_INTERACTIVE_THREAD_ID).await;

        assert_eq!(
            manager
                .loaded_status_for_thread(NON_INTERACTIVE_THREAD_ID)
                .await,
            ChatStatus::Active {
                active_flags: vec![],
            },
        );
    }

    #[tokio::test]
    async fn status_updates_track_single_thread() {
        let manager = ThreadWatchManager::new();
        manager
            .upsert_thread(test_thread(
                INTERACTIVE_THREAD_ID,
                datax_app_server_protocol::SessionSource::Cli,
            ))
            .await;

        manager.note_turn_started(INTERACTIVE_THREAD_ID).await;
        assert_eq!(
            manager
                .loaded_status_for_thread(INTERACTIVE_THREAD_ID)
                .await,
            ChatStatus::Active {
                active_flags: vec![],
            },
        );

        let permission_guard = manager
            .note_permission_requested(INTERACTIVE_THREAD_ID)
            .await;
        assert_eq!(
            manager
                .loaded_status_for_thread(INTERACTIVE_THREAD_ID)
                .await,
            ChatStatus::Active {
                active_flags: vec![ChatActiveFlag::WaitingOnApproval],
            },
        );

        let user_input_guard = manager
            .note_user_input_requested(INTERACTIVE_THREAD_ID)
            .await;
        assert_eq!(
            manager
                .loaded_status_for_thread(INTERACTIVE_THREAD_ID)
                .await,
            ChatStatus::Active {
                active_flags: vec![
                    ChatActiveFlag::WaitingOnApproval,
                    ChatActiveFlag::WaitingOnUserInput,
                ],
            },
        );

        drop(permission_guard);
        wait_for_status(
            &manager,
            INTERACTIVE_THREAD_ID,
            ChatStatus::Active {
                active_flags: vec![ChatActiveFlag::WaitingOnUserInput],
            },
        )
        .await;

        drop(user_input_guard);
        wait_for_status(
            &manager,
            INTERACTIVE_THREAD_ID,
            ChatStatus::Active {
                active_flags: vec![],
            },
        )
        .await;

        manager
            .note_turn_completed(INTERACTIVE_THREAD_ID, false)
            .await;
        assert_eq!(
            manager
                .loaded_status_for_thread(INTERACTIVE_THREAD_ID)
                .await,
            ChatStatus::Idle,
        );
    }

    #[test]
    fn resolves_in_progress_turn_to_active_status() {
        let status = resolve_thread_status(ChatStatus::Idle, /*has_in_progress_turn*/ true);
        assert_eq!(
            status,
            ChatStatus::Active {
                active_flags: Vec::new(),
            }
        );

        let status =
            resolve_thread_status(ChatStatus::NotLoaded, /*has_in_progress_turn*/ true);
        assert_eq!(
            status,
            ChatStatus::Active {
                active_flags: Vec::new(),
            }
        );
    }

    #[test]
    fn keeps_status_when_no_in_progress_turn() {
        assert_eq!(
            resolve_thread_status(ChatStatus::Idle, /*has_in_progress_turn*/ false),
            ChatStatus::Idle
        );
        assert_eq!(
            resolve_thread_status(ChatStatus::SystemError, /*has_in_progress_turn*/ false),
            ChatStatus::SystemError
        );
    }

    #[tokio::test]
    async fn system_error_sets_idle_flag_until_next_turn() {
        let manager = ThreadWatchManager::new();
        manager
            .upsert_thread(test_thread(
                INTERACTIVE_THREAD_ID,
                datax_app_server_protocol::SessionSource::Cli,
            ))
            .await;

        manager.note_turn_started(INTERACTIVE_THREAD_ID).await;
        manager.note_system_error(INTERACTIVE_THREAD_ID).await;

        assert_eq!(
            manager
                .loaded_status_for_thread(INTERACTIVE_THREAD_ID)
                .await,
            ChatStatus::SystemError,
        );

        manager.note_turn_started(INTERACTIVE_THREAD_ID).await;
        assert_eq!(
            manager
                .loaded_status_for_thread(INTERACTIVE_THREAD_ID)
                .await,
            ChatStatus::Active {
                active_flags: vec![],
            },
        );
    }

    #[tokio::test]
    async fn shutdown_marks_thread_not_loaded() {
        let manager = ThreadWatchManager::new();
        manager
            .upsert_thread(test_thread(
                INTERACTIVE_THREAD_ID,
                datax_app_server_protocol::SessionSource::Cli,
            ))
            .await;

        manager.note_turn_started(INTERACTIVE_THREAD_ID).await;
        manager.note_thread_shutdown(INTERACTIVE_THREAD_ID).await;

        assert_eq!(
            manager
                .loaded_status_for_thread(INTERACTIVE_THREAD_ID)
                .await,
            ChatStatus::NotLoaded,
        );
    }

    #[tokio::test]
    async fn loaded_statuses_default_to_not_loaded_for_untracked_threads() {
        let manager = ThreadWatchManager::new();
        manager
            .upsert_thread(test_thread(
                INTERACTIVE_THREAD_ID,
                datax_app_server_protocol::SessionSource::Cli,
            ))
            .await;
        manager.note_turn_started(INTERACTIVE_THREAD_ID).await;

        let statuses = manager
            .loaded_statuses_for_threads(vec![
                INTERACTIVE_THREAD_ID.to_string(),
                NON_INTERACTIVE_THREAD_ID.to_string(),
            ])
            .await;

        assert_eq!(
            statuses.get(INTERACTIVE_THREAD_ID),
            Some(&ChatStatus::Active {
                active_flags: vec![],
            }),
        );
        assert_eq!(
            statuses.get(NON_INTERACTIVE_THREAD_ID),
            Some(&ChatStatus::NotLoaded),
        );
    }

    #[tokio::test]
    async fn has_running_turns_tracks_runtime_running_flag_only() {
        let manager = ThreadWatchManager::new();
        manager
            .upsert_thread(test_thread(
                INTERACTIVE_THREAD_ID,
                datax_app_server_protocol::SessionSource::Cli,
            ))
            .await;

        assert_eq!(manager.running_turn_count().await, 0);

        let _permission_guard = manager
            .note_permission_requested(INTERACTIVE_THREAD_ID)
            .await;
        assert_eq!(manager.running_turn_count().await, 0);

        manager.note_turn_started(INTERACTIVE_THREAD_ID).await;
        assert_eq!(manager.running_turn_count().await, 1);

        manager
            .note_turn_completed(INTERACTIVE_THREAD_ID, false)
            .await;
        assert_eq!(manager.running_turn_count().await, 0);
    }

    #[tokio::test]
    async fn status_change_emits_notification() {
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel(8);
        let manager = ThreadWatchManager::new_with_outgoing(Arc::new(OutgoingMessageSender::new(
            outgoing_tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        )));

        manager
            .upsert_thread(test_thread(
                INTERACTIVE_THREAD_ID,
                datax_app_server_protocol::SessionSource::Cli,
            ))
            .await;
        assert_eq!(
            recv_status_changed_notification(&mut outgoing_rx).await,
            ChatStatusChangedNotification {
                chat_id: INTERACTIVE_THREAD_ID.to_string(),
                status: ChatStatus::Idle,
            },
        );

        manager.note_turn_started(INTERACTIVE_THREAD_ID).await;
        assert_eq!(
            recv_status_changed_notification(&mut outgoing_rx).await,
            ChatStatusChangedNotification {
                chat_id: INTERACTIVE_THREAD_ID.to_string(),
                status: ChatStatus::Active {
                    active_flags: vec![],
                },
            },
        );

        manager.remove_thread(INTERACTIVE_THREAD_ID).await;
        assert_eq!(
            recv_status_changed_notification(&mut outgoing_rx).await,
            ChatStatusChangedNotification {
                chat_id: INTERACTIVE_THREAD_ID.to_string(),
                status: ChatStatus::NotLoaded,
            },
        );
    }

    #[tokio::test]
    async fn silent_upsert_skips_initial_notification() {
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel(8);
        let manager = ThreadWatchManager::new_with_outgoing(Arc::new(OutgoingMessageSender::new(
            outgoing_tx,
            datax_analytics::AnalyticsEventsClient::disabled(),
        )));

        manager
            .upsert_thread_silently(test_thread(
                INTERACTIVE_THREAD_ID,
                datax_app_server_protocol::SessionSource::Cli,
            ))
            .await;

        assert_eq!(
            manager
                .loaded_status_for_thread(INTERACTIVE_THREAD_ID)
                .await,
            ChatStatus::Idle,
        );
        assert!(
            timeout(Duration::from_millis(100), outgoing_rx.recv())
                .await
                .is_err(),
            "silent upsert should not emit chat/status/changed"
        );

        manager.note_turn_started(INTERACTIVE_THREAD_ID).await;
        assert_eq!(
            recv_status_changed_notification(&mut outgoing_rx).await,
            ChatStatusChangedNotification {
                chat_id: INTERACTIVE_THREAD_ID.to_string(),
                status: ChatStatus::Active {
                    active_flags: vec![],
                },
            },
        );
    }

    #[tokio::test]
    async fn status_watchers_receive_only_their_thread_updates() {
        let manager = ThreadWatchManager::new();
        manager
            .upsert_thread(test_thread(
                INTERACTIVE_THREAD_ID,
                datax_app_server_protocol::SessionSource::Cli,
            ))
            .await;
        manager
            .upsert_thread(test_thread(
                NON_INTERACTIVE_THREAD_ID,
                datax_app_server_protocol::SessionSource::AppServer,
            ))
            .await;
        let interactive_thread_id = ThreadId::from_string(INTERACTIVE_THREAD_ID)
            .expect("interactive thread id should parse");
        let non_interactive_thread_id = ThreadId::from_string(NON_INTERACTIVE_THREAD_ID)
            .expect("non-interactive thread id should parse");
        let mut interactive_rx = manager
            .subscribe(interactive_thread_id)
            .await
            .expect("interactive status watcher should subscribe");
        let mut non_interactive_rx = manager
            .subscribe(non_interactive_thread_id)
            .await
            .expect("non-interactive status watcher should subscribe");

        manager.note_turn_started(INTERACTIVE_THREAD_ID).await;

        timeout(Duration::from_secs(1), interactive_rx.changed())
            .await
            .expect("timed out waiting for interactive status update")
            .expect("interactive status watcher should remain open");
        assert_eq!(
            *interactive_rx.borrow(),
            ChatStatus::Active {
                active_flags: vec![],
            },
        );
        assert!(
            timeout(Duration::from_millis(100), non_interactive_rx.changed())
                .await
                .is_err(),
            "unrelated thread watcher should not receive an update"
        );
        assert_eq!(*non_interactive_rx.borrow(), ChatStatus::Idle);
    }

    async fn wait_for_status(
        manager: &ThreadWatchManager,
        chat_id: &str,
        expected_status: ChatStatus,
    ) {
        timeout(Duration::from_secs(1), async {
            loop {
                let status = manager.loaded_status_for_thread(chat_id).await;
                if status == expected_status {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("timed out waiting for status");
    }

    async fn recv_status_changed_notification(
        outgoing_rx: &mut mpsc::Receiver<OutgoingEnvelope>,
    ) -> ChatStatusChangedNotification {
        let envelope = timeout(Duration::from_secs(1), outgoing_rx.recv())
            .await
            .expect("timed out waiting for outgoing notification")
            .expect("outgoing channel closed unexpectedly");
        let OutgoingEnvelope::Broadcast { message } = envelope else {
            panic!("expected broadcast notification");
        };
        let OutgoingMessage::AppServerNotification(ChatStatusChanged(notification)) = message
        else {
            panic!("expected chat/status/changed notification");
        };
        notification
    }

    fn test_thread(chat_id: &str, source: datax_app_server_protocol::SessionSource) -> Chat {
        Chat {
            id: chat_id.to_string(),
            session_id: chat_id.to_string(),
            forked_from_id: None,
            parent_chat_id: None,
            preview: String::new(),
            ephemeral: false,
            model_provider: "mock-provider".to_string(),
            created_at: 0,
            updated_at: 0,
            recency_at: Some(0),
            status: ChatStatus::NotLoaded,
            path: None,
            cwd: test_path_buf("/tmp").abs(),
            cli_version: "test".to_string(),
            agent_nickname: None,
            agent_role: None,
            source,
            chat_source: None,
            git_info: None,
            name: None,
            interactions: Vec::new(),
        }
    }
}
