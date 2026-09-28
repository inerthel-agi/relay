const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

test("notification assets no longer embed the unused YouTube player", () => {
  const html = fs.readFileSync(__dirname + "/index.html", "utf8");
  const css = fs.readFileSync(__dirname + "/notifications.css", "utf8");
  const script = fs.readFileSync(__dirname + "/notifications.js", "utf8");
  assert.doesNotMatch(html, /id="music(?:-|"\s)/);
  assert.doesNotMatch(css, /\.music-card/);
  assert.doesNotMatch(script, /new\s+window\.YT\.Player|requestMusicNowPlaying/);
});

function classList() {
  const values = new Set();
  return {
    add: (...names) => names.forEach((name) => values.add(name)),
    remove: (...names) => names.forEach((name) => values.delete(name)),
    contains: (name) => values.has(name),
    toggle(name, force) {
      if (force === undefined ? !values.has(name) : force) {
        values.add(name);
        return true;
      }
      values.delete(name);
      return false;
    },
  };
}

function createHarness(target = "obs", language = "en", preview = false, autoGrantStage = true, { withTheme = false } = {}) {
  const sockets = [];
  const timers = new Map();
  const timerDelays = [];
  const windowListeners = new Map();
  let nextTimerId = 1;
  const cssProperties = {};
  const youtubePlayers = [];
  let youtubeHostReplacementCount = 0;
  const elements = {
    "#notification": {
      classList: classList(),
      attributes: new Map(),
      setAttribute(name, value) { this.attributes.set(name, value); },
    },
    "#notification-avatar": { src: "", onerror: null },
    "#notification-author": { textContent: "" },
    "#notification-guild-tag": { hidden: true },
    "#notification-guild-tag-badge": {
      src: "",
      hidden: true,
      onerror: null,
      removeAttribute(name) {
        if (name === "src") this.src = "";
      },
    },
    "#notification-guild-tag-name": { textContent: "" },
    "#notification-message": {
      textContent: "",
      children: [],
      replaceChildren() { this.children = []; this.textContent = ""; },
      append(node) { this.children.push(node); },
    },
    "#music": {
      classList: classList(),
      attributes: new Map(),
      setAttribute(name, value) { this.attributes.set(name, value); },
    },
    "#music-youtube-player": {
      id: "music-youtube-player",
      className: "music-card__player",
      style: { display: "" },
      attributes: new Map([["aria-hidden", "true"]]),
      setAttribute(name, value) { this.attributes.set(name, value); },
      replaceChildren() {},
      innerHTML: "",
    },
    "#music-artwork": { src: "", alt: "", onerror: null, removeAttribute(name) {
      if (name === "src") this.src = "";
    } },
    "#music-label": { textContent: "" },
    "#music-title": { textContent: "" },
    "#music-artist": { textContent: "" },
    "#music-meta": { textContent: "", hidden: true },
    "#music-time": { textContent: "", hidden: true },
    "#music-progress": {
      hidden: true,
      attributes: {},
      setAttribute(name, value) { this.attributes[name] = value; },
      style: {},
    },
    "#music-progress-fill": { style: { width: "0%" } },
    "#notification-move-label": { textContent: "" },
  };
  const youtubeHostParent = {
    replaceChild(next) {
      youtubeHostReplacementCount += 1;
      const prepared = {
        id: next.id || "music-youtube-player",
        className: next.className || "music-card__player",
        style: next.style || { display: "" },
        attributes: next.attributes || new Map(),
        setAttribute(name, value) { this.attributes.set(name, value); },
        replaceChildren() {},
        innerHTML: "",
        parentNode: youtubeHostParent,
      };
      elements["#music-youtube-player"] = prepared;
    },
  };
  elements["#music-youtube-player"].parentNode = youtubeHostParent;

  class MockWebSocket {
    constructor(url) {
      this.url = url;
      this.readyState = 1;
      this.sent = [];
      this.listeners = new Map();
      sockets.push(this);
    }

    addEventListener(type, listener) {
      this.listeners.set(type, listener);
    }

    emit(type, data) {
      this.listeners.get(type)?.({ data });
    }

    send(data) {
      const message = JSON.parse(data);
      this.sent.push(message);
      if (
        autoGrantStage
        && message.type === "stageClock"
        && message.payload?.lane === "notification"
        && message.payload?.busy === true
      ) {
        this.emit("message", JSON.stringify({
          type: "stageClock",
          payload: { mediaBusy: false, musicBusy: false, notificationBusy: true },
        }));
      }
    }

    close() {}
  }

  const search = `?secret=private&target=${target}&locked=0&lang=${language}${preview ? "&preview=1" : ""}`;
  const location = {
    protocol: "http:",
    host: "localhost:4590",
    hostname: "localhost",
    port: "4590",
    origin: "http://localhost:4590",
    href: `http://localhost:4590/notifications${search}`,
    search,
    replace() {},
    reloads: 0,
    reload() { this.reloads += 1; },
  };

  class FakeYoutubePlayer {
    constructor(id, options) {
      this.id = id;
      this.options = options;
      this.loaded = null;
      this.volume = null;
      this.currentTime = 0;
      this.muteCalls = 0;
      this.unMuteCalls = 0;
      this.stopCalls = 0;
      this.destroyCalls = 0;
      this.unloadModuleCalls = [];
      this.setOptionCalls = [];
      this.iframeAttributes = {};
      this.iframe = {
        style: { display: "" },
        src: "https://www.youtube.com/embed/test",
        setAttribute: (name, value) => {
          this.iframeAttributes[name] = value;
        },
        getAttribute: (name) => this.iframeAttributes[name],
        removeAttribute(name) {
          delete this.iframeAttributes[name];
          if (name === "src") this.src = "";
        },
      };
      youtubePlayers.push(this);
    }

    ready() { this.options.events.onReady({ target: this }); }
    loadVideoById(options) { this.loaded = options; }
    playVideo() {}
    stopVideo() { this.stopCalls += 1; }
    destroy() { this.destroyCalls += 1; }
    mute() { this.muteCalls += 1; }
    unMute() { this.unMuteCalls += 1; }
    setVolume(value) { this.volume = value; }
    getCurrentTime() { return this.currentTime; }
    unloadModule(name) { this.unloadModuleCalls.push(name); }
    setOption(module, option, value) { this.setOptionCalls.push([module, option, value]); }
    getIframe() { return this.iframe; }
    emitState(data) { this.options.events.onStateChange({ data }); }
    emitError() { this.options.events.onError({ data: 150 }); }
  }

  const pings = [];
  class MockAudio {
    constructor() {
      this.src = "";
      this.volume = 1;
      this.playCount = 0;
      pings.push(this);
    }

    play() {
      this.playCount += 1;
      return Promise.resolve();
    }
  }

  const window = {
    location,
    addEventListener(type, listener) { windowListeners.set(type, listener); },
    clearTimeout(id) { timers.delete(id); },
    setTimeout(callback, delay) {
      const id = nextTimerId++;
      timers.set(id, { callback, delay });
      timerDelays.push(delay);
      return id;
    },
    YT: { Player: FakeYoutubePlayer, PlayerState: { ENDED: 0, PLAYING: 1 } },
  };
  const context = vm.createContext({
    URL,
    URLSearchParams,
    WebSocket: MockWebSocket,
    Audio: MockAudio,
    document: {
      documentElement: {
        classList: classList(),
        dataset: {},
        lang: "en",
        style: { setProperty(name, value) { cssProperties[name] = value; } },
      },
      head: {
        appendChild() {},
      },
      querySelector: (selector) => elements[selector],
      createElement: (tagName) => ({
        tagName,
        id: "",
        className: "",
        src: "",
        alt: "",
        async: false,
        style: { display: "" },
        attributes: new Map(),
        onerror: null,
        setAttribute(name, value) { this.attributes.set(name, value); },
        addEventListener() {},
        replaceWith() {},
        replaceChildren() {},
        innerHTML: "",
      }),
      createTextNode: (textContent) => ({ textContent }),
    },
    encodeURIComponent,
    window,
  });
  const source = fs.readFileSync(__dirname + "/notifications.js", "utf8");
  vm.runInContext(fs.readFileSync(__dirname + "/../outputs/layout.js", "utf8"), context);
  if (withTheme) vm.runInContext(fs.readFileSync(__dirname + "/../outputs/theme.js", "utf8"), context);
  vm.runInContext(fs.readFileSync(__dirname + "/../outputs/connection.js", "utf8"), context);
  vm.runInContext(source, context);

  return {
    context,
    cssProperties,
    elements,
    location,
    pings,
    sockets,
    socket: sockets[0],
    timers,
    timerDelays,
    youtubePlayers,
    youtubeHostReplacementCount: () => youtubeHostReplacementCount,
    emitWindow: (type) => windowListeners.get(type)?.({ type }),
    runNextTimer() {
      const [id] = timers.keys();
      if (id == null) return false;
      const entry = timers.get(id);
      timers.delete(id);
      entry?.callback?.();
      return true;
    },
    runTimerByDelay(delay) {
      for (const [id, entry] of timers.entries()) {
        if (entry.delay !== delay) continue;
        timers.delete(id);
        entry.callback?.();
        return true;
      }
      return false;
    },
    /** Id of the card this page last reported as visible, or null. */
    shownId() {
      const state = sockets.at(-1)?.sent.filter((message) => message.type === "notificationState").at(-1);
      return state?.payload?.visible ? state.payload.notification.id : null;
    },
    /** Callback of the pending display timer (8 s by default), left scheduled. */
    displayTimer(delay = 8000) {
      return [...timers.values()].find((entry) => entry.delay === delay)?.callback;
    },
    /** Lets the current card's display time run out. */
    endCard(delay = 8000) {
      for (const [id, entry] of timers.entries()) {
        if (entry.delay !== delay) continue;
        timers.delete(id);
        entry.callback?.();
        return true;
      }
      return false;
    },
  };
}

