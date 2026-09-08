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

function createHarness(target = "obs", language = "en", preview = false, autoGrantStage = true) {
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
    "#notification-clock": {
      src: "",
      muted: false,
      onended: null,
      onerror: null,
      pause() {},
      load() {},
      play() { return Promise.resolve(); },
      removeAttribute(name) {
        if (name === "src") this.src = "";
      },
    },
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
        && message.payload?.lane === "tts"
        && message.payload?.busy === true
      ) {
        this.emit("message", JSON.stringify({
          type: "stageClock",
          payload: { mediaBusy: false, musicBusy: false, ttsBusy: true },
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
  vm.runInContext(source, context);

  return {
    context,
    cssProperties,
    elements,
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
  };
}

test("notification geometry previews stay visible without consuming live TTS or playing sound", () => {
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
  preview.socket.emit("message", JSON.stringify({ type: "tts", payload: notification("ignored") }));
  preview.socket.emit("message", JSON.stringify({ type: "clear" }));

  assert.equal(preview.cssProperties["--crop-left"], "15%");
  assert.equal(preview.cssProperties["--content-scale"], "1.25");
  assert.equal(preview.elements["#notification-author"].textContent, "Aperçu en direct");
  assert.equal(card.classList.contains("is-visible"), true);
  assert.equal(preview.elements["#notification-clock"].src, "");
  assert.equal(preview.pings.length, 0);

  const panelSource = fs.readFileSync(__dirname + "/../gui/panel.js", "utf8");
  assert.match(panelSource, /path = metadata\.previewKey === "notificationUrl" \? "\/notifications" : "\/medias"/);
  assert.match(panelSource, /url\.searchParams\.set\("preview", "1"\)/);
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
  // TTS toast geometry fits the native widget, including the rounded bottom.
  assert.match(
    css,
    /html\.notification-widget \.notification-card\s*\{[^}]*--notification-scale:\s*calc\([\s\S]*1\.15/s,
  );
  assert.doesNotMatch(css, /100cqh\s*\/\s*84px/);
  // The presence dot keeps the chosen accent; the redundant side signal is hidden.
  assert.match(css, /\.notification-card__presence,[^}]*background:\s*var\(--accent\)/s);
  assert.match(css, /\.notification-card__signal\s*\{[^}]*display:\s*none/s);
  assert.doesNotMatch(css, /#9fc9ff/);
  assert.doesNotMatch(css, /159 201 255/);
  const source = fs.readFileSync(__dirname + "/notifications.js", "utf8");
  assert.match(source, /probeWatchdog[\s\S]*probe\.close\(\)/);
  // No outer glow on transparent OBS / widget chrome (opaque cards, no blur halo).
  assert.match(css, /\.notification-card\s*\{[^}]*box-shadow:\s*none[^}]*filter:\s*none/s);
  assert.match(css, /\.notification-card\s*\{[^}]*border:\s*1px\s+solid\s+#2a2e36/s);
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
        payload: { ttsNotificationsObsEnabled: true },
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
      type: "tts",
      payload: notification(`${target}-blocked`),
    }));
    assert.equal(harness.elements["#notification-clock"].src, "");
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

test("ordinary TTS cleanup leaves the legacy YouTube host untouched", () => {
  const harness = createHarness("widget");
  harness.socket.emit("message", JSON.stringify({
    type: "tts",
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

test("music occupancy does not replace TTS and queued TTS resumes on musicIdle", async () => {
  const { elements, socket, youtubePlayers } = createHarness("widget");
  const audio = elements["#notification-clock"];

  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("live-tts") }));
  socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: { playbackId: "wait-1", videoId: "dQw4w9WgXcQ", title: "Track" },
  }));
  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("queued-tts", "Next") }));

  assert.match(audio.src, /\/tts-audio\/live-tts\?secret=private$/);
  assert.equal(elements["#music"].classList.contains("is-visible"), false);
  assert.equal(youtubePlayers.length, 0);

  audio.onended();
  await nextMicrotask();
  assert.equal(audio.src, "");

  socket.emit("message", JSON.stringify({ type: "musicIdle" }));
  assert.match(audio.src, /\/tts-audio\/queued-tts\?secret=private$/);
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
    type: "tts",
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
  assert.equal(widget.elements["#notification-clock"].src, "");

  widget.socket.emit("message", JSON.stringify({ type: "musicIdle" }));
  assert.match(
    widget.elements["#notification-clock"].src,
    /\/tts-audio\/wait-after-stop\?secret=private$/,
  );
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
    type: "tts",
    payload: notification("blocked-after-close"),
  }));
  assert.equal(widget.elements["#notification-clock"].src, "");
});

