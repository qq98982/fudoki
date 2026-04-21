use std::sync::{Arc, Mutex};

use axum::http::{header, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct OpenAiCompatibleConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub default_voice: String,
    pub default_format: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SpeakRequest {
    #[serde(default)]
    pub provider: Option<String>,
    pub text: String,
    #[serde(default)]
    pub voice: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub speed: Option<f32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApiErrorResponse {
    pub error: ApiErrorBody,
}

#[derive(Clone, Debug)]
pub struct SynthesizedSpeech {
    pub content_type: String,
    pub bytes: Vec<u8>,
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

#[derive(Clone)]
pub struct TtsConfig {
    openai_compatible: Option<OpenAiCompatibleConfig>,
    default_provider: String,
    last_error: Arc<Mutex<Option<String>>>,
}

impl TtsConfig {
    fn env_trimmed_nonempty(key: &str) -> Option<String> {
        let value = std::env::var(key).ok()?;
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    pub fn disabled() -> Self {
        Self {
            openai_compatible: None,
            default_provider: "system".to_string(),
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn enabled(
        openai_compatible: OpenAiCompatibleConfig,
        default_provider: Option<String>,
    ) -> Self {
        let openai_id = "openai-compatible".to_string();
        Self {
            openai_compatible: Some(openai_compatible),
            default_provider: default_provider.unwrap_or(openai_id),
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn from_env() -> Self {
        let base_url = Self::env_trimmed_nonempty("FUDOKI_TTS_OPENAI_BASE_URL");
        let api_key = Self::env_trimmed_nonempty("FUDOKI_TTS_OPENAI_API_KEY");
        let model = Self::env_trimmed_nonempty("FUDOKI_TTS_OPENAI_MODEL");

        match (base_url, api_key, model) {
            (Some(base_url), Some(api_key), Some(model)) => {
                let default_voice =
                    Self::env_trimmed_nonempty("FUDOKI_TTS_OPENAI_VOICE").unwrap_or_else(|| "alloy".to_string());
                let default_format =
                    Self::env_trimmed_nonempty("FUDOKI_TTS_OPENAI_FORMAT").unwrap_or_else(|| "mp3".to_string());
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
        let has_online = self.openai_compatible.is_some();

        providers.push(TtsProviderView {
            id: "system".to_string(),
            status: "available".to_string(),
            defaults: None,
        });

        if let Some(cfg) = &self.openai_compatible {
            // Fail closed: if we can't read the error state (e.g. poisoned mutex), do not
            // report this provider as available.
            let status = match self.last_error.lock() {
                Ok(guard) => {
                    if guard.is_some() {
                        "unavailable"
                    } else {
                        "available"
                    }
                }
                Err(_) => "unavailable",
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

        let default_provider = match self.default_provider.as_str() {
            "system" => "system".to_string(),
            "openai-compatible" if has_online => "openai-compatible".to_string(),
            _ if has_online => "openai-compatible".to_string(),
            _ => "system".to_string(),
        };

        TtsProvidersResponse {
            default_provider,
            providers,
        }
    }

    pub fn set_last_error(&self, error: impl Into<String>) {
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = Some(error.into());
        }
    }

    fn clear_last_error(&self) {
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = None;
        }
    }

    fn api_error(code: &'static str, message: impl Into<String>) -> ApiErrorResponse {
        ApiErrorResponse {
            error: ApiErrorBody {
                code: code.to_string(),
                message: message.into(),
            },
        }
    }

    pub async fn synthesize(
        &self,
        request: &SpeakRequest,
    ) -> Result<SynthesizedSpeech, (StatusCode, ApiErrorResponse)> {
        let provider = request
            .provider
            .as_deref()
            .unwrap_or(self.default_provider.as_str());

        if provider != "openai-compatible" {
            let err = Self::api_error(
                "tts_bad_request",
                format!("Unsupported TTS provider: {provider}"),
            );
            self.set_last_error(err.error.message.clone());
            return Err((StatusCode::BAD_REQUEST, err));
        }

        let cfg = match &self.openai_compatible {
            Some(cfg) => cfg,
            None => {
                let err = Self::api_error(
                    "tts_bad_request",
                    "TTS provider not configured: openai-compatible",
                );
                self.set_last_error(err.error.message.clone());
                return Err((StatusCode::BAD_REQUEST, err));
            }
        };

        let base_url = cfg.base_url.trim_end_matches('/');
        let url = format!("{base_url}/audio/speech");

        let voice = request.voice.as_deref().unwrap_or(cfg.default_voice.as_str());
        let response_format = request
            .format
            .as_deref()
            .unwrap_or(cfg.default_format.as_str());
        let speed = request.speed.unwrap_or(1.0);

        let client = reqwest::Client::new();
        let resp = client
            .post(url)
            .bearer_auth(&cfg.api_key)
            .json(&serde_json::json!({
                "model": cfg.model,
                "input": request.text,
                "voice": voice,
                "response_format": response_format,
                "speed": speed,
            }))
            .send()
            .await
            .map_err(|_e| {
                let message = "TTS request failed: upstream request error".to_string();
                let err = Self::api_error("tts_request_failed", message.clone());
                self.set_last_error(message);
                (StatusCode::BAD_GATEWAY, err)
            })?;

        let status = resp.status();
        if !status.is_success() {
            let message = format!("TTS request failed: {status}");
            let err = Self::api_error("tts_request_failed", message.clone());
            self.set_last_error(message);
            return Err((StatusCode::BAD_GATEWAY, err));
        }

        let content_type = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("audio/mpeg")
            .to_string();

        let bytes = resp.bytes().await.map_err(|_e| {
            let message = "TTS request failed: upstream response error".to_string();
            let err = Self::api_error("tts_request_failed", message.clone());
            self.set_last_error(message);
            (StatusCode::BAD_GATEWAY, err)
        })?;

        self.clear_last_error();
        Ok(SynthesizedSpeech {
            content_type,
            bytes: bytes.to_vec(),
        })
    }

    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn __poison_last_error_mutex_for_test(&self) {
        let last_error = self.last_error.clone();
        let _ = std::panic::catch_unwind(move || {
            let _guard = last_error.lock().expect("lock last_error");
            panic!("intentional panic while holding last_error lock (test helper)");
        });
    }
}
