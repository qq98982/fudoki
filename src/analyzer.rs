use std::path::PathBuf;
use std::sync::Arc;

use sudachi::analysis::stateless_tokenizer::StatelessTokenizer;
use sudachi::analysis::Mode;
use sudachi::analysis::Tokenize;
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;

use crate::english::classify_english_token;
use crate::models::TokenPayload;
use crate::number_reading::enhance_numeric_token_readings;
use crate::reading_normalization::normalize_token_readings;

pub struct Analyzer {
    tokenizer: StatelessTokenizer<Arc<JapaneseDictionary>>,
}

impl Analyzer {
    pub fn new() -> Self {
        let config = Config::new(Some(PathBuf::from("resources/sudachi.json")), None, None)
            .expect("load sudachi config");
        let dictionary = JapaneseDictionary::from_cfg(&config).expect("load sudachi dictionary");
        let tokenizer = StatelessTokenizer::new(Arc::new(dictionary));
        Self { tokenizer }
    }

    pub fn analyze(&self, text: &str) -> Vec<Vec<TokenPayload>> {
        text.lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                let morphemes = self.tokenizer.tokenize(line, Mode::C, false).expect("tokenize");

                let tokens = morphemes
                    .iter()
                    .map(|morpheme| {
                        let surface = morpheme.surface().to_string();
                        if is_english_like_token(&surface) {
                            let english = classify_english_token(&surface);
                            return TokenPayload {
                                surface: surface.clone(),
                                lemma: surface,
                                reading: english.reading,
                                tts_text: english.tts_text,
                                pos: vec!["名詞".to_string()],
                                source: english.source.to_string(),
                                confidence: english.confidence as f32 / 100.0,
                            };
                        }

                        let reading = morpheme.reading_form().to_string();
                        TokenPayload {
                            surface: surface.clone(),
                            lemma: morpheme.dictionary_form().to_string(),
                            reading: reading.clone(),
                            tts_text: if reading.is_empty() { surface } else { reading },
                            pos: morpheme.part_of_speech().iter().map(|s| s.to_string()).collect(),
                            source: "sudachi".to_string(),
                            confidence: 1.0,
                        }
                    })
                    .collect::<Vec<_>>();

                let mut tokens = tokens;
                enhance_numeric_token_readings(&mut tokens);
                normalize_token_readings(&mut tokens);

                Some(tokens)
            })
            .collect()
    }
}

fn is_english_like_token(surface: &str) -> bool {
    let mut has_ascii_letter = false;

    for ch in surface.chars() {
        if ch.is_ascii_alphabetic() {
            has_ascii_letter = true;
            continue;
        }

        if ch.is_ascii_digit() || matches!(ch, '.' | '+' | '-' | '_') {
            continue;
        }

        return false;
    }

    has_ascii_letter
}