test("notification geometry previews stay visible without consuming live notifications or playing sound", () => {
  const preview = createHarness("widget", "fr", true);
  const card = preview.elements["#notification"];

  assert.match(preview.socket.url, /role=notification&source=notification&client=preview&secret=private$/);

  assert.equal(card.classList.contains("is-visible"), true);
  assert.equal(preview.elements["#notification-author"].textContent, "Aperçu en direct");
  assert.equal(preview.elements["#notification-message"].textContent, "Votre notification apparaîtra ici.");

  preview.socket.emit("message", JSON.stringify({
    type: "config",
    payload: {
      notificationSoundEnabled: true,
      notificationWidgetGeometry: { cropLeft: 15, contentScale: 125 },
    },
  }));
  preview.socket.emit("message", JSON.stringify({ type: "notification", payload: notification("ignored") }));
  preview.socket.emit("message", JSON.stringify({ type: "clear" }));

  assert.equal(preview.cssProperties["--crop-left"], "15%");
  assert.equal(preview.cssProperties["--content-scale"], "1.25");
  assert.equal(preview.elements["#notification-author"].textContent, "Aperçu en direct");
  assert.equal(card.classList.contains("is-visible"), true);
  assert.equal(preview.pings.length, 0);

  const panelSource = fs.readFileSync(__dirname + "/../gui/panel.js", "utf8");
  assert.match(panelSource, /path = metadata\.previewKey === "notificationUrl" \? "\/notifications" : "\/medias"/);
  assert.match(panelSource, /url\.searchParams\.set\("preview", "1"\)/);
});

