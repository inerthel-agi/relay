use super::*;

#[test]
fn french_interface_uses_french_music_copy() {
    assert_eq!(
        music_strings_for_language("fr").results_title,
        "Résultats YouTube"
    );
    assert_eq!(music_strings_for_language("fr-FR").cancel, "Annuler");
    assert!(
        music_strings_for_language("fr")
            .search_cooldown
            .contains("{seconds}")
    );
}

#[test]
fn every_language_says_how_long_the_search_limit_lasts() {
    for language in ["en", "fr", "es", "de", "ru", "zh", "ko", "ja", "id"] {
        let strings = music_strings_for_language(language);
        assert!(
            strings.search_hourly_limit.contains("{minutes}"),
            "{language}"
        );
        assert!(language == "en" || strings.search_hourly_limit != EN.search_hourly_limit);
    }
}

#[test]
fn unknown_language_falls_back_to_english() {
    assert_eq!(
        music_strings_for_language("pt-BR").results_title,
        "YouTube results"
    );
}
