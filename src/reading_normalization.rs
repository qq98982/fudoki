use crate::models::TokenPayload;

pub fn normalize_token_readings(tokens: &mut [TokenPayload]) {
    normalize_common_particle_pronunciations(tokens);
}

fn normalize_common_particle_pronunciations(tokens: &mut [TokenPayload]) {
    for token in tokens {
        if !is_particle(token) {
            continue;
        }

        let normalized = match token.surface.as_str() {
            "は" => Some("ワ"),
            "へ" => Some("エ"),
            "を" => Some("オ"),
            _ => None,
        };

        let Some(reading) = normalized else {
            continue;
        };

        token.reading = reading.to_string();
        token.tts_text = reading.to_string();
    }
}

fn is_particle(token: &TokenPayload) -> bool {
    token.pos.first().is_some_and(|part| part == "助詞")
}
