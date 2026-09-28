const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

function classList() {
  const values = new Set();
  return {
    add: (...names) => names.forEach((name) => values.add(name)),
    remove: (...names) => names.forEach((name) => values.delete(name)),
    contains: (name) => values.has(name),
    toggle(name, force) {
      if (force) values.add(name);
      else values.delete(name);
    },
  };
}

test("media previews stay visible and isolated from the live media queue", () => {
  const preview = createHarness("?secret=private&preview=1&lang=fr");
  const image = preview.elements["#image"];
  const author = preview.elements["#author"];

  assert.equal(image.src, "/overlay-assets/relay-radar.png");
  assert.equal(image.classList.contains("is-visible"), true);
  assert.equal(image.style.height, "100%");
  assert.equal(
    vm.runInContext('document.documentElement.classList.contains("overlay-preview")', preview.context),
    true,
  );
  assert.equal(preview.elements["#author-name"].textContent, "Aperçu en direct");
  assert.equal(author.hidden, false);

  preview.socket.emit("message", JSON.stringify({
    type: "config",
    payload: { mediaObsGeometry: { cropRight: 18, contentScale: 140 } },
  }));
  preview.socket.emit("message", JSON.stringify({
    type: "media",
    payload: { kind: "video", url: "https://cdn.discordapp.com/ignored.mp4" },
  }));
  preview.socket.emit("message", JSON.stringify({ type: "clear" }));

  assert.equal(preview.cssProperties["--crop-right"], "18%");
  assert.equal(preview.cssProperties["--content-scale"], "1.4");
  assert.equal(image.src, "/overlay-assets/relay-radar.png");
  assert.equal(image.classList.contains("is-visible"), true);
  assert.equal(preview.elements["#video"].src, "");
  assert.equal(preview.timerDelays().length, 0);

  const panelSource = fs.readFileSync(__dirname + "/../gui/panel.js", "utf8");
  assert.match(panelSource, /previewUrl\.searchParams\.set\("preview", "1"\)/);
  assert.match(panelSource, /url\.searchParams\.set\("preview", "1"\)/);
  assert.match(panelSource, /http:\/\/127\.0\.0\.1:\$\{port\}\$\{path\}/);
  assert.match(panelSource, /path = metadata\.previewKey === "notificationUrl" \? "\/notifications" : "\/medias"/);
  const overlayCss = require("./test-source.cjs").styles();
  assert.match(overlayCss, /html\.overlay-preview,\s*html\.overlay-preview body\s*\{[^}]*background:\s*#000/s);
});

test("media outputs identify their source and local client context", () => {
  const obs = createHarness("?secret=private", "visual");
  assert.match(obs.socket.url, /role=overlay&source=visual&client=obs&secret=private$/);

  const widget = createHarness("?secret=private&widget=1", "audio");
  assert.match(widget.socket.url, /role=overlay&source=audio&client=widget&secret=private$/);

  const preview = createHarness("?secret=private&preview=1", "visual");
  assert.match(preview.socket.url, /role=overlay&source=visual&client=preview&secret=private$/);
});

test("local visual tests reach matching outputs without affecting previews", () => {
  const visual = createHarness("?secret=private", "visual");
  sendMediaClock(visual);
  visual.socket.emit("message", JSON.stringify({
    type: "testOutput",
    payload: {
      target: "visual",
      media: { kind: "image", url: "/overlay-assets/relay-radar.png" },
    },
  }));
  sendMediaGrant(visual, true, { videoBusy: true });
  assert.equal(visual.elements["#image"].src, "/overlay-assets/relay-radar.png");

  const audio = createHarness("?secret=private", "audio");
  audio.socket.emit("message", JSON.stringify({
    type: "testOutput",
    payload: {
      target: "visual",
      media: { kind: "image", url: "/overlay-assets/relay-radar.png" },
    },
  }));
  assert.equal(audio.elements["#image"].src, "");

  const preview = createHarness("?secret=private&preview=1", "visual");
  preview.socket.emit("message", JSON.stringify({
    type: "testOutput",
    payload: {
      target: "visual",
      media: { kind: "image", url: "https://cdn.discordapp.com/ignored.png" },
    },
  }));
  assert.equal(preview.elements["#image"].src, "/overlay-assets/relay-radar.png");
});

function element() {
  const listeners = new Map();
  return {
    classList: classList(),
    hidden: false,
    src: "",
    textContent: "",
    complete: false,
    naturalWidth: 0,
    naturalHeight: 0,
    videoWidth: 0,
    videoHeight: 0,
    loop: false,
    style: { width: "", height: "", left: "", bottom: "", maxWidth: "" },
    getBoundingClientRect() {
      return this.rect || { left: 0, top: 0, right: 0, bottom: 0, width: 0, height: 0 };
    },
    onerror: null,
    addEventListener(type, listener) {
      if (!listeners.has(type)) listeners.set(type, new Set());
      listeners.get(type).add(listener);
    },
    removeEventListener(type, listener) { listeners.get(type)?.delete(listener); },
    emit(type) {
      for (const listener of [...(listeners.get(type) || [])]) listener({ type });
    },
    removeAttribute(name) {
      if (name === "src") this.src = "";
    },
    setAttribute() {},
    load() {},
    pauseCalls: 0,
    pause() { this.pauseCalls += 1; },
    play() { return Promise.resolve(); },
  };
}

function createHarness(search = "?secret=private", mode = "all", autoGrantStage = true, metaSecret = "") {
  const selectors = [
    "#image", "#video", "#audio", "#audio-card", "#audio-artwork", "#audio-title", "#audio-artist", "#audio-media-text",
    "#audio-time", "#audio-progress", "#audio-progress-fill",
    "#audio-author", "#audio-author-avatar", "#audio-author-name",
    "#author", "#author-avatar", "#author-name", "#media-text", "#youtube-player",
    "#youtube-credit", "#youtube-credit-label",
    "#youtube-credit-channel", "#youtube-credit-source", "#youtube-credit-added",
    "#youtube-credit-time", "#youtube-credit-progress-fill",
    "#widget-move-label",
  ];
  const elements = Object.fromEntries(selectors.map((selector) => [selector, element()]));
  elements["#audio"].currentTime = 0;
  elements["#audio"].duration = Number.NaN;
  elements["#audio-progress"].setAttribute = function setAttribute(name, value) {
    this.attributes = this.attributes || {};
    this.attributes[name] = value;
  };
  elements["#audio-progress"].hidden = true;
  elements["#audio-time"].hidden = true;
  elements["#audio-progress-fill"].style.width = "0%";
  elements["#youtube-player"].replaceChildren = function replaceChildren() {
    this.innerHTML = "";
  };
  elements["#youtube-player"].innerHTML = "";
  elements["#youtube-player"].setAttribute = function setAttribute(name, value) {
    this.attributes = this.attributes || {};
    this.attributes[name] = value;
    if (name === "aria-hidden") this.ariaHidden = value;
  };
  elements["#youtube-credit"].hidden = true;
  elements["#youtube-credit"].setAttribute = function setAttribute(name, value) {
    this.attributes = this.attributes || {};
    this.attributes[name] = value;
    if (name === "aria-hidden") this.ariaHidden = value;
  };
  const sockets = [];
  const youtubePlayers = [];
  const timers = [];
  const cssProperties = {};
  let clockNow = 1_000_000;

  class MockWebSocket {
    constructor(url) {
      this.url = url;
      this.listeners = new Map();
      this.readyState = 1;
      this.sent = [];
      sockets.push(this);
    }
    addEventListener(type, listener) { this.listeners.set(type, listener); }
    emit(type, data) { this.listeners.get(type)?.({ data }); }
    send(data) {
      const message = JSON.parse(data);
      this.sent.push(message);
      if (
        autoGrantStage
        && message.type === "stageClock"
        && message.payload?.lane === "media"
        && message.payload?.busy === true
      ) {
        this.emit("message", JSON.stringify({
          type: "stageClock",
          payload: { mediaBusy: true, musicBusy: false, notificationBusy: false },
        }));
      }
    }
    close() { this.readyState = 3; }
  }

  const location = {
    protocol: "http:",
    host: "127.0.0.1:4590",
    port: "4590",
    href: `http://127.0.0.1:4590/overlay${search}`,
    search,
    replaced: "",
    replace(url) { this.replaced = String(url); },
    reloads: 0,
    reload() { this.reloads += 1; },
  };
  class FakeYoutubePlayer {
    constructor(id, options) {
      this.id = id;
      this.options = options;
      this.loaded = undefined;
      this.stopCalls = 0;
      this.destroyCalls = 0;
      this.seekCalls = [];
      this.muteCalls = 0;
      this.unMuteCalls = 0;
      this.unloadModuleCalls = [];
      this.setOptionCalls = [];
      this.volume = undefined;
      this.currentTime = 0;
      this.iframeAttributes = {};
      this.iframe = {
        setAttribute: (name, value) => {
          this.iframeAttributes[name] = value;
        },
        getAttribute: (name) => this.iframeAttributes[name],
      };
      youtubePlayers.push(this);
    }

    ready() { this.options.events.onReady({ target: this }); }
    loadVideoById(options) { this.loaded = options; }
    seekTo(seconds, allowSeekAhead) {
      this.seekCalls.push({ seconds, allowSeekAhead });
      this.currentTime = seconds;
    }
    playVideo() {}
    stopVideo() { this.stopCalls += 1; }
    destroy() { this.destroyCalls += 1; }
    mute() { this.muteCalls += 1; }
    unMute() { this.unMuteCalls += 1; }
    setVolume(value) { this.volume = value; }
    unloadModule(name) { this.unloadModuleCalls.push(name); }
    setOption(module, option, value) { this.setOptionCalls.push([module, option, value]); }
    getCurrentTime() { return this.currentTime; }
    getIframe() { return this.iframe; }
    emitState(data) { this.options.events.onStateChange({ data }); }
    emitError() { this.options.events.onError({ data: 150 }); }
  }

  const window = {
    location,
    innerWidth: 640,
    innerHeight: 360,
    addEventListener() {},
    clearTimeout() {},
    setTimeout(callback, delay) { timers.push({ callback, delay }); return timers.length; },
    getComputedStyle() {
      return {
        getPropertyValue(name) {
          if (Object.prototype.hasOwnProperty.call(cssProperties, name)) {
            return cssProperties[name];
          }
          if (name === "--content-scale") return "1";
          return "";
        },
      };
    },
    YT: { Player: FakeYoutubePlayer, PlayerState: { ENDED: 0, BUFFERING: 3 } },
  };
  const context = vm.createContext({
    URL,
    URLSearchParams,
    Date: { now: () => clockNow },
    WebSocket: MockWebSocket,
    getComputedStyle: (...args) => window.getComputedStyle(...args),
    document: {
      documentElement: {
        classList: classList(),
        dataset: {},
        lang: "en",
        style: { setProperty(name, value) { cssProperties[name] = value; } },
      },
      querySelector: (selector) => {
        if (selector === 'meta[name="relay-mode"]') return { content: mode };
        if (selector === 'meta[name="relay-secret"]') {
          return metaSecret ? { content: metaSecret } : null;
        }
        return elements[selector];
      },
    },
    encodeURIComponent,
    window,
  });
  vm.runInContext(fs.readFileSync(__dirname + "/../outputs/layout.js", "utf8"), context);
  vm.runInContext(fs.readFileSync(__dirname + "/../outputs/connection.js", "utf8"), context);
  vm.runInContext(fs.readFileSync(__dirname + "/overlay.js", "utf8"), context);
  return {
    context,
    cssProperties,
    elements,
    location,
    queuedAuthors: () => vm.runInContext("queue.map((media) => media.author.username)", context),
    runNextTimer: () => timers.shift()?.callback(),
    runTimerByDelay(delay) {
      const index = timers.findIndex((timer) => timer.delay === delay);
      if (index < 0) return false;
      timers.splice(index, 1)[0].callback();
      return true;
    },
    timerDelays: () => timers.map((timer) => timer.delay),
    setNow(value) { clockNow = value; },
    socket: sockets[0],
    sockets,
    youtubePlayers,
  };
}

function sendMediaClock(harness, payload = {}, socket = harness.sockets.at(-1)) {
  socket.emit("message", JSON.stringify({
    type: "mediaClock",
    payload: {
      videoBusy: false,
      audioBusy: false,
      ...payload,
    },
  }));
}

function sendMediaGrant(
  harness,
  granted,
  clock = {},
  socket = harness.sockets.at(-1),
) {
  socket.emit("message", JSON.stringify({
    type: "mediaGrant",
    payload: {
      granted,
      clock: { videoBusy: false, audioBusy: false, ...clock },
    },
  }));
}

function sendMedia(harnesses, media) {
  const targets = Array.isArray(harnesses) ? harnesses : [harnesses];
  for (const harness of targets) {
    harness.socket.emit("message", JSON.stringify({ type: "media", payload: media }));
    // Unit tests have no server: after a media-lane claim, grant the stage when free.
    const musicBusy = vm.runInContext("musicBusy", harness.context);
    const notificationBusy = vm.runInContext("notificationBusy", harness.context);
    const claimPending = vm.runInContext("mediaStageClaimPending", harness.context);
    if (claimPending && !musicBusy && !notificationBusy) {
      harness.socket.emit("message", JSON.stringify({
        type: "stageClock",
        payload: { mediaBusy: true, musicBusy: false, notificationBusy: false },
      }));
    }
  }
}

test("short OBS pages authenticate from the injected relay-secret meta tag", () => {
  const harness = createHarness("", "visual", true, "private");
  assert.match(harness.socket.url, /secret=private/);
});

test("media output applies live crop and scale for OBS and widgets", () => {
  const obs = createHarness();
  obs.socket.emit("message", JSON.stringify({
    type: "config",
    payload: {
      mediaObsGeometry: {
        cropTop: 4, cropRight: 8, cropBottom: 12, cropLeft: 16, contentScale: 125,
      },
    },
  }));
  assert.deepEqual(
    Object.fromEntries(Object.entries(obs.cssProperties).filter(([name]) => name.startsWith("--crop"))),
    { "--crop-top": "4%", "--crop-right": "8%", "--crop-bottom": "12%", "--crop-left": "16%" },
  );
  assert.equal(obs.cssProperties["--content-scale"], "1.25");

  const widget = createHarness("?secret=private&widget=1");
  widget.socket.emit("message", JSON.stringify({
    type: "config",
    payload: {
      mediaObsGeometry: { contentScale: 75 },
      mediaWidgetGeometry: { cropTop: 10, contentScale: 180 },
    },
  }));
  // Widget ignores crop and caps content scale at 100% so images are never edge-clipped.
  assert.equal(widget.cssProperties["--crop-top"], "0%");
  assert.equal(widget.cssProperties["--crop-bottom"], "0%");
  assert.equal(widget.cssProperties["--content-scale"], "1");

  const css = require("./test-source.cjs").styles();
  assert.match(css, /clip-path: inset\(var\(--crop-top\)/);
  assert.match(css, /\.audio-card[\s\S]*var\(--content-scale\)/);
  assert.match(css, /\.audio-card\s*\{[^}]*top:\s*16px/s);
  assert.match(css, /\.audio-card__progress\s*\{[^}]*height:\s*4px/s);
  assert.match(css, /\.overlay__author[\s\S]*var\(--content-scale\)/);
});

test("media fitting preserves portrait, landscape and square ratios with space for attribution", () => {
  const { context } = createHarness();
  for (const [width, height] of [[408, 720], [1280, 720], [512, 512]]) {
    for (const scale of [0.5, 1, 2]) {
      const layout = context.mediaLayout(width, height, 640, 360, scale, 44 * scale);
      assert.ok(Math.abs(layout.width / layout.height - width / height) < 0.00001);
      assert.ok(layout.left >= 12);
      assert.ok(Math.abs(layout.left + layout.displayedWidth - 628) < 0.001);
      assert.ok(layout.width * scale <= 616);
      assert.ok(layout.bottom + 44 * scale <= 348.001);
    }
  }
});

test("audio attribution is inside the card and respects the author toggle", () => {
  const { context, elements } = createHarness();
  vm.runInContext('setAuthor({ kind: "audio", author: { username: "Listener" } })', context);
  assert.equal(elements["#author"].hidden, true);
  assert.equal(elements["#audio-author"].hidden, false);
  assert.equal(elements["#audio-author-name"].textContent, "Listener");
  vm.runInContext('config.showAuthor = false; setAuthor({ kind: "audio", author: { username: "Listener" } })', context);
  assert.equal(elements["#audio-author"].hidden, true);
  assert.equal(elements["#audio-author-name"].textContent, "");
  assert.equal(elements["#audio-author-avatar"].src, "");
});

test("hiding authors removes YouTube requester credits live on OBS and Windows", async () => {
  for (const [search, mode] of [["?secret=private", "youtube"], ["?secret=private&widget=1", "all"]]) {
    const view = createHarness(search, mode);
    const send = (type, payload) => view.socket.emit("message", JSON.stringify({ type, payload }));
    send("config", { showAuthor: false });
    send("musicPlay", {
      playbackId: "private-credit", videoId: "dQw4w9WgXcQ", title: "Track title",
      channelTitle: "Artist", durationSeconds: 180, requestedBy: "Private listener",
    });
    await Promise.resolve();
    const player = view.youtubePlayers[0];
    player.ready();
    const credit = view.elements["#youtube-credit-added"];
    assert.equal(credit.hidden, true);
    assert.equal(credit.textContent, "");
    assert.equal(view.elements["#youtube-credit-channel"].textContent, "Track title");
    send("config", { showAuthor: true });
    assert.equal(credit.hidden, false);
    assert.equal(credit.textContent, "Added by Private listener");
    send("config", { showAuthor: false });
    send("appearance", { language: "fr" });
    assert.equal(credit.hidden, true);
    assert.equal(credit.textContent, "");
    send("config", { showAuthor: true });
    assert.equal(credit.textContent, "Ajouté par Private listener");
    assert.equal(player.stopCalls, 0);
    send("musicIdle");
    send("config", { showAuthor: true });
    assert.equal(credit.textContent, "");
  }
});

test("hiding the current media author clears identity and restores it when enabled", () => {
  const { elements, socket } = createHarness();
  const send = (type, payload) => socket.emit("message", JSON.stringify({ type, payload }));
  send("media", {
    kind: "image", url: "https://cdn.discordapp.com/image.png",
    author: { username: "Private author", displayAvatarUrl: "https://cdn.discordapp.com/avatar.png" },
  });
  elements["#image"].emit("load");
  send("config", { showAuthor: false });
  assert.equal(elements["#author"].hidden, true);
  assert.equal(elements["#author-name"].textContent, "");
  assert.equal(elements["#author-avatar"].src, "");
  send("config", { showAuthor: true });
  assert.equal(elements["#author-name"].textContent, "Private author");
  assert.equal(elements["#author"].hidden, false);
  assert.equal(elements["#author"].classList.contains("is-visible"), true);
});

test("media overlay reads the camelCase Discord avatar and falls back locally", () => {
  const { elements, socket } = createHarness();
  const avatar = elements["#author-avatar"];

  socket.emit("message", JSON.stringify({
    type: "media",
    payload: {
      kind: "image",
      url: "https://cdn.discordapp.com/media.png",
      author: {
        username: "inerthel",
        displayAvatarUrl: "https://cdn.discordapp.com/avatar.png",
      },
    },
  }));

  assert.equal(avatar.src, "https://cdn.discordapp.com/avatar.png");
  assert.equal(elements["#author-name"].textContent, "inerthel");
  assert.equal(elements["#author"].hidden, false);

  avatar.onerror();
  assert.equal(avatar.src, "/overlay-assets/relay-radar.png");
  assert.equal(avatar.onerror, null);
});

test("media messages are bounded upstream and independently visible in OBS and widgets", () => {
  const obs = createHarness();
  obs.socket.emit("message", JSON.stringify({
    type: "config",
    payload: { showMediaTextObs: true, showMediaTextWidget: false },
  }));
  obs.socket.emit("message", JSON.stringify({
    type: "media",
    payload: {
      kind: "image",
      url: "https://cdn.discordapp.com/setup.png",
      text: "<strong>Regardez mon setup</strong>",
    },
  }));
  obs.elements["#image"].naturalWidth = 1280;
  obs.elements["#image"].naturalHeight = 720;
  obs.elements["#image"].rect = {
    left: 100,
    top: 10,
    right: 500,
    bottom: 340,
    width: 400,
    height: 330,
  };
  obs.elements["#image"].emit("load");

  assert.equal(
    obs.elements["#media-text"].textContent,
    "<strong>Regardez mon setup</strong>",
  );
  assert.equal(obs.elements["#media-text"].hidden, false);
  assert.equal(obs.elements["#media-text"].classList.contains("is-visible"), true);
  const image = obs.elements["#image"];
  const caption = obs.elements["#media-text"];
  assert.ok(Number.parseFloat(caption.style.top) > Number.parseFloat(image.style.top) + Number.parseFloat(image.style.height) / 2);
  assert.equal(caption.style.bottom, "auto");
  assert.equal(Number.parseFloat(caption.style.maxWidth), Number.parseFloat(image.style.width));

  const widget = createHarness("?secret=private&widget=1");
  widget.socket.emit("message", JSON.stringify({
    type: "config",
    payload: { showMediaTextObs: true, showMediaTextWidget: false },
  }));
  widget.socket.emit("message", JSON.stringify({
    type: "media",
    payload: {
      kind: "image",
      url: "https://cdn.discordapp.com/setup.png",
      text: "Widget caption",
    },
  }));
  assert.equal(widget.elements["#media-text"].hidden, true);

  widget.socket.emit("message", JSON.stringify({
    type: "config",
    payload: { showMediaTextWidget: true },
  }));
  assert.equal(widget.elements["#media-text"].textContent, "Widget caption");
  assert.equal(widget.elements["#media-text"].hidden, false);
});

test("audio captions stay in the Now Playing card instead of covering the artwork", () => {
  const harness = createHarness();
  harness.socket.emit("message", JSON.stringify({
    type: "config",
    payload: { showMediaTextObs: true },
  }));
  harness.socket.emit("message", JSON.stringify({
    type: "media",
    payload: {
      kind: "audio",
      url: "https://cdn.discordapp.com/voice.ogg",
      title: "Stay With Me",
      text: "Message attached to the audio",
    },
  }));

  assert.equal(harness.elements["#audio-media-text"].textContent, "Message attached to the audio");
  assert.equal(harness.elements["#audio-media-text"].hidden, false);
  assert.equal(harness.elements["#media-text"].hidden, true);
});

test("media overlay follows a configured server port once it responds", () => {
  const { location, sockets, socket } = createHarness();
  const source = fs.readFileSync(__dirname + "/../outputs/connection.js", "utf8");
  assert.match(source, /probeWatchdog[\s\S]*probe\.close\(\)/);

  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { port: 4600 },
  }));
  socket.emit("close");

  const probe = sockets[1];
  assert.ok(probe.url.includes(":4600/ws"));
  assert.equal(location.replaced, "");
  probe.emit("open");
  assert.equal(location.replaced, "http://127.0.0.1:4600/overlay?secret=private");
});