test("Windows widget plays the configured notification sound per message", () => {
  const { pings, socket } = createHarness("widget");

  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { notificationSoundEnabled: true, mediaVolume: 80 },
  }));
  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("1") }));

  assert.equal(pings.length, 1);
  assert.equal(pings[0].playCount, 1);
  assert.equal(pings[0].volume, 0.8);
  assert.match(pings[0].src, /^\/notification-sound\?secret=private$/);

  const disabled = createHarness("widget");
  disabled.socket.emit("message", JSON.stringify({ type: "tts", payload: notification("1") }));
  assert.equal(disabled.pings.length, 0);
});

test("OBS notification page plays the sound only with its own toggle", () => {
  const widgetOnly = createHarness("obs");
  widgetOnly.socket.emit("message", JSON.stringify({
    type: "config",
    payload: { ttsNotificationsObsEnabled: true, notificationSoundEnabled: true },
  }));
  widgetOnly.socket.emit("message", JSON.stringify({ type: "tts", payload: notification("1") }));
  assert.equal(widgetOnly.pings.length, 0);

  const obsEnabled = createHarness("obs");
  obsEnabled.socket.emit("message", JSON.stringify({
    type: "config",
    payload: { ttsNotificationsObsEnabled: true, notificationSoundObsEnabled: true },
  }));
  obsEnabled.socket.emit("message", JSON.stringify({ type: "tts", payload: notification("1") }));
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
      tts: {
        text: "Tagged notification",
        visualOnly: true,
        author: { username: "Stealthy." },
        guildTag: {
          name: "RE",
          badgeUrl: "https://cdn.discordapp.com/guild-tag-badges/1/badge.png",
        },
        segments: [{ kind: "text", value: "Tagged notification" }],
      },
    },
  }));

  assert.equal(elements["#notification-author"].textContent, "Stealthy.");
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

test("OBS notifications follow TTS FIFO, skip, clear, and configured queue limit", async () => {
  const { elements, socket } = createHarness();
  const card = elements["#notification"];
  const audio = elements["#notification-clock"];

  assert.match(socket.url, /role=notification&source=notification&client=obs&secret=private$/);
  assert.equal(audio.muted, true);

  socket.emit("message", JSON.stringify({
    type: "tts",
    payload: notification("ignored"),
  }));
  assert.equal(audio.src, "");

  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { ttsNotificationsObsEnabled: true, ttsQueueLimit: 1 },
  }));
  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("1", "Alice") }));
  await nextMicrotask();

  assert.match(audio.src, /\/tts-audio\/1\?secret=private$/);
  assert.equal(elements["#notification-author"].textContent, "Alice");
  assert.equal(elements["#notification-message"].textContent, "Message 1");
  assert.match(elements["#notification-avatar"].src, /avatars\/1\.png$/);
  assert.equal(card.classList.contains("is-visible"), true);

  const firstEnded = audio.onended;
  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("2") }));
  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("dropped") }));
  firstEnded();
  await nextMicrotask();

  assert.match(audio.src, /\/tts-audio\/2\?secret=private$/);
  firstEnded();
  assert.match(audio.src, /\/tts-audio\/2\?secret=private$/);

  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("3") }));
  socket.emit("message", JSON.stringify({ type: "skip" }));
  await nextMicrotask();
  assert.match(audio.src, /\/tts-audio\/3\?secret=private$/);

  socket.emit("message", JSON.stringify({ type: "clear" }));
  assert.equal(audio.src, "");
  assert.equal(audio.onended, null);
  assert.equal(card.classList.contains("is-visible"), false);
});

