mod artwork;
mod bot;
mod changelog;
mod channel_cleanup;
mod clock;
mod commands;
mod config;
mod credentials;
mod custom_commands;
mod media_compat;
mod media_library;
mod model;
mod music;
mod music_cleanup;
mod music_i18n;
mod notification_widget;
mod privacy;
mod reaction_protection;
mod reaction_trim;
mod reactions;
mod reactions_messages;
mod server;
mod stage_scheduler;
mod state;
mod updater;
mod widget;
mod youtube;
mod youtube_download;

use std::{
    env,
    path::PathBuf,
    process::Command,
    sync::{Arc, OnceLock, atomic::Ordering},
    time::Duration,
};

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, Theme, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::{
    bot::start_bot,
    changelog::get_changelog_markdown,
    commands::{
        apply_config, approve_pending_media, clear_notification_sound, clear_overlay,
        clear_pending_media, control_audio, download_history_media, get_bootstrap,
        get_media_artwork, get_runtime_status, pick_notification_sound, preview_output_sample,
        refresh_channels, regenerate_secret, reject_pending_media, remove_queued_media,
        replay_media, save_command_settings, save_credentials, save_custom_commands,
        set_interface_preferences, set_media_caption_visibility, set_notification_sound_enabled,
        set_notification_sound_obs_enabled, set_notification_widget_locked,
        set_notification_widget_visible, set_output_geometry, set_skip_shortcut, set_widget_locked,
        skip_media, store_youtube_api_key, test_output, toggle_widget,
    },
    config::{DEFAULT_SKIP_SHORTCUT, migrate_legacy_config},
    model::{MediaKind, RelayEvent, ServerStatus},
    music_cleanup::{confirm_music_cleanup, preview_music_cleanup},
    notification_widget::restore as restore_notification_widget,
    server::start_server,
    state::{AppCore, MediaDeliveryRequest},
    updater::{check_for_updates, download_and_install_update, get_app_version},
    widget::restore as restore_widget,
};

