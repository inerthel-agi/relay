# Windows Music Now Playing Card Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render a complete live Now Playing card over the lower-left corner of the large Windows YouTube widget.

**Architecture:** Extend the existing YouTube credit component rather than creating another overlay. Widget mode receives richer markup, styling, and a playback-time updater; OBS keeps the existing compact channel/requester presentation.

**Tech Stack:** Vanilla HTML/CSS/JavaScript, YouTube IFrame Player API, Node.js test runner.

## Global Constraints

- Keep the Windows video full-size.
- Apply the rich card only to Windows widget YouTube playback.
- Preserve OBS `/youtube` and non-music media behavior.
- Add no dependency.

---

### Task 1: Build and drive the complete Now Playing card

**Files:**
- Modify: `overlay/index.html`
- Modify: `overlay/overlay.css`
- Modify: `overlay/overlay.js`
- Modify: `overlay/overlay.test.cjs`

**Interfaces:**
- Consumes: `musicPlay` fields `title`, `channelTitle`, `thumbnail`, `requestedBy`, `durationSeconds`, `startSeconds`, and `endSeconds`.
- Produces: `showYoutubeCredit(payload)`, `updateYoutubeCreditProgress()`, and `hideYoutubeCredit()` managing the card lifecycle.

- [ ] **Step 1: Add a failing widget behavior test**

Extend the Windows YouTube test payload:

```js
title: "Less than a Lover",
channelTitle: "JennieRubyJaneVEVO",
thumbnail: "https://i.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg",
durationSeconds: 219,
startSeconds: 0,
requestedBy: "inerthel",
```

Assert the card displays its thumbnail, label, title, channel, requester, initial `0:00 → 3:39` range, and widget modifier class. Set the fake player current time to `62`, execute the 1000 ms timer, and assert `1:02 → 3:39` plus progress near `28.31%`. Stop playback and assert the card is hidden and cleared.

- [ ] **Step 2: Run the focused test and verify RED**

```powershell
node --test overlay/overlay.test.cjs
```

Expected: FAIL because the richer elements and live progress do not exist.

- [ ] **Step 3: Add semantic card markup**

Keep `#youtube-credit-channel` and `#youtube-credit-added` for compatibility, and add:

```html
<img id="youtube-credit-thumbnail" class="youtube-credit__thumbnail" alt="">
<div class="youtube-credit__copy">
  <span id="youtube-credit-label" class="youtube-credit__label">Now playing</span>
  <strong id="youtube-credit-channel" class="youtube-credit__title"></strong>
  <span id="youtube-credit-source" class="youtube-credit__source" hidden></span>
  <div class="youtube-credit__meta">
    <span id="youtube-credit-added" class="youtube-credit__added"></span>
    <span id="youtube-credit-time" class="youtube-credit__time" hidden></span>
  </div>
  <div class="youtube-credit__transport" aria-hidden="true">
    <span class="youtube-credit__equalizer"><i></i><i></i><i></i><i></i></span>
    <span class="youtube-credit__progress">
      <i id="youtube-credit-progress-fill"></i>
    </span>
  </div>
</div>
```

- [ ] **Step 4: Add widget-only visual styling**

Preserve the compact base rules for OBS. Under `.youtube-credit--widget`, use a 360 px maximum width, a 76 px thumbnail column, dark translucent background, one-line ellipsis for title/channel/meta, red accent transport, and responsive sizing bounded by the viewport. Do not alter player dimensions.

- [ ] **Step 5: Implement metadata and time updates**

Query the new elements. In widget mode, populate every field, set the thumbnail, add `.youtube-credit--widget`, and compute:

```js
const start = Math.max(0, Number(payload.startSeconds) || 0);
const end = Number(payload.endSeconds) > start
  ? Number(payload.endSeconds)
  : Math.max(start, Number(payload.durationSeconds) || 0);
```

Use `youtubePlayer.getCurrentTime()` every second, clamp it to `[start, end]`, format with `formatAudioClock`, and set progress relative to the selected range. Store and clear the timer in every credit teardown. OBS skips the widget updater and retains its current primary channel label.

- [ ] **Step 6: Verify focused and full JavaScript suites**

```powershell
node --check overlay/overlay.js
node --check overlay/overlay.test.cjs
node --test overlay/overlay.test.cjs
node --test
```

Expected: all syntax checks and all tests pass.

- [ ] **Step 7: Rebuild the portable Windows executable**

From `src-tauri`:

```powershell
$env:CARGO_TARGET_DIR = "target-windows-music-video"
cargo tauri build --no-bundle
```

Expected:

```text
src-tauri/target-windows-music-video/release/relay.exe
```