test("a notification page reloads only after a Relay restart", () => {
  const { socket, location } = createHarness();
  const config = (relaySession) => socket.emit("message", JSON.stringify({ type: "config", payload: { relaySession } }));
  config("first");
  config("first");
  assert.equal(location.reloads, 0);
  config("second");
  assert.equal(location.reloads, 1);
});

test("notification output applies live crop and scale for OBS and widgets", () => {
  const obs = createHarness("obs");
  assert.match(obs.socket.url, /role=notification&source=notification&client=obs&secret=private$/);
  obs.socket.emit("message", JSON.stringify({
    type: "config",
    payload: {
      notificationObsGeometry: {
        cropTop: 3, cropRight: 6, cropBottom: 9, cropLeft: 12, contentScale: 150,
      },
    },
  }));
  assert.equal(obs.cssProperties["--crop-left"], "12%");
  assert.equal(obs.cssProperties["--content-scale"], "1.5");

  const widget = createHarness("widget");
  assert.match(widget.socket.url, /role=notification&source=notification&client=widget&secret=private$/);
  widget.socket.emit("message", JSON.stringify({
    type: "config",
    payload: {
      notificationObsGeometry: { contentScale: 60 },
      notificationWidgetGeometry: { cropBottom: 20, contentScale: 90 },
    },
  }));
  assert.equal(widget.cssProperties["--crop-bottom"], "20%");
  assert.equal(widget.cssProperties["--content-scale"], "0.9");

  const css = fs.readFileSync(__dirname + "/notifications.css", "utf8");
  assert.match(css, /clip-path: inset\(var\(--crop-top\)/);
  assert.match(css, /\.notification-card[\s\S]*var\(--notification-scale\)/);
  assert.match(css, /grid-template-columns: calc\(32px \* var\(--notification-scale\)\)/);
  assert.doesNotMatch(css, /\.notification-card\.is-visible\s*\{[^}]*scale\(/);
  // Idle cards must not paint — opacity:0 + inset:4px left a large black WebView2 hole.
  assert.match(css, /\.notification-card\[aria-hidden="true"\]\s*\{[^}]*display:\s*none/s);
  // Notifications fit the widget width and use only the height their content needs.
  assert.match(
    css,
    /html\.notification-widget \.notification-card\s*\{[^}]*width:\s*100%[^}]*max-width:\s*360px[^}]*height:\s*auto[^}]*max-height:\s*100%[^}]*min-height:\s*0/s,
  );
  // Notification toast geometry fits the native widget, including the rounded bottom.
  assert.match(
    css,
    /html\.notification-widget \.notification-card\s*\{[^}]*--notification-scale:\s*calc\([\s\S]*1\.15/s,
  );
  assert.doesNotMatch(css, /100cqh\s*\/\s*84px/);
  // The always-on presence dot was removed; the redundant side signal stays hidden.
  assert.doesNotMatch(css, /notification-card__presence/);
  assert.match(css, /\.notification-card__signal\s*\{[^}]*display:\s*none/s);
  assert.doesNotMatch(css, /#9fc9ff/);
  assert.doesNotMatch(css, /159 201 255/);
  const source = fs.readFileSync(__dirname + "/../outputs/connection.js", "utf8");
  assert.match(source, /probeWatchdog[\s\S]*probe\.close\(\)/);
  // No outer glow on transparent OBS / widget chrome (opaque cards, no blur halo).
  assert.match(css, /\.notification-card\s*\{[^}]*box-shadow:\s*var\(--out-shadow\)[^}]*filter:\s*none/s);
  assert.match(css, /\.notification-card\s*\{[^}]*border:\s*var\(--out-border\)\s+solid\s+var\(--out-line\)/s);
  // Shared styles may only use hard, opaque offset shadows (no blur radius).
  const theme = fs.readFileSync(__dirname + "/../outputs/theme.css", "utf8");
  for (const [, value] of theme.matchAll(/--out-shadow:\s*([^;]+);/g)) {
    assert.match(value.trim(), /^(none|-?\d+px -?\d+px 0 #[0-9a-f]{6})$/i, value);
  }
  assert.doesNotMatch(theme, /backdrop-filter|drop-shadow|blur\(/);
  assert.doesNotMatch(css, /color-scheme:\s*(dark|only\s+light)/);
  assert.doesNotMatch(css, /backdrop-filter/);
  assert.doesNotMatch(css, /drop-shadow/);
  assert.doesNotMatch(css, /box-shadow:\s*0\s/);
  assert.doesNotMatch(css, /border:\s*1px\s+solid\s+rgb\(255\s+255\s+255\s*\/\s*18%\)/);

  assert.doesNotMatch(css, /min-height:\s*calc\(160px/);
});

test("musicPlay only marks stage occupancy for every notification target", async () => {
  for (const target of ["widget", "obs"]) {
    const harness = createHarness(target);
    if (target === "obs") {
      harness.socket.emit("message", JSON.stringify({
        type: "config",
        payload: { notificationsObsEnabled: true },
      }));
    }
    harness.socket.emit("message", JSON.stringify({
      type: "musicPlay",
      payload: {
        playbackId: `${target}-music`,
        videoId: "dQw4w9WgXcQ",
        title: "Stage-owned track",
      },
    }));
    await Promise.resolve();

    harness.socket.emit("message", JSON.stringify({
      type: "notification",
      payload: notification(`${target}-blocked`),
    }));
    assert.equal(harness.shownId(), null);
    assert.equal(harness.elements["#music"].classList.contains("is-visible"), false);
    assert.notEqual(harness.elements["#music"].attributes.get("aria-hidden"), "false");
    assert.equal(harness.youtubePlayers.length, 0);
    for (const name of [
      "requestMusicNowPlaying",
      "showMusicNowPlaying",
      "beginMusicYoutube",
      "createMusicYoutubePlayer",
    ]) {
      assert.equal(vm.runInContext(`globalThis.${name}`, harness.context), undefined);
      assert.equal(vm.runInContext(`window.${name}`, harness.context), undefined);
    }
    assert.equal(
      harness.socket.sent.some((message) => (
        message.type === "stageClock" && message.payload?.lane === "media"
      )),
      false,
    );
  }
});

test("ordinary notification cleanup leaves the legacy YouTube host untouched", () => {
  const harness = createHarness("widget");
  harness.socket.emit("message", JSON.stringify({
    type: "notification",
    payload: notification("host-stable"),
  }));

  assert.equal(harness.youtubeHostReplacementCount(), 0);
  assert.equal(harness.youtubePlayers.length, 0);
  assert.equal(harness.elements["#music"].classList.contains("is-visible"), false);
});

test("beforeunload leaves an inactive legacy YouTube host untouched", () => {
  const harness = createHarness("widget");
  harness.emitWindow("beforeunload");

  assert.equal(harness.youtubeHostReplacementCount(), 0);
  assert.equal(harness.youtubePlayers.length, 0);
});

test("music occupancy does not replace a notification and queued notifications resume on musicIdle", async () => {
  const { elements, socket, youtubePlayers, shownId, endCard } = createHarness("widget");

  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("live-notification") }));
  socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: { playbackId: "wait-1", videoId: "dQw4w9WgXcQ", title: "Track" },
  }));
  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("queued-notification", "Next") }));

  assert.equal(shownId(), "live-notification");
  assert.equal(elements["#music"].classList.contains("is-visible"), false);
  assert.equal(youtubePlayers.length, 0);

  endCard();
  await nextMicrotask();
  assert.equal(shownId(), null);

  socket.emit("message", JSON.stringify({ type: "musicIdle" }));
  assert.equal(shownId(), "queued-notification");
  assert.equal(elements["#notification-author"].textContent, "Next");
});

