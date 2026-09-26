//! Panic button: empty every output at once, then hold new content until the
//! streamer resumes. Pending moderation items are kept.

use std::sync::Arc;

use tauri::AppHandle;

use crate::{model::RelayEvent, state::AppCore, widget};

/// Pauses Relay first so nothing new slips in, then clears what is on screen.
pub async fn trigger(app: &AppHandle, core: &Arc<AppCore>) {
    core.set_outputs_paused(true).await;
    core.clear_all_music().await;
    core.stage_scheduler.clear().await;
    core.unpin_message().await;
    crate::reactions::stop(core, None).await;
    let _ = core.relay_tx.send(RelayEvent::Clear);
    if widget::state(app, core).await.visible {
        let _ = widget::toggle(app, core.clone()).await;
    }
}

pub async fn resume(core: &Arc<AppCore>) {
    core.set_outputs_paused(false).await;
}
