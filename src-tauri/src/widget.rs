use std::sync::{Arc, Mutex, atomic::Ordering};

use anyhow::{Context, Result};
use serde::Serialize;
use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowSizeConstraints,
};

use crate::{
    config::{MIN_WIDGET_HEIGHT, MIN_WIDGET_WIDTH},
    credentials::load_or_create_relay_secret,
    state::AppCore,
};

const WIDGET_LABEL: &str = "widget";
const ASPECT_RATIO: f64 = 16.0 / 9.0;
/// Host used by Windows widgets so YouTube IFrame embeds receive an accepted Referer.
/// YouTube error 150 rejects `http://127.0.0.1` even on loopback; `http://localhost` works.
pub(crate) const YOUTUBE_EMBED_HOST: &str = "localhost";
static WIDGET_TRANSITION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn establish_ephemeral_ownership(marker: &std::sync::atomic::AtomicBool, persistent: bool) -> bool {
    let owns_cleanup = !persistent;
    marker.store(owns_cleanup, Ordering::Relaxed);
    owns_cleanup
}

fn clear_ephemeral_ownership(marker: &std::sync::atomic::AtomicBool) {
    marker.store(false, Ordering::Relaxed);
}

fn complete_persistent_takeover(marker: &std::sync::atomic::AtomicBool) {
    clear_ephemeral_ownership(marker);
}

fn claim_ephemeral_ownership(marker: &std::sync::atomic::AtomicBool) -> bool {
    marker.swap(false, Ordering::Relaxed)
}

fn restore_ephemeral_ownership(marker: &std::sync::atomic::AtomicBool) {
    marker.store(true, Ordering::Relaxed);
}

fn rollback_ephemeral_ownership(
    marker: &std::sync::atomic::AtomicBool,
    owns_cleanup: bool,
    window_may_be_visible: bool,
) {
    if owns_cleanup && !window_may_be_visible {
        clear_ephemeral_ownership(marker);
    }
}

pub(crate) fn youtube_embed_host() -> &'static str {
    YOUTUBE_EMBED_HOST
}

/// Recommended OBS Visual Browser Source (medias + stickers + notifications + YouTube).
/// Must use `localhost` so the embedded `/youtube` layer accepts the Referer.
pub(crate) fn obs_visual_url(port: u16) -> String {
    format!("http://{}:{port}/obs/visual", youtube_embed_host())
}

/// Relay Audio has no YouTube layer, so 127.0.0.1 is fine.
pub(crate) fn obs_audio_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/obs/audio")
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetState {
    pub visible: bool,
    pub locked: bool,
}

pub async fn state(app: &AppHandle, core: &Arc<AppCore>) -> WidgetState {
    let config = core.config.read().await;
    let visible = app
        .get_webview_window(WIDGET_LABEL)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(config.widget_visible);
    WidgetState {
        visible,
        locked: config.widget_locked,
    }
}

pub async fn restore(app: &AppHandle, core: Arc<AppCore>) -> Result<()> {
    if core.config.read().await.widget_visible {
        let window = ensure_window(app, core.clone()).await?;
        window.show()?;
    }
    Ok(())
}

pub async fn toggle(app: &AppHandle, core: Arc<AppCore>) -> Result<WidgetState> {
    let _transition = WIDGET_TRANSITION.lock().await;
    let is_visible = app
        .get_webview_window(WIDGET_LABEL)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false);

    if is_visible {
        if let Some(window) = app.get_webview_window(WIDGET_LABEL) {
            flush_live_position(&window, &core).await?;
            update_visibility(&core, false).await?;
            window.eval("window.setWidgetVisible?.(false);")?;
            window.hide()?;
        } else {
            update_visibility(&core, false).await?;
        }
        wait_for_widget_disconnect(&core).await;
        complete_persistent_takeover(&core.widget_ephemeral_wake);
    } else {
        return show_unlocked(app, core, true).await;
    }
    Ok(state(app, &core).await)
}

pub async fn show(app: &AppHandle, core: Arc<AppCore>, focus: bool) -> Result<WidgetState> {
    let _transition = WIDGET_TRANSITION.lock().await;
    show_unlocked(app, core, focus).await
}