test("musicStop keeps music occupancy until authoritative idle", async () => {
  const widget = createHarness("widget");
  widget.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: {
      playbackId: "stop-1",
      videoId: "dQw4w9WgXcQ",
      title: "Track",
    },
  }));
  await Promise.resolve();
  widget.socket.emit("message", JSON.stringify({
    type: "notification",
    payload: notification("wait-after-stop"),
  }));

  widget.socket.emit("message", JSON.stringify({
    type: "musicStop",
    payload: { playbackId: "stop-1" },
  }));
  widget.socket.emit("message", JSON.stringify({
    type: "musicStop",
    payload: { playbackId: "stop-1" },
  }));
  assert.equal(widget.elements["#music"].classList.contains("is-visible"), false);
  assert.equal(widget.youtubePlayers.length, 0);
  assert.equal(widget.shownId(), null);

  widget.socket.emit("message", JSON.stringify({ type: "musicIdle" }));
  assert.equal(widget.shownId(), "wait-after-stop");
});

test("transient WebSocket close retains music stage occupancy without local playback", async () => {
  const widget = createHarness("widget");
  widget.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: { playbackId: "ws-1", videoId: "dQw4w9WgXcQ", title: "Stay" },
  }));
  await Promise.resolve();
  assert.equal(widget.elements["#music"].classList.contains("is-visible"), false);
  assert.equal(widget.youtubePlayers.length, 0);

  widget.socket.emit("close");
  assert.equal(widget.elements["#music"].classList.contains("is-visible"), false);
  assert.equal(widget.youtubePlayers.length, 0);
  widget.socket.emit("message", JSON.stringify({
    type: "notification",
    payload: notification("blocked-after-close"),
  }));
  assert.equal(widget.shownId(), null);
});

test("Windows widget plays the configured notification sound per message", () => {
  const { pings, socket } = createHarness("widget");

  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { notificationSoundEnabled: true, mediaVolume: 80 },
  }));
  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("1") }));

  assert.equal(pings.length, 1);
  assert.equal(pings[0].playCount, 1);
  assert.equal(pings[0].volume, 0.8);
  assert.match(pings[0].src, /^\/notification-sound\?secret=private$/);

  const disabled = createHarness("widget");
  disabled.socket.emit("message", JSON.stringify({ type: "notification", payload: notification("1") }));
  assert.equal(disabled.pings.length, 0);
});

