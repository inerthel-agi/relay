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
fn unknown_language_falls_back_to_english() {
    assert_eq!(
        music_strings_for_language("pt-BR").results_title,
        "YouTube results"
    );
}
