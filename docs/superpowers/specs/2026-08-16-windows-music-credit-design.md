# Windows Music Now Playing Card

## Goal

Show complete live YouTube metadata in the Windows media widget without reducing the large video.

## Design

- Scope: Windows media widget YouTube playback only.
- Placement: a compact 360 × 104 px translucent card over the lower-left corner.
- Left column: the existing YouTube thumbnail, cropped without distortion.
- Right column:
  - uppercase “NOW PLAYING” label;
  - decoded YouTube title, bold and truncated to one line;
  - YouTube channel name;
  - localized “Added by” label followed by the Discord requester;
  - live elapsed time, arrow, and selected playback end.
- Footer: thin red progress indicator and restrained animated equalizer.
- The timer starts from `startSeconds`, ends at `endSeconds` when present or `durationSeconds`, and refreshes while the YouTube player is active.
- The card remains visible for the whole playback and disappears on stop, idle, natural ending, error teardown, or clear.
- OBS behavior remains unchanged.
- Missing individual metadata fields are hidden without breaking the layout.

## Data flow

The existing `musicPlay` payload already carries `title`, `channelTitle`, `thumbnail`, `requestedBy`, `durationSeconds`, `startSeconds`, and optional `endSeconds`. The combined Windows widget stores this payload in the existing YouTube credit renderer. A local interval reads the IFrame player’s current time when available, with a wall-clock fallback, then updates the elapsed label and progress. OBS `/youtube` keeps its current compact channel/requester credit.

## Verification

- A Windows music playback displays the thumbnail, title, channel, requester, and selected duration.
- The elapsed clock and progress advance and stop at the selected end.
- Long values are ellipsized and the card does not resize or cover most of the player.
- Missing metadata and image loading failure remain readable.
- The card is cleared with every playback teardown path.
- OBS credit behavior and non-music media remain unchanged.