test("OBS notification page plays the sound only with its own toggle", () => {
  const widgetOnly = createHarness("obs");
  widgetOnly.socket.emit("message", JSON.stringify({
    type: "config",
    payload: { notificationsObsEnabled: true, notificationSoundEnabled: true },
  }));
  widgetOnly.socket.emit("message", JSON.stringify({ type: "notification", payload: notification("1") }));
  assert.equal(widgetOnly.pings.length, 0);

  const obsEnabled = createHarness("obs");
  obsEnabled.socket.emit("message", JSON.stringify({
    type: "config",
    payload: { notificationsObsEnabled: true, notificationSoundObsEnabled: true },
  }));
  obsEnabled.socket.emit("message", JSON.stringify({ type: "notification", payload: notification("1") }));
  assert.equal(obsEnabled.pings.length, 1);
  assert.equal(obsEnabled.pings[0].playCount, 1);
});

test("notification widget move label follows the Relay language", () => {
  const english = createHarness("widget", "en");
  const french = createHarness("widget", "fr");
  assert.equal(english.elements["#notification-move-label"].textContent, "Move notification");
  assert.equal(french.elements["#notification-move-label"].textContent, "Déplacer la notification");
});

test("notifications show enabled Discord guild tags without a timestamp", () => {
  const { elements, socket } = createHarness("obs");
  socket.emit("message", JSON.stringify({
    type: "testOutput",
    payload: {
      target: "notification",
      notification: {
        text: "Tagged notification",
        author: { username: "inerthel" },
        guildTag: {
          name: "RE",
          badgeUrl: "https://cdn.discordapp.com/guild-tag-badges/1/badge.png",
        },
        segments: [{ kind: "text", value: "Tagged notification" }],
      },
    },
  }));

  assert.equal(elements["#notification-author"].textContent, "inerthel");
  assert.equal(elements["#notification-guild-tag"].hidden, false);
  assert.equal(elements["#notification-guild-tag-name"].textContent, "RE");
  assert.equal(elements["#notification-guild-tag-badge"].hidden, false);
  assert.match(elements["#notification-guild-tag-badge"].src, /guild-tag-badges/);

  elements["#notification-guild-tag-badge"].onerror();
  assert.equal(elements["#notification-guild-tag-badge"].hidden, true);
});

function notification(id, username = `User ${id}`) {
  return {
    id,
    text: `Message ${id}`,
    author: {
      username,
      displayAvatarUrl: `https://cdn.discordapp.com/avatars/${id}.png`,
    },
  };
}

function nextMicrotask() {
  return new Promise((resolve) => setImmediate(resolve));
}

test("author privacy anonymizes current, pinned and queued notifications on OBS and Windows", () => {
  for (const target of ["obs", "widget"]) {
    const { elements, socket } = createHarness(target, "fr");
    const send = (type, payload) => socket.emit("message", JSON.stringify({ type, payload }));
    const first = {
      ...visualNotification("private"),
      guildTag: { name: "TEAM", badgeUrl: "https://cdn.discordapp.com/badge.png" },
    };
    send("config", { notificationsObsEnabled: true });
    send("notification", first);
    send("messagePin", { pinned: true, message: first });
    const body = elements["#notification-message"].children.map(node => node.textContent).join("");
    const reports = socket.sent.length;
    send("config", { showAuthor: false });
    assert.equal(elements["#notification-author"].textContent, "Anonyme");
    assert.equal(elements["#notification-avatar"].src, "/overlay-assets/relay-radar.png");
    assert.equal(elements["#notification-guild-tag"].hidden, true);
    assert.equal(elements["#notification-guild-tag-name"].textContent, "");
    assert.equal(elements["#notification-guild-tag-badge"].src, "");
    assert.equal(elements["#notification-message"].children.map(node => node.textContent).join(""), body);
    assert.equal(socket.sent.length, reports);
    send("appearance", { language: "de" });
    assert.equal(elements["#notification-author"].textContent, "Anonym");
    send("config", { showAuthor: true });
    assert.equal(elements["#notification-author"].textContent, first.author.username);
    assert.equal(elements["#notification-avatar"].src, first.author.displayAvatarUrl);
    assert.equal(elements["#notification-guild-tag-name"].textContent, "TEAM");
    assert.equal(elements["#notification-guild-tag"].hidden, false);
    send("config", { showAuthor: false });
    send("notification", visualNotification("queued"));
    send("messagePin", { pinned: false });
    assert.equal(elements["#notification-author"].textContent, "Anonym");
    assert.equal(elements["#notification-avatar"].src, "/overlay-assets/relay-radar.png");
    assert.equal(elements["#notification"].classList.contains("is-visible"), true);
    const reconnect = createHarness(target, "fr");
    reconnect.socket.emit("message", JSON.stringify({
      type: "config", payload: { showAuthor: false, notificationsObsEnabled: true },
    }));
    reconnect.socket.emit("message", JSON.stringify({ type: "messagePin", payload: { pinned: true, message: first } }));
    assert.equal(reconnect.elements["#notification-author"].textContent, "Anonyme");
    assert.equal(reconnect.elements["#notification-guild-tag"].hidden, true);
  }
});

