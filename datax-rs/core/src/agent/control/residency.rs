use super::AgentControl;
use crate::agent::AgentStatus;
use crate::chat_manager::ChatManagerState;
use crate::config::Config;
use crate::datax_chat::DataxChat;
use datax_protocol::ChatId;
use datax_protocol::error::CodexErr;
use datax_protocol::error::Result as CodexResult;
use datax_protocol::protocol::MultiAgentVersion;
use datax_protocol::protocol::SessionSource;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::warn;

#[derive(Default)]
pub(super) struct V2Residency {
    state: Mutex<V2ResidencyState>,
}

#[derive(Default)]
struct V2ResidencyState {
    residents: VecDeque<ChatId>,
    pending_slots: usize,
}

pub(super) struct V2ResidencySlot {
    residency: Arc<V2Residency>,
    active: bool,
}

impl V2ResidencySlot {
    pub(super) fn commit(mut self, chat_id: ChatId) {
        self.residency.commit_slot(chat_id);
        self.active = false;
    }
}

impl Drop for V2ResidencySlot {
    fn drop(&mut self) {
        if self.active {
            self.residency.release_pending_slot();
        }
    }
}

impl AgentControl {
    pub(super) async fn reserve_v2_residency_slot(
        &self,
        state: &Arc<ChatManagerState>,
        config: &Config,
        protected_chat_id: Option<ChatId>,
    ) -> CodexResult<V2ResidencySlot> {
        let capacity = config
            .effective_agent_max_threads(MultiAgentVersion::V2)
            .unwrap_or(usize::MAX);
        Arc::clone(&self.v2_residency)
            .reserve_slot(state, capacity, protected_chat_id)
            .await
    }

    pub(super) async fn touch_loaded_v2_residency(
        &self,
        state: &Arc<ChatManagerState>,
        chat_id: ChatId,
    ) {
        if let Ok(thread) = state.get_chat(chat_id).await
            && is_resident_candidate(thread.as_ref())
        {
            self.v2_residency.touch(chat_id);
        }
    }

    pub(super) fn forget_v2_residency(&self, chat_id: ChatId) {
        self.v2_residency.remove(chat_id);
    }
}

impl V2Residency {
    async fn reserve_slot(
        self: Arc<Self>,
        manager: &Arc<ChatManagerState>,
        capacity: usize,
        protected_chat_id: Option<ChatId>,
    ) -> CodexResult<V2ResidencySlot> {
        loop {
            if self.try_reserve_pending_slot(capacity) {
                return Ok(V2ResidencySlot {
                    residency: self,
                    active: true,
                });
            }
            if !self
                .try_unload_one_resident(manager, protected_chat_id)
                .await
            {
                return Err(CodexErr::AgentLimitReached {
                    max_threads: capacity,
                });
            }
        }
    }

    fn try_reserve_pending_slot(&self, capacity: usize) -> bool {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.residents.len().saturating_add(state.pending_slots) >= capacity {
            return false;
        }
        state.pending_slots += 1;
        true
    }

    async fn try_unload_one_resident(
        &self,
        manager: &Arc<ChatManagerState>,
        protected_chat_id: Option<ChatId>,
    ) -> bool {
        let candidates_to_scan = self.resident_count();
        for _ in 0..candidates_to_scan {
            let Some(candidate_chat_id) = self.pop_lru_candidate(protected_chat_id) else {
                return false;
            };
            let Some(candidate_thread) = manager
                .get_chat(candidate_chat_id)
                .await
                .ok()
                .filter(|thread| is_resident_candidate(thread))
            else {
                continue;
            };
            if !is_unloadable(candidate_thread.as_ref()).await {
                self.touch(candidate_chat_id);
                continue;
            }
            candidate_thread.ensure_rollout_materialized().await;
            if let Err(err) = candidate_thread.shutdown_and_wait().await {
                warn!(
                    "failed to shut down v2 resident thread before unloading {candidate_chat_id}: {err}"
                );
                self.touch(candidate_chat_id);
                continue;
            }
            let _ = manager.remove_chat(&candidate_chat_id).await;
            return true;
        }
        false
    }

    fn resident_count(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .residents
            .len()
    }

    fn pop_lru_candidate(&self, protected_chat_id: Option<ChatId>) -> Option<ChatId> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let candidates_to_scan = state.residents.len();
        for _ in 0..candidates_to_scan {
            let candidate_chat_id = state.residents.pop_front()?;
            if Some(candidate_chat_id) == protected_chat_id {
                state.residents.push_back(candidate_chat_id);
                continue;
            }
            return Some(candidate_chat_id);
        }
        None
    }

    fn touch(&self, chat_id: ChatId) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        touch_resident(&mut state.residents, chat_id);
    }

    fn remove(&self, chat_id: ChatId) {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .residents
            .retain(|resident_chat_id| *resident_chat_id != chat_id);
    }

    fn commit_slot(&self, chat_id: ChatId) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.pending_slots = state.pending_slots.saturating_sub(1);
        touch_resident(&mut state.residents, chat_id);
    }

    fn release_pending_slot(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.pending_slots = state.pending_slots.saturating_sub(1);
    }
}

fn touch_resident(residents: &mut VecDeque<ChatId>, chat_id: ChatId) {
    residents.retain(|resident_chat_id| *resident_chat_id != chat_id);
    residents.push_back(chat_id);
}

fn is_resident_candidate(thread: &DataxChat) -> bool {
    thread.multi_agent_version() == Some(MultiAgentVersion::V2)
        && is_v2_resident_session_source(&thread.session_source)
}

pub(super) fn is_v2_resident_session_source(session_source: &SessionSource) -> bool {
    matches!(session_source, SessionSource::SubAgent(_))
}

async fn is_unloadable(thread: &DataxChat) -> bool {
    matches!(
        thread.agent_status().await,
        AgentStatus::Completed(_) | AgentStatus::Errored(_) | AgentStatus::Interrupted
    ) && thread.codex.session.active_turn.lock().await.is_none()
        && !thread
            .codex
            .session
            .input_queue
            .has_pending_mailbox_items()
            .await
}

#[cfg(test)]
#[path = "residency_tests.rs"]
mod tests;