/// Show the native media widget without persisting its visibility preference.
pub async fn wake_ephemeral(app: &AppHandle, core: Arc<AppCore>) -> Result<WidgetState> {
    let _transition = WIDGET_TRANSITION.lock().await;
    let persistently_visible = core.config.read().await.widget_visible;
    let owns_cleanup =
        establish_ephemeral_ownership(&core.widget_ephemeral_wake, persistently_visible);
    let window = match ensure_window(app, core.clone()).await {
        Ok(window) => window,
        Err(error) => {
            rollback_ephemeral_ownership(&core.widget_ephemeral_wake, owns_cleanup, false);
            return Err(error);
        }
    };
    if let Err(error) = window.show() {
        let window_may_be_visible = window.hide().is_err() || window.is_visible().unwrap_or(true);
        rollback_ephemeral_ownership(
            &core.widget_ephemeral_wake,
            owns_cleanup,
            window_may_be_visible,
        );
        return Err(error.into());
    }
    if let Err(error) = window.eval("window.setWidgetVisible?.(true);") {
        let window_may_be_visible = window.hide().is_err() || window.is_visible().unwrap_or(true);
        rollback_ephemeral_ownership(
            &core.widget_ephemeral_wake,
            owns_cleanup,
            window_may_be_visible,
        );
        return Err(error.into());
    }
    Ok(state(app, &core).await)
}

/// Hide only a media widget that music playback woke ephemerally.
pub async fn dismiss_ephemeral_if_idle(
    app: &AppHandle,
    core: Arc<AppCore>,
) -> Result<Option<WidgetState>> {
    let _transition = WIDGET_TRANSITION.lock().await;
    if !claim_ephemeral_ownership(&core.widget_ephemeral_wake) {
        return Ok(None);
    }
    if let Some(window) = app.get_webview_window(WIDGET_LABEL) {
        if let Err(error) = flush_live_position(&window, &core).await {
            restore_ephemeral_ownership(&core.widget_ephemeral_wake);
            return Err(error);
        }
        let visibility_result = window.eval("window.setWidgetVisible?.(false);");
        let hide_result = window.hide();
        if visibility_result.is_err() || hide_result.is_err() {
            restore_ephemeral_ownership(&core.widget_ephemeral_wake);
        }
        visibility_result?;
        hide_result?;
    }
    Ok(Some(state(app, &core).await))
}

async fn show_unlocked(app: &AppHandle, core: Arc<AppCore>, focus: bool) -> Result<WidgetState> {
    let window = ensure_window(app, core.clone()).await?;
    window.show()?;
    window.eval("window.setWidgetVisible?.(true);")?;
    if focus && !core.config.read().await.widget_locked {
        let _ = window.set_focus();
    }
    update_visibility(&core, true).await?;
    complete_persistent_takeover(&core.widget_ephemeral_wake);
    Ok(state(app, &core).await)
}