test("local notification tests bypass the OBS delivery toggle", () => {
  const { elements, socket } = createHarness("obs");
  socket.emit("message", JSON.stringify({
    type: "testOutput",
    payload: {
      target: "notification",
      tts: {
        text: "Relay notification test",
        visualOnly: true,
        author: { username: "Relay test" },
        segments: [{ kind: "text", value: "Relay notification test" }],
      },
    },
  }));

  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
  assert.equal(elements["#notification-author"].textContent, "Relay test");
  assert.equal(elements["#notification-message"].children[0].textContent, "Relay notification test");
  assert.equal(elements["#notification-clock"].src, "");
});

test("Windows notification widget remains independent from the OBS toggle", async () => {
  const { elements, socket } = createHarness("widget");
  const audio = elements["#notification-clock"];

  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { ttsNotificationsObsEnabled: false, ttsQueueLimit: 50 },
  }));
  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("widget") }));
  await nextMicrotask();

  assert.match(audio.src, /\/tts-audio\/widget\?secret=private$/);
  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
});

test("OBS and Windows outputs keep separate queues while displaying the same TTS event", async () => {
  const obs = createHarness("obs");
  const windows = createHarness("widget");
  const config = JSON.stringify({
    type: "config",
    payload: { ttsNotificationsObsEnabled: true, ttsQueueLimit: 50 },
  });
  obs.socket.emit("message", config);
  windows.socket.emit("message", config);

  const event = JSON.stringify({ type: "tts", payload: notification("shared") });
  obs.socket.emit("message", event);
  windows.socket.emit("message", event);
  await nextMicrotask();

  assert.match(obs.elements["#notification-clock"].src, /\/tts-audio\/shared\?secret=private$/);
  assert.match(windows.elements["#notification-clock"].src, /\/tts-audio\/shared\?secret=private$/);
  assert.equal(obs.elements["#notification"].classList.contains("is-visible"), true);
  assert.equal(windows.elements["#notification"].classList.contains("is-visible"), true);

  obs.socket.emit("message", JSON.stringify({ type: "clear" }));
  assert.equal(obs.elements["#notification"].classList.contains("is-visible"), false);
  assert.equal(obs.elements["#notification-clock"].src, "");
  assert.equal(windows.elements["#notification"].classList.contains("is-visible"), true);
  assert.match(windows.elements["#notification-clock"].src, /\/tts-audio\/shared\?secret=private$/);
});

test("notification playback clears when Relay disconnects", async () => {
  const { elements, socket } = createHarness("widget");
  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("disconnect") }));
  await nextMicrotask();
  socket.emit("close");
  assert.equal(elements["#notification-clock"].src, "");
  assert.equal(elements["#notification"].attributes.get("aria-hidden"), "true");
});

test("emoji messages render visually without requesting TTS audio", () => {
  const { elements, socket } = createHarness("widget");
  socket.emit("message", JSON.stringify({
    type: "tts",
    payload: {
      ...notification("emoji", "Emoji user"),
      visualOnly: true,
      segments: [
        { kind: "text", value: "Hello " },
        { kind: "emoji", value: "👋", url: null },
        { kind: "emoji", value: ":relay:", url: "https://cdn.discordapp.com/emojis/1.webp" },
      ],
    },
  }));

  assert.equal(elements["#notification-clock"].src, "");
  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
  assert.equal(elements["#notification-message"].children.length, 3);
  assert.equal(elements["#notification-message"].children[2].tagName, "img");
});

test("Discord stickers render visually in TTS notifications", () => {
  const { elements, socket } = createHarness("widget");
  socket.emit("message", JSON.stringify({
    type: "tts",
    payload: {
      ...notification("sticker", "Sticker user"),
      visualOnly: true,
      segments: [{ kind: "sticker", value: "Relay dance", url: "https://media.discordapp.net/stickers/1.gif" }],
    },
  }));

  const message = elements["#notification-message"];
  assert.equal(elements["#notification-clock"].src, "");
  assert.equal(message.children.length, 1);
  assert.equal(message.children[0].tagName, "img");
  assert.equal(message.children[0].className, "notification-card__sticker");
  assert.equal(message.children[0].alt, "Relay dance");
});