test("OBS notifications follow FIFO order, skip, clear, and the configured queue limit", async () => {
  const { elements, socket, shownId, displayTimer } = createHarness();
  const card = elements["#notification"];

  assert.match(socket.url, /role=notification&source=notification&client=obs&secret=private$/);

  socket.emit("message", JSON.stringify({
    type: "notification",
    payload: notification("ignored"),
  }));
  assert.equal(shownId(), null);

  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { notificationsObsEnabled: true, notificationQueueLimit: 1 },
  }));
  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("1", "Alice") }));
  await nextMicrotask();

  assert.equal(shownId(), "1");
  assert.equal(elements["#notification-author"].textContent, "Alice");
  assert.equal(elements["#notification-message"].textContent, "Message 1");
  assert.match(elements["#notification-avatar"].src, /avatars\/1\.png$/);
  assert.equal(card.classList.contains("is-visible"), true);

  const firstEnded = displayTimer();
  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("2") }));
  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("dropped") }));
  firstEnded();
  await nextMicrotask();

  assert.equal(shownId(), "2");
  // A stale timer from the previous card never ends the new one.
  firstEnded();
  assert.equal(shownId(), "2");

  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("3") }));
  socket.emit("message", JSON.stringify({ type: "skip" }));
  await nextMicrotask();
  assert.equal(shownId(), "3");

  socket.emit("message", JSON.stringify({ type: "clear" }));
  assert.equal(shownId(), null);
  assert.equal(card.classList.contains("is-visible"), false);
});

test("local notification tests bypass the OBS delivery toggle", () => {
  const { elements, socket } = createHarness("obs");
  socket.emit("message", JSON.stringify({
    type: "testOutput",
    payload: {
      target: "notification",
      notification: {
        text: "Relay notification test",
        author: { username: "Relay test" },
        segments: [{ kind: "text", value: "Relay notification test" }],
      },
    },
  }));

  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
  assert.equal(elements["#notification-author"].textContent, "Relay test");
  assert.equal(elements["#notification-message"].children[0].textContent, "Relay notification test");
});

test("Windows notification widget remains independent from the OBS toggle", async () => {
  const { elements, socket, shownId } = createHarness("widget");

  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { notificationsObsEnabled: false, notificationQueueLimit: 50 },
  }));
  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("widget") }));
  await nextMicrotask();

  assert.equal(shownId(), "widget");
  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
});

test("OBS and Windows outputs keep separate queues while displaying the same notification", async () => {
  const obs = createHarness("obs");
  const windows = createHarness("widget");
  const config = JSON.stringify({
    type: "config",
    payload: { notificationsObsEnabled: true, notificationQueueLimit: 50 },
  });
  obs.socket.emit("message", config);
  windows.socket.emit("message", config);

  const event = JSON.stringify({ type: "notification", payload: notification("shared") });
  obs.socket.emit("message", event);
  windows.socket.emit("message", event);
  await nextMicrotask();

  assert.equal(obs.shownId(), "shared");
  assert.equal(windows.shownId(), "shared");
  assert.equal(obs.elements["#notification"].classList.contains("is-visible"), true);
  assert.equal(windows.elements["#notification"].classList.contains("is-visible"), true);

  obs.socket.emit("message", JSON.stringify({ type: "clear" }));
  assert.equal(obs.elements["#notification"].classList.contains("is-visible"), false);
  assert.equal(obs.shownId(), null);
  assert.equal(windows.elements["#notification"].classList.contains("is-visible"), true);
  assert.equal(windows.shownId(), "shared");
});

test("notification playback clears when Relay disconnects", async () => {
  const { elements, socket } = createHarness("widget");
  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("disconnect") }));
  await nextMicrotask();
  socket.emit("close");
  assert.equal(elements["#notification"].attributes.get("aria-hidden"), "true");
});

test("emoji messages render visually", () => {
  const { elements, socket } = createHarness("widget");
  socket.emit("message", JSON.stringify({
    type: "notification",
    payload: {
      ...notification("emoji", "Emoji user"),
      segments: [
        { kind: "text", value: "Hello " },
        { kind: "emoji", value: "👋", url: null },
        { kind: "emoji", value: ":relay:", url: "https://cdn.discordapp.com/emojis/1.webp" },
      ],
    },
  }));

  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
  assert.equal(elements["#notification-message"].children.length, 3);
  assert.equal(elements["#notification-message"].children[2].tagName, "img");
});

test("Discord stickers render visually in notifications", () => {
  const { elements, socket } = createHarness("widget");
  socket.emit("message", JSON.stringify({
    type: "notification",
    payload: {
      ...notification("sticker", "Sticker user"),
      segments: [{ kind: "sticker", value: "Relay dance", url: "https://media.discordapp.net/stickers/1.gif" }],
    },
  }));

  const message = elements["#notification-message"];
  assert.equal(message.children.length, 1);
  assert.equal(message.children[0].tagName, "img");
  assert.equal(message.children[0].className, "notification-card__sticker");
  assert.equal(message.children[0].alt, "Relay dance");
});

test("emoji then plain text messages both notify and keep the queue moving", async () => {
  const { elements, socket, shownId, endCard } = createHarness("widget");
  const card = elements["#notification"];

  socket.emit("message", JSON.stringify({
    type: "notification",
    payload: {
      ...notification("emoji-first"),
      segments: [{ kind: "emoji", value: "\u{1F44B}", url: null }],
    },
  }));
  assert.equal(card.classList.contains("is-visible"), true);
  assert.equal(shownId(), "emoji-first");

  // The next message waits its turn instead of cutting the current card.
  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("test", "Tester") }));
  assert.equal(shownId(), "emoji-first");
  endCard();
  await nextMicrotask();
  assert.equal(elements["#notification-author"].textContent, "Tester");
  assert.equal(shownId(), "test");

  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("second", "Second user") }));
  endCard();
  await nextMicrotask();
  assert.equal(elements["#notification-author"].textContent, "Second user");
  assert.equal(shownId(), "second");
  assert.equal(card.classList.contains("is-visible"), true);
});