const TRAY_PANEL_LABEL: &str = "tray-panel";
const TRAY_PANEL_WIDTH: f64 = 336.0;
const TRAY_PANEL_HEIGHT: f64 = 486.0;
const TRAY_PANEL_MARGIN: i32 = 10;
const MAIN_WINDOW_TITLE: &str = "Relay";
const STARTUP_ARGUMENT: &str = "--startup";
const WINDOWS_RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if !is_startup_launch(&args) {
                show_main_window(app);
            }
        }))
        .setup(|app| {
            load_dotenv();
            let startup_launch = is_startup_launch(&env::args().collect::<Vec<_>>());
            let config_directory = app.path().app_config_dir()?;
            migrate_legacy_config(&config_directory)?;
            let config_path = config_directory.join("config.json");
            let core = AppCore::load(config_path)?;
            app.manage(core.clone());

            register_skip_shortcut(app, core.clone())?;
            build_tray_panel(app)?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().expect("application icon").clone())
                .tooltip("Relay")
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        position,
                        button: MouseButton::Left | MouseButton::Right,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_tray_panel(tray.app_handle(), position);
                    }
                })
                .build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                window.set_icon(app.default_window_icon().expect("application icon").clone())?;
                window.set_title(MAIN_WINDOW_TITLE)?;
                hide_main_titlebar_identity(&window)?;
                schedule_main_titlebar_identity(window.clone());
                let window_to_hide = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_to_hide.hide();
                    }
                });
                if !startup_launch {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let server_started = match start_server(core.clone()).await {
                    Ok(()) => true,
                    Err(error) => {
                        core.server_status.write().await.error = Some(error.to_string());
                        false
                    }
                };
                if server_started {
                    let _ = restore_widget(&app_handle, core.clone()).await;
                    let _ = restore_notification_widget(&app_handle, core.clone()).await;
                    if let Err(error) = commands::reactions::restore_audio(&app_handle, &core).await
                    {
                        core.server_status.write().await.error = Some(error);
                    }
                    let (media_delivery, requests) = tokio::sync::mpsc::unbounded_channel();
                    core.set_media_delivery(media_delivery).await;
                    tauri::async_runtime::spawn(deliver_media_to_local_widget(
                        app_handle.clone(),
                        core.clone(),
                        requests,
                    ));
                    let music_events = core.relay_tx.subscribe();
                    tauri::async_runtime::spawn(manage_music_media_widget(
                        app_handle.clone(),
                        core.clone(),
                        music_events,
                    ));
                }
                if let Err(error) = start_bot(core.clone()).await {
                    core.bot_status.write().await.error = Some(error.to_string());
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::music_queue::get_music_queue,
            commands::music_queue::move_music_queue,
            commands::music_queue::remove_music_queue,
            commands::music_queue::save_music_queue_settings,
            commands::media_library::get_media_library,
            commands::media_library::import_library_media,
            commands::media_library::save_history_to_library,
            commands::media_library::rename_library_media,
            commands::media_library::delete_library_media,
            commands::media_library::play_library_media,
            commands::media_library::preview_library_asset,
            commands::message_pin::get_message_status,
            commands::message_pin::pin_message,
            commands::message_pin::unpin_message,
            commands::reactions::get_reactions,
            commands::reactions::get_reaction_access_options,
            commands::reactions::save_reactions,
            commands::reactions::import_reaction_sound,
            commands::reactions::preview_reaction_trim,
            commands::reactions::finish_reaction_trim,
            commands::reactions::cancel_reaction_trim,
            commands::reactions::trigger_reaction,
            commands::reactions::stop_reaction,
            commands::reactions::preview_reaction_sound,
            preview_music_cleanup,
            confirm_music_cleanup,
            get_bootstrap,
            get_runtime_status,
            remove_queued_media,
            preview_output_sample,
            refresh_channels,
            set_interface_preferences,
            set_output_geometry,
            save_credentials,
            store_youtube_api_key,
            apply_config,
            set_media_caption_visibility,
            save_command_settings,
            save_custom_commands,
            clear_overlay,
            replay_media,
            download_history_media,
            set_skip_shortcut,
            skip_media,
            test_output,
            control_audio,
            get_media_artwork,
            approve_pending_media,
            reject_pending_media,
            clear_pending_media,
            regenerate_secret,
            toggle_widget,
            set_widget_locked,
            set_notification_widget_visible,
            set_notification_widget_locked,
            set_notification_sound_enabled,
            set_notification_sound_obs_enabled,
            pick_notification_sound,
            clear_notification_sound,
            tray_open_control_panel,
            tray_toggle_media_widget,
            tray_toggle_media_widget_lock,
            tray_toggle_notification_widget,
            tray_toggle_notification_widget_lock,
            get_start_with_windows,
            set_start_with_windows,
            tray_quit,
            set_window_theme,
            open_help_link,
            get_app_version,
            get_changelog_markdown,
            check_for_updates,
            download_and_install_update,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Relay");
}

async fn deliver_media_to_local_widget(
    app: AppHandle,
    core: Arc<AppCore>,
    mut requests: tokio::sync::mpsc::UnboundedReceiver<MediaDeliveryRequest>,
) {
    while let Some(request) = requests.recv().await {
        let (should_wake, widget_connected) = {
            let widget_visible = core.config.read().await.widget_visible;
            let status = core.server_status.read().await;
            (
                should_wake_media_widget(&status, request.kind, widget_visible),
                media_widget_connected(&status),
            )
        };
        let has_ephemeral_ownership = core.widget_ephemeral_wake.load(Ordering::Relaxed);
        if should_show_media_widget(should_wake, has_ephemeral_ownership, widget_connected)
            && widget::show(&app, core.clone(), false).await.is_ok()
        {
            for _ in 0..40 {
                let connected = {
                    let status = core.server_status.read().await;
                    media_widget_connected(&status)
                };
                if connected {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
        let _ = request.ready.send(());
    }
}

async fn manage_music_media_widget(
    app: AppHandle,
    core: Arc<AppCore>,
    mut events: tokio::sync::broadcast::Receiver<RelayEvent>,
) {
    loop {
        match events.recv().await {
            Ok(RelayEvent::MusicPlay(_)) => {
                let visible = widget::state(&app, &core).await.visible;
                if should_wake_music_widget(visible) {
                    if widget::wake_ephemeral(&app, core.clone()).await.is_err() {
                        continue;
                    }
                    for _ in 0..40 {
                        let connected = {
                            let status = core.server_status.read().await;
                            media_widget_connected(&status)
                        };
                        if connected {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                }
            }
            Ok(RelayEvent::MusicIdle) | Ok(RelayEvent::Clear) => {
                dismiss_ephemeral_media_widget_with_retry(&app, core.clone()).await;
            }
            Ok(_) => {}
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }
}

async fn dismiss_ephemeral_media_widget_with_retry(app: &AppHandle, core: Arc<AppCore>) {
    const ATTEMPTS: usize = 3;
    for attempt in 0..ATTEMPTS {
        if widget::dismiss_ephemeral_if_idle(app, core.clone())
            .await
            .is_ok()
        {
            return;
        }
        if attempt + 1 < ATTEMPTS {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

fn should_wake_music_widget(widget_visible: bool) -> bool {
    !widget_visible
}

fn should_show_media_widget(
    should_wake: bool,
    has_ephemeral_ownership: bool,
    widget_connected: bool,
) -> bool {
    should_wake || (has_ephemeral_ownership && widget_connected)
}

fn should_wake_media_widget(status: &ServerStatus, kind: MediaKind, widget_visible: bool) -> bool {
    if widget_visible && media_widget_connected(status) {
        return false;
    }
    match kind {
        MediaKind::Image | MediaKind::Gif | MediaKind::Video => {
            status.outputs.visual.obs_clients == 0
        }
        MediaKind::Audio => status.outputs.audio.obs_clients == 0,
    }
}

fn media_widget_connected(status: &ServerStatus) -> bool {
    status.outputs.visual.widget_clients > 0 || status.outputs.audio.widget_clients > 0
}

fn register_skip_shortcut(app: &mut tauri::App, core: Arc<AppCore>) -> tauri::Result<()> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

    let configured = core
        .config
        .try_read()
        .ok()
        .map(|config| config.skip_shortcut.clone())
        .unwrap_or_default();
    let shortcut = configured
        .parse::<Shortcut>()
        .or_else(|_| DEFAULT_SKIP_SHORTCUT.parse::<Shortcut>())
        .expect("Relay default shortcut must be valid");
    app.handle()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())?;
    let _ = app
        .global_shortcut()
        .on_shortcut(shortcut, move |_app, _pressed_shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                let core = core.clone();
                tauri::async_runtime::spawn(async move {
                    core.skip_playback().await;
                });
            }
        });
    Ok(())
}

fn build_tray_panel(app: &mut tauri::App) -> tauri::Result<()> {
    let window =
        WebviewWindowBuilder::new(app, TRAY_PANEL_LABEL, WebviewUrl::App("tray.html".into()))
            .title("Relay - Tray")
            .inner_size(TRAY_PANEL_WIDTH, TRAY_PANEL_HEIGHT)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .minimizable(false)
            .maximizable(false)
            .visible(false)
            .focused(false)
            .build()?;

    let window_to_hide = window.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::Focused(false) => {
            let _ = window_to_hide.hide();
        }
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = window_to_hide.hide();
        }
        _ => {}
    });
    Ok(())
}

fn toggle_tray_panel(app: &AppHandle, click: PhysicalPosition<f64>) {
    let Some(window) = app.get_webview_window(TRAY_PANEL_LABEL) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    if let Ok(position) = tray_panel_position(app, click) {
        let _ = window.set_position(position);
    }
    let _ = window.eval("window.refreshTray?.()");
    let _ = window.show();
    let _ = window.set_focus();
}

fn tray_panel_position(
    app: &AppHandle,
    click: PhysicalPosition<f64>,
) -> tauri::Result<PhysicalPosition<i32>> {
    let monitors = app.available_monitors()?;
    let click_x = click.x.round() as i32;
    let click_y = click.y.round() as i32;
    let monitor = monitors
        .iter()
        .find(|monitor| {
            let area = monitor.work_area();
            click_x >= area.position.x
                && click_y >= area.position.y
                && click_x < area.position.x + area.size.width as i32
                && click_y < area.position.y + area.size.height as i32
        })
        .cloned()
        .or(app.primary_monitor()?)
        .or_else(|| monitors.into_iter().next());
    let Some(monitor) = monitor else {
        return Ok(PhysicalPosition::new(click_x, click_y));
    };

    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    let width = (TRAY_PANEL_WIDTH * scale).round() as i32;
    let height = (TRAY_PANEL_HEIGHT * scale).round() as i32;
    let min_x = area.position.x + TRAY_PANEL_MARGIN;
    let max_x = (area.position.x + area.size.width as i32 - width - TRAY_PANEL_MARGIN).max(min_x);
    let min_y = area.position.y + TRAY_PANEL_MARGIN;
    let max_y = (area.position.y + area.size.height as i32 - height - TRAY_PANEL_MARGIN).max(min_y);
    let x = (click_x - width / 2).clamp(min_x, max_x);
    let above = click_y - height - TRAY_PANEL_MARGIN;
    let y = if above >= min_y {
        above.min(max_y)
    } else {
        (click_y + TRAY_PANEL_MARGIN).clamp(min_y, max_y)
    };
    Ok(PhysicalPosition::new(x, y))
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn is_startup_launch(args: &[String]) -> bool {
    args.iter().any(|argument| argument == STARTUP_ARGUMENT)
}

fn registry_status(args: &[&str]) -> Result<bool, String> {
    let mut command = Command::new("reg.exe");
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
        .args(args)
        .output()
        .map(|output| output.status.success())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_start_with_windows() -> Result<bool, String> {
    registry_status(&["query", WINDOWS_RUN_KEY, "/v", "Relay"])
}

#[tauri::command]
fn set_start_with_windows(enabled: bool) -> Result<bool, String> {
    let updated = if enabled {
        let executable = env::current_exe().map_err(|error| error.to_string())?;
        let value = format!("\"{}\" {STARTUP_ARGUMENT}", executable.display());
        registry_status(&[
            "add",
            WINDOWS_RUN_KEY,
            "/v",
            "Relay",
            "/t",
            "REG_SZ",
            "/d",
            &value,
            "/f",
        ])?
    } else if get_start_with_windows()? {
        registry_status(&["delete", WINDOWS_RUN_KEY, "/v", "Relay", "/f"])?
    } else {
        return Ok(false);
    };
    if updated {
        Ok(enabled)
    } else {
        Err("Windows refused to update the startup setting.".into())
    }
}

#[tauri::command]
fn set_window_theme(
    window: WebviewWindow,
    theme: String,
    caption: [u8; 3],
    text: [u8; 3],
    border: [u8; 3],
) -> Result<(), String> {
    let dark = match theme.as_str() {
        "dark" => true,
        "light" => false,
        _ => return Err("Unsupported window theme.".into()),
    };
    window
        .set_theme(Some(if dark { Theme::Dark } else { Theme::Light }))
        .map_err(|error| error.to_string())?;
    apply_titlebar_theme(&window, caption, text, border).map_err(|error| error.to_string())?;
    hide_main_titlebar_identity(&window).map_err(|error| error.to_string())?;
    schedule_main_titlebar_identity(window);
    Ok(())
}

#[tauri::command]
fn open_help_link(link: String) -> Result<(), String> {
    let url = resolve_external_link(&link)?;
    Command::new("rundll32.exe")
        .arg("url.dll,FileProtocolHandler")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn resolve_external_link(link: &str) -> Result<String, String> {
    let url = match link {
        "discord" => "https://discord.com/developers/applications",
        "obs" => "https://obsproject.com/kb/browser-source",
        "github" => "https://github.com/inerthel-agi",
        "relay-releases" => "https://github.com/inerthel-agi/relay/releases/latest",
        "relay-changelog" => "https://github.com/inerthel-agi/relay/blob/main/CHANGELOG.md",
        "google-cloud" => "https://console.cloud.google.com/",
        "youtube-api-library" => {
            "https://console.cloud.google.com/apis/library/youtube.googleapis.com"
        }
        "google-credentials" => "https://console.cloud.google.com/apis/credentials",
        "privacy-global" => {
            "https://unctad.org/page/data-protection-and-privacy-legislation-worldwide"
        }
        _ if is_discord_invite_url(link) => return Ok(link.to_owned()),
        _ => return Err("Unsupported external link.".into()),
    };
    Ok(url.to_owned())
}

fn is_discord_invite_url(link: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(link) else {
        return false;
    };
    if url.scheme() != "https"
        || url.host_str() != Some("discord.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/oauth2/authorize"
        || url.fragment().is_some()
    {
        return false;
    }

    let mut client_id = false;
    let mut permissions = false;
    let mut scope = false;
    let mut query_count = 0;
    for (key, value) in url.query_pairs() {
        query_count += 1;
        match key.as_ref() {
            "client_id" => {
                client_id = (17..=20).contains(&value.len())
                    && value.chars().all(|character| character.is_ascii_digit());
            }
            "permissions" => permissions = value == "268510208",
            "scope" => scope = value == "bot applications.commands",
            _ => return false,
        }
    }
    query_count == 3 && client_id && permissions && scope
}

#[cfg(test)]
mod tray_page_tests {
    use super::tray_panel_page;

    #[test]
    fn tray_opens_only_known_panel_pages() {
        assert_eq!(tray_panel_page("moderation"), Some("moderation"));
        assert_eq!(tray_panel_page("discord"), Some("discord"));
        assert_eq!(tray_panel_page("about"), None);
        assert_eq!(tray_panel_page("../moderation"), None);
    }
}

#[cfg(test)]
mod media_widget_wake_tests {
    use super::{should_show_media_widget, should_wake_music_widget};

    #[test]
    fn music_wakes_only_a_hidden_media_widget() {
        assert!(should_wake_music_widget(false));
        assert!(!should_wake_music_widget(true));
    }

    #[test]
    fn normal_media_fallback_wakes_the_widget() {
        assert!(should_show_media_widget(true, false, false));
    }

    #[test]
    fn connected_ephemeral_widget_becomes_persistent_even_with_obs() {
        assert!(should_show_media_widget(false, true, true));
    }

    #[test]
    fn connected_non_ephemeral_widget_is_not_shown_redundantly() {
        assert!(!should_show_media_widget(false, false, true));
    }
}

#[cfg(test)]
mod external_link_tests {
    use super::{
        MAIN_WINDOW_TITLE, is_discord_invite_url, is_startup_launch, resolve_external_link,
    };

    const INVITE: &str = "https://discord.com/oauth2/authorize?client_id=123456789012345678&permissions=268510208&scope=bot%20applications.commands";

    #[test]
    fn accepts_the_generated_discord_invite_url() {
        assert_eq!(resolve_external_link(INVITE), Ok(INVITE.to_owned()));
    }

    #[test]
    fn opens_releases_on_the_current_github_account() {
        assert_eq!(
            resolve_external_link("relay-releases"),
            Ok("https://github.com/inerthel-agi/relay/releases/latest".to_owned())
        );
    }

    #[test]
    fn opens_the_bundled_changelog_on_github() {
        assert_eq!(
            resolve_external_link("relay-changelog"),
            Ok("https://github.com/inerthel-agi/relay/blob/main/CHANGELOG.md".to_owned())
        );
    }

    #[test]
    fn opens_the_creator_profile_on_github() {
        assert_eq!(
            resolve_external_link("github"),
            Ok("https://github.com/inerthel-agi".to_owned())
        );
    }

    #[test]
    fn rejects_untrusted_or_modified_discord_invite_urls() {
        for link in [
            "https://discord.com.evil.example/oauth2/authorize?client_id=123456789012345678&permissions=268510208&scope=bot%20applications.commands",
            "https://discord.com/oauth2/authorize?client_id=123456789012345678&permissions=8&scope=bot%20applications.commands",
            "https://discord.com/oauth2/authorize?client_id=123456789012345678&permissions=268510208&scope=bot%20applications.commands&redirect_uri=https://example.com",
        ] {
            assert!(!is_discord_invite_url(link));
        }
    }

    #[test]
    fn detects_the_startup_launch() {
        assert!(is_startup_launch(&["Relay.exe".into(), "--startup".into()]));
    }

    #[test]
    fn main_window_keeps_the_relay_title_and_default_icon() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(config["app"]["windows"][0]["title"], MAIN_WINDOW_TITLE);
        let source = include_str!("lib.rs");
        assert!(source.contains("window.set_icon(app.default_window_icon()"));
        assert!(source.contains("hide_main_titlebar_identity(&window)"));
        assert!(source.contains("schedule_main_titlebar_identity(window.clone())"));
        assert!(source.contains("ExtractIconExW"));
        assert!(source.contains("SetClassLongPtrW(hwnd, GCLP_HICON"));
        assert!(source.contains("SetClassLongPtrW(hwnd, GCLP_HICONSM"));
        assert!(source.contains("for icon_type in [ICON_SMALL, ICON_SMALL2]"));
        let legacy_clear = ["[ICON_SMALL, ICON_BIG, ", "ICON_SMALL2]"].concat();
        assert!(!source.contains(&legacy_clear));
        assert!(source.contains("(DWMWA_TEXT_COLOR, colorref(caption))"));
        assert!(include_str!("../Cargo.toml").contains("\"Win32_UI_Shell\""));
        assert!(!include_bytes!("../icons/icon.ico").is_empty());
    }
}

#[cfg(target_os = "windows")]
fn hide_main_titlebar_identity(window: &WebviewWindow) -> windows::core::Result<()> {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        GCLP_HICON, GCLP_HICONSM, GWL_EXSTYLE, GetWindowLongPtrW, ICON_BIG, ICON_SMALL,
        ICON_SMALL2, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
        SendMessageW, SetClassLongPtrW, SetWindowLongPtrW, SetWindowPos, WM_GETICON, WM_SETICON,
        WS_EX_DLGMODALFRAME,
    };

    let hwnd = main_window_handle(window)?;
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_DLGMODALFRAME.0 as isize);
        let big_icon = SendMessageW(
            hwnd,
            WM_GETICON,
            Some(WPARAM(ICON_BIG as usize)),
            Some(LPARAM(0)),
        )
        .0;
        let small_icon = SendMessageW(
            hwnd,
            WM_GETICON,
            Some(WPARAM(ICON_SMALL as usize)),
            Some(LPARAM(0)),
        )
        .0;
        if big_icon == 0 {
            let taskbar_icon = if small_icon != 0 {
                small_icon
            } else {
                executable_large_icon().map_or(0, |icon| icon.0 as isize)
            };
            if taskbar_icon != 0 {
                SetClassLongPtrW(hwnd, GCLP_HICON, taskbar_icon);
                SendMessageW(
                    hwnd,
                    WM_SETICON,
                    Some(WPARAM(ICON_BIG as usize)),
                    Some(LPARAM(taskbar_icon)),
                );
            }
        }
        if let Some(icon) = transparent_titlebar_icon() {
            SetClassLongPtrW(hwnd, GCLP_HICONSM, icon.0 as isize);
        }
        for icon_type in [ICON_SMALL, ICON_SMALL2] {
            SendMessageW(
                hwnd,
                WM_SETICON,
                Some(WPARAM(icon_type as usize)),
                Some(LPARAM(0)),
            );
        }
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
        )?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn transparent_titlebar_icon() -> Option<windows::Win32::UI::WindowsAndMessaging::HICON> {
    use windows::Win32::UI::WindowsAndMessaging::{CreateIcon, HICON};

    static ICON: OnceLock<isize> = OnceLock::new();
    let handle = *ICON.get_or_init(|| {
        let and_mask = [0xff_u8; 32];
        let xor_mask = [0_u8; 32];
        unsafe { CreateIcon(None, 16, 16, 1, 1, and_mask.as_ptr(), xor_mask.as_ptr()) }
            .map_or(0, |icon| icon.0 as isize)
    });
    (handle != 0).then_some(HICON(handle as *mut _))
}

#[cfg(target_os = "windows")]
fn schedule_main_titlebar_identity(window: WebviewWindow) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let _ = hide_main_titlebar_identity(&window);
    });
}

#[cfg(target_os = "windows")]
fn executable_large_icon() -> Option<windows::Win32::UI::WindowsAndMessaging::HICON> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::UI::{Shell::ExtractIconExW, WindowsAndMessaging::HICON},
        core::PCWSTR,
    };

    let executable = env::current_exe().ok()?;
    let path = executable
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut icon = HICON(std::ptr::null_mut());
    let extracted = unsafe { ExtractIconExW(PCWSTR(path.as_ptr()), 0, Some(&mut icon), None, 1) };
    (extracted > 0 && !icon.is_invalid()).then_some(icon)
}