test("audio overlay uses embedded artwork and falls back to the Relay logo", () => {
  const { elements, socket } = createHarness();
  const artwork = elements["#audio-artwork"];

  socket.emit("message", JSON.stringify({
    type: "media",
    payload: {
      kind: "audio",
      url: "https://cdn.discordapp.com/track.mp3",
      artworkId: "123456789012345678",
      title: "BOOMBAYAH (Japanese ver.)",
      artist: "BLACKPINK",
    },
  }));

  assert.equal(
    artwork.src,
    "/media-artwork/123456789012345678?secret=private",
  );
  assert.equal(elements["#audio-title"].textContent, "BOOMBAYAH (Japanese ver.)");
  assert.equal(elements["#audio-artist"].textContent, "BLACKPINK");
  assert.equal(elements["#audio-artist"].hidden, false);
  artwork.onerror();
  assert.equal(artwork.src, "/overlay-assets/relay-radar.png");
});

test("Discord audio Now Playing shows live clock and progress bar", () => {
  const { elements, socket } = createHarness();
  const audio = elements["#audio"];

  socket.emit("message", JSON.stringify({
    type: "media",
    payload: {
      kind: "audio",
      url: "https://cdn.discordapp.com/track.mp3",
      title: "Runnin' Down A Dream",
      artist: "Tom Petty",
    },
  }));

  audio.duration = 148;
  audio.currentTime = 0;
  audio.emit("loadeddata");
  audio.emit("timeupdate");
  assert.equal(elements["#audio-time"].hidden, false);
  assert.equal(elements["#audio-time"].textContent, "0:00 → 2:28");
  assert.equal(elements["#audio-progress"].hidden, false);
  assert.equal(elements["#audio-progress-fill"].style.width, "0%");

  audio.currentTime = 55;
  audio.emit("timeupdate");
  assert.equal(elements["#audio-time"].textContent, "0:55 → 2:28");
  assert.equal(elements["#audio-progress-fill"].style.width, `${(55 / 148) * 100}%`);
});