test("a spoken notification stays visible even when audio playback fails", async () => {
  const { elements, socket, timers } = createHarness("widget");
  const card = elements["#notification"];
  const audio = elements["#notification-clock"];
  audio.play = () => Promise.reject(new Error("playback blocked"));

  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("silent", "Muted") }));
  assert.equal(card.classList.contains("is-visible"), true);
  await nextMicrotask();

  assert.equal(card.classList.contains("is-visible"), true);
  assert.equal(elements["#notification-author"].textContent, "Muted");
  assert.equal(elements["#notification-message"].textContent, "Message silent");

  for (const entry of [...timers.values()]) entry.callback();
  assert.equal(card.classList.contains("is-visible"), false);

  audio.play = () => Promise.resolve();
  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("next", "Speaker") }));
  await nextMicrotask();
  assert.match(audio.src, /\/tts-audio\/next\?secret=private$/);
  assert.equal(card.classList.contains("is-visible"), true);
});

test("emoji then plain text messages both notify and keep the queue moving", async () => {
  const { elements, socket } = createHarness("widget");
  const card = elements["#notification"];
  const audio = elements["#notification-clock"];

  socket.emit("message", JSON.stringify({
    type: "tts",
    payload: {
      ...notification("emoji-first"),
      visualOnly: true,
      segments: [{ kind: "emoji", value: "👋", url: null }],
    },
  }));
  assert.equal(card.classList.contains("is-visible"), true);
  assert.equal(audio.src, "");

  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("test", "Tester") }));
  await nextMicrotask();
  assert.equal(card.classList.contains("is-visible"), true);
  assert.equal(elements["#notification-author"].textContent, "Tester");
  assert.match(audio.src, /\/tts-audio\/test\?secret=private$/);

  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("second", "Second user") }));
  audio.onended();
  await nextMicrotask();
  assert.equal(elements["#notification-author"].textContent, "Second user");
  assert.match(audio.src, /\/tts-audio\/second\?secret=private$/);
  assert.equal(card.classList.contains("is-visible"), true);
});

test("visual notifications stay visible for the configured duration", () => {
  const { elements, socket, timerDelays } = createHarness("widget");
  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { notificationDurationMs: 12000 },
  }));
  socket.emit("message", JSON.stringify({
    type: "tts",
    payload: {
      ...notification("timed"),
      visualOnly: true,
      segments: [{ kind: "emoji", value: "👋", url: null }],
    },
  }));

  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
  assert.equal(timerDelays.at(-1), 12000);
});

test("visual notifications fall back to eight seconds without a configured duration", () => {
  const { socket, timerDelays } = createHarness("widget");
  socket.emit("message", JSON.stringify({
    type: "tts",
    payload: {
      ...notification("default-timed"),
      visualOnly: true,
      segments: [{ kind: "emoji", value: "👋", url: null }],
    },
  }));

  assert.equal(timerDelays.at(-1), 8000);
});

test("a visual emoji notification never blocks the next spoken message", async () => {
  const { elements, socket } = createHarness("widget");
  socket.emit("message", JSON.stringify({
    type: "tts",
    payload: {
      ...notification("emoji"),
      visualOnly: true,
      segments: [{ kind: "emoji", value: "👋", url: null }],
    },
  }));
  socket.emit("message", JSON.stringify({
    type: "tts",
    payload: notification("after-emoji", "Friend"),
  }));
  await nextMicrotask();

  assert.match(elements["#notification-clock"].src, /\/tts-audio\/after-emoji\?secret=private$/);
  assert.equal(elements["#notification-author"].textContent, "Friend");
  assert.equal(elements["#notification-message"].textContent, "Message after-emoji");
});