test("visual notifications stay visible for the configured duration", () => {
  const { elements, socket, timerDelays } = createHarness("widget");
  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { notificationDurationMs: 12000 },
  }));
  socket.emit("message", JSON.stringify({
    type: "notification",
    payload: {
      ...notification("timed"),
      segments: [{ kind: "emoji", value: "👋", url: null }],
    },
  }));

  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
  assert.equal(timerDelays.at(-1), 12000);
});

test("visual notifications fall back to eight seconds without a configured duration", () => {
  const { socket, timerDelays } = createHarness("widget");
  socket.emit("message", JSON.stringify({
    type: "notification",
    payload: {
      ...notification("default-timed"),
      segments: [{ kind: "emoji", value: "👋", url: null }],
    },
  }));

  assert.equal(timerDelays.at(-1), 8000);
});

test("notifications wait for the media stage and YouTube", () => {
  const { elements, socket, shownId } = createHarness("widget");

  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: true, notificationBusy: false },
  }));
  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("queued-media") }));
  assert.equal(shownId(), null);
  assert.equal(elements["#notification"].classList.contains("is-visible"), false);

  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, notificationBusy: false },
  }));
  assert.equal(shownId(), "queued-media");
  assert.deepEqual(socket.sent.at(-1), {
    type: "stageClock",
    payload: { lane: "notification", busy: true },
  });

  const music = createHarness("widget");
  music.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: { playbackId: "yt-1", title: "Track", durationSeconds: 30 },
  }));
  music.socket.emit("message", JSON.stringify({ type: "notification", payload: notification("queued-yt") }));
  assert.equal(music.shownId(), null);

  music.socket.emit("message", JSON.stringify({ type: "musicIdle" }));
  assert.equal(music.shownId(), "queued-yt");
});

test("notifications wait for the server stage grant before becoming visible", () => {
  const { elements, socket, shownId } = createHarness("widget", "en", false, false);
  socket.emit("message", JSON.stringify({
    type: "notification",
    payload: notification("race"),
  }));

  assert.equal(elements["#notification"].classList.contains("is-visible"), false);
  assert.equal(shownId(), null);
  assert.deepEqual(socket.sent.at(-1), {
    type: "stageClock",
    payload: { lane: "notification", busy: true },
  });

  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, notificationBusy: true },
  }));
  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
  assert.equal(shownId(), "race");
});

test("OBS hides the completed card while the next message waits for media", () => {
  const { elements, socket, shownId, endCard } = createHarness("obs", "en", false, false);
  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { notificationsObsEnabled: true },
  }));
  socket.emit("message", JSON.stringify({
    type: "notification",
    payload: notification("first"),
  }));
  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, notificationBusy: true },
  }));
  socket.emit("message", JSON.stringify({
    type: "notification",
    payload: notification("second"),
  }));
  assert.equal(elements["#notification"].classList.contains("is-visible"), true);

  endCard();

  assert.equal(elements["#notification"].classList.contains("is-visible"), false);
  assert.equal(shownId(), null);
  assert.deepEqual(socket.sent.at(-1), {
    type: "stageClock",
    payload: { lane: "notification", busy: true },
  });

  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: {
      mediaBusy: true,
      musicBusy: false,
      notificationBusy: false,
      granted: false,
      lane: "notification",
    },
  }));
  assert.equal(elements["#notification"].classList.contains("is-visible"), false);
});

test("OBS notifications recover when stageClock clears a missed musicIdle", () => {
  const { elements, socket, shownId } = createHarness("obs");
  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { notificationsObsEnabled: true },
  }));
  // Simulate a lagged client that still thinks YouTube is active.
  socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: { playbackId: "stuck", title: "Track", durationSeconds: 30 },
  }));
  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("blocked") }));
  assert.equal(elements["#notification"].classList.contains("is-visible"), false);
  assert.equal(shownId(), null);

  // Server watch clock recovers without a musicIdle relay event.
  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, notificationBusy: false },
  }));
  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
  assert.equal(shownId(), "blocked");
});

test("skip and clear release the notification stage without leaving it stuck", () => {
  const { elements, socket } = createHarness("widget");
  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("live") }));
  assert.deepEqual(socket.sent.at(-1), {
    type: "stageClock",
    payload: { lane: "notification", busy: true },
  });

  socket.emit("message", JSON.stringify({ type: "skip" }));
  assert.deepEqual(socket.sent.at(-1), {
    type: "stageClock",
    payload: { lane: "notification", busy: false },
  });

  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("again") }));
  socket.emit("message", JSON.stringify({ type: "clear" }));
  assert.deepEqual(socket.sent.at(-1), {
    type: "stageClock",
    payload: { lane: "notification", busy: false },
  });
  assert.equal(elements["#notification"].classList.contains("is-visible"), false);
});

test("sticker-only notifications use a compact layout and plain text resets it", () => {
  const { socket, elements } = createHarness("widget");
  socket.emit("message", JSON.stringify({ type: "testOutput", payload: { target: "notification", notification: { segments: [{ kind: "sticker", url: "demo.png" }] } } }));
  assert.equal(elements["#notification"].classList.contains("is-sticker-only"), true);
  socket.emit("message", JSON.stringify({ type: "clear" }));
  socket.emit("message", JSON.stringify({ type: "testOutput", payload: { target: "notification", notification: { text: "Next message" } } }));
  assert.equal(elements["#notification"].classList.contains("is-sticker-only"), false);
});

