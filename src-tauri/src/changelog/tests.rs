use super::{BUNDLED_CHANGELOG, changelog_body_for_language};

const CURRENT_HEADINGS: [&str; 9] = [
    "### English",
    "### Français",
    "### Español",
    "### Deutsch",
    "### Русский",
    "### 简体中文",
    "### 한국어",
    "### 日本語",
    "### Bahasa Indonesia",
];

#[test]
fn bundled_changelog_includes_the_current_release_section() {
    let heading = format!("## [{}]", env!("CARGO_PKG_VERSION"));
    assert!(
        BUNDLED_CHANGELOG.contains(&heading)
            || BUNDLED_CHANGELOG
                .split("## [Unreleased]")
                .nth(1)
                .unwrap_or("")
                .split("\n## [")
                .next()
                .unwrap_or("")
                .contains(&format!("Target version: {}.", env!("CARGO_PKG_VERSION"))),
        "bundled CHANGELOG.md must include {heading}"
    );
    for heading in CURRENT_HEADINGS {
        assert!(
            BUNDLED_CHANGELOG.contains(heading),
            "bundled CHANGELOG.md must include {heading}"
        );
    }
}

#[test]
fn bundled_changelog_skips_unreleased_before_the_latest_version() {
    let latest = BUNDLED_CHANGELOG
        .lines()
        .find(|line| line.starts_with("## [") && !line.starts_with("## [Unreleased]"))
        .expect("published changelog section");
    assert!(!latest.contains("Unreleased"));
    let published = latest.split(['[', ']']).nth(1).unwrap();
    let parse = |value: &str| {
        value
            .split('.')
            .map(|part| part.parse::<u32>().unwrap())
            .collect::<Vec<_>>()
    };
    assert!(parse(published) <= parse(env!("CARGO_PKG_VERSION")));
}

#[test]
fn selects_translated_sections_and_falls_back_to_english() {
    let body = "### English\n\n#### Added\n\n- New feature.\n\n### Français\n\n#### Ajouté\n\n- Nouvelle fonctionnalité.\n\n### Deutsch\n\n#### Hinzugefügt\n\n- Neue Funktion.\n";
    assert!(changelog_body_for_language(body, "en").contains("New feature."));
    assert!(!changelog_body_for_language(body, "en").contains("Neue Funktion."));
    assert!(changelog_body_for_language(body, "fr").contains("Nouvelle fonctionnalité."));
    assert!(changelog_body_for_language(body, "de").contains("Neue Funktion."));
    assert!(changelog_body_for_language(body, "ja").contains("New feature."));
    assert!(!changelog_body_for_language(body, "ja").contains("#### Ajouté"));
}
