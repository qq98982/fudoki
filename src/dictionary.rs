use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
struct ChunkRoot {
    words: Vec<Entry>,
}

#[derive(Debug, Clone, Deserialize)]
struct Entry {
    #[serde(default)]
    kanji: Vec<TextValue>,
    #[serde(default)]
    kana: Vec<TextValue>,
    #[serde(default)]
    sense: Vec<Sense>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextValue {
    pub text: String,
    #[serde(default)]
    pub common: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct Gloss {
    text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LanguageSource {
    lang: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Sense {
    #[serde(default, rename = "partOfSpeech")]
    part_of_speech: Vec<String>,
    #[serde(default)]
    field: Vec<String>,
    #[serde(default)]
    misc: Vec<String>,
    #[serde(default)]
    info: Vec<String>,
    #[serde(default)]
    gloss: Vec<Gloss>,
    #[serde(default, rename = "languageSource")]
    language_source: Vec<LanguageSource>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SensePayload {
    pub gloss: String,
    pub part_of_speech: Vec<String>,
    pub field: Vec<String>,
    pub misc: Vec<String>,
    pub info: Vec<String>,
    pub chinese_source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryPayload {
    pub word: String,
    pub query: String,
    pub kanji: Vec<TextValue>,
    pub kana: Vec<TextValue>,
    pub senses: Vec<SensePayload>,
    pub has_multiple_meanings: bool,
    pub total_results: usize,
    pub lookup_source: String,
}

#[derive(Default)]
pub struct DictionaryService {
    entries: Vec<Entry>,
    index: HashMap<String, Vec<usize>>,
}

impl DictionaryService {
    pub fn new() -> Self {
        let mut service = Self::default();
        let mut chunk_paths = fs::read_dir("static/libs/dict/chunks")
            .expect("read chunk dir")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with("jmdict_chunk_") && name.ends_with(".json")
                    })
            })
            .collect::<Vec<PathBuf>>();
        chunk_paths.sort();

        for path in chunk_paths {
            let reader = BufReader::new(File::open(&path).expect("open chunk file"));
            let chunk: ChunkRoot = serde_json::from_reader(reader).expect("parse chunk");
            for entry in chunk.words {
                let index = service.entries.len();
                service.entries.push(entry);
                let stored = &service.entries[index];

                for item in &stored.kanji {
                    service
                        .index
                        .entry(item.text.clone())
                        .or_default()
                        .push(index);
                }
                for item in &stored.kana {
                    service
                        .index
                        .entry(item.text.clone())
                        .or_default()
                        .push(index);
                }
            }
        }

        service
    }

    pub fn is_ready(&self) -> bool {
        !self.entries.is_empty()
    }

    pub fn lookup(&self, term: &str) -> Option<DictionaryPayload> {
        let term = term.trim();
        if term.is_empty() {
            return None;
        }

        let entry_indexes = self.index.get(term)?;
        let first_index = *entry_indexes.first()?;
        let entry = self.entries.get(first_index)?;

        let senses = entry
            .sense
            .iter()
            .map(|sense| SensePayload {
                gloss: sense
                    .gloss
                    .iter()
                    .filter_map(|gloss| gloss.text.clone())
                    .collect::<Vec<_>>()
                    .join("; "),
                part_of_speech: sense.part_of_speech.clone(),
                field: sense.field.clone(),
                misc: sense.misc.clone(),
                info: sense.info.clone(),
                chinese_source: sense
                    .language_source
                    .iter()
                    .find(|source| source.lang.as_deref() == Some("chi"))
                    .and_then(|source| source.text.clone()),
            })
            .collect::<Vec<_>>();

        Some(DictionaryPayload {
            word: term.to_string(),
            query: term.to_string(),
            kanji: entry.kanji.clone(),
            kana: entry.kana.clone(),
            has_multiple_meanings: senses.len() > 1,
            total_results: entry_indexes.len(),
            senses,
            lookup_source: "jmdict".to_string(),
        })
    }
}
