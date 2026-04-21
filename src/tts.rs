use std::time::Duration;
use std::sync::{Arc, Mutex};

use axum::http::{header, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct OpenAiCompatibleConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub model_options: Vec<String>,
    pub default_voice: String,
    pub voice_options: Vec<String>,
    pub default_format: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SpeakRequest {
    #[serde(default)]
    pub provider: Option<String>,
    pub text: String,
    #[serde(default)]
    pub model: Option<String>,
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
    pub model: String,
    pub voice: String,
    pub format: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TtsProviderOptions {
    pub models: Vec<String>,
    pub voices: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TtsProviderView {
    pub id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<TtsProviderDefaults>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<TtsProviderOptions>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TtsProvidersResponse {
    pub default_provider: String,
    pub providers: Vec<TtsProviderView>,
}

#[derive(Clone)]
pub struct TtsConfig {
    client: reqwest::Client,
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

    fn build_http_client() -> reqwest::Client {
        // Keep this concrete to avoid hanging requests when upstream misbehaves.
        reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .expect("build reqwest client")
    }

    fn parse_option_list(key: &str) -> Vec<String> {
        Self::env_trimmed_nonempty(key)
            .map(|raw| {
                raw.split(',')
                    .map(|part| part.trim())
                    .filter(|part| !part.is_empty())
                    .map(|part| part.to_string())
                    .fold(Vec::<String>::new(), |mut acc, item| {
                        if !acc.contains(&item) {
                            acc.push(item);
                        }
                        acc
                    })
            })
            .unwrap_or_default()
    }

    fn default_or_first_valid(default_value: String, allowed_values: &mut Vec<String>) -> String {
        if allowed_values.is_empty() {
            allowed_values.push(default_value.clone());
            return default_value;
        }
        if allowed_values.contains(&default_value) {
            return default_value;
        }
        allowed_values[0].clone()
    }

    pub fn disabled() -> Self {
        Self {
            client: Self::build_http_client(),
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
            client: Self::build_http_client(),
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
                let mut model_options = Self::parse_option_list("FUDOKI_TTS_OPENAI_MODEL_OPTIONS");
                let model = Self::default_or_first_valid(model, &mut model_options);
                let default_voice =
                    Self::env_trimmed_nonempty("FUDOKI_TTS_OPENAI_VOICE").unwrap_or_else(|| "alloy".to_string());
                let mut voice_options = Self::parse_option_list("FUDOKI_TTS_OPENAI_VOICE_OPTIONS");
                let default_voice = Self::default_or_first_valid(default_voice, &mut voice_options);
                let default_format =
                    Self::env_trimmed_nonempty("FUDOKI_TTS_OPENAI_FORMAT").unwrap_or_else(|| "mp3".to_string());
                let default_provider = Self::env_trimmed_nonempty("FUDOKI_TTS_DEFAULT_PROVIDER");

                Self::enabled(
                    OpenAiCompatibleConfig {
                        base_url,
                        api_key,
                        model,
                        model_options,
                        default_voice,
                        voice_options,
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
            options: None,
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
                    model: cfg.model.clone(),
                    voice: cfg.default_voice.clone(),
                    format: cfg.default_format.clone(),
                }),
                options: Some(TtsProviderOptions {
                    models: cfg.model_options.clone(),
                    voices: cfg.voice_options.clone(),
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
            return Err((StatusCode::BAD_REQUEST, err));
        }

        let cfg = match &self.openai_compatible {
            Some(cfg) => cfg,
            None => {
                let err = Self::api_error(
                    "tts_bad_request",
                    "TTS provider not configured: openai-compatible",
                );
                return Err((StatusCode::BAD_REQUEST, err));
            }
        };

        let base_url = cfg.base_url.trim_end_matches('/');
        let url = format!("{base_url}/audio/speech");

        let model = request.model.as_deref().unwrap_or(cfg.model.as_str());
        let voice = request.voice.as_deref().unwrap_or(cfg.default_voice.as_str());
        let response_format = request
            .format
            .as_deref()
            .unwrap_or(cfg.default_format.as_str());
        let speed = request.speed.unwrap_or(1.0).clamp(0.5, 2.0);

        if !cfg.model_options.iter().any(|allowed| allowed == model) {
            let err = Self::api_error(
                "tts_bad_request",
                format!("Unsupported TTS model: {model}"),
            );
            return Err((StatusCode::BAD_REQUEST, err));
        }

        if !cfg.voice_options.iter().any(|allowed| allowed == voice) {
            let err = Self::api_error(
                "tts_bad_request",
                format!("Unsupported TTS voice: {voice}"),
            );
            return Err((StatusCode::BAD_REQUEST, err));
        }

        let resp = self
            .client
            .post(url)
            .bearer_auth(&cfg.api_key)
            .json(&serde_json::json!({
                "model": model,
                "input": request.text,
                "voice": voice,
                "response_format": response_format,
                "speed": speed,
            }))
            .send()
            .await
            .map_err(|e| {
                eprintln!("tts: upstream request failed: {e}");
                let message = "TTS request failed: upstream request error".to_string();
                let err = Self::api_error("tts_request_failed", message.clone());
                self.set_last_error(message);
                (StatusCode::BAD_GATEWAY, err)
            })?;

        let status = resp.status();
        if !status.is_success() {
            let message = format!("TTS request failed: {status}");
            eprintln!("tts: upstream returned non-success: {message}");
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

        let bytes = resp.bytes().await.map_err(|e| {
            eprintln!("tts: failed reading upstream audio bytes: {e}");
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
