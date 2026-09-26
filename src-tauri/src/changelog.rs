const BUNDLED_CHANGELOG: &str = include_str!("../../CHANGELOG.md");

#[tauri::command]
pub fn get_changelog_markdown() -> &'static str {
    BUNDLED_CHANGELOG
}

pub fn changelog_body_for_language(body: &str, language: &str) -> String {
    let mut buckets = std::collections::BTreeMap::<String, Vec<&str>>::new();
    let mut current = "default".to_owned();
    buckets.insert(current.clone(), Vec::new());
    for line in body.lines() {
        if let Some(heading) = line.strip_prefix("### ") {
            current = heading.trim().to_lowercase();
            buckets.entry(current.clone()).or_default();
            continue;
        }
        buckets.entry(current.clone()).or_default().push(line);
    }

    let mut keys = changelog_heading_aliases(language).to_vec();
    if language != "en" {
        keys.push("english");
    }
    for key in keys {
        let Some(lines) = buckets.get(key) else {
            continue;
        };
        let text = lines.join("\n").trim().to_owned();
        if !text.is_empty() {
            return text;
        }
    }
    body.trim().to_owned()
}

fn changelog_heading_aliases(language: &str) -> &'static [&'static str] {
    match language {
        "fr" => &["français", "francais"],
        "es" => &["español", "espanol", "spanish"],
        "de" => &["deutsch", "german"],
        "ru" => &["русский", "russian"],
        "zh" => &["简体中文", "chinese"],
        "ko" => &["한국어", "korean"],
        "ja" => &["日本語", "japanese"],
        "id" => &["bahasa indonesia", "indonesian"],
        _ => &["english"],
    }
}

#[cfg(test)]
mod tests;
