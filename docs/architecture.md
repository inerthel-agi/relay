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
| Output clients | `overlay/`, `tts/`, `notifications/`, `stickers/` | OBS Browser Sources and widget-facing playback clients |
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

The local server exposes authenticated routes for visual media, audio, TTS, notifications, stickers, cached media, and a WebSocket event stream. Relay displays the exact private Browser Source URLs in the application; contributors must not invent, publish, or log them.

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
- Interface and Browser Source tests: `node --test gui/*.test.cjs overlay/*.test.cjs notifications/*.test.cjs stickers/*.test.cjs tts/*.test.cjs` from the repository root.
- Windows-dependent checks such as signed installer verification remain optional local smoke checks because they require configured Windows or release state.

## Refactored modules and output controls

Panel translations and output presets use native JavaScript modules. Shared output placement lives in `outputs/layout.js`; generated local samples live in `outputs/samples/`. HTTP handlers live in `src-tauri/src/server/http_routes.rs`, Discord music handlers in `src-tauri/src/bot/music_handlers.rs`, and state music/cache operations in `src-tauri/src/state/`. Rust tests are stored in each module directory.

See [Output controls and refactoring](refactoring-and-output-controls.md) for feature locations, compatibility and validation boundaries.
