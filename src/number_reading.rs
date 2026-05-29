use crate::models::TokenPayload;

pub fn enhance_numeric_token_readings(tokens: &mut [TokenPayload]) {
    for index in 0..tokens.len() {
        let Some(number) = normalize_digits(&tokens[index].surface) else {
            continue;
        };

        let previous_surface = index
            .checked_sub(1)
            .and_then(|value| tokens.get(value))
            .map(|token| token.surface.as_str());
        let next_surface = tokens.get(index + 1).map(|token| token.surface.as_str());

        let reading = infer_numeric_reading(&number, previous_surface, next_surface);
        if reading.is_empty() {
            continue;
        }

        tokens[index].reading = reading.clone();
        tokens[index].tts_text = reading;
        tokens[index].source = "numeric_guess".to_string();
        tokens[index].confidence = 0.65;
    }
}

fn infer_numeric_reading(number: &str, previous_surface: Option<&str>, next_surface: Option<&str>) -> String {
    if previous_surface == Some("第") {
        return cardinal_reading(number);
    }

    match next_surface {
        Some("つ") => tsu_counter_reading(number).unwrap_or_else(|| cardinal_reading(number)),
        Some("人") => person_counter_reading(number).unwrap_or_else(|| cardinal_reading(number)),
        _ => cardinal_reading(number),
    }
}

fn normalize_digits(surface: &str) -> Option<String> {
    let mut normalized = String::with_capacity(surface.len());
    for ch in surface.chars() {
        match ch {
            '0'..='9' => normalized.push(ch),
            '０'..='９' => normalized.push(char::from_u32(ch as u32 - '０' as u32 + '0' as u32)?),
            _ => return None,
        }
    }

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn tsu_counter_reading(number: &str) -> Option<String> {
    match number {
        "1" => Some("ヒト".to_string()),
        "2" => Some("フタ".to_string()),
        "3" => Some("ミッ".to_string()),
        "4" => Some("ヨッ".to_string()),
        "5" => Some("イツ".to_string()),
        "6" => Some("ムッ".to_string()),
        "7" => Some("ナナ".to_string()),
        "8" => Some("ヤッ".to_string()),
        "9" => Some("ココノ".to_string()),
        "10" => Some("トオ".to_string()),
        _ => None,
    }
}

fn person_counter_reading(number: &str) -> Option<String> {
    match number {
        "1" => Some("ヒト".to_string()),
        "2" => Some("フタ".to_string()),
        _ => None,
    }
}

fn cardinal_reading(number: &str) -> String {
    if number == "0" {
        return "ゼロ".to_string();
    }

    let trimmed = number.trim_start_matches('0');
    if trimmed.is_empty() {
        return "ゼロ".to_string();
    }

    let groups = split_groups_of_four(trimmed);
    let large_units = ["", "マン", "オク", "チョウ"];
    let mut reading = String::new();

    for (index, group) in groups.iter().enumerate() {
        let value: u16 = group.parse().unwrap_or(0);
        if value == 0 {
            continue;
        }

        reading.push_str(&read_under_ten_thousand(value));
        let unit_index = groups.len() - index - 1;
        if let Some(unit) = large_units.get(unit_index) {
            reading.push_str(unit);
        }
    }

    if reading.is_empty() {
        "ゼロ".to_string()
    } else {
        reading
    }
}

fn split_groups_of_four(number: &str) -> Vec<&str> {
    let mut groups = Vec::new();
    let mut end = number.len();
    while end > 0 {
        let start = end.saturating_sub(4);
        groups.push(&number[start..end]);
        end = start;
    }
    groups.reverse();
    groups
}

fn read_under_ten_thousand(value: u16) -> String {
    let thousands = value / 1000;
    let hundreds = (value % 1000) / 100;
    let tens = (value % 100) / 10;
    let ones = value % 10;

    let mut reading = String::new();

    if thousands > 0 {
        let thousands_reading = match thousands {
            1 => "セン".to_string(),
            3 => "サンゼン".to_string(),
            8 => "ハッセン".to_string(),
            _ => format!("{}セン", digit_reading(thousands as u8)),
        };
        reading.push_str(&thousands_reading);
    }

    if hundreds > 0 {
        let hundreds_reading = match hundreds {
            1 => "ヒャク".to_string(),
            3 => "サンビャク".to_string(),
            6 => "ロッピャク".to_string(),
            8 => "ハッピャク".to_string(),
            _ => format!("{}ヒャク", digit_reading(hundreds as u8)),
        };
        reading.push_str(&hundreds_reading);
    }

    if tens > 0 {
        if tens == 1 {
            reading.push_str("ジュウ");
        } else {
            reading.push_str(digit_reading(tens as u8));
            reading.push_str("ジュウ");
        }
    }

    if ones > 0 {
        reading.push_str(digit_reading(ones as u8));
    }

    reading
}

fn digit_reading(value: u8) -> &'static str {
    match value {
        0 => "ゼロ",
        1 => "イチ",
        2 => "ニ",
        3 => "サン",
        4 => "ヨン",
        5 => "ゴ",
        6 => "ロク",
        7 => "ナナ",
        8 => "ハチ",
        9 => "キュウ",
        _ => "",
    }
}