test("YouTube music uses the official IFrame player and rejects stale stops", async () => {
  const audio = createHarness("?secret=private", "youtube");
  audio.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: {
      playbackId: "p1",
      videoId: "dQw4w9WgXcQ",
      startSeconds: 0,
      endSeconds: 30,
      channelTitle: "Rick Astley",
      title: "Never Gonna Give You Up",
      requestedBy: "inerthel",
    },
  }));
  await Promise.resolve();
  const player = audio.youtubePlayers[0];
  assert.ok(player);
  assert.equal(player.options.width, "100%");
  assert.equal(player.options.height, "100%");
  assert.equal(player.options.playerVars.cc_load_policy, 0);
  assert.equal(player.options.playerVars.iv_load_policy, 3);
  assert.equal(audio.elements["#youtube-player"].classList.contains("youtube-player--obs"), true);
  player.ready();
  assert.ok(player.muteCalls >= 1);
  assert.ok(player.unMuteCalls >= 1);
  assert.deepEqual(player.unloadModuleCalls, ["captions"]);
  assert.equal(player.setOptionCalls[0]?.[0], "captions");
  assert.equal(player.setOptionCalls[0]?.[1], "track");
  assert.equal(JSON.stringify(player.setOptionCalls[0]?.[2]), "{}");
  assert.equal(JSON.stringify(player.loaded), JSON.stringify({
    videoId: "dQw4w9WgXcQ",
    startSeconds: 0,
    endSeconds: 30,
  }));
  assert.equal(audio.elements["#youtube-credit-channel"].textContent, "Never Gonna Give You Up");
  assert.equal(audio.elements["#youtube-credit-added"].textContent, "Added by inerthel");
  assert.equal(audio.elements["#youtube-credit"].hidden, false);

  audio.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: {
      playbackId: "p2",
      videoId: "9bZkp7q19f0",
      startSeconds: 0,
      channelTitle: "PSY",
      title: "Gangnam Style",
      requestedBy: "ada",
    },
  }));
  await Promise.resolve();
  assert.ok(player.destroyCalls >= 1);
  assert.equal(vm.runInContext("youtubePlaybackId", audio.context), "p2");
  const nextPlayer = audio.youtubePlayers[1];
  assert.ok(nextPlayer);
  nextPlayer.ready();
  assert.equal(nextPlayer.loaded.videoId, "9bZkp7q19f0");
  assert.equal(audio.elements["#youtube-credit-channel"].textContent, "Gangnam Style");
  assert.equal(audio.elements["#youtube-credit-added"].textContent, "Added by ada");

  audio.socket.emit("message", JSON.stringify({
    type: "musicStop",
    payload: { playbackId: "p1" },
  }));
  assert.equal(vm.runInContext("youtubePlaybackId", audio.context), "p2");
  assert.equal(nextPlayer.destroyCalls, 0);
  nextPlayer.emitState(0);
  assert.ok(audio.socket.sent.some((message) => (
    message.type === "musicEnded" && message.payload?.playbackId === "p2"
  )));
  assert.ok(nextPlayer.destroyCalls >= 1);
  assert.equal(audio.elements["#youtube-player"].classList.contains("youtube-player--obs"), false);
  assert.equal(audio.elements["#youtube-credit"].hidden, true);
});

