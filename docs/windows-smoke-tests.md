# Windows smoke-test matrix

Run the automated checks in [CONTRIBUTING.md](../CONTRIBUTING.md) first. Use this matrix only for behavior that needs Windows, Discord, OBS, configured media codecs, or a signed release artifact.

Use a disposable Discord server and synthetic local media. Do not use tokens, private Browser Source URLs, real addresses, real contact details, or personal image metadata.

## Before testing

1. Start Relay with `cargo tauri dev` from `src-tauri` or install a locally built package.
2. Use a disposable bot and test channel. Grant only the permissions needed for the scenario.
3. Open the relevant OBS Browser Source or Windows widget before checking Output readiness.
4. Record the Relay version, Windows version, test result, and any sanitized error message.

## Runtime and outputs

| Scenario | Steps | Expected result |
| --- | --- | --- |
| Start without credentials | Launch Relay before configuring Discord credentials. | The interface remains usable and shows the bot as offline. |
| Bot connection | Save valid credentials and select a test Relay channel. | The bot status becomes online and visible text channels can be refreshed. |
| Local server | Open the Overlay page. | The server status is online and the preview can connect. |
| OBS source | Add one generated Browser Source URL to OBS. | Output readiness lists an OBS client for that output. |
| Windows widgets | Show, move, and lock each widget. | The widget follows its visibility and lock settings without blocking Relay. |
| Local output test | Connect an OBS source or widget, then use **Test output**. | The test appears only on the selected local output and no Discord message is sent. |
| Output disconnect and reconnect | Start a media or reaction output, disconnect the Browser Source or widget, then reconnect it. | Relay remains responsive, reports the changed output state, and the new client receives its current hydration state. |

## v1.3.6 modules

| Scenario | Steps | Expected result |
| --- | --- | --- |
| Pin a message | Send a notification, click **Pin**, wait past its configured duration, then click **Unpin**. | The message stays visible while pinned, other outputs continue, and queued notifications resume after unpinning. |
| Media library copy | Import a synthetic image or video, move the original file, restart Relay, and play the library item. | The managed copy remains available and plays through the normal media queue. |
| Music request limit | Set the per-member pending limit to `3`, start one track, and submit four waiting requests from one member. | The first three wait; the fourth is refused with a clear reason. Set the value to `0` and repeat to verify that the per-member limit is disabled. |
| Duplicate music request | Start one track, queue a video, then request the same YouTube video again while it is waiting. | The second request is refused by default. Disabling duplicate protection allows it. |
| Music queue interleaving | Keep one media item active, queue two music items with a media or message item between them, move one music item up or down, then remove it. | Only the music positions change. The current item is not moved, and media/message slots keep their order. |
| Reaction access | Enable reactions, configure one allowed channel and one allowed role, then try `/relay reaction` from a wrong channel, wrong role, and matching channel and role. | The first two requests are refused. The matching request starts the enabled reaction. |
| Reaction local test | Use the panel's **Test** button with a sound and optional library image or GIF. | The sound and visual play in the panel only; no Discord cooldown or live output is used. |
| Reaction output disconnect | Trigger a reaction, disconnect the **Relay Reactions** Browser Source during playback, and reconnect it. | Relay stops or expires the active reaction cleanly, does not leave music ducked, and the new source receives the current reaction snapshot. |
| Reaction volume change | Start a music track, trigger a reaction, change the Relay media volume while the reaction is active, then let it end and repeat with **Stop**. | Music is reduced to the configured reaction percentage during the reaction and returns to the newly selected volume afterward. |
| Deleted reaction sound | Delete or move the imported reaction sound file before triggering it. | Relay refuses the reaction with a clear error and does not publish an incomplete output. |
| Offline reaction save | Stop the bot connection, save reaction settings, reconnect the bot, and inspect the slash-command list. | Settings save locally while offline and `/relay reaction` synchronizes after reconnection. |

## Discord media and moderation

| Scenario | Steps | Expected result |
| --- | --- | --- |
| Image, GIF, audio, video, and sticker | Post one synthetic sample of each supported type in the watched channel. | Each accepted item follows its configured output route and duration. |
| H.264 and HEVC video | Post short synthetic H.264 and HEVC files separately. | Relay stays responsive. Record the observed initial delay and any FFmpeg fallback result. |
| Manual moderation | Enable review for the tested media type and post a synthetic sample. | The item enters the local review queue and reaches outputs only after approval. |
| Automatic filter | Add `RELAY-PRIVACY-SMOKE-ONLY` as a temporary filter word, then post that exact text. | The configured block or review action occurs before public Relay outputs. Remove the test word afterwards. |
| Privacy scanner | Add the same temporary value as a protected private string and repeat the test. | Relay records only the classification and category. The test value is not copied into logs or visible history. |
| Delete blocked messages | Enable deletion, grant Manage Messages in the disposable channel, and repeat a known blocking test. | The blocked test message is deleted only when Relay has the required Discord permission. |

## Image metadata and OCR

| Scenario | Steps | Expected result |
| --- | --- | --- |
| EXIF/GPS | Use a locally created image with synthetic metadata only. | The configured privacy policy handles detected metadata without exposing its values in Relay errors or logs. |
| Local OCR | Use a locally created image containing `RELAY-PRIVACY-SMOKE-ONLY`. | OCR failure does not stop Relay. A confirmed match follows the configured privacy policy. |
| Large or malformed file | Attach a safe test file near the configured limit and a malformed image. | Relay rejects unsafe input cleanly and continues receiving later media. |

## Release-only checks

| Scenario | Steps | Expected result |
| --- | --- | --- |
| Signed update | Run against an official signed installer and release feed. | Relay accepts only the expected signed update path. |
| Message notifications | In the Relay channel, send plain text, emoji, a sticker, then text with an image and a video. | Text and emoji appear as visual cards without speech; the sticker uses the sticker output; the mixed message shows one card, then each media once. The configured notification sound remains optional. |
| Former channels | Start Relay with a configuration that has different media and message channels. | The Discord page asks which channel to keep; both keep their previous role until the choice. |

Do not mark a pull request as fully smoke-tested when a required integration is unavailable. State the skipped row and why in the pull request instead.
