# Embedded audio artwork

## Understanding

- Relay reads only artwork embedded inside Discord audio attachments.
- Supported formats follow the metadata parser capabilities, including common
  MP3, M4A, FLAC, OGG, and compatible audio containers.
- Relay never searches an external artwork provider.
- Missing or invalid artwork falls back to the existing Relay logo.
- Artwork extraction failure never blocks audio playback.
- Visible shadows are removed from the audio card and author badge.

## Limits and privacy

- Audio metadata inspection downloads at most 50 MiB per attachment.
- Embedded artwork is accepted up to 2 MiB.
- Artwork is held in a 50-item memory cache and is never persisted by Relay.
- Artwork is served only by an authenticated local route.
- Cache misses during replay fall back to the Relay logo.

## Design

The Discord handler inspects audio attachments before submitting their media
event. A bounded HTTP reader downloads the attachment, then a blocking metadata
worker extracts the first embedded picture. The application core caches the
picture by Discord attachment ID. Media events carry only that local artwork ID,
not the image bytes. The overlay requests the artwork through Relay's existing
secret-protected local server.

## Decision log

- Chosen: Rust metadata extraction. Browser-side parsing was rejected because
  remote media access and format support are inconsistent in OBS.
- Chosen: embedded artwork only. External lookup was rejected for privacy and
  correctness.
- Chosen: bounded volatile cache. Data URLs and permanent files were rejected
  due to WebSocket size and retention costs.
