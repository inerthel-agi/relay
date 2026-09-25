use super::{BUNDLED_CHANGELOG, changelog_body_for_language};

#[test]
fn bundled_changelog_includes_the_current_release_section() {
    let heading = format!("## [{}]", crate::updater::get_app_version());
    assert!(
        BUNDLED_CHANGELOG.contains(&heading)
            || BUNDLED_CHANGELOG
                .split("## [Unreleased]")
                .nth(1)
                .unwrap_or("")
                .split("\n## [")
                .next()
                .unwrap_or("")
                .contains(&format!(
                    "Target version: {}.",
                    crate::updater::get_app_version()
                )),
        "bundled CHANGELOG.md must include {heading}"
    );
    assert!(BUNDLED_CHANGELOG.contains("### English"));
    assert!(
        BUNDLED_CHANGELOG
            .lines()
            .filter(|line| line.starts_with("### "))
            .all(|line| line == "### English")
    );
}

#[test]
fn bundled_changelog_latest_release_matches_public_version() {
    let latest = BUNDLED_CHANGELOG
        .lines()
        .find(|line| line.starts_with("## [") && !line.starts_with("## [Unreleased]"))
        .expect("published changelog section");
    assert!(!latest.contains("Unreleased"));
    let published = latest.split(['[', ']']).nth(1).unwrap();
    assert_eq!(published, crate::updater::get_app_version());
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
