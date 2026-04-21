use std::path::PathBuf;
use std::sync::Arc;

use sudachi::analysis::stateless_tokenizer::StatelessTokenizer;
use sudachi::analysis::Mode;
use sudachi::analysis::Tokenize;
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;

use crate::english::classify_english_token;
use crate::models::TokenPayload;

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
                        if surface.chars().all(|c| {
                            c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-' | '_')
                        }) {
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

                Some(tokens)
            })
            .collect()
    }
}
