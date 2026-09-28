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
  };
}

function createHarness() {
  const elements = {
    "#sticker": {
      alt: "",
      classList: classList(),
      onerror: null,
      onload: null,
      src: "",
      removeAttribute(name) { if (name === "src") this.src = ""; },
    },
    "#sticker-fallback": {
      classList: classList(),
      hidden: true,
    },
  };
  const sockets = [];
  class MockWebSocket {
    constructor(url) {
      this.url = url;
      this.listeners = new Map();
      this.readyState = 1;
      this.sent = [];
      sockets.push(this);
    }
    send(data) { this.sent.push(JSON.parse(data)); }
    addEventListener(type, listener) { this.listeners.set(type, listener); }
    emit(type, data) { this.listeners.get(type)?.({ data }); }
    close() {}
  }
  const window = {
    location: {
      protocol: "http:", host: "127.0.0.1:4590", port: "4590",
      href: "http://127.0.0.1:4590/stickers?secret=private", search: "?secret=private",
      replace() {},
    },
    addEventListener() {},
    clearTimeout() {},
    requestAnimationFrame(callback) { callback(); },
    setTimeout() { return 1; },
  };
  const context = vm.createContext({
    URL,
    URLSearchParams,
    WebSocket: MockWebSocket,
    document: { querySelector: (selector) => elements[selector] },
    encodeURIComponent,
    window,
  });
  vm.runInContext(fs.readFileSync(__dirname + "/../outputs/connection.js", "utf8"), context);
  vm.runInContext(fs.readFileSync(__dirname + "/stickers.js", "utf8"), context);
  return { elements, socket: sockets[0] };
}

test("sticker output identifies itself and accepts isolated tests", () => {
  const { elements, socket } = createHarness();
  assert.match(socket.url, /role=sticker&source=sticker&client=obs&secret=private$/);

  socket.emit("message", JSON.stringify({
    type: "testOutput",
    payload: {
      target: "sticker",
      sticker: {
        name: "Relay sticker test",
        format: "png",
        url: "/overlay-assets/relay-radar.png",
      },
    },
  }));

  assert.equal(elements["#sticker"].src, "");
  socket.emit("message", JSON.stringify({
    type: "stageClock",
    payload: { mediaBusy: true, musicBusy: false, notificationBusy: false },
  }));
  assert.equal(elements["#sticker"].src, "/overlay-assets/relay-radar.png");
  assert.equal(elements["#sticker"].alt, "Relay sticker test");
});

test("the skip shortcut ends the sticker on screen and releases the media stage", () => {
  const { elements, socket } = createHarness();
  const sticker = (name) => ({ type: "sticker", payload: { name, format: "png", url: `/stickers/${name}.png` } });
  socket.emit("message", JSON.stringify(sticker("first")));
  socket.emit("message", JSON.stringify(sticker("second")));
  socket.emit("message", JSON.stringify({ type: "stageClock", payload: { mediaBusy: true } }));
  assert.equal(elements["#sticker"].alt, "first");
  const before = socket.sent.length;

  socket.emit("message", JSON.stringify({ type: "skip" }));
  // The first sticker is gone and the stage is released.
  assert.equal(elements["#sticker"].src, "");
  assert.deepEqual(socket.sent.slice(before).map((message) => message.payload), [{ lane: "media", busy: false }]);
  // Once the server confirms, the page claims the stage for the next sticker.
  socket.emit("message", JSON.stringify({ type: "stageClock", payload: { mediaBusy: false } }));
  assert.deepEqual(socket.sent.at(-1).payload, { lane: "media", busy: true });
  socket.emit("message", JSON.stringify({ type: "stageClock", payload: { mediaBusy: true } }));
  assert.equal(elements["#sticker"].alt, "second");
});

test("sticker reconnect keeps pending work and probes a moved server", () => {
  const source = fs.readFileSync(__dirname + "/stickers.js", "utf8");
  assert.match(
    source,
    /function interruptStickerPlayback\(\)\s*\{(?![^}]*queue\.length\s*=\s*0)[^}]*currentSticker\s*=\s*undefined/s,
  );
  assert.match(source, /socket\.addEventListener\("close"[\s\S]*interruptStickerPlayback\(\)/);
  assert.match(source, /function moveToPendingPort\(\)[\s\S]*RelayConnection\.moveToPort\(pendingPort/);
  const shared = fs.readFileSync(__dirname + "/../outputs/connection.js", "utf8");
  assert.match(shared, /moveToPort\([\s\S]*new WebSocket[\s\S]*probe\.addEventListener\("open"/);
  assert.match(shared, /probeWatchdog[\s\S]*probe\.close\(\)/);
  assert.match(
    source,
    /configuredPort[\s\S]*Number\.isInteger\(configuredPort\)[\s\S]*configuredPort <= 65535/,
  );
});