#[cfg(target_os = "windows")]
fn main_window_handle(
    window: &WebviewWindow,
) -> windows::core::Result<windows::Win32::Foundation::HWND> {
    window.hwnd().map_err(|error| {
        windows::core::Error::new(
            windows::core::HRESULT(0x80004005_u32 as i32),
            error.to_string(),
        )
    })
}

#[cfg(target_os = "windows")]
fn apply_titlebar_theme(
    window: &WebviewWindow,
    caption: [u8; 3],
    _text: [u8; 3],
    border: [u8; 3],
) -> windows::core::Result<()> {
    use std::{ffi::c_void, mem::size_of};
    use windows::Win32::Graphics::Dwm::{
        DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DwmSetWindowAttribute,
    };

    let hwnd = main_window_handle(window)?;
    for (attribute, color) in [
        (DWMWA_CAPTION_COLOR, colorref(caption)),
        (DWMWA_TEXT_COLOR, colorref(caption)),
        (DWMWA_BORDER_COLOR, colorref(border)),
    ] {
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                attribute,
                (&color as *const u32).cast::<c_void>(),
                size_of::<u32>() as u32,
            )?;
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
const fn colorref([red, green, blue]: [u8; 3]) -> u32 {
    red as u32 | ((green as u32) << 8) | ((blue as u32) << 16)
}

#[tauri::command]
fn tray_open_control_panel(app: AppHandle, window: WebviewWindow, page: Option<String>) {
    let _ = window.hide();
    show_main_window(&app);
    if let Some(page) = page.as_deref().and_then(tray_panel_page) {
        let _ = app.emit_to("main", "relay-open-page", page);
    }
}

/// Panel pages the tray may open directly; anything else just shows the panel.
fn tray_panel_page(page: &str) -> Option<&'static str> {
    ["overview", "discord", "moderation", "overlay"]
        .into_iter()
        .find(|allowed| *allowed == page)
}

