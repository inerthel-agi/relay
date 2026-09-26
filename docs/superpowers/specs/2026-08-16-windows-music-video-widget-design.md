# Dedicated Windows Music Video Widget

Date: 2026-08-16
Status: Approved design

## Context

Relay currently renders YouTube music as a compact thumbnail inside the TTS
notification window. Enlarging that window does not produce an OBS-like video:
the widget stylesheet deliberately limits the player to a small thumbnail.
Earlier attempts to switch the shared native window between music and TTS sizes
caused visible large-to-small-to-large geometry jumps.

## Decisions

- Render Windows music in the existing 16:9 media widget, not in the TTS
  notification widget.
- Keep the TTS notification widget compact and independent.
- Automatically show the media widget when music starts if it was hidden.
- Automatically hide it when the music queue becomes idle, but only when Relay
  opened it automatically.
- Preserve the existing exclusive FIFO stage rules.
- Reuse the existing media-widget size, position, lock, volume, and sound
  settings. No public configuration schema change is required.

## Architecture

The native media window remains the single Windows surface for large images,
videos, and YouTube music. Its existing overlay page already contains the same
full-size YouTube IFrame renderer used by OBS; widget playback is enabled there
instead of creating another player.

The notification window becomes TTS-only. It still observes the shared stage
clock so TTS waits for active music, but it no longer creates a YouTube player,
shows a Now Playing card, or wakes for `MusicPlay`.

An in-memory ephemeral-wake flag records whether Relay opened the media widget
only for music. Manual widget visibility remains persisted exactly as today.

## Event Flow

1. `MusicPlay` is broadcast.
2. If the media widget is hidden, Relay shows it without changing the saved
   `widgetVisible` preference and marks the wake as ephemeral.
3. The media widget connects to `/overlay?widget=1`, receives the current
   playback, and starts the existing full-size YouTube renderer.
4. If TTS or another exclusive visual already owns the stage, playback waits in
   the existing FIFO queue. The transparent widget may be created while waiting,
   but no video is shown early.
5. Natural YouTube completion reports `musicEnded`; queued music continues
   without hiding the window.
6. `MusicIdle` or `Clear` hides the media widget only when the ephemeral flag is
   still set.

`MusicStop` alone must not hide the window because it can be immediately followed
by another `MusicPlay`.

## Visibility Rules

- A widget already visible before music starts stays visible after music ends.
- Manually showing the widget clears the ephemeral flag and persists visibility.
- Normal media delivered while the widget was music-woken converts the window to
  normal persisted visibility, preventing an idle music event from hiding active
  media.
- Manually hiding the widget always wins and clears the ephemeral state.
- Restart behavior continues to follow the persisted media-widget preference.

## Layout and Controls

The large video uses the media widget's current 16:9 geometry and full-frame
`object-fit: contain` behavior. The Music page describes that Now Playing uses
the media widget and points its size controls at the same media-widget width and
height. TTS geometry controls remain separate.

The player follows `widgetSoundEnabled` and `mediaVolume`. Locking the media
widget keeps the existing click-through behavior.

## Error Handling

- The notification widget must never create a duplicate desktop YouTube player.
- Stale `MusicStop` and `musicEnded` events remain filtered by playback ID.
- A YouTube embed error unloads only the failed local player and does not stop a
  valid OBS player.
- WebSocket reconnect restores the current server playback in the media widget.

## Verification

Automated coverage must verify:

- Windows media widget starts YouTube music at full size.
- Notification widget does not create or display a music player.
- Hidden media widget auto-opens for music and auto-hides on `MusicIdle`.
- A manually visible media widget is never auto-hidden.
- Normal media cancels music-only ephemeral visibility.
- TTS and music preserve the existing exclusive FIFO behavior.
- OBS YouTube behavior and audio remain unchanged.
- Music settings and media-widget geometry stay synchronized.

Manual smoke test:

1. Keep both Windows widgets hidden and start a track.
2. Confirm a 16:9 video window appears and the TTS widget stays hidden.
3. Send a TTS message during playback and confirm it waits.
4. Let music finish and confirm the auto-opened video window disappears.
5. Manually show the media widget, play another track, and confirm the window
   remains visible afterward.
