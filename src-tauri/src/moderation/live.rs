//! Background moderation work: the one-second queue tick (safety delay,
//! expiry) and the automatic live preset that follows OBS's stream state.

use std::sync::Arc;
use std::time::Duration;

use super::log::LogAction;
use super::presets;
use crate::state::AppCore;

const LIVE_CHECK_EVERY_TICKS: u64 = 15;

pub async fn run(core: Arc<AppCore>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut ticks = 0_u64;
    loop {
        interval.tick().await;
        core.moderation_tick().await;
        ticks = ticks.wrapping_add(1);
        if ticks.is_multiple_of(LIVE_CHECK_EVERY_TICKS) {
            check_live(&core).await;
        }
    }
}

/// Applies the live preset when OBS starts streaming and restores the
/// previous settings when it stops. An unreachable OBS changes nothing.
async fn check_live(core: &AppCore) {
    let settings = core.config.read().await.moderation.clone();
    let Some(preset) = settings.live_preset else {
        if let Some(restore) = settings.live_restore {
            let _ = core
                .update_config(|config| {
                    presets::apply(config, &restore);
                    config.moderation.live_restore = None;
                })
                .await;
        }
        return;
    };
    let password = crate::credentials::load_obs_password().ok().flatten();
    let Ok(streaming) =
        crate::obs::stream_active(settings.live_obs_port, password.as_deref()).await
    else {
        return;
    };
    if let Ok(mut runtime) = core.moderation.lock() {
        runtime.live_streaming = streaming;
    }
    match (streaming, settings.live_restore.is_some()) {
        (true, false) => {
            let applied = core
                .update_config(|config| {
                    config.moderation.live_restore = Some(presets::capture(config));
                    presets::apply(config, &presets::fields(preset));
                })
                .await;
            if applied.is_ok() {
                core.record_moderation(None, LogAction::Held, "live_preset_on", None);
            }
        }
        (false, true) => {
            let restored = core
                .update_config(|config| {
                    if let Some(restore) = config.moderation.live_restore.take() {
                        presets::apply(config, &restore);
                    }
                })
                .await;
            if restored.is_ok() {
                core.record_moderation(None, LogAction::Released, "live_preset_off", None);
            }
        }
        _ => {}
    }
}
