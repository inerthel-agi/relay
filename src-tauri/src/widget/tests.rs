use super::*;
use std::sync::atomic::AtomicBool;

#[test]
fn youtube_embed_host_avoids_loopback_ip_referrer() {
    assert_eq!(youtube_embed_host(), "localhost");
}

#[test]
fn clamps_media_widget_size_to_the_screen_and_ratio() {
    assert_eq!(
        clamp_logical_size(2_000.0, 1_000.0, 1_280.0, 720.0, true),
        (1_280.0, 720.0)
    );
    assert_eq!(
        clamp_logical_size(900.0, 400.0, 1_920.0, 1_080.0, false),
        (900.0, 400.0)
    );
}

#[test]
fn media_widget_position_flush_writes_the_live_physical_coordinates() {
    let mut config = crate::config::AppConfig::default();
    write_position(&mut config, PhysicalPosition::new(2140, 72));
    assert_eq!(config.widget_x, Some(2140));
    assert_eq!(config.widget_y, Some(72));
}

#[test]
fn hidden_wake_claims_ownership_but_persistent_visibility_does_not() {
    let marker = AtomicBool::new(false);

    assert!(establish_ephemeral_ownership(&marker, false));
    assert!(marker.load(Ordering::Relaxed));
    assert!(!establish_ephemeral_ownership(&marker, true));
    assert!(!marker.load(Ordering::Relaxed));
}

#[test]
fn ownership_is_retained_until_persistent_takeover_completes() {
    let marker = AtomicBool::new(true);

    assert!(marker.load(Ordering::Relaxed));
    complete_persistent_takeover(&marker);

    assert!(!marker.load(Ordering::Relaxed));
}

#[test]
fn idle_claim_consumes_ephemeral_ownership() {
    let marker = AtomicBool::new(true);

    assert!(claim_ephemeral_ownership(&marker));
    assert!(!marker.load(Ordering::Relaxed));
    assert!(!claim_ephemeral_ownership(&marker));
}

#[test]
fn failed_transition_restores_ephemeral_ownership() {
    let marker = AtomicBool::new(true);
    assert!(claim_ephemeral_ownership(&marker));

    restore_ephemeral_ownership(&marker);

    assert!(marker.load(Ordering::Relaxed));
}

#[test]
fn failed_wake_rolls_back_only_before_the_window_can_be_visible() {
    let marker = AtomicBool::new(false);
    assert!(establish_ephemeral_ownership(&marker, false));

    rollback_ephemeral_ownership(&marker, true, false);
    assert!(!marker.load(Ordering::Relaxed));

    assert!(establish_ephemeral_ownership(&marker, false));
    rollback_ephemeral_ownership(&marker, true, true);
    assert!(marker.load(Ordering::Relaxed));
}

#[test]
fn the_other_widget_follows_a_drag_by_the_same_amount() {
    let at = PhysicalPosition::new;
    assert_eq!(
        follow_target(Some(at(100, 50)), at(130, 40), at(900, 600)),
        Some(at(930, 590))
    );
    // First position seen, or a move placed from code (already remembered): nothing follows.
    assert_eq!(follow_target(None, at(130, 40), at(900, 600)), None);
    assert_eq!(
        follow_target(Some(at(130, 40)), at(130, 40), at(900, 600)),
        None
    );
}

#[test]
fn only_a_drag_of_a_focused_unlocked_widget_of_the_same_size_is_followed() {
    let size = PhysicalSize::new(640, 360);
    assert!(is_drag(true, true, Some(size), size));
    // Locked, not focused (a screen change) or a first move: not followed.
    assert!(!is_drag(false, true, Some(size), size));
    assert!(!is_drag(true, false, Some(size), size));
    assert!(!is_drag(true, true, None, size));
    // Resized from the left or top edge: the window moved but was not dragged.
    assert!(!is_drag(
        true,
        true,
        Some(PhysicalSize::new(700, 360)),
        size
    ));
}
