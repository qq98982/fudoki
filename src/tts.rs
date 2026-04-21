use std::sync::{Arc, Mutex};

use serde::Serialize;

#[derive(Clone, Debug)]
pub struct OpenAiCompatibleConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub default_voice: String,
    pub default_format: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TtsProviderDefaults {
    pub voice: String,
    pub format: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TtsProviderView {
    pub id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<TtsProviderDefaults>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TtsProvidersResponse {
    pub default_provider: String,
    pub providers: Vec<TtsProviderView>,
}

#[derive(Clone, Debug)]
pub struct TtsConfig {
    openai_compatible: Option<OpenAiCompatibleConfig>,
    default_provider: String,
    last_error: Arc<Mutex<Option<String>>>,
}

impl TtsConfig {
    pub fn disabled() -> Self {
        Self {
            openai_compatible: None,
            default_provider: "system".to_string(),
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn enabled(openai_compatible: OpenAiCompatibleConfig, default_provider: Option<String>) -> Self {
        let openai_id = "openai-compatible".to_string();
        Self {
            openai_compatible: Some(openai_compatible),
            default_provider: default_provider.unwrap_or(openai_id),
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn from_env() -> Self {
        let base_url = std::env::var("FUDOKI_TTS_OPENAI_BASE_URL").ok();
        let api_key = std::env::var("FUDOKI_TTS_OPENAI_API_KEY").ok();

        match (base_url, api_key) {
            (Some(base_url), Some(api_key)) => {
                let model =
                    std::env::var("FUDOKI_TTS_OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini-tts".to_string());
                let default_voice =
                    std::env::var("FUDOKI_TTS_OPENAI_DEFAULT_VOICE").unwrap_or_else(|_| "alloy".to_string());
                let default_format =
                    std::env::var("FUDOKI_TTS_OPENAI_DEFAULT_FORMAT").unwrap_or_else(|_| "mp3".to_string());
                let default_provider = std::env::var("FUDOKI_TTS_DEFAULT_PROVIDER").ok();

                Self::enabled(
                    OpenAiCompatibleConfig {
                        base_url,
                        api_key,
                        model,
                        default_voice,
                        default_format,
                    },
                    default_provider.or_else(|| Some("openai-compatible".to_string())),
                )
            }
            _ => Self::disabled(),
        }
    }

    pub fn providers_response(&self) -> TtsProvidersResponse {
        let mut providers = Vec::new();

        providers.push(TtsProviderView {
            id: "system".to_string(),
            status: "available".to_string(),
            defaults: None,
        });

        if let Some(cfg) = &self.openai_compatible {
            let status = if self
                .last_error
                .lock()
                .ok()
                .and_then(|g| g.as_ref().map(|_| ()))
                .is_some()
            {
                "unavailable"
            } else {
                "available"
            };

            providers.push(TtsProviderView {
                id: "openai-compatible".to_string(),
                status: status.to_string(),
                defaults: Some(TtsProviderDefaults {
                    voice: cfg.default_voice.clone(),
                    format: cfg.default_format.clone(),
                }),
            });
        }

        TtsProvidersResponse {
            default_provider: self.default_provider.clone(),
            providers,
        }
    }

    pub fn set_last_error(&self, error: impl Into<String>) {
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = Some(error.into());
        }
    }
}