test("YouTube outputs follow the server clock despite iframe readiness and buffering drift", async () => {
  const obs = createHarness("?secret=private", "youtube");
  const widget = createHarness("?secret=private&widget=1", "all");
  const payload = {
    playbackId: "clocked",
    videoId: "dQw4w9WgXcQ",
    startSeconds: 10,
    endSeconds: 40,
    serverStartedAtMs: 1_000_000,
  };
  obs.socket.emit("message", JSON.stringify({ type: "musicPlay", payload }));
  widget.socket.emit("message", JSON.stringify({ type: "musicPlay", payload }));
  await Promise.resolve();

  const obsPlayer = obs.youtubePlayers[0];
  const widgetPlayer = widget.youtubePlayers[0];
  obs.setNow(1_004_000);
  obsPlayer.ready();
  widget.setNow(1_007_000);
  widgetPlayer.ready();
  assert.equal(obsPlayer.loaded.startSeconds, 14);
  assert.equal(widgetPlayer.loaded.startSeconds, 17);

  obs.setNow(1_008_000);
  widget.setNow(1_008_000);
  obsPlayer.currentTime = 14;
  widgetPlayer.currentTime = 15;
  assert.equal(obs.runTimerByDelay(500), true);
  widgetPlayer.emitState(3);
  assert.equal(widget.runTimerByDelay(500), true);
  assert.equal(obsPlayer.seekCalls.at(-1).seconds, 18);
  assert.equal(widgetPlayer.seekCalls.at(-1).seconds, 18);
});

test("late YouTube joins finish expired ranges and bound full playback", async () => {
  const expired = createHarness("?secret=private", "youtube");
  expired.setNow(1_040_000);
  expired.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: {
      playbackId: "expired",
      videoId: "dQw4w9WgXcQ",
      startSeconds: 10,
      endSeconds: 40,
      durationSeconds: 120,
      serverStartedAtMs: 1_000_000,
    },
  }));
  await Promise.resolve();
  const expiredPlayer = expired.youtubePlayers[0];
  expiredPlayer.ready();
  assert.equal(expiredPlayer.loaded, undefined);
  assert.ok(expired.socket.sent.some((message) => (
    message.type === "musicEnded" && message.payload?.playbackId === "expired"
  )));

  const full = createHarness("?secret=private", "youtube");
  full.setNow(1_010_000);
  full.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: {
      playbackId: "full-bounded",
      videoId: "dQw4w9WgXcQ",
      durationSeconds: 120,
      serverStartedAtMs: 1_000_000,
    },
  }));
  await Promise.resolve();
  const fullPlayer = full.youtubePlayers[0];
  fullPlayer.ready();
  assert.equal(fullPlayer.loaded.startSeconds, 10);
  assert.equal(fullPlayer.loaded.endSeconds, 120);
});

test("late callbacks from a destroyed YouTube player cannot finish its replacement", async () => {
  for (const callback of ["ended", "error"]) {
    const harness = createHarness("?secret=private&widget=1", "all");
    harness.socket.emit("message", JSON.stringify({
      type: "musicPlay",
      payload: {
        playbackId: `${callback}-p1`, videoId: "dQw4w9WgXcQ", serverStartedAtMs: 1_000_000,
      },
    }));
    await Promise.resolve();
    const firstPlayer = harness.youtubePlayers[0];
    firstPlayer.ready();

    harness.socket.emit("message", JSON.stringify({
      type: "musicPlay",
      payload: {
        playbackId: `${callback}-p2`, videoId: "9bZkp7q19f0", serverStartedAtMs: 1_000_000,
      },
    }));
    await Promise.resolve();
    const secondPlayer = harness.youtubePlayers[1];
    secondPlayer.ready();
    const firstSeekCalls = firstPlayer.seekCalls.length;

    if (callback === "ended") firstPlayer.emitState(0);
    else firstPlayer.emitError();

    assert.equal(vm.runInContext("youtubePlaybackId", harness.context), `${callback}-p2`);
    assert.equal(secondPlayer.destroyCalls, 0);
    assert.equal(firstPlayer.seekCalls.length, firstSeekCalls);
    assert.equal(
      harness.socket.sent.some((message) => (
        message.type === "musicEnded" && message.payload?.playbackId === `${callback}-p2`
      )),
      false,
    );
  }
});

test("OBS YouTube stop clears the player so no paused chrome remains", async () => {
  const obs = createHarness("?secret=private", "youtube");
  obs.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: {
      playbackId: "stop-1",
      videoId: "dQw4w9WgXcQ",
      startSeconds: 0,
      channelTitle: "Channel",
      requestedBy: "bob",
    },
  }));
  await Promise.resolve();
  const player = obs.youtubePlayers[0];
  player.ready();
  assert.equal(obs.elements["#youtube-credit"].hidden, false);

  obs.socket.emit("message", JSON.stringify({
    type: "musicStop",
    payload: { playbackId: "stop-1" },
  }));
  assert.equal(vm.runInContext("youtubePlayer", obs.context), undefined);
  assert.ok(player.destroyCalls >= 1);
  assert.equal(obs.elements["#youtube-player"].classList.contains("youtube-player--obs"), false);
  assert.equal(obs.elements["#youtube-credit"].hidden, true);
  assert.equal(obs.elements["#youtube-credit-channel"].textContent, "");

  obs.socket.emit("message", JSON.stringify({ type: "musicIdle" }));
  assert.equal(vm.runInContext("youtubePlayer", obs.context), undefined);
});

test("Windows combined overlay owns full-frame YouTube playback", async () => {
  const widget = createHarness("?secret=private&widget=1", "all");
  widget.socket.emit("message", JSON.stringify({
    type: "config",
    payload: { mediaVolume: 40, widgetSoundEnabled: true },
  }));
  widget.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: {
      playbackId: "w1",
      videoId: "dQw4w9WgXcQ",
      startSeconds: 0,
      title: "Less than a Lover",
      channelTitle: "JennieRubyJaneVEVO",
      thumbnail: "https://i.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg",
      durationSeconds: 219,
      requestedBy: "inerthel",
    },
  }));
  await Promise.resolve();
  const player = widget.youtubePlayers[0];
  assert.ok(player);
  assert.equal(player.options.width, "100%");
  assert.equal(player.options.height, "100%");
  assert.equal(widget.elements["#youtube-player"].classList.contains("youtube-player--obs"), true);
  player.ready();
  assert.equal(
    widget.elements["#youtube-credit-channel"].textContent,
    "Less than a Lover",
  );
  assert.equal(widget.elements["#youtube-credit-source"].textContent, "JennieRubyJaneVEVO");
  assert.equal(widget.elements["#youtube-credit-label"].textContent, "Now playing");
  assert.equal(
    widget.elements["#youtube-credit-added"].textContent,
    "Added by inerthel",
  );
  assert.equal(widget.elements["#youtube-credit-time"].textContent, "0:00 → 3:39");
  assert.equal(widget.elements["#youtube-credit"].classList.contains("youtube-credit--widget"), true);
  assert.equal(widget.elements["#youtube-credit"].hidden, false);
  player.currentTime = 62;
  assert.equal(widget.runTimerByDelay(1000), true);
  assert.equal(widget.elements["#youtube-credit-time"].textContent, "1:02 → 3:39");
  assert.equal(widget.elements["#youtube-credit-progress-fill"].style.width, "28.31%");
  assert.ok(player.unMuteCalls >= 1);
  assert.equal(player.volume, 40);
  assert.equal(vm.runInContext("youtubePlaybackId", widget.context), "w1");

  widget.socket.emit("message", JSON.stringify({
    type: "musicStop",
    payload: { playbackId: "stale-widget-id" },
  }));
  assert.equal(vm.runInContext("youtubePlaybackId", widget.context), "w1");
  assert.equal(player.destroyCalls, 0);

  player.emitState(0);
  assert.ok(widget.socket.sent.some((message) => (
    message.type === "musicEnded" && message.payload?.playbackId === "w1"
  )));
  assert.ok(player.destroyCalls >= 1);
  assert.equal(widget.elements["#youtube-credit"].hidden, true);
});