test("TTS waits for media stage and YouTube before playing", () => {
  const { elements, socket } = createHarness("widget");

  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: true, ttsBusy: false },
  }));
  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("queued-media") }));
  assert.equal(elements["#notification-clock"].src, "");
  assert.equal(elements["#notification"].classList.contains("is-visible"), false);

  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, ttsBusy: false },
  }));
  assert.match(elements["#notification-clock"].src, /\/tts-audio\/queued-media\?secret=private$/);
  assert.deepEqual(socket.sent.at(-1), {
    type: "stageClock",
    payload: { lane: "tts", busy: true },
  });

  const music = createHarness("widget");
  music.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: { playbackId: "yt-1", title: "Track", durationSeconds: 30 },
  }));
  music.socket.emit("message", JSON.stringify({ type: "tts", payload: notification("queued-yt") }));
  assert.equal(music.elements["#notification-clock"].src, "");

  music.socket.emit("message", JSON.stringify({ type: "musicIdle" }));
  assert.match(music.elements["#notification-clock"].src, /\/tts-audio\/queued-yt\?secret=private$/);
});

test("TTS waits for the server stage grant before becoming visible", () => {
  const { elements, socket } = createHarness("widget", "en", false, false);
  socket.emit("message", JSON.stringify({
    type: "tts",
    payload: notification("race"),
  }));

  assert.equal(elements["#notification"].classList.contains("is-visible"), false);
  assert.equal(elements["#notification-clock"].src, "");
  assert.deepEqual(socket.sent.at(-1), {
    type: "stageClock",
    payload: { lane: "tts", busy: true },
  });

  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, ttsBusy: true },
  }));
  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
  assert.match(elements["#notification-clock"].src, /\/tts-audio\/race\?secret=private$/);
});

test("OBS hides the completed TTS card while the next message waits for media", () => {
  const { elements, socket } = createHarness("obs", "en", false, false);
  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { ttsNotificationsObsEnabled: true },
  }));
  socket.emit("message", JSON.stringify({
    type: "tts",
    payload: notification("first"),
  }));
  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, ttsBusy: true },
  }));
  socket.emit("message", JSON.stringify({
    type: "tts",
    payload: notification("second"),
  }));
  assert.equal(elements["#notification"].classList.contains("is-visible"), true);

  elements["#notification-clock"].onended();

  assert.equal(elements["#notification"].classList.contains("is-visible"), false);
  assert.match(elements["#notification-clock"].src, /^$/);
  assert.deepEqual(socket.sent.at(-1), {
    type: "stageClock",
    payload: { lane: "tts", busy: true },
  });

  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: {
      mediaBusy: true,
      musicBusy: false,
      ttsBusy: false,
      granted: false,
      lane: "tts",
    },
  }));
  assert.equal(elements["#notification"].classList.contains("is-visible"), false);
});

test("OBS notifications recover when stageClock clears a missed musicIdle", () => {
  const { elements, socket } = createHarness("obs");
  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { ttsNotificationsObsEnabled: true },
  }));
  // Simulate a lagged client that still thinks YouTube is active.
  socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: { playbackId: "stuck", title: "Track", durationSeconds: 30 },
  }));
  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("blocked") }));
  assert.equal(elements["#notification"].classList.contains("is-visible"), false);
  assert.equal(elements["#notification-clock"].src, "");

  // Server watch clock recovers without a musicIdle relay event.
  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, ttsBusy: false },
  }));
  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
  assert.match(elements["#notification-clock"].src, /\/tts-audio\/blocked\?secret=private$/);
});

test("skip and clear release the TTS stage without leaving it stuck", () => {
  const { elements, socket } = createHarness("widget");
  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("live") }));
  assert.deepEqual(socket.sent.at(-1), {
    type: "stageClock",
    payload: { lane: "tts", busy: true },
  });

  socket.emit("message", JSON.stringify({ type: "skip" }));
  assert.equal(elements["#notification-clock"].src, "");
  assert.deepEqual(socket.sent.at(-1), {
    type: "stageClock",
    payload: { lane: "tts", busy: false },
  });

  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("again") }));
  socket.emit("message", JSON.stringify({ type: "clear" }));
  assert.deepEqual(socket.sent.at(-1), {
    type: "stageClock",
    payload: { lane: "tts", busy: false },
  });
  assert.equal(elements["#notification"].classList.contains("is-visible"), false);
});

