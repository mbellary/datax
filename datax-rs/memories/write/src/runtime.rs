use datax_core::ChatManager;
use datax_core::DataxChat;
use datax_core::ModelClient;
use datax_core::NewChat;
use datax_core::Prompt;
use datax_core::ResponseEvent;
use datax_core::StartChatOptions;
use datax_core::config::Config;
use datax_core::content_items_to_text;
use datax_core::detached_memory_responses_metadata;
use datax_core::resolve_installation_id;
use datax_features::Feature;
use datax_login::AuthManager;
use datax_login::CodexAuth;
use datax_login::auth_env_telemetry::collect_auth_env_telemetry;
use datax_login::default_client::originator;
use datax_model_provider::ModelProvider;
use datax_model_provider::SharedModelProvider;
use datax_model_provider::create_model_provider;
use datax_otel::SessionTelemetry;
use datax_otel::TelemetryAuthMode;
use datax_protocol::ChatId;
use datax_protocol::SessionId;
use datax_protocol::config_types::ReasoningSummary;
use datax_protocol::openai_models::ModelInfo;
use datax_protocol::openai_models::ReasoningEffort;
use datax_protocol::protocol::InitialHistory;
use datax_protocol::protocol::InternalSessionSource;
use datax_protocol::protocol::Op;
use datax_protocol::protocol::SessionSource;
use datax_protocol::protocol::ThreadSource;
use datax_protocol::protocol::TokenUsage;
use datax_protocol::user_input::UserInput;
use datax_rollout_trace::InferenceTraceContext;
use datax_state::StateRuntime;
use datax_terminal_detection::user_agent;
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;

pub(crate) struct SpawnedConsolidationAgent {
    pub(crate) chat_id: ChatId,
    pub(crate) thread: Arc<DataxChat>,
}

#[derive(Clone, Debug)]
pub(crate) struct StageOneRequestContext {
    pub(crate) model_info: ModelInfo,
    pub(crate) session_telemetry: SessionTelemetry,
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
    pub(crate) reasoning_summary: ReasoningSummary,
    pub(crate) service_tier: Option<String>,
}

impl StageOneRequestContext {
    pub(crate) fn start_timer(&self, name: &str) -> Option<datax_otel::Timer> {
        self.session_telemetry.start_timer(name, &[]).ok()
    }

    pub(crate) fn counter(&self, name: &str, inc: i64, tags: &[(&str, &str)]) {
        self.session_telemetry.counter(name, inc, tags);
    }

    pub(crate) fn histogram(&self, name: &str, value: i64, tags: &[(&str, &str)]) {
        self.session_telemetry.histogram(name, value, tags);
    }
}

pub(crate) struct MemoryStartupContext {
    chat_id: ChatId,
    thread: Arc<DataxChat>,
    chat_manager: Arc<ChatManager>,
    auth_manager: Arc<AuthManager>,
    provider: SharedModelProvider,
    session_telemetry: SessionTelemetry,
}

impl MemoryStartupContext {
    pub(crate) fn new(
        chat_manager: Arc<ChatManager>,
        auth_manager: Arc<AuthManager>,
        chat_id: ChatId,
        thread: Arc<DataxChat>,
        config: &Config,
        source: SessionSource,
    ) -> Self {
        let provider = create_model_provider(
            config.model_provider.clone(),
            Some(Arc::clone(&auth_manager)),
        );
        Self::new_with_provider(
            chat_manager,
            auth_manager,
            chat_id,
            thread,
            config,
            source,
            provider,
        )
    }

    #[cfg(test)]
    pub(crate) fn new_for_testing(
        chat_manager: Arc<ChatManager>,
        auth_manager: Arc<AuthManager>,
        chat_id: ChatId,
        thread: Arc<DataxChat>,
        config: &Config,
        source: SessionSource,
        provider: SharedModelProvider,
    ) -> Self {
        Self::new_with_provider(
            chat_manager,
            auth_manager,
            chat_id,
            thread,
            config,
            source,
            provider,
        )
    }

    fn new_with_provider(
        chat_manager: Arc<ChatManager>,
        auth_manager: Arc<AuthManager>,
        chat_id: ChatId,
        thread: Arc<DataxChat>,
        config: &Config,
        source: SessionSource,
        provider: SharedModelProvider,
    ) -> Self {
        let auth = auth_manager.auth_cached();
        let auth = auth.as_ref();
        let auth_mode = auth.map(CodexAuth::auth_mode).map(TelemetryAuthMode::from);
        let account_id = auth.and_then(CodexAuth::get_account_id);
        let account_email = auth.and_then(CodexAuth::get_account_email);
        let model = config.model.as_deref().unwrap_or("unknown");
        let auth_env_telemetry = collect_auth_env_telemetry(
            &config.model_provider,
            auth_manager.codex_api_key_env_enabled(),
        );
        let session_telemetry = SessionTelemetry::new(
            chat_id,
            model,
            model,
            account_id,
            account_email,
            auth_mode,
            originator().value,
            config.otel.log_user_prompt,
            user_agent(),
            source,
        )
        .with_auth_env(auth_env_telemetry.to_otel_metadata());

        Self {
            chat_id,
            thread,
            chat_manager,
            auth_manager,
            provider,
            session_telemetry,
        }
    }

    pub(crate) fn chat_id(&self) -> ChatId {
        self.chat_id
    }

    pub(crate) fn state_db(&self) -> Option<Arc<StateRuntime>> {
        self.thread.state_db()
    }

    pub(crate) fn provider(&self) -> &dyn ModelProvider {
        self.provider.as_ref()
    }