test("Discord audio overlays ignore YouTube musicPlay events", async () => {
  const audio = createHarness("?secret=private", "audio");
  audio.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: {
      playbackId: "ignored",
      videoId: "dQw4w9WgXcQ",
      startSeconds: 0,
    },
  }));
  await Promise.resolve();
  assert.equal(audio.youtubePlayers.length, 0);
});

test("YouTube embed errors do not clear server-side playback via musicEnded", async () => {
  const audio = createHarness("?secret=private", "youtube");
  audio.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: {
      playbackId: "err-1",
      videoId: "dQw4w9WgXcQ",
      startSeconds: 0,
    },
  }));
  await Promise.resolve();
  const player = audio.youtubePlayers[0];
  player.ready();
  const beforeMusicEnded = audio.socket.sent.filter((message) => message.type === "musicEnded").length;
  player.emitError();
  assert.equal(
    audio.socket.sent.filter((message) => message.type === "musicEnded").length,
    beforeMusicEnded,
  );
  assert.equal(vm.runInContext("youtubePlaybackId", audio.context), undefined);
  assert.ok(player.stopCalls >= 1);
});

test("OBS YouTube reports natural endings", async () => {
  const obs = createHarness("?secret=private", "youtube");
  obs.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: {
      playbackId: "obs-end",
      videoId: "dQw4w9WgXcQ",
      startSeconds: 0,
    },
  }));
  await Promise.resolve();
  const player = obs.youtubePlayers[0];
  player.ready();
  player.emitState(0);
  assert.ok(obs.socket.sent.some((message) => (
    message.type === "musicEnded" && message.payload?.playbackId === "obs-end"
  )));
});

test("OBS YouTube follows volume settings", async () => {
  const obs = createHarness("?secret=private", "youtube");
  obs.socket.emit("message", JSON.stringify({
    type: "config",
    payload: { widgetSoundEnabled: false, mediaVolume: 80 },
  }));
  obs.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: { playbackId: "obs-1", videoId: "dQw4w9WgXcQ", startSeconds: 0 },
  }));
  await Promise.resolve();
  const player = obs.youtubePlayers[0];
  assert.ok(player);
  player.ready();
  assert.ok(player.muteCalls >= 1);
  // OBS /youtube is not a widget window — it unmutes for stream audio.
  assert.ok(player.unMuteCalls >= 1);
  assert.equal(player.volume, 80);

  player.emitState(1);
  assert.equal(player.iframeAttributes.allow, "autoplay; encrypted-media; picture-in-picture");
  assert.equal(player.iframeAttributes.referrerpolicy, "strict-origin-when-cross-origin");
});

test("YouTube iframe keeps a referrer so WebView embeds are authorized", async () => {
  const audio = createHarness("?secret=private", "youtube");
  audio.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: { playbackId: "ref-1", videoId: "dQw4w9WgXcQ", startSeconds: 0 },
  }));
  await Promise.resolve();
  const player = audio.youtubePlayers[0];
  player.ready();
  assert.equal(player.iframeAttributes.referrerpolicy, "strict-origin-when-cross-origin");
  assert.equal(player.iframeAttributes.allow, "autoplay; encrypted-media; picture-in-picture");
});

test("desktop media widget stays muted to avoid duplicate OBS audio", () => {
  const { elements, socket } = createHarness("?secret=private&widget=1");

  socket.emit("message", JSON.stringify({
    type: "media",
    payload: { kind: "audio", url: "https://cdn.discordapp.com/track.mp3" },
  }));

  assert.equal(elements["#audio"].muted, true);
  assert.equal(elements["#audio"].volume, 0);
});

test("desktop media widget plays sound locally when widget sound is enabled", () => {
  const { elements, socket } = createHarness("?secret=private&widget=1");

  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { widgetSoundEnabled: true, mediaVolume: 80 },
  }));
  socket.emit("message", JSON.stringify({
    type: "media",
    payload: { kind: "audio", url: "https://cdn.discordapp.com/track.mp3" },
  }));

  assert.equal(elements["#audio"].muted, false);
  assert.equal(elements["#audio"].volume, 0.8);

  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { widgetSoundEnabled: false, mediaVolume: 80 },
  }));
  assert.equal(elements["#audio"].muted, true);
  assert.equal(elements["#audio"].volume, 0);
});

test("desktop media widget keeps video visible when WebView2 blocks audible autoplay", async () => {
  const { elements, socket } = createHarness("?secret=private&widget=1");
  const video = elements["#video"];
  let playCalls = 0;
  video.play = () => {
    playCalls += 1;
    return playCalls === 1
      ? Promise.reject({ name: "NotAllowedError" })
      : Promise.resolve();
  };

  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { widgetSoundEnabled: true, mediaVolume: 69 },
  }));
  socket.emit("message", JSON.stringify({
    type: "media",
    payload: { kind: "video", url: "https://cdn.discordapp.com/clip.mp4" },
  }));
  video.emit("loadeddata");
  await Promise.resolve();

  assert.equal(playCalls, 2);
  assert.equal(video.muted, true);
  assert.equal(video.volume, 0);
  assert.equal(video.classList.contains("is-visible"), true);
});

test("desktop media widget restores active media after its reconnect stageClock", () => {
  for (const [selector, media] of [
    ["#image", { kind: "image", url: "https://cdn.discordapp.com/photo.png" }],
    ["#video", { kind: "gif", contentType: "video/mp4", url: "https://cdn.discordapp.com/loop.mp4" }],
    ["#video", { kind: "video", url: "https://cdn.discordapp.com/clip.mp4" }],
    ["#audio", { kind: "audio", url: "https://cdn.discordapp.com/track.mp3" }],
  ]) {
    const widget = createHarness("?secret=private&widget=1");
    widget.socket.emit("message", JSON.stringify({ type: "media", payload: media }));
    vm.runInContext("window.setWidgetVisible(false)", widget.context);
    assert.equal(widget.elements[selector].src, "");
    assert.equal(widget.socket.readyState, 3);
    vm.runInContext("window.setWidgetVisible(true)", widget.context);
    assert.equal(widget.sockets.length, 2);
    assert.equal(widget.elements[selector].src, "");
    widget.sockets.at(-1).emit("message", JSON.stringify({
      type: "stageClock",
      payload: { mediaBusy: false, musicBusy: false, notificationBusy: false },
    }));
    assert.equal(widget.elements[selector].src, media.url);
    widget.socket.emit("close");
    assert.equal(widget.elements[selector].src, media.url);
  }
});

test("native hide discards deferred YouTube before authoritative idle hydration", async () => {
  const widget = createHarness("?secret=private&widget=1", "all");
  widget.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: true, notificationBusy: true },
  }));
  widget.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: { playbackId: "hidden-stale-yt", videoId: "dQw4w9WgXcQ" },
  }));
  assert.equal(vm.runInContext("deferredYoutubeQueue.length", widget.context), 1);

  vm.runInContext("window.setWidgetVisible(false)", widget.context);
  assert.equal(vm.runInContext("stageClockReady", widget.context), false);
  assert.equal(vm.runInContext("deferredYoutubeQueue.length", widget.context), 0);
  assert.equal(widget.socket.readyState, 3);

  vm.runInContext("window.setWidgetVisible(true)", widget.context);
  const reconnected = widget.sockets.at(-1);
  reconnected.emit("open");
  reconnected.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, notificationBusy: false },
  }));
  await Promise.resolve();

  assert.equal(vm.runInContext("deferredYoutubeQueue.length", widget.context), 0);
  assert.equal(vm.runInContext("youtubePlaybackId", widget.context), undefined);
  assert.equal(widget.youtubePlayers.length, 0);
});

test("OBS audio uses the unchanged bytes cached by Relay", () => {
  const { elements, socket } = createHarness();
  socket.emit("message", JSON.stringify({
    type: "media",
    payload: {
      kind: "audio",
      audioId: "123456789012345678",
      url: "https://cdn.discordapp.com/track.mp3",
    },
  }));

  assert.equal(
    elements["#audio"].src,
    "/media-audio/123456789012345678?secret=private",
  );
});

test("audio playback reports live state and follows player controls", async () => {
  const harness = createHarness("?secret=private", "audio");
  const { elements, socket } = harness;
  sendMediaClock(harness);
  const media = {
    kind: "audio", url: "https://cdn.discordapp.com/track.mp3", filename: "track.mp3",
  };
  socket.emit("message", JSON.stringify({ type: "media", payload: media }));
  sendMediaGrant(harness, true, { audioBusy: true });
  elements["#audio"].emit("loadeddata");
  elements["#audio"].emit("playing");
  assert.equal(socket.sent.at(-1).type, "audioPlayback");
  assert.equal(socket.sent.at(-1).payload.status, "playing");

  socket.emit("message", JSON.stringify({ type: "audioControl", payload: { action: "pause" } }));
  assert.equal(socket.sent.at(-1).payload.status, "paused");
  assert.ok(elements["#audio"].pauseCalls > 0);

  socket.emit("message", JSON.stringify({ type: "audioControl", payload: { action: "resume" } }));
  await Promise.resolve();
  assert.equal(socket.sent.at(-1).payload.status, "playing");

  socket.emit("message", JSON.stringify({ type: "audioControl", payload: { action: "skip" } }));
  assert.equal(socket.sent.at(-1).payload.status, "idle");
});