async fn wait_for_widget_disconnect(core: &Arc<AppCore>) {
    for _ in 0..40 {
        let status = core.server_status.read().await;
        if status.outputs.visual.widget_clients == 0 && status.outputs.audio.widget_clients == 0 {
            return;
        }
        drop(status);
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}

pub async fn set_locked(app: &AppHandle, core: Arc<AppCore>, locked: bool) -> Result<WidgetState> {
    if locked && let Some(window) = app.get_webview_window(WIDGET_LABEL) {
        flush_live_position(&window, &core).await?;
    }
    core.update_config(|config| config.widget_locked = locked)
        .await?;
    if let Some(window) = app.get_webview_window(WIDGET_LABEL) {
        apply_lock(&window, locked)?;
    }
    Ok(state(app, &core).await)
}

pub async fn toggle_lock(app: &AppHandle, core: Arc<AppCore>) -> Result<WidgetState> {
    let locked = !core.config.read().await.widget_locked;
    set_locked(app, core, locked).await
}

pub fn clamp_requested_size(
    app: &AppHandle,
    width: f64,
    height: f64,
    keep_ratio: bool,
) -> Result<(f64, f64)> {
    let monitor = app
        .get_webview_window(WIDGET_LABEL)
        .and_then(|window| window.current_monitor().ok().flatten())
        .or(app.primary_monitor()?)
        .or_else(|| app.available_monitors().ok()?.into_iter().next())
        .context("no display is available")?;
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    Ok(clamp_logical_size(
        width,
        height,
        area.size.width as f64 / scale,
        area.size.height as f64 / scale,
        keep_ratio,
    ))
}

pub fn apply_configured_size(app: &AppHandle, width: f64, height: f64) -> Result<()> {
    if let Some(window) = app.get_webview_window(WIDGET_LABEL) {
        window.set_size(LogicalSize::new(width, height))?;
    }
    Ok(())
}

pub async fn refresh(app: &AppHandle, core: &Arc<AppCore>) -> Result<()> {
    navigate_open_window(app, WIDGET_LABEL, widget_url(core).await?)
}

/// Points an open widget window at its current URL (after a port or secret change).
pub(crate) fn navigate_open_window(app: &AppHandle, label: &str, url: String) -> Result<()> {
    if let Some(window) = app.get_webview_window(label) {
        let url: tauri::Url = url.parse()?;
        if needs_navigation(&window, &url) {
            window.navigate(url)?;
        }
    }
    Ok(())
}

/// Reloading interrupts the media playing in the window, so only navigate
/// when the connection itself changed (port or secret). Language and lock
/// changes are already applied live through the WebSocket and apply_lock.
pub fn needs_navigation(window: &WebviewWindow, target: &tauri::Url) -> bool {
    let Ok(current) = window.url() else {
        return true;
    };
    current.origin() != target.origin()
        || query_param(&current, "secret") != query_param(target, "secret")
}

fn query_param(url: &tauri::Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

async fn ensure_window(app: &AppHandle, core: Arc<AppCore>) -> Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(WIDGET_LABEL) {
        return Ok(window);
    }

    let config = core.config.read().await.clone();
    let url = widget_url(&core).await?;
    let (position, max_width, max_height) = resolved_geometry(
        app,
        config.widget_x.zip(config.widget_y),
        config.widget_width,
        config.widget_height,
    )?;
    let (width, height) = clamp_logical_size(
        config.widget_width,
        config.widget_height,
        max_width,
        max_height,
        config.widget_keep_aspect_ratio,
    );
    let window = WebviewWindowBuilder::new(app, WIDGET_LABEL, WebviewUrl::External(url.parse()?))
        .title("Relay - Widget")
        .inner_size(width, height)
        .min_inner_size(MIN_WIDGET_WIDTH, MIN_WIDGET_HEIGHT)
        .max_inner_size(max_width, max_height)
        .resizable(!config.widget_locked)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .minimizable(false)
        .maximizable(false)
        .focused(false)
        .visible(false)
        .build()?;

    window.set_position(position)?;
    apply_lock(&window, config.widget_locked)?;
    apply_monitor_constraints(&window)?;
    watch_geometry(&window, core.clone());
    if width != config.widget_width || height != config.widget_height {
        core.update_config(|config| {
            config.widget_width = width;
            config.widget_height = height;
        })
        .await?;
    }
    Ok(window)
}

/// Locked widgets ignore the mouse and cannot be resized; shared by both widgets.
pub(crate) fn apply_lock(window: &WebviewWindow, locked: bool) -> Result<()> {
    window.set_resizable(!locked)?;
    window.set_focusable(!locked)?;
    window.set_ignore_cursor_events(locked)?;
    window.eval(format!(
        "window.setWidgetLocked?.({})",
        if locked { "true" } else { "false" }
    ))?;
    Ok(())
}

fn watch_geometry(window: &WebviewWindow, core: Arc<AppCore>) {
    let window = window.clone();
    let last_size = Arc::new(Mutex::new(window.inner_size().unwrap_or_default()));
    window.clone().on_window_event(move |event| match event {
        tauri::WindowEvent::Moved(position) => {
            let _ = apply_monitor_constraints(&window);
            persist_position(core.clone(), *position);
        }
        tauri::WindowEvent::Resized(size) => {
            let size = *size;
            let previous = {
                let mut last = last_size.lock().expect("widget size lock poisoned");
                let previous = *last;
                *last = size;
                previous
            };
            let keep_ratio = core
                .config
                .try_read()
                .map(|config| config.widget_keep_aspect_ratio)
                .unwrap_or(false);
            if keep_ratio && let Ok(Some(monitor)) = window.current_monitor() {
                let adjusted = constrain_physical_aspect(size, previous, &monitor);
                if adjusted != size {
                    let _ = window.set_size(adjusted);
                    return;
                }
            }
            persist_size(core.clone(), &window, size);
        }
        tauri::WindowEvent::ScaleFactorChanged { .. } => {
            let _ = apply_monitor_constraints(&window);
        }
        _ => {}
    });
}

fn write_position(config: &mut crate::config::AppConfig, position: PhysicalPosition<i32>) {
    config.widget_x = Some(position.x);
    config.widget_y = Some(position.y);
}

fn persist_position(core: Arc<AppCore>, position: PhysicalPosition<i32>) {
    let generation = core.widget_move_generation.fetch_add(1, Ordering::Relaxed) + 1;
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        if core.widget_move_generation.load(Ordering::Relaxed) != generation {
            return;
        }
        let _ = core
            .update_config(|config| write_position(config, position))
            .await;
    });
}

async fn flush_live_position(window: &WebviewWindow, core: &Arc<AppCore>) -> Result<()> {
    let Ok(position) = window.outer_position() else {
        return Ok(());
    };
    core.widget_move_generation.fetch_add(1, Ordering::Relaxed);
    core.update_config(|config| write_position(config, position))
        .await?;
    Ok(())
}

fn persist_size(core: Arc<AppCore>, window: &WebviewWindow, size: PhysicalSize<u32>) {
    let generation = core
        .widget_resize_generation
        .fetch_add(1, Ordering::Relaxed)
        + 1;
    let scale = window.scale_factor().unwrap_or(1.0);
    let logical = size.to_logical::<f64>(scale);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        if core.widget_resize_generation.load(Ordering::Relaxed) != generation {
            return;
        }
        let _ = core
            .update_config(|config| {
                config.widget_width = logical.width;
                config.widget_height = logical.height;
            })
            .await;
    });
}