test("sticker-only notifications use a compact layout and plain text resets it", () => {
  const { socket, elements } = createHarness("widget");
  socket.emit("message", JSON.stringify({ type: "testOutput", payload: { target: "notification", tts: { visualOnly: true, segments: [{ kind: "sticker", url: "demo.png" }] } } }));
  assert.equal(elements["#notification"].classList.contains("is-sticker-only"), true);
  socket.emit("message", JSON.stringify({ type: "clear" }));
  socket.emit("message", JSON.stringify({ type: "testOutput", payload: { target: "notification", tts: { visualOnly: true, text: "Next message" } } }));
  assert.equal(elements["#notification"].classList.contains("is-sticker-only"), false);
});

test("presence indicator includes its border within a small proportional diameter", () => {
  const css = fs.readFileSync(__dirname + "/notifications.css", "utf8");
  const dot = css.slice(css.indexOf(".notification-card__presence {"), css.indexOf("\n}", css.indexOf(".notification-card__presence {")));
  assert.match(dot, /box-sizing: border-box/);
  assert.match(dot, /width: calc\(9px \* var\(--notification-scale\)\)/);
  assert.match(dot, /height: calc\(9px \* var\(--notification-scale\)\)/);
});

function visualNotification(id, username = `User ${id}`) {
  return {
    ...notification(id, username),
    visualOnly: true,
    segments: [{ kind: "text", value: `Message ${id}` }],
  };
}

test("a pinned visual notification stays visible while later messages queue", () => {
  const { elements, socket, timers } = createHarness("widget");
  const first = visualNotification("pinned");

  socket.emit("message", JSON.stringify({ type: "tts", payload: first }));
  socket.emit("message", JSON.stringify({
    type: "messagePin",
    payload: { pinned: true, message: first },
  }));
  assert.equal(elements["#notification-author"].textContent, "User pinned");
  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
  assert.equal([...timers.values()].some(({ delay }) => delay === 8000), false);

  socket.emit("message", JSON.stringify({ type: "tts", payload: visualNotification("queued") }));
  assert.equal(elements["#notification-author"].textContent, "User pinned");

  socket.emit("message", JSON.stringify({ type: "messagePin", payload: { pinned: false } }));
  assert.equal(elements["#notification-author"].textContent, "User queued");
  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
});

test("a pinned spoken notification keeps its card after audio ends and resumes on removal", async () => {
  const { elements, socket, timers } = createHarness("widget");
  const first = notification("spoken-pin");
  socket.emit("message", JSON.stringify({ type: "tts", payload: first }));
  socket.emit("message", JSON.stringify({
    type: "messagePin",
    payload: { pinned: true, message: first },
  }));
  assert.equal(elements["#notification"].classList.contains("is-visible"), true);
  assert.equal(elements["#notification-clock"].src, "");

  socket.emit("message", JSON.stringify({ type: "tts", payload: notification("after-pin") }));
  assert.equal(elements["#notification-author"].textContent, "User spoken-pin");
  socket.emit("message", JSON.stringify({ type: "messagePin", payload: { pinned: false } }));
  await nextMicrotask();
  assert.equal(elements["#notification-author"].textContent, "User after-pin");
});

test("reconnected notifications wait for the pin snapshot before draining queued messages", () => {
  const { elements, socket, sockets, runNextTimer } = createHarness("widget");
  const first = visualNotification("reconnect-pin");
  socket.emit("message", JSON.stringify({ type: "tts", payload: first }));
  socket.emit("message", JSON.stringify({ type: "messagePin", payload: { pinned: true, message: first } }));
  socket.emit("message", JSON.stringify({ type: "tts", payload: visualNotification("reconnect-queued") }));

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