test("overlay clears playback and visuals when Relay stops", () => {
  const { elements, socket } = createHarness();
  socket.emit("message", JSON.stringify({
    type: "media",
    payload: { kind: "audio", url: "https://cdn.discordapp.com/track.mp3" },
  }));
  socket.emit("close");

  assert.equal(elements["#audio"].src, "");
  assert.equal(elements["#audio-card"].hidden, true);
  assert.ok(elements["#audio"].pauseCalls > 0);
});

test("images, GIFs and videos wait for the exclusive OBS visual lease", () => {
  const cases = [
    {
      label: "image",
      media: { kind: "image", url: "https://cdn.discordapp.com/still.png" },
      selector: "#image",
    },
    {
      label: "GIF",
      media: { kind: "gif", contentType: "image/gif", url: "https://cdn.discordapp.com/loop.gif" },
      selector: "#image",
    },
    {
      label: "video GIF",
      media: { kind: "gif", contentType: "video/mp4", url: "https://media.klipy.com/loop.mp4" },
      selector: "#video",
    },
    {
      label: "video",
      media: { kind: "video", url: "https://cdn.discordapp.com/clip.mp4" },
      selector: "#video",
    },
  ];

  for (const { label, media, selector } of cases) {
    const visual = createHarness("", "visual");
    sendMediaClock(visual, { audioBusy: true });
    sendMedia(visual, media);
    assert.equal(visual.elements[selector].src, "", `${label} started over active OBS audio`);

    sendMediaClock(visual, { audioBusy: false });
    assert.deepEqual(
      visual.socket.sent.at(-1),
      { type: "mediaClock", payload: { busy: true } },
      `${label} did not request the OBS visual lease`,
    );
    assert.equal(visual.elements[selector].src, "");

    sendMediaGrant(visual, true, { videoBusy: true });
    assert.equal(visual.elements[selector].src, media.url);
  }
});

test("audio waits for the active video lease before starting", () => {
  const visual = createHarness("", "visual");
  const audio = createHarness("", "audio");
  const video = { kind: "video", url: "https://cdn.discordapp.com/clip.mp4" };
  const track = { kind: "audio", url: "https://cdn.discordapp.com/track.mp3" };

  sendMediaClock(visual);
  sendMediaClock(audio);
  sendMedia([visual, audio], video);
  assert.equal(visual.elements["#video"].src, "");
  assert.deepEqual(visual.socket.sent.at(-1), { type: "mediaClock", payload: { busy: true } });

  sendMediaGrant(visual, true, { videoBusy: true });
  sendMediaClock(audio, { videoBusy: true });
  sendMedia([visual, audio], track);
  visual.elements["#video"].emit("loadeddata");
  assert.equal(visual.elements["#video"].src, video.url);
  assert.equal(audio.elements["#audio"].src, "");

  visual.elements["#video"].emit("ended");
  assert.equal(visual.runTimerByDelay(320), true);
  assert.deepEqual(visual.socket.sent.at(-1), { type: "mediaClock", payload: { busy: false } });
  sendMediaClock(audio, { videoBusy: false });
  assert.deepEqual(audio.socket.sent.at(-1), { type: "mediaClock", payload: { busy: true } });
  assert.equal(audio.elements["#audio"].src, "");

  sendMediaGrant(audio, true, { audioBusy: true });
  assert.equal(audio.elements["#audio"].src, track.url);
  assert.equal(visual.elements["#video"].src, "");
});

test("video waits for the active audio lease before starting", () => {
  const visual = createHarness("", "visual");
  const audio = createHarness("", "audio");
  const track = { kind: "audio", url: "https://cdn.discordapp.com/track.mp3" };
  const video = { kind: "video", url: "https://cdn.discordapp.com/clip.mp4" };

  sendMediaClock(visual);
  sendMediaClock(audio);
  sendMedia([visual, audio], track);
  sendMediaGrant(audio, true, { audioBusy: true });
  sendMediaClock(visual, { audioBusy: true });
  sendMedia([visual, audio], video);

  assert.equal(audio.elements["#audio"].src, track.url);
  assert.equal(visual.elements["#video"].src, "");

  audio.elements["#audio"].emit("ended");
  assert.equal(audio.runTimerByDelay(320), true);
  sendMediaClock(visual, { audioBusy: false });
  assert.deepEqual(visual.socket.sent.at(-1), { type: "mediaClock", payload: { busy: true } });

  sendMediaGrant(visual, true, { videoBusy: true });
  assert.equal(visual.elements["#video"].src, video.url);
  assert.equal(audio.elements["#audio"].src, "");
});

test("reconnecting split outputs require one exclusive lease before resuming", () => {
  const visual = createHarness("", "visual");
  const audio = createHarness("", "audio");

  sendMediaClock(visual);
  sendMediaClock(audio);
  sendMedia(visual, { kind: "video", url: "https://cdn.discordapp.com/lost.mp4" });
  sendMedia(visual, { kind: "video", url: "https://cdn.discordapp.com/resume.mp4" });
  sendMedia(audio, { kind: "audio", url: "https://cdn.discordapp.com/lost.mp3" });
  sendMedia(audio, { kind: "audio", url: "https://cdn.discordapp.com/resume.mp3" });
  visual.socket.emit("close");
  audio.socket.emit("close");
  assert.equal(visual.runTimerByDelay(1000), true);
  assert.equal(audio.runTimerByDelay(1000), true);

  const visualSocket = visual.sockets.at(-1);
  const audioSocket = audio.sockets.at(-1);
  visualSocket.emit("open");
  audioSocket.emit("open");
  sendMediaClock(visual, {}, visualSocket);
  sendMediaClock(audio, {}, audioSocket);
  assert.equal(visual.elements["#video"].src, "");
  assert.equal(audio.elements["#audio"].src, "");

  visualSocket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, notificationBusy: false },
  }));
  audioSocket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, notificationBusy: false },
  }));
  sendMediaGrant(visual, true, { videoBusy: true }, visualSocket);
  sendMediaGrant(audio, false, { videoBusy: true }, audioSocket);
  assert.equal(visual.elements["#video"].src, "https://cdn.discordapp.com/resume.mp4");
  assert.equal(audio.elements["#audio"].src, "");
});

test("combined widget waits for stageClock before resuming retained media after reconnect", () => {
  const widget = createHarness("?secret=private&widget=1", "all");
  widget.socket.emit("message", JSON.stringify({
    type: "media",
    payload: { kind: "image", url: "https://cdn.discordapp.com/first.png" },
  }));
  widget.socket.emit("message", JSON.stringify({
    type: "media",
    payload: { kind: "image", url: "https://cdn.discordapp.com/retained.png" },
  }));
  assert.equal(widget.elements["#image"].src, "https://cdn.discordapp.com/first.png");

  widget.socket.emit("close");
  assert.equal(widget.runTimerByDelay(1000), true);
  const reconnected = widget.sockets.at(-1);
  reconnected.emit("open");
  assert.equal(widget.elements["#image"].src, "");

  reconnected.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, notificationBusy: false },
  }));
  assert.equal(widget.elements["#image"].src, "https://cdn.discordapp.com/retained.png");
});

test("idle reconnect drops stale deferred YouTube", async () => {
  const widget = createHarness("?secret=private&widget=1", "all");
  widget.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: true, notificationBusy: true },
  }));
  widget.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: { playbackId: "stale-reconnect-yt", videoId: "dQw4w9WgXcQ" },
  }));
  assert.equal(vm.runInContext("deferredYoutubeQueue.length", widget.context), 1);

  widget.socket.emit("close");
  assert.equal(widget.runTimerByDelay(1000), true);
  const reconnected = widget.sockets.at(-1);
  reconnected.emit("open");
  reconnected.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, notificationBusy: false },
  }));
  await Promise.resolve();

  assert.equal(vm.runInContext("deferredYoutubeQueue.length", widget.context), 0);
  assert.equal(vm.runInContext("youtubePlaybackId", widget.context), undefined);
  assert.equal(widget.youtubePlayers.length, 0);
});

test("active reconnect retains deferred YouTube until authoritative stageClock", async () => {
  const widget = createHarness("?secret=private&widget=1", "all");
  widget.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: true, notificationBusy: true },
  }));
  widget.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: { playbackId: "reconnect-yt", videoId: "dQw4w9WgXcQ" },
  }));
  assert.equal(vm.runInContext("deferredYoutubeQueue.length", widget.context), 1);

  widget.socket.emit("close");
  assert.equal(vm.runInContext("deferredYoutubeQueue.length", widget.context), 1);
  assert.equal(widget.runTimerByDelay(1000), true);
  const reconnected = widget.sockets.at(-1);
  reconnected.emit("open");
  await Promise.resolve();
  assert.equal(widget.youtubePlayers.length, 0);

  reconnected.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: true, notificationBusy: false },
  }));
  await Promise.resolve();
  assert.equal(widget.youtubePlayers.length, 1);
  assert.equal(vm.runInContext("youtubePlaybackId", widget.context), "reconnect-yt");
});

