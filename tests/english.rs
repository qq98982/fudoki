use fudoki_backend::english::classify_english_token;

#[test]
fn all_caps_token_is_treated_as_abbreviation() {
    let result = classify_english_token("API");
    assert_eq!(result.kind, "abbreviation");
    assert_eq!(result.reading, "エーピーアイ");
    assert_eq!(result.tts_text, "エーピーアイ");
}

#[test]
fn known_technical_term_uses_japanese_usage() {
    let result = classify_english_token("React");
    assert_eq!(result.kind, "override");
    assert_eq!(result.reading, "リアクト");
    assert_eq!(result.tts_text, "リアクト");
}

#[test]
fn unknown_english_term_keeps_original_text_for_tts() {
    let result = classify_english_token("browser");
    assert_eq!(result.kind, "unknown");
    assert_eq!(result.reading, "");
    assert_eq!(result.tts_text, "browser");
}
