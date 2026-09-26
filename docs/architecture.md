# Relay architecture

Relay is a Windows desktop application that relays Discord media to authenticated local OBS Browser Sources and optional Windows widgets. Its desktop shell is Tauri 2, its core is Rust, and its interface is static HTML, CSS, and JavaScript.

## Runtime components

| Component | Location | Responsibility |
|---|---|---|
| Desktop shell | `src-tauri/src/lib.rs` | Tauri lifecycle, system tray, windows, shortcuts, command registration, startup migration |
| Discord gateway | `src-tauri/src/bot.rs` | Serenity client, channel events, Discord commands, media classification and deletion requests |
| Application state | `src-tauri/src/state.rs` | Configuration updates, privacy gate, queues, history, replay, caches and relay events |
| Privacy scanner | `src-tauri/src/privacy.rs` | Local text, EXIF, GPS and OCR analysis with risk classification and sanitized decisions |
| Local server | `src-tauri/src/server.rs` | Authenticated localhost HTTP and WebSocket output routes |
| Output clients | `overlay/`, `notifications/`, `stickers/`, `reactions/`, `outputs/` | OBS Browser Sources and widget-facing playback clients; `outputs/layout.js` holds shared placement; `outputs/theme.css` and `outputs/theme.js` hold the shared stream style |
| Sound reactions | `src-tauri/src/reactions.rs`, `src-tauri/src/reaction_*.rs` | Reaction library, trimming, access rules and playback leases |
| Panic button | `src-tauri/src/panic.rs`, `src-tauri/src/stage_scheduler.rs` | Clears every output and pauses the scheduler; paused content is dropped, reactions and music requests are refused |
| Bot check | `src-tauri/src/discord_check.rs` | Message Content Intent state and missing channel permissions per feature |
| OBS setup | `src-tauri/src/obs.rs` | obs-websocket 5 client on 127.0.0.1: authentication, scene list, Browser Source creation and update |
| Control panel | `gui/` | Tauri control interface, translations, personalization and local moderation controls |

## Media path

1. Serenity receives a Discord message in a configured Relay channel.
2. Relay identifies supported attachments, stickers, or supported Discord GIF embeds.
3. The privacy scanner checks message text, attachment names, and applicable local image metadata or OCR signals.
4. `AppCore` applies the current privacy policy before a media item reaches history, cache, moderation approval, WebSocket, OBS, or a widget.
5. Allowed media enters the relevant local FIFO queue. Medium-risk items can enter the existing local moderation queue. Blocked items remain out of public Relay outputs.
6. The local Axum server broadcasts authorized events to connected output clients. Browser Sources and Windows widgets keep independent display state where required.

## Trust boundaries

- Discord is external. Relay receives messages, attachments, stickers, and gateway events through Discord.
- Relay's local HTTP and WebSocket server binds to `127.0.0.1` only.
- Output pages require Relay's local authorization mechanism. They cannot use the control-panel command surface.
- Credentials and Relay's local secret use Windows Credential Manager. The persisted application configuration does not contain the Discord token.
- Media downloads are bounded. Direct media and redirect targets are restricted to approved HTTPS host families.
- Privacy logs store classifications, category codes, and actions. They do not reproduce detected text, coordinates, OCR output, metadata values, or configured private strings.

## Persistence and lifetime

- Application configuration is stored in Relay's Tauri application configuration directory and is atomically replaced after validation.
- Discord credentials and the local Relay secret are stored separately in Windows Credential Manager.
- History, pending moderation items, audio, artwork, media compatibility output, and output queues are memory-backed and bounded. They are not intended to survive a restart.
- Interface language, theme, design, font, and sidebar preferences are local UI preferences.

## Local output routes

The local server exposes authenticated routes for the `/obs/visual` and `/obs/audio` composites, visual media, audio, notifications, stickers, reactions, cached media, and a WebSocket event stream. Message notifications are visual only; Relay has no speech synthesis. Relay displays the exact private Browser Source URLs in the application; contributors must not invent, publish, or log them.

## Source map for contributors

| Change | Primary files |
|---|---|
| Discord message handling | `src-tauri/src/bot.rs`, `src-tauri/src/model.rs` |
| Moderation and privacy | `src-tauri/src/privacy.rs`, `src-tauri/src/state.rs`, `gui/panel.js` |
| Config migration and validation | `src-tauri/src/config.rs`, `src-tauri/src/commands.rs` |
| Custom Discord actions | `src-tauri/src/custom_commands.rs`, `src-tauri/src/bot.rs` |
| Local output server | `src-tauri/src/server.rs` and the matching output directory |
| Windows widgets | `src-tauri/src/widget.rs`, `src-tauri/src/notification_widget.rs` |
| Message notifications | `src-tauri/src/state.rs`, `notifications/` |
| UI, themes, and translations | `gui/panel.html`, `gui/panel.css`, `gui/panel.js`, `gui/tray.*` |

## Validation layers

- Rust unit and integration tests: `cargo test` from `src-tauri`.
- Rust style and static analysis: `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` from `src-tauri`.
- Interface and Browser Source tests: `node --test gui/*.test.cjs overlay/*.test.cjs notifications/*.test.cjs stickers/*.test.cjs reactions/*.test.cjs` from the repository root.
- Windows-dependent checks such as signed installer verification remain optional local smoke checks because they require configured Windows or release state.

## Refactored modules and output controls

Panel translations, output presets, privacy filter parsing (`gui/privacy-filters.mjs`) and changelog rendering (`gui/changelog-markdown.mjs`) use native JavaScript modules. `gui/panel.js` keeps startup, configuration forms, history and outputs; page features live in their own modules:

| Module | Role |
|---|---|
| `gui/overview.mjs` | Setup checklist, dashboard, sidebar badges, channel links, trial mode |
| `gui/custom-commands.mjs` | Custom command editor and its pure helpers |
| `gui/settings-search.mjs` | Top bar search and the `reveal` helper used by internal links |
| `gui/autosave.mjs` | Automatic saving of settings forms and the Start with Windows switch |
| `gui/diagnostics.mjs` | Translated error categories and the sanitized diagnostic report |
| `gui/panic.mjs` | Top bar panic button and the paused banner |
| `gui/discord-check.mjs` | Discord → Bot check results |
| `gui/obs-setup.mjs` | Automatic OBS source setup on OBS & widgets |

Each module exports an `initialize…` function that receives the panel helpers it needs (`$`, `t`, `invoke`, `setSaveState`, getters for shared state) and returns a small API. `panel.js` creates them after the `// Page modules` marker. `gui/test-source.cjs` lists the panel modules that tests read alongside `panel.js`. Moderation lives in `src-tauri/src/moderation.rs` (gate, word lists, link and spam rules, presets, decision log, live mode) with its queue helpers in `src-tauri/src/state/moderation_queue.rs`, its commands in `src-tauri/src/commands/moderation.rs` and its page in `gui/moderation-ui.mjs`; see [Moderation](moderation-design.md). Shared output placement lives in `outputs/layout.js` and the stream style in `outputs/theme.css`/`outputs/theme.js`, served at `/output-theme.css`, `/output-theme.js` and `/output-fonts/{file}` (bundled fonts only, allowed by `font-src 'self'`); generated local samples live in `outputs/samples/`. HTTP handlers live in `src-tauri/src/server/http_routes.rs`, Discord music handlers in `src-tauri/src/bot/music_handlers.rs`, and state music/cache operations in `src-tauri/src/state/`. Rust tests are stored in each module directory.

See [Output controls and refactoring](refactoring-and-output-controls.md) for feature locations, compatibility and validation boundaries.
