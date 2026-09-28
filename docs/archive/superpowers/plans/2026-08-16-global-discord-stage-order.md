# Global Discord Stage Order Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Guarantee one non-preemptive Discord chronology across TTS, media, stickers, and music on OBS and Windows.

**Architecture:** Add a server-side scheduler whose placeholders are registered before asynchronous preparation. Ready payloads are dispatched one at a time in `StageOrderKey` order, with stage lifecycle feedback from the existing clock and bounded demotion for slow items.

**Tech Stack:** Rust 2024, Tokio synchronization/timers, existing `broadcast` and `watch` channels, vanilla JavaScript clients unchanged unless a proven lifecycle gap requires metadata.

## Global Constraints

- No new dependency.
- Preserve existing serialized event shapes when possible.
- Never log credentials or Discord tokens.
- Bind only to `127.0.0.1`.
- Active content is never preempted.
- Pending readiness timeout: 20 seconds.
- Unclaimed dispatch timeout: 3 seconds.

---

### Task 1: Scheduler core

**Files:**
- Create: `src-tauri/src/stage_scheduler.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**

```rust
pub struct StageOrderKey {
    pub timestamp_ms: u64,
    pub message_id: u64,
    pub part: u16,
    pub insertion: u64,
}

pub enum ScheduledStageEvent {
    Media(MediaEvent),
    Sticker(StickerEvent),
    Tts(TtsEvent),
    Music(MusicPlaybackEvent),
}

pub struct StageTicket { /* opaque id */ }

impl StageScheduler {
    pub fn new(relay_tx: broadcast::Sender<RelayEvent>) -> Self;
    pub async fn register(&self, key: StageOrderKey, lane: StageLane) -> StageTicket;
    pub async fn ready(&self, ticket: StageTicket, event: ScheduledStageEvent);
    pub async fn cancel(&self, ticket: StageTicket);
    pub async fn stage_state(&self, media_busy: bool, music_busy: bool, tts_busy: bool);
    pub async fn clear(&self);
    pub async fn skip_active(&self);
}
```

- [ ] Write unit tests for timestamp ordering, snowflake ordering, text-before-attachment, non-preemption, 20-second demotion with paused Tokio time, cancellation, clear, and unclaimed dispatch.
- [ ] Run `cargo test stage_scheduler --lib` and confirm RED.
- [ ] Implement the minimal scheduler and confirm GREEN.

### Task 2: Register before asynchronous preparation

**Files:**
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/bot.rs`
- Modify: `src-tauri/src/model.rs` only if an internal ticket/order field is required.

**Interfaces:**

```rust
AppCore::register_stage_output(timestamp_ms, message_id, part, lane) -> StageTicket
AppCore::complete_media(ticket, MediaEvent)
AppCore::complete_tts(ticket, TtsEvent)
AppCore::complete_sticker(ticket, StickerEvent)
AppCore::cancel_stage_output(ticket)
```

- [ ] Write state tests where an image completes before an older TTS but `relay_tx` receives TTS first.
- [ ] Register all known message parts before the first network/synthesis await.
- [ ] Cancel tickets on privacy denial, unsupported content, processing failure, and queue rejection.
- [ ] Keep history insertion when media becomes ready, independently from display dispatch.
- [ ] Run focused state and bot tests.

### Task 3: Reserve and release the shared stage

**Files:**
- Modify: `src-tauri/src/server.rs`
- Modify: `src-tauri/src/stage_scheduler.rs`

- [ ] Extend server integration tests with active 21:59 video, ready image 22:01, delayed TTS 22:00; assert broadcast order video → TTS → image.
- [ ] Feed global `StageClockState` transitions into the scheduler.
- [ ] Reserve the selected lane before broadcast so an opposite claimant cannot win.
- [ ] Complete the active ticket only after at least one matching claim followed by global idle, or after the 3-second unclaimed timeout.
- [ ] Preserve ref-counting for simultaneous OBS and Windows consumers.
- [ ] Run `cargo test server::tests --lib`.

### Task 4: Controls, replay, moderation, and music

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/music.rs` and `src-tauri/src/bot.rs` only where confirmation order must be retained.

- [ ] Test that clear cancels pending tickets, skip advances exactly once, and replay appends at request time.
- [ ] Test that moderation does not hold a live placeholder and approval gets a new order key.
- [ ] Register music at selection confirmation and prevent queued tracks from bypassing older Discord outputs.
- [ ] Preserve natural `MusicStop`/`MusicIdle` cleanup.

### Task 5: End-to-end validation

**Files:**
- Test: existing Rust and JavaScript suites.

- [ ] Run `node --check` on every modified JavaScript file.
- [ ] Run `node --test`.
- [ ] Run `cargo fmt --check`; report unrelated pre-existing formatting separately.
- [ ] Run `cargo test --lib`.
- [ ] Build with:

```powershell
$env:CARGO_TARGET_DIR = "target-windows-music-video"
cargo tauri build --no-bundle
```

Expected executable:

```text
src-tauri/target-windows-music-video/release/relay.exe
```
