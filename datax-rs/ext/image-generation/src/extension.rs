use std::sync::Arc;

use datax_core::config::Config;
use datax_extension_api::ConfigContributor;
use datax_extension_api::ExtensionData;
use datax_extension_api::ExtensionFuture;
use datax_extension_api::ExtensionRegistryBuilder;
use datax_extension_api::ThreadLifecycleContributor;
use datax_extension_api::ThreadStartInput;
use datax_extension_api::ToolCall;
use datax_extension_api::ToolContributor;
use datax_extension_api::ToolExecutor;
use datax_login::AuthManager;
use datax_model_provider::create_model_provider;
use datax_model_provider_info::ModelProviderInfo;
use datax_utils_absolute_path::AbsolutePathBuf;

use crate::backend::CodexImagesBackend;
use crate::tool::ImageGenerationTool;

#[derive(Clone)]
struct ImageGenerationExtension {
    auth_manager: Arc<AuthManager>,
}

#[derive(Clone)]
struct ImageGenerationExtensionConfig {
    available: bool,
    provider: ModelProviderInfo,
    codex_home: AbsolutePathBuf,
}

impl From<&Config> for ImageGenerationExtensionConfig {
    /// Resolves whether standalone image generation should be available for a thread.
    fn from(config: &Config) -> Self {
        Self {
            // Core selects this executor per turn using the feature flag or model metadata.
            available: config.model_provider.is_openai(),
            provider: config.model_provider.clone(),
            codex_home: config.codex_home.clone(),
        }
    }
}

impl ThreadLifecycleContributor<Config> for ImageGenerationExtension {
    /// Seeds image-generation availability when a thread begins.
    fn on_thread_start<'a>(
        &'a self,
        input: ThreadStartInput<'a, Config>,
    ) -> ExtensionFuture<'a, ()> {
        Box::pin(async move {
            input
                .chat_store
                .insert(ImageGenerationExtensionConfig::from(input.config));
        })
    }
}

impl ConfigContributor<Config> for ImageGenerationExtension {
    /// Refreshes image-generation availability after thread configuration changes.
    fn on_config_changed(
        &self,
        _session_store: &ExtensionData,
        chat_store: &ExtensionData,
        _previous_config: &Config,
        new_config: &Config,
    ) {
        chat_store.insert(ImageGenerationExtensionConfig::from(new_config));
    }
}

impl ToolContributor for ImageGenerationExtension {
    /// Creates the image-generation tool exposed by this installed extension.
    fn tools(
        &self,
        _session_store: &ExtensionData,
        chat_store: &ExtensionData,
    ) -> Vec<Arc<dyn ToolExecutor<ToolCall>>> {
        let Some(config) = chat_store.get::<ImageGenerationExtensionConfig>() else {
            return Vec::new();
        };
        if !config.available || !self.auth_manager.current_auth_uses_codex_backend() {
            return Vec::new();
        }

        vec![Arc::new(ImageGenerationTool::new(
            CodexImagesBackend::new(create_model_provider(
                config.provider.clone(),
                Some(self.auth_manager.clone()),
            )),
            config.codex_home.clone(),
            chat_store.level_id().to_string(),
        ))]
    }
}

/// Installs the standalone image-generation extension contributors.
pub fn install(registry: &mut ExtensionRegistryBuilder<Config>, auth_manager: Arc<AuthManager>) {
    let extension = Arc::new(ImageGenerationExtension { auth_manager });
    registry.thread_lifecycle_contributor(extension.clone());
    registry.config_contributor(extension.clone());
    registry.tool_contributor(extension);
}
