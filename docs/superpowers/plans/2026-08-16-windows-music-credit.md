# Windows Music Credit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Display the active YouTube title and Discord requester in a compact lower-left credit on the Windows media widget.

**Architecture:** Reuse the existing YouTube credit DOM and CSS. Extend its renderer to accept the combined Windows widget, selecting the track title there while preserving the YouTube channel label on the OBS `/youtube` output.

**Tech Stack:** Vanilla JavaScript, CSS, Node.js test runner.

## Global Constraints

- Apply only to YouTube playback in the Windows media widget.
- Preserve existing OBS and non-music media behavior.
- Add no dependency.

---

### Task 1: Render Windows music metadata

**Files:**
- Modify: `overlay/overlay.test.cjs`
- Modify: `overlay/overlay.js:189-209`

**Interfaces:**
- Consumes: existing `musicPlay` payload fields `title`, `channelTitle`, and `requestedBy`.
- Produces: `showYoutubeCredit(payload)` rendering the title/requester in widget mode.

- [ ] **Step 1: Write the failing test**

Extend `Windows combined overlay owns full-frame YouTube playback` with a `title` and assertions:

```js
title: "Never Gonna Give You Up",
channelTitle: "Hidden Channel",
requestedBy: "widget-user",
```

```js
assert.equal(
  widget.elements["#youtube-credit-channel"].textContent,
  "Never Gonna Give You Up",
);
assert.equal(
  widget.elements["#youtube-credit-added"].textContent,
  "Added by widget-user",
);
assert.equal(widget.elements["#youtube-credit"].hidden, false);
```

- [ ] **Step 2: Verify the test fails**

Run:

```powershell
node --test overlay/overlay.test.cjs
```

Expected: the Windows credit remains hidden because `showYoutubeCredit` currently accepts only `relayMode === "youtube"`.

- [ ] **Step 3: Implement the minimal renderer change**

Update the credit guard and primary label:

```js
function showYoutubeCredit(payload = {}) {
  if ((relayMode !== "youtube" && !isWidgetWindow) || !youtubeCreditElement) return;
  const primary = decodeBasicHtmlEntities(
    isWidgetWindow ? payload.title || "" : payload.channelTitle || "",
  ).trim();
  const requester = decodeBasicHtmlEntities(payload.requestedBy || "").trim();
```

Use `primary` for `youtubeCreditChannelElement.textContent`, visibility, and the empty-state condition. Keep the localized requester line unchanged.

- [ ] **Step 4: Verify focused and complete tests**

Run:

```powershell
node --check overlay/overlay.js
node --check overlay/overlay.test.cjs
node --test overlay/overlay.test.cjs
node --test
```

Expected: syntax checks succeed and all tests pass.

- [ ] **Step 5: Rebuild the portable executable**

Run from `src-tauri`:

```powershell
$env:CARGO_TARGET_DIR = "target-windows-music-video"
cargo tauri build --no-bundle
```

Expected executable:

```text
src-tauri/target-windows-music-video/release/relay.exe
```