test("the compact card follows its text, shows three lines and animates its exit", () => {
  const css = fs.readFileSync(__dirname + "/notifications.css", "utf8");
  const card = css.slice(css.indexOf(".notification-card {"), css.indexOf("\n}", css.indexOf(".notification-card {")));
  assert.match(card, /width: fit-content/);
  assert.match(card, /min-height: calc\(56px \* var\(--notification-scale\)\)/);
  assert.match(card, /max-width: min\(calc\(340px/);
  // Colors, borders and motion come from the shared stream style.
  assert.match(card, /background-color: var\(--out-surface\)/);
  assert.match(card, /border: var\(--out-border\) solid var\(--out-line\)/);
  assert.match(card, /display var\(--out-exit-duration\) allow-discrete/);
  assert.match(css, /@starting-style\s*\{\s*\.notification-card\.is-visible/);
  assert.match(css, /\.notification-card__message\s*\{[^}]*-webkit-line-clamp: 3/s);
  // No overshooting curve: the entrance no longer bounces.
  assert.doesNotMatch(css, /cubic-bezier\([^)]*1\.12\)/);
  const html = fs.readFileSync(__dirname + "/index.html", "utf8");
  assert.ok(html.indexOf("/output-theme.css") < html.indexOf("notifications.css"));
  assert.ok(html.indexOf("/output-theme.js") < html.indexOf("notifications.js"));
});

test("appearance messages set the stream style on the notification page", () => {
  const harness = createHarness("obs", "en", false, true, { withTheme: true });
  const root = harness.context.document.documentElement;
  assert.equal(root.dataset.outputStyle, "graphite");
  assert.equal(root.dataset.outputBackground, "dark");
  harness.socket.emit("message", JSON.stringify({
    type: "appearance",
    payload: { language: "en", theme: "light", accentRgb: [20, 20, 120], fontScale: 100, design: "paper", outputStyle: "auto", outputBackground: "auto" },
  }));
  assert.equal(root.dataset.outputStyle, "paper");
  assert.equal(root.dataset.outputBackground, "light");
  assert.equal(harness.cssProperties["--accent-ink"], "#ffffff");
  harness.socket.emit("message", JSON.stringify({
    type: "appearance",
    payload: { language: "en", theme: "light", accentRgb: [88, 185, 137], fontScale: 100, design: "paper", outputStyle: "subtitle", outputBackground: "dark" },
  }));
  assert.equal(root.dataset.outputStyle, "subtitle");
  assert.equal(root.dataset.outputBackground, "dark");
});

function visualNotification(id, username = `User ${id}`) {
  return {
    ...notification(id, username),
    segments: [{ kind: "text", value: `Message ${id}` }],
  };
}

test("a pinned visual notification stays visible while later messages queue", () => {
  const { elements, socket, timers } = createHarness("widget");
  const first = visualNotification("pinned");

  socket.emit("message", JSON.stringify({ type: "notification", payload: first }));
  socket.emit("message", JSON.stringify({
    type: "messagePin",
    payload: { pinned: true, message: first },
  }));
  assert.equal(elements["#notification-author"].textContent, "User pinned");
  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
  assert.equal([...timers.values()].some(({ delay }) => delay === 8000), false);

  socket.emit("message", JSON.stringify({ type: "notification", payload: visualNotification("queued") }));
  assert.equal(elements["#notification-author"].textContent, "User pinned");

  socket.emit("message", JSON.stringify({ type: "messagePin", payload: { pinned: false } }));
  assert.equal(elements["#notification-author"].textContent, "User queued");
  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
});

test("a pinned plain-text notification keeps its card and the queue resumes on removal", async () => {
  const { elements, socket, timers } = createHarness("widget");
  const first = notification("spoken-pin");
  socket.emit("message", JSON.stringify({ type: "notification", payload: first }));
  socket.emit("message", JSON.stringify({
    type: "messagePin",
    payload: { pinned: true, message: first },
  }));
  assert.equal(elements["#notification"].classList.contains("is-visible"), true);

  socket.emit("message", JSON.stringify({ type: "notification", payload: notification("after-pin") }));
  assert.equal(elements["#notification-author"].textContent, "User spoken-pin");
  socket.emit("message", JSON.stringify({ type: "messagePin", payload: { pinned: false } }));
  await nextMicrotask();
  assert.equal(elements["#notification-author"].textContent, "User after-pin");
});

test("reconnected notifications wait for the pin snapshot before draining queued messages", () => {
  const { elements, socket, sockets, runNextTimer } = createHarness("widget");
  const first = visualNotification("reconnect-pin");
  socket.emit("message", JSON.stringify({ type: "notification", payload: first }));
  socket.emit("message", JSON.stringify({ type: "messagePin", payload: { pinned: true, message: first } }));
  socket.emit("message", JSON.stringify({ type: "notification", payload: visualNotification("reconnect-queued") }));

  socket.emit("close");
  assert.equal(elements["#notification"].classList.contains("is-visible"), false);
  runNextTimer();
  const reconnected = sockets.at(-1);
  reconnected.emit("open");
  assert.equal(elements["#notification"].classList.contains("is-visible"), false);
  reconnected.emit("message", JSON.stringify({
    type: "messagePin",
    payload: { pinned: true, message: first },
  }));
  assert.equal(elements["#notification-author"].textContent, "User reconnect-pin");
});