test("media widget move label follows the Relay language", () => {
  const expected = {
    en: "Move overlay",
    fr: "Déplacer l’overlay",
    es: "Mover overlay",
    de: "Overlay verschieben",
    ru: "Переместить оверлей",
    zh: "移动叠加层",
    ko: "오버레이 이동",
    ja: "オーバーレイを移動",
    id: "Pindahkan overlay",
  };
  for (const [language, label] of Object.entries(expected)) {
    const widget = createHarness(`?secret=private&widget=1&lang=${language}`);
    assert.equal(widget.elements["#widget-move-label"].textContent, label);
  }
});

test("multi-user bursts keep every image and GIF in FIFO order", () => {
  const { queuedAuthors, socket } = createHarness();
  const authors = Array.from({ length: 25 }, (_, index) => `User ${index + 1}`);
  for (const [index, username] of authors.entries()) {
    socket.emit("message", JSON.stringify({
      type: "media",
      payload: {
        kind: index % 2 === 0 ? "image" : "gif",
        url: `https://cdn.discordapp.com/media-${index}.png`,
        author: { username, displayAvatarUrl: "" },
      },
    }));
  }

  assert.deepEqual([...queuedAuthors()], authors.slice(1));
});

test("Discord gifv embeds render through the muted video player", () => {
  const { elements, socket } = createHarness();
  socket.emit("message", JSON.stringify({
    type: "media",
    payload: {
      kind: "gif",
      contentType: "video/mp4",
      url: "https://media.tenor.com/example.mp4",
      author: { username: "Friend", displayAvatarUrl: "" },
    },
  }));

  assert.equal(elements["#video"].src, "https://media.tenor.com/example.mp4");
  assert.equal(elements["#video"].muted, true);
  assert.equal(elements["#video"].volume, 0);
});

test("video GIFs loop until the configured GIF duration expires", () => {
  const { elements, socket, timerDelays } = createHarness();
  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { displayDurationMs: 11111, gifDurationMs: 12345 },
  }));
  socket.emit("message", JSON.stringify({
    type: "media",
    payload: { kind: "gif", contentType: "video/mp4", url: "https://media.klipy.com/gif.mp4" },
  }));

  elements["#video"].videoWidth = 408;
  elements["#video"].videoHeight = 300;
  elements["#video"].emit("loadeddata");

  assert.equal(elements["#video"].loop, true);
  assert.ok(timerDelays().includes(12345));
  assert.ok(!timerDelays().includes(11111));
});

test("static images keep their independent image duration", () => {
  const { elements, socket, timerDelays } = createHarness();
  socket.emit("message", JSON.stringify({
    type: "config",
    payload: { displayDurationMs: 11111, gifDurationMs: 12345 },
  }));
  socket.emit("message", JSON.stringify({
    type: "media",
    payload: { kind: "image", url: "https://cdn.discordapp.com/still.png" },
  }));

  elements["#image"].naturalWidth = 1280;
  elements["#image"].naturalHeight = 720;
  elements["#image"].emit("load");

  assert.ok(timerDelays().includes(11111));
  assert.ok(!timerDelays().includes(12345));
});

test("Discord picker GIFs prefer Relay's local inline media route", () => {
  const { elements, socket } = createHarness();
  socket.emit("message", JSON.stringify({
    type: "media",
    payload: {
      kind: "gif",
      contentType: "video/mp4",
      cachedMediaId: "123456789012345678-embed-0",
      url: "https://images-ext-1.discordapp.net/external/file.mp4",
    },
  }));
  assert.equal(
    elements["#video"].src,
    "/media-cache/123456789012345678-embed-0?secret=private",
  );
});

test("a stalled image advances to the next queued media", () => {
  const { elements, runNextTimer, socket } = createHarness();
  socket.emit("message", JSON.stringify({
    type: "media", payload: { kind: "image", url: "https://cdn.discordapp.com/stalled.png" },
  }));
  socket.emit("message", JSON.stringify({
    type: "media", payload: { kind: "image", url: "https://cdn.discordapp.com/next.png" },
  }));

  runNextTimer();
  runNextTimer();
  assert.equal(elements["#image"].src, "https://cdn.discordapp.com/next.png");
});

test("media and YouTube wait while a notification holds the shared stage", () => {
  const { context, elements, socket, youtubePlayers } = createHarness("?secret=private", "all");
  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, notificationBusy: true },
  }));
  socket.emit("message", JSON.stringify({
    type: "media",
    payload: { kind: "image", url: "https://cdn.discordapp.com/wait.png" },
  }));
  assert.equal(elements["#image"].src, "");

  socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: {
      playbackId: "yt-deferred",
      videoId: "dQw4w9WgXcQ",
      title: "Track",
    },
  }));
  assert.equal(vm.runInContext("youtubePlaybackId", context), undefined);
  assert.equal(youtubePlayers.length, 0);

  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, notificationBusy: false },
  }));
  assert.equal(vm.runInContext("youtubePlaybackId", context), "yt-deferred");
  assert.equal(elements["#image"].src, "");
  assert.ok(socket.sent.some((message) => (
    message.type === "stageClock"
    && message.payload?.lane === "media"
    && message.payload?.busy === true
  )));
});

test("media waits for the server stage grant before becoming visible", () => {
  const { elements, socket } = createHarness("?secret=private", "all", false);
  socket.emit("message", JSON.stringify({
    type: "media",
    payload: { kind: "image", url: "https://cdn.discordapp.com/race.png" },
  }));

  assert.equal(elements["#image"].src, "");
  assert.deepEqual(socket.sent.at(-1), {
    type: "stageClock",
    payload: { lane: "media", busy: true },
  });

  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: true, musicBusy: false, notificationBusy: false },
  }));
  assert.equal(elements["#image"].src, "https://cdn.discordapp.com/race.png");
});

test("musicStop removes a matching deferred YouTube playback", async () => {
  const harness = createHarness("?secret=private&widget=1", "all");
  harness.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: true, notificationBusy: true },
  }));
  harness.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: { playbackId: "deferred-stop", videoId: "dQw4w9WgXcQ" },
  }));
  assert.equal(vm.runInContext("deferredYoutubeQueue.length", harness.context), 1);

  harness.socket.emit("message", JSON.stringify({
    type: "musicStop",
    payload: { playbackId: "deferred-stop" },
  }));
  assert.equal(vm.runInContext("deferredYoutubeQueue.length", harness.context), 0);
  harness.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, notificationBusy: false },
  }));
  await Promise.resolve();
  assert.equal(harness.youtubePlayers.length, 0);
});

test("musicIdle drops deferred YouTube before resuming Discord media", async () => {
  const harness = createHarness("?secret=private&widget=1", "all");
  harness.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: true, notificationBusy: true },
  }));
  harness.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: { playbackId: "deferred-idle", videoId: "dQw4w9WgXcQ" },
  }));
  harness.socket.emit("message", JSON.stringify({
    type: "media",
    payload: { kind: "image", url: "https://cdn.discordapp.com/resume.png" },
  }));

  harness.socket.emit("message", JSON.stringify({ type: "musicIdle" }));
  assert.equal(vm.runInContext("deferredYoutubeQueue.length", harness.context), 0);
  harness.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, notificationBusy: false },
  }));
  await Promise.resolve();
  assert.equal(harness.youtubePlayers.length, 0);
  assert.equal(harness.elements["#image"].src, "https://cdn.discordapp.com/resume.png");
});

test("OBS /medias waits for YouTube musicBusy before showing images", () => {
  const visual = createHarness("?secret=private", "visual");
  sendMediaClock(visual);

  visual.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: true, notificationBusy: false },
  }));
  sendMedia(visual, { kind: "image", url: "https://cdn.discordapp.com/queued.png" });
  assert.equal(visual.elements["#image"].src, "", "image started over active YouTube");

  visual.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: {
      playbackId: "obs-yt",
      videoId: "dQw4w9WgXcQ",
      title: "Track",
    },
  }));
  assert.equal(visual.youtubePlayers.length, 0, "visual iframe must not embed YouTube");
  assert.equal(visual.elements["#image"].src, "");

  visual.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, notificationBusy: false },
  }));
  assert.deepEqual(
    visual.socket.sent.at(-1),
    { type: "mediaClock", payload: { busy: true } },
    "media did not request the OBS visual lease after music ended",
  );
  assert.equal(visual.elements["#image"].src, "");

  sendMediaGrant(visual, true, { videoBusy: true });
  assert.equal(visual.elements["#image"].src, "https://cdn.discordapp.com/queued.png");
});

test("OBS /youtube waits for peer mediaBusy before starting music", async () => {
  const youtube = createHarness("?secret=private", "youtube");
  youtube.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: true, musicBusy: false, notificationBusy: false },
  }));
  youtube.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: {
      playbackId: "wait-media",
      videoId: "dQw4w9WgXcQ",
      title: "Track",
    },
  }));
  await Promise.resolve();
  assert.equal(vm.runInContext("youtubePlaybackId", youtube.context), undefined);
  assert.equal(youtube.youtubePlayers.length, 0);

  youtube.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: true, notificationBusy: false },
  }));
  await Promise.resolve();
  assert.equal(vm.runInContext("youtubePlaybackId", youtube.context), "wait-media");
  assert.equal(youtube.youtubePlayers.length, 1);
});

