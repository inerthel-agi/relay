# Output controls and application refactoring

## User-facing changes

- Overlay > Size and crop: choose an anchor and horizontal/vertical margins for each existing output target. Legacy is the default for existing configurations.
- Each target supports up to 12 named presets in local UI storage. A preset contains geometry and, for Windows widgets, dimensions and aspect-ratio preference. It contains no credentials or private URLs.
- Overlay > Live preview: select portrait, landscape, GIF, video, audio, notification, or sticker. The preview works without Discord. Test output sends the selected sample only when a matching live output is connected.
- Media > Waiting queue: inspect the scheduler's waiting items and pending music. Removal refuses active playback. Entries still undergoing preparation cannot be removed through this control.
- History: combine a text search over filename, title, artist and author with a media-type filter. Replay and download keep their existing behavior.
- Output readiness: distinguish a disconnected server, a disabled notification output, a missing live output, and a connected output.

## Refactoring scope

| Area | Result |
|---|---|
| Panel localization | Dictionaries moved from panel.js to translations.mjs; regional overrides retained. |
| Presets | Validation, bounded storage and controls isolated in output-presets.mjs. |
| Fonts | Font declarations isolated in panel-fonts.css. |
| Output positioning | Dependency-free layout calculations in outputs/layout.js; native CSS handles notification alignment. |
| Audio presentation | Audio-card rules isolated from media rendering in overlay/audio-card.css. |
| HTTP server | HTTP pages, static resources, and media responses moved to server/http_routes.rs; WebSocket coordination remains in server.rs. |
| Discord music | Music interaction handlers moved to bot/music_handlers.rs. |
| Application state | Music lifecycle and media caches moved to state/music_playback.rs and state/media_cache.rs. |
| Rust tests | Tests for 19 modules moved to module-local tests.rs files without changing their module paths. |
| Remaining modules | Privacy classification, custom-command authorization, credentials, updater verification, notification and widget lifecycle retain their implementations; tests and static analysis cover them. No speculative algorithm rewrite. |

## Compatibility and safety

OutputGeometry uses serde defaults for new fields. Legacy preserves the already approved output layout. Margins are validated from 0 to 200 pixels. Presets validate untrusted local storage before applying values. Native configuration validation remains authoritative.

Queue inspection returns display metadata rather than complete media URLs or detected private text. Removing a waiting scheduler ticket is checked under the scheduler lock. Playback already dispatched cannot be removed by this operation.

The server still binds only to loopback. Output pages do not acquire panel command permissions. Updater signature verification is unchanged.

## Validation

Run from src-tauri: cargo fmt --check, cargo clippy --offline --all-targets -- -D warnings, cargo test --offline.
Run the JavaScript test files in gui-tests, overlay, notifications, stickers, and reactions using node --test.

Browser smoke coverage: full panel module loading, video preview playback, saving/restoring an anchor preset, queue removal and author filtering. These checks use a local simulated backend; they do not claim an end-to-end Discord/OBS run.

Sample assets were generated locally using the already installed FFmpeg, with testsrc2. No runtime dependency was added. The signed-release test remains conditional on its environment. Speech synthesis and speech-pack tests have been removed.
