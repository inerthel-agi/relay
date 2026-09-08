use super::{
    MonitorBounds, configured_size, next_position_for_size_change, preserve_or_rescue_position,
    right_anchored_position, saved_position, write_position,
};
use crate::config::AppConfig;
use tauri::PhysicalPosition;

fn primary_1080p() -> MonitorBounds {
    MonitorBounds {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        scale: 1.0,
    }
}

#[test]
fn notification_widget_defaults_remain_compact() {
    assert_eq!(
        (
            crate::config::DEFAULT_NOTIFICATION_WIDGET_WIDTH,
            crate::config::DEFAULT_NOTIFICATION_WIDGET_HEIGHT,
        ),
        (400.0, 104.0)
    );
}

#[test]
fn notification_widget_allows_compact_overlay_heights() {
    const {
        assert!(crate::config::MIN_WIDGET_HEIGHT <= 100.0);
    }
    assert_eq!(crate::config::MIN_WIDGET_HEIGHT, 90.0);
}

#[test]
fn configured_size_uses_notification_dimensions() {
    let config = AppConfig {
        notification_widget_width: 540.0,
        notification_widget_height: 112.0,
        music_widget_width: 980.0,
        music_widget_height: 360.0,
        ..AppConfig::default()
    };
    assert_eq!(configured_size(&config), (540.0, 112.0));
}

#[test]
fn right_anchored_resize_keeps_the_right_edge() {
    let next = right_anchored_position(
        PhysicalPosition::new(1400, 40),
        530.0,
        160.0,
        980.0,
        180.0,
        1.0,
    );
    assert_eq!(next, PhysicalPosition::new(950, 40));
}

#[test]
fn height_resize_keeps_the_top_edge_fixed() {
    let next = right_anchored_position(
        PhysicalPosition::new(1400, 40),
        980.0,
        160.0,
        980.0,
        280.0,
        1.0,
    );
    assert_eq!(next, PhysicalPosition::new(1400, 40));
}

#[test]
fn non_anchored_resize_keeps_exact_top_left() {
    let saved = PhysicalPosition::new(2020, 0);
    let next = next_position_for_size_change(saved, 460.0, 90.0, 540.0, 360.0, 1.0, false);
    assert_eq!(next, saved);
}

#[test]
fn panel_resize_still_right_anchors_when_requested() {
    let next = next_position_for_size_change(
        PhysicalPosition::new(2020, 0),
        460.0,
        90.0,
        540.0,
        360.0,
        1.0,
        true,
    );
    assert_eq!(next, PhysicalPosition::new(1940, 0));
}

#[test]
fn restore_keeps_flush_right_edge_dock() {
    let width = 980.0;
    let height = 180.0;
    let monitor = primary_1080p();
    let x = monitor.width - (width as i32);
    let saved = PhysicalPosition::new(x, 12);
    let restored = preserve_or_rescue_position(saved, width, height, &[monitor], Some(monitor));
    assert_eq!(restored, saved);
}

#[test]
fn restore_does_not_nudge_when_frame_extends_past_work_area_inset() {
    // Simulate docking flush to the true screen right while an 8px work-area
    // inset exists on that edge — old clamp_position_to_area shifted left.
    let monitor = primary_1080p();
    let width = 980.0;
    let height = 180.0;
    let x = monitor.width - (width as i32);
    let saved = PhysicalPosition::new(x, 20);
    let restored = preserve_or_rescue_position(saved, width, height, &[monitor], Some(monitor));
    assert_eq!(restored, saved);
    assert_eq!(restored.x + width as i32, monitor.width);
}

#[test]
fn restore_rescues_only_when_completely_off_every_monitor() {
    let monitor = primary_1080p();
    let rescued = preserve_or_rescue_position(
        PhysicalPosition::new(8000, 4000),
        980.0,
        180.0,
        &[monitor],
        Some(monitor),
    );
    assert_eq!(rescued, PhysicalPosition::new(monitor.width - 980, 0));
}

#[test]
fn write_position_synchronizes_the_shared_widget_dock() {
    let mut config = AppConfig {
        notification_widget_x: Some(100),
        notification_widget_y: Some(20),
        music_widget_x: Some(880),
        music_widget_y: Some(40),
        ..AppConfig::default()
    };

    write_position(&mut config, PhysicalPosition::new(2100, 12));
    assert_eq!(config.notification_widget_x, Some(2100));
    assert_eq!(config.notification_widget_y, Some(12));
    assert_eq!(config.music_widget_x, Some(2100));
    assert_eq!(config.music_widget_y, Some(12));
}

#[test]
fn saved_position_uses_the_shared_widget_dock() {
    let config = AppConfig {
        notification_widget_x: Some(100),
        notification_widget_y: Some(20),
        music_widget_x: Some(880),
        music_widget_y: Some(40),
        ..AppConfig::default()
    };
    assert_eq!(saved_position(&config), Some((100, 20)));
}

#[test]
fn persist_position_keys_round_trip_through_config_store() {
    use crate::config::ConfigStore;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let store = ConfigStore::new(path.clone());
    let config = AppConfig {
        notification_widget_x: Some(2020),
        notification_widget_y: Some(48),
        music_widget_x: Some(1880),
        music_widget_y: Some(64),
        ..AppConfig::default()
    };
    store.save(&config).unwrap();

    let loaded = ConfigStore::new(path.clone()).load().unwrap();
    assert_eq!(loaded.notification_widget_x, Some(2020));
    assert_eq!(loaded.notification_widget_y, Some(48));
    assert_eq!(loaded.music_widget_x, Some(1880));
    assert_eq!(loaded.music_widget_y, Some(64));

    let persisted: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(persisted["notificationWidgetX"], 2020);
    assert_eq!(persisted["notificationWidgetY"], 48);
    assert_eq!(persisted["musicWidgetX"], 1880);
    assert_eq!(persisted["musicWidgetY"], 64);
}