test("OBS /audios waits for YouTube before Discord file audio", () => {
  const audio = createHarness("?secret=private", "audio");
  sendMediaClock(audio);
  audio.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: true, notificationBusy: false },
  }));
  sendMedia(audio, { kind: "audio", url: "https://cdn.discordapp.com/track.mp3" });
  assert.equal(audio.elements["#audio"].src, "", "Discord audio started over YouTube");

  audio.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, notificationBusy: false },
  }));
  assert.deepEqual(audio.socket.sent.at(-1), { type: "mediaClock", payload: { busy: true } });
  sendMediaGrant(audio, true, { audioBusy: true });
  assert.equal(audio.elements["#audio"].src, "https://cdn.discordapp.com/track.mp3");
});

test("OBS /medias waits for peer Discord-audio mediaBusy before showing a GIF", () => {
  const visual = createHarness("?secret=private", "visual");
  sendMediaClock(visual);
  visual.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: true, musicBusy: false, notificationBusy: false },
  }));
  sendMedia(visual, { kind: "gif", url: "https://cdn.discordapp.com/overlap.gif" });
  assert.equal(visual.elements["#image"].src, "", "GIF started over peer Discord audio");

  visual.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, notificationBusy: false },
  }));
  assert.deepEqual(visual.socket.sent.at(-1), { type: "mediaClock", payload: { busy: true } });
  sendMediaGrant(visual, true, { videoBusy: true });
  assert.equal(visual.elements["#image"].src, "https://cdn.discordapp.com/overlap.gif");
});

test("OBS visual recovers when stageClock clears a missed musicIdle", () => {
  const visual = createHarness("?secret=private", "visual");
  sendMediaClock(visual);
  visual.socket.emit("message", JSON.stringify({
    type: "musicPlay",
    payload: {
      playbackId: "stuck",
      videoId: "dQw4w9WgXcQ",
      title: "Track",
    },
  }));
  sendMedia(visual, { kind: "gif", url: "https://cdn.discordapp.com/loop.gif" });
  assert.equal(visual.elements["#image"].src, "");

  // Server watch clock recovers without a musicIdle relay event.
  visual.socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: false, musicBusy: false, notificationBusy: false },
  }));
  assert.deepEqual(visual.socket.sent.at(-1), { type: "mediaClock", payload: { busy: true } });
  sendMediaGrant(visual, true, { videoBusy: true });
  assert.equal(visual.elements["#image"].src, "https://cdn.discordapp.com/loop.gif");
});

test("the combined audio output no longer embeds the voice source", () => {
  const html = fs.readFileSync(__dirname + "/obs-audio.html", "utf8");
  assert.doesNotMatch(html, /src="\/tts"/);
  assert.match(html, /src="\/audios"/);
});

test("an OBS page reloads only when Relay restarted since it was opened", () => {
  const { socket, location, context } = createHarness("?secret=private", "youtube");
  const config = (payload) => socket.emit("message", JSON.stringify({ type: "config", payload }));
  config({ relaySession: "first", musicObsGeometry: { contentScale: 150 } });
  config({ relaySession: "first", musicObsGeometry: { contentScale: 200 } });
  assert.equal(location.reloads, 0);
  assert.equal(vm.runInContext("config.musicObsGeometry.contentScale", context), 200);

  // Reconnected to a restarted (maybe updated) Relay: load the current page.
  config({ relaySession: "second", musicObsGeometry: { contentScale: 300 } });
  assert.equal(location.reloads, 1);
  assert.equal(vm.runInContext("config.musicObsGeometry.contentScale", context), 200);
});

test("the OBS YouTube video has its own size, anchor and crop", () => {
  const youtube = createHarness("?secret=private", "youtube");
  const root = youtube.context.document.documentElement;
  const config = (payload) => youtube.socket.emit("message", JSON.stringify({ type: "config", payload }));
  config({ musicVideoObsGeometry: {} });
  // Defaults keep the untouched full-canvas player.
  assert.equal(root.classList.contains("youtube-video-sized"), false);

  config({
    mediaObsGeometry: { contentScale: 80, cropTop: 30 },
    musicObsGeometry: { contentScale: 200, cropTop: 25 },
    musicVideoObsGeometry: { contentScale: 60, anchor: "bottomRight", marginX: 40, marginY: 30, cropTop: 10, cropLeft: 5 },
  });
  assert.equal(root.classList.contains("youtube-video-sized"), true);
  assert.equal(youtube.cssProperties["--video-scale"], "0.6");
  assert.equal(youtube.cssProperties["--video-origin"], "right bottom");
  assert.equal(youtube.cssProperties["--video-shift-x"], "-40px");
  assert.equal(youtube.cssProperties["--video-shift-y"], "-30px");
  assert.equal(youtube.cssProperties["--video-crop-top"], "10%");
  assert.equal(youtube.cssProperties["--video-crop-left"], "5%");
  // The crop applies to the video only, never to the whole canvas and card.
  assert.equal(youtube.cssProperties["--crop-top"], "0%");
  assert.equal(youtube.cssProperties["--credit-scale"], "2");
  // The card follows the visible video: 60% anchored bottom right, 40/30px margins, crops.
  assert.equal(youtube.cssProperties["--video-inset-left"], "max(0px, calc(43% + -40px))");
  assert.equal(youtube.cssProperties["--video-inset-bottom"], "max(0px, calc(0% + 30px))");
  assert.equal(youtube.cssProperties["--video-inset-top"], "max(0px, calc(46% + -30px))");
  assert.equal(youtube.cssProperties["--video-inset-right"], "max(0px, calc(0% + 40px))");

  config({ musicVideoObsGeometry: { contentScale: 300, anchor: "legacy" } });
  assert.equal(youtube.cssProperties["--video-scale"], "3");
  assert.equal(youtube.cssProperties["--video-origin"], "center center");
  assert.equal(youtube.cssProperties["--video-shift-x"], "0px");

  // The video alone can shrink to a small corner player, down to 10%.
  config({ musicVideoObsGeometry: { contentScale: 20 } });
  assert.equal(youtube.cssProperties["--video-scale"], "0.2");
  config({ musicVideoObsGeometry: { contentScale: 5 } });
  assert.equal(youtube.cssProperties["--video-scale"], "0.1");

  // The media page and the Windows widget never move the YouTube player.
  const media = createHarness("?secret=private", "visual");
  media.socket.emit("message", JSON.stringify({ type: "config", payload: { musicVideoObsGeometry: { contentScale: 50 } } }));
  assert.equal(media.cssProperties["--video-scale"], undefined);
});

test("the OBS YouTube card uses its own size and anchor, not the media settings", () => {
  const preview = createHarness("?secret=private&preview=1&sample=music");
  const credit = preview.elements["#youtube-credit"];
  assert.equal(credit.hidden, false);
  assert.equal(credit.classList.contains("is-visible"), true);

  preview.socket.emit("message", JSON.stringify({
    type: "config",
    payload: {
      mediaObsGeometry: { contentScale: 50 },
      musicObsGeometry: { contentScale: 300, anchor: "bottomRight", marginX: 30, marginY: 60 },
    },
  }));
  assert.equal(preview.cssProperties["--content-scale"], "3");
  assert.equal(preview.cssProperties["--credit-scale"], "3");
  // The card grows from its anchored corner: margins stay in canvas pixels.
  assert.equal(credit.style.transformOrigin, "right bottom");
  // Margins count from the visible YouTube video.
  assert.equal(credit.style.right, "calc(var(--video-inset-right, 0px) + 30px)");
  assert.equal(credit.style.bottom, "calc(var(--video-inset-bottom, 0px) + 60px)");
  assert.equal(credit.style.left, "auto");
  assert.equal(credit.style.top, "auto");

  preview.socket.emit("message", JSON.stringify({
    type: "config",
    payload: { musicObsGeometry: { contentScale: 100, anchor: "legacy" } },
  }));
  assert.equal(preview.cssProperties["--credit-scale"], "1");
  assert.equal(credit.style.transformOrigin, "");
  assert.equal(credit.style.right, "");
  assert.equal(credit.style.bottom, "");

  // The YouTube page itself follows the same setting.
  const youtube = createHarness("?secret=private", "youtube");
  youtube.socket.emit("message", JSON.stringify({
    type: "config",
    payload: { mediaObsGeometry: { contentScale: 80 }, musicObsGeometry: { contentScale: 250, anchor: "topCenter", marginY: 25 } },
  }));
  const youtubeCredit = youtube.elements["#youtube-credit"];
  assert.equal(youtube.cssProperties["--credit-scale"], "2.5");
  assert.equal(youtubeCredit.style.transformOrigin, "center top");
  assert.equal(youtubeCredit.style.marginInline, "auto");
  assert.equal(youtubeCredit.style.top, "calc(var(--video-inset-top, 0px) + 25px)");

  // The Windows widget card keeps its own layout.
  const widget = createHarness("?secret=private&widget=1", "visual");
  widget.socket.emit("message", JSON.stringify({
    type: "config",
    payload: { musicObsGeometry: { contentScale: 300, anchor: "topLeft" } },
  }));
  assert.equal(widget.cssProperties["--credit-scale"], undefined);
});