    pub(crate) fn counter(&self, name: &str, inc: i64, tags: &[(&str, &str)]) {
        self.session_telemetry.counter(name, inc, tags);
    }

    pub(crate) fn histogram(&self, name: &str, value: i64, tags: &[(&str, &str)]) {
        self.session_telemetry.histogram(name, value, tags);
    }

    pub(crate) fn start_timer(&self, name: &str) -> Option<datax_otel::Timer> {
        self.session_telemetry.start_timer(name, &[]).ok()
    }

    pub(crate) async fn stage_one_request_context(
        &self,
        config: &Config,
        model_name: &str,
        reasoning_effort: ReasoningEffort,
    ) -> StageOneRequestContext {
        let config_snapshot = self.thread.config_snapshot().await;
        let model_info = self
            .chat_manager
            .get_models_manager()
            .get_model_info(model_name, &config.to_models_manager_config())
            .await;
        let reasoning_summary = config
            .model_reasoning_summary
            .unwrap_or(model_info.default_reasoning_summary);

        StageOneRequestContext {
            model_info,
            session_telemetry: self
                .session_telemetry
                .clone()
                .with_model(model_name, model_name),
            reasoning_effort: Some(reasoning_effort),
            reasoning_summary,
            service_tier: config_snapshot.service_tier,
        }
    }

    pub(crate) async fn stream_stage_one_prompt(
        &self,
        config: &Config,
        prompt: &Prompt,
        context: &StageOneRequestContext,
    ) -> anyhow::Result<(String, Option<TokenUsage>)> {
        let installation_id = resolve_installation_id(&config.codex_home).await?;
        let config_snapshot = self.thread.config_snapshot().await;
        let session_source = config_snapshot.session_source;
        let session_id = SessionId::from(self.chat_id);
        let session_id_string = session_id.to_string();
        let model_client = ModelClient::new(
            Some(Arc::clone(&self.auth_manager)),
            self.chat_id,
            config.model_provider.clone(),
            session_source.clone(),
            config.model_verbosity,
            config.features.enabled(Feature::EnableRequestCompression),
            config.features.enabled(Feature::RuntimeMetrics),
            /*beta_features_header*/ None,
            config.features.enabled(Feature::ItemIds),
            /*attestation_provider*/ None,
        );

        let mut client_session = model_client.new_session();
        let window_id = format!("{}:0", self.chat_id);
        let responses_metadata = detached_memory_responses_metadata(
            installation_id,
            session_id_string,
            self.chat_id.to_string(),
            window_id,
            &session_source,
            &config.cwd,
            /*sandbox*/ None,
        )
        .await;
        let mut stream = client_session
            .stream(
                prompt,
                &context.model_info,
                &context.session_telemetry,
                context.reasoning_effort.clone(),
                context.reasoning_summary,
                context.service_tier.clone(),
                &responses_metadata,
                &InferenceTraceContext::disabled(),
            )
            .await?;

        let mut result = String::new();
        let mut token_usage = None;
        while let Some(message) = stream.next().await.transpose()? {
            match message {
                ResponseEvent::OutputTextDelta(delta) => result.push_str(&delta),
                ResponseEvent::OutputItemDone(item) => {
                    if result.is_empty()
                        && let datax_protocol::models::ResponseItem::Message { content, .. } = item
                        && let Some(text) = content_items_to_text(&content)
                    {
                        result.push_str(&text);
                    }
                }
                ResponseEvent::Completed {
                    token_usage: usage, ..
                } => {
                    token_usage = usage;
                    break;
                }
                _ => {}
            }
        }

        Ok((result, token_usage))
    }

    pub(crate) async fn spawn_consolidation_agent(
        &self,
        config: Config,
        prompt: Vec<UserInput>,
    ) -> anyhow::Result<SpawnedConsolidationAgent> {
        let environments = self
            .chat_manager
            .default_environment_selections(&config.cwd);
        let NewChat {
            chat_id,
            chat: thread,
            ..
        } = self
            .chat_manager
            .start_chat_with_options(StartChatOptions {
                config,
                initial_history: InitialHistory::New,
                session_source: Some(SessionSource::Internal(
                    InternalSessionSource::MemoryConsolidation,
                )),
                thread_source: Some(ThreadSource::MemoryConsolidation),
                dynamic_tools: Vec::new(),
                metrics_service_name: None,
                parent_trace: None,
                environments,
                thread_extension_init: Default::default(),
                supports_openai_form_elicitation: false,
            })
            .await?;

        let agent = SpawnedConsolidationAgent { chat_id, thread };
        if let Err(err) = agent
            .thread
            .submit(Op::UserInput {
                items: prompt,
                final_output_json_schema: None,
                responsesapi_client_metadata: None,
                additional_context: Default::default(),
                thread_settings: Default::default(),
            })
            .await
        {
            if let Err(shutdown_err) = self.shutdown_consolidation_agent(agent).await {
                tracing::warn!(
                    "failed to shut down consolidation agent after submit error: {shutdown_err}"
                );
            }
            return Err(err.into());
        }

        Ok(agent)
    }

    pub(crate) async fn shutdown_consolidation_agent(
        &self,
        agent: SpawnedConsolidationAgent,
    ) -> anyhow::Result<()> {
        let SpawnedConsolidationAgent { chat_id, thread } = agent;
        let thread = self
            .chat_manager
            .remove_chat(&chat_id)
            .await
            .unwrap_or(thread);

        tokio::time::timeout(Duration::from_secs(10), thread.shutdown_and_wait())
            .await
            .map_err(|_| {
                anyhow::anyhow!("memory consolidation agent {chat_id} shutdown timed out")
            })??;

        Ok(())
    }
}