#[tauri::command]
async fn tray_toggle_media_widget(
    app: AppHandle,
    core: tauri::State<'_, Arc<AppCore>>,
) -> Result<widget::WidgetState, String> {
    widget::toggle(&app, core.inner().clone())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn tray_toggle_media_widget_lock(
    app: AppHandle,
    core: tauri::State<'_, Arc<AppCore>>,
) -> Result<widget::WidgetState, String> {
    widget::toggle_lock(&app, core.inner().clone())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn tray_toggle_notification_widget(
    app: AppHandle,
    core: tauri::State<'_, Arc<AppCore>>,
) -> Result<notification_widget::NotificationWidgetState, String> {
    let current = notification_widget::state(&app, &core).await;
    notification_widget::set_visible(&app, core.inner().clone(), !current.visible)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn tray_toggle_notification_widget_lock(
    app: AppHandle,
    core: tauri::State<'_, Arc<AppCore>>,
) -> Result<notification_widget::NotificationWidgetState, String> {
    let current = notification_widget::state(&app, &core).await;
    notification_widget::set_locked(&app, core.inner().clone(), !current.locked)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn tray_quit(app: AppHandle) {
    app.exit(0);
}

fn load_dotenv() {
    let mut candidates = Vec::<PathBuf>::new();
    if let Ok(executable) = env::current_exe()
        && let Some(parent) = executable.parent()
    {
        candidates.push(parent.join(".env"));
    }
    if let Ok(current_directory) = env::current_dir() {
        candidates.push(current_directory.join(".env"));
    }
    for candidate in candidates {
        if candidate.is_file() {
            let _ = dotenvy::from_path(candidate);
            break;
        }
    }
}