fn resolved_geometry(
    app: &AppHandle,
    saved: Option<(i32, i32)>,
    width: f64,
    height: f64,
) -> Result<(PhysicalPosition<i32>, f64, f64)> {
    let monitors = app.available_monitors()?;
    let saved_monitor = saved.and_then(|(x, y)| {
        monitors.iter().find(|monitor| {
            let area = monitor.work_area();
            x >= area.position.x
                && y >= area.position.y
                && x < area.position.x + area.size.width as i32
                && y < area.position.y + area.size.height as i32
        })
    });
    let has_saved_monitor = saved_monitor.is_some();
    let monitor = match saved_monitor {
        Some(monitor) => monitor.clone(),
        None => app
            .primary_monitor()?
            .or_else(|| monitors.into_iter().next())
            .context("no display is available")?,
    };
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    let max_width = area.size.width as f64 / scale;
    let max_height = area.size.height as f64 / scale;
    let position = if has_saved_monitor {
        let (x, y) = saved.expect("saved monitor requires a saved position");
        PhysicalPosition::new(x, y)
    } else {
        let physical_width = (width.min(max_width) * scale).round() as i32;
        let physical_height = (height.min(max_height) * scale).round() as i32;
        PhysicalPosition::new(
            area.position.x + (area.size.width as i32 - physical_width) / 2,
            area.position.y + (area.size.height as i32 - physical_height) / 2,
        )
    };
    Ok((position, max_width, max_height))
}

/// Both widgets stay between the minimum size and the monitor work area.
pub(crate) fn apply_monitor_constraints(window: &WebviewWindow) -> Result<()> {
    let monitor = window
        .current_monitor()?
        .context("no display is available")?;
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    let max_height = area.size.height as f64 / scale;
    window.set_size_constraints(WindowSizeConstraints {
        min_width: Some(tauri::LogicalUnit::new(MIN_WIDGET_WIDTH).into()),
        min_height: Some(tauri::LogicalUnit::new(MIN_WIDGET_HEIGHT.min(max_height)).into()),
        max_width: Some(tauri::LogicalUnit::new(area.size.width as f64 / scale).into()),
        max_height: Some(tauri::LogicalUnit::new(max_height).into()),
    })?;
    Ok(())
}

fn clamp_logical_size(
    width: f64,
    height: f64,
    max_width: f64,
    max_height: f64,
    keep_ratio: bool,
) -> (f64, f64) {
    if !keep_ratio {
        return (
            width.clamp(MIN_WIDGET_WIDTH, max_width),
            height.clamp(MIN_WIDGET_HEIGHT, max_height),
        );
    }
    let width = width.clamp(MIN_WIDGET_WIDTH, max_width.min(max_height * ASPECT_RATIO));
    let height = width / ASPECT_RATIO;
    (width, height)
}

fn constrain_physical_aspect(
    size: PhysicalSize<u32>,
    previous: PhysicalSize<u32>,
    monitor: &tauri::Monitor,
) -> PhysicalSize<u32> {
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let min_width = (MIN_WIDGET_WIDTH * scale).round() as u32;
    let min_height = (MIN_WIDGET_HEIGHT * scale).round() as u32;
    let max_width = area.size.width;
    let max_height = area.size.height;
    if size.width.abs_diff(previous.width) >= size.height.abs_diff(previous.height) {
        let width = size.width.clamp(
            min_width,
            max_width.min((max_height as f64 * ASPECT_RATIO) as u32),
        );
        PhysicalSize::new(width, (width as f64 / ASPECT_RATIO).round() as u32)
    } else {
        let height = size.height.clamp(
            min_height,
            max_height.min((max_width as f64 / ASPECT_RATIO) as u32),
        );
        PhysicalSize::new((height as f64 * ASPECT_RATIO).round() as u32, height)
    }
}

async fn update_visibility(core: &Arc<AppCore>, visible: bool) -> Result<()> {
    core.update_config(|config| config.widget_visible = visible)
        .await
        .map(|_| ())
}

async fn widget_url(core: &Arc<AppCore>) -> Result<String> {
    let config = core.config.read().await.clone();
    let secret = load_or_create_relay_secret()?;
    let preferences = core.interface_preferences.read().await;
    Ok(format!(
        "http://{}:{}/overlay?secret={secret}&widget=1&locked={}&lang={}",
        youtube_embed_host(),
        config.port,
        if config.widget_locked { 1 } else { 0 },
        preferences.language,
    ))
}

#[cfg(test)]
mod tests;
