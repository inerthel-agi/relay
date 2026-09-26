# Relay personalization and queue recovery

## Understanding

- Keep the existing light and dark themes.
- Add RGB accent controls with values from 0 to 255.
- Add a readable font scale from 80% to 140%.
- Support English, French, Spanish and German.
- Apply preferences to the control panel, Windows widgets and OBS outputs.
- Keep preferences local and apply changes without restarting Relay.
- Prevent one stalled TTS or media item from blocking the remaining FIFO queue.

## Assumptions

- Custom RGB changes accents, controls and indicators, not the base background.
- Missing translations fall back to English.
- Invalid or unreachable media is skipped individually after a bounded timeout.
- Playback errors, stalls, aborts and empty media states all advance the queue safely.
- Closing Relay still clears every active output.

## Decision log

1. Use CSS custom properties for accent and font scale to avoid duplicating themes.
2. Store UI preferences in local storage and synchronize them to local output pages.
3. Add Spanish and German while preserving English as the translation fallback.
4. Keep client-side FIFO queues and add watchdog recovery instead of changing the WebSocket protocol.
5. Guard completion by playback generation so duplicate browser events cannot skip two items.

## Final design

The control panel gains a Personalization page with theme, language, RGB accent,
font scale, live preview and reset controls. Relay propagates the current language,
accent and font scale to Windows widget URLs and the permanent OBS pages. Values are
validated before use and remain on the local computer.

Media and TTS outputs maintain FIFO order. Each active item receives a load watchdog;
video, audio and speech playback also receive stall/error recovery. A generation guard
makes finishing idempotent. When an item cannot load or stops progressing, only that
item is cleared and the next queued item starts. Relay shutdown clears active playback
and pending client-side items as before.
