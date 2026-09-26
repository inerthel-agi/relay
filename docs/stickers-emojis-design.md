# Sticker and emoji integration

## Understanding

- Discord stickers use a dedicated OBS Browser Source at `/stickers`.
- Sticker playback has an independent FIFO queue and configurable duration.
- Unicode, static custom, and animated custom Discord emojis are supported in the TTS channel.
- Any message containing an emoji skips speech synthesis entirely.
- Emoji messages still appear visually in the TTS notification output with their author.
- Existing media, audio, and emoji-free TTS behavior remains unchanged.

## Assumptions

- Sticker queue capacity is 50 items.
- Default sticker display duration is 8 seconds.
- Discord sticker formats PNG, APNG, GIF, and Lottie are accepted when they can be rendered safely.
- Failed or expired assets time out and advance the queue.
- Assets are cached locally with bounded item and byte limits.
- Relay remains local-only and adds no telemetry or external service.

## Architecture

The Discord bot emits a typed `StickerEvent` rather than reusing `MediaEvent`. Sticker assets
are downloaded into a bounded cache and exposed through an authenticated local route. The
`/stickers` page subscribes with the `sticker` WebSocket role and maintains its own FIFO queue,
playback timer, watchdog, and reconnect state.

TTS input is parsed before synthesis. Emoji-bearing messages emit a visual-only TTS notification
event containing renderable emoji tokens and author metadata. They never enter the synthesis
lock or audio cache. Emoji-free messages follow the existing speech path unchanged.

## Error handling and limits

- A missing sticker asset is skipped without blocking later items.
- Queue overflow drops only the newest item after the configured capacity is reached.
- Relay shutdown broadcasts `clear`, which empties sticker and notification queues.
- Custom emoji URLs are generated only from validated Discord snowflake IDs.
- Unicode parsing is deterministic and covered by regression tests.

## Decision log

- Chose a dedicated `/stickers` source over sharing `/medias` to preserve independent placement.
- Chose typed events over media-kind reuse to keep queues and timing isolated.
- Chose visual-only emoji notifications whenever an emoji appears, per user request.
- Chose bounded local caching over direct remote playback for reliability in OBS.
- Kept the existing local server and WebSocket hub instead of adding another service.
