use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub text: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct TokenPayload {
    pub surface: String,
    pub lemma: String,
    pub reading: String,
    pub tts_text: String,
    pub pos: Vec<String>,
    pub source: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResponse {
    pub lines: Vec<Vec<TokenPayload>>,
}
