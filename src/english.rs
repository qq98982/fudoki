#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnglishToken {
    pub kind: &'static str,
    pub reading: String,
    pub tts_text: String,
    pub source: &'static str,
    pub confidence: u8,
}

fn abbreviation_reading(token: &str) -> String {
    token
        .chars()
        .map(|ch| match ch {
            'A' => "エー",
            'B' => "ビー",
            'C' => "シー",
            'D' => "ディー",
            'E' => "イー",
            'F' => "エフ",
            'G' => "ジー",
            'H' => "エイチ",
            'I' => "アイ",
            'J' => "ジェー",
            'K' => "ケー",
            'L' => "エル",
            'M' => "エム",
            'N' => "エヌ",
            'O' => "オー",
            'P' => "ピー",
            'Q' => "キュー",
            'R' => "アール",
            'S' => "エス",
            'T' => "ティー",
            'U' => "ユー",
            'V' => "ブイ",
            'W' => "ダブリュー",
            'X' => "エックス",
            'Y' => "ワイ",
            'Z' => "ズィー",
            _ => "",
        })
        .collect()
}

pub fn classify_english_token(token: &str) -> EnglishToken {
    let lower = token.to_ascii_lowercase();
    let override_reading = match lower.as_str() {
        "react" => Some("リアクト"),
        "docker" => Some("ドッカー"),
        "github" => Some("ギットハブ"),
        "typescript" => Some("タイプスクリプト"),
        "javascript" => Some("ジャバスクリプト"),
        _ => None,
    };

    if token.chars().all(|c| c.is_ascii_uppercase()) {
        let reading = abbreviation_reading(token);
        return EnglishToken {
            kind: "abbreviation",
            reading: reading.clone(),
            tts_text: reading,
            source: "abbreviation",
            confidence: 100,
        };
    }

    if let Some(reading) = override_reading {
        return EnglishToken {
            kind: "override",
            reading: reading.to_string(),
            tts_text: reading.to_string(),
            source: "override",
            confidence: 100,
        };
    }

    EnglishToken {
        kind: "unknown",
        reading: String::new(),
        tts_text: token.to_string(),
        source: "unknown_english",
        confidence: 0,
    }
}
