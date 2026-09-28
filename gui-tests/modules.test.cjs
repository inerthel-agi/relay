const assert = require("node:assert/strict");
const test = require("node:test");

const { initializeModules } = require("../gui/modules.mjs");

class FakeElement {
  constructor(tagName, id = "") {
    this.tagName = tagName.toUpperCase();
    this.id = id;
    this.children = [];
    this.parentElement = null;
    this.parentNode = null;
    this.listeners = new Map();
    this.attributes = new Map();
    this.dataset = {};
    this.style = {};
    this.className = "";
    this.type = "";
    this.value = "";
    this.checked = false;
    this.disabled = false;
    this.hidden = false;
    this.readOnly = false;
    this.min = "";
    this.max = "";
    this.placeholder = "";
    this.focused = false;
    this._textContent = "";
  }

  get textContent() {
    if (this._textContent) return this._textContent;
    return this.children.map((child) => child.textContent || "").join("");
  }

  set textContent(value) {
    this._textContent = String(value ?? "");
    this.children = [];
  }

  get firstChild() {
    return this.children[0];
  }

  append(...nodes) {
    for (const child of nodes.flat()) {
      if (child == null) continue;
      this.children.push(child);
      if (typeof child === "object") {
        child.parentElement = this;
        child.parentNode = this;
      }
    }
    return this;
  }

  appendChild(child) {
    this.append(child);
    return child;
  }

  replaceChildren(...nodes) {
    this.children = [];
    this._textContent = "";
    this.append(...nodes);
  }

  remove() {
    if (!this.parentElement) return;
    this.parentElement.children = this.parentElement.children.filter((child) => child !== this);
    this.parentElement = null;
    this.parentNode = null;
  }

  setAttribute(name, value) {
    this.attributes.set(name, String(value));
    if (name === "role") this.role = String(value);
  }

  addEventListener(type, listener) {
    const listeners = this.listeners.get(type) || [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }

  async dispatch(type, event = {}) {
    const results = [];
    for (const listener of this.listeners.get(type) || []) {
      results.push(listener({ target: this, currentTarget: this, ...event }));
    }
    await Promise.all(results);
  }

  async click() {
    await this.dispatch("click");
  }

  async input(value) {
    this.value = value;
    await this.dispatch("input");
  }

  async change(value) {
    this.value = value;
    await this.dispatch("change");
  }

  focus() {
    this.focused = true;
  }

  closest(selector) {
    let current = this;
    while (current) {
      if (selector === "section" && current.tagName === "SECTION") return current;
      current = current.parentElement;
    }
    return null;
  }

  matches(selector) {
    if (selector.includes(",")) return selector.split(",").some((part) => this.matches(part.trim()));
    if (selector === "button") return this.tagName === "BUTTON";
    if (selector === "input") return this.tagName === "INPUT";
    if (/^[a-z]+$/i.test(selector)) return this.tagName === selector.toUpperCase();
    if (selector.startsWith(".")) return this.className.split(/\s+/).includes(selector.slice(1));
    if (selector === "[data-i18n]") return Boolean(this.dataset.i18n);
    const roleMatch = selector.match(/^\[role="([^"]+)"\]$/);
    if (roleMatch) return this.attributes.get("role") === roleMatch[1];
    if (selector === '[role="status"]') return this.attributes.get("role") === "status";
    return false;
  }

  querySelector(selector) {
    return this.querySelectorAll(selector)[0] || null;
  }

  querySelectorAll(selector) {
    const found = [];
    const visit = (element) => {
      for (const child of element.children) {
        if (child.matches?.(selector)) found.push(child);
        visit(child);
      }
    };
    visit(this);
    return found;
  }
}

function deferred() {
  let resolve;
  const promise = new Promise((result) => { resolve = result; });
  return { promise, resolve };
}

async function flush() {
  for (let index = 0; index < 5; index += 1) {
    await new Promise((resolve) => setImmediate(resolve));
  }
}

function createHarness({
  library = [{ id: "asset-1", name: "Funny.png", kind: "image", contentType: "image/png", sizeBytes: 1024 * 1024 }],
  reactions = { enabled: false, widgetEnabled: false, globalCooldownSeconds: 10, memberCooldownSeconds: 30, musicPercent: 25, allowedChannelIds: [], allowedRoleIds: [], geometry: { anchor: "legacy", marginX: 0, marginY: 0, contentScale: 100 }, definitions: [] },
  musicQueue = [],
  messageStatus = { current: null, pinned: false },
  previewLibraryAsset = null,
  reactionTriggers = [],
} = {}) {
  const roots = new Map([
    ["message-module", new FakeElement("section", "message-module")],
    ["library-module", new FakeElement("section", "library-module")],
    ["music-queue-module", new FakeElement("section", "music-queue-module")],
    ["reaction-module", new FakeElement("section", "reaction-module")],
  ]);
  const calls = [];
  const revokedUrls = [];
  const audioInstances = [];
  const listeners = new Map();
  let libraryResult = { items: library, totalBytes: library.reduce((sum, item) => sum + item.sizeBytes, 0) };
  let musicResult = musicQueue;
  let currentMessageStatus = messageStatus;
  let previewAssetResult = previewLibraryAsset || { bytes: [137, 80, 78, 71], contentType: "image/png" };

  const document = {
    getElementById(id) { return roots.get(id) || null; },
    createElement(tagName) { return new FakeElement(tagName); },
    createTextNode(textContent) {
      const text = new FakeElement("span");
      text.nodeType = 3;
      text.textContent = textContent;
      return text;
    },
  };
  const window = {
    addEventListener(type, listener) { listeners.set(type, listener); },
  };
  const previous = {
    document: global.document,
    window: global.window,
    URL: global.URL,
    Audio: global.Audio,
    navigator: global.navigator,
  };
  let nextObjectUrl = 1;
  global.document = document;
  global.window = window;
  global.URL = {
    createObjectURL() { const url = `blob:test-${nextObjectUrl++}`; return url; },
    revokeObjectURL(url) { revokedUrls.push(url); },
  };
  window.AudioContext = class FakeAudioContext {
    constructor() { this.destination = {}; this.pauseCount = 0; this.startCount = 0; audioInstances.push(this); }
    async resume() { this.resumed = true; }
    async decodeAudioData() { return { duration: 5 }; }
    createGain() { const gain = { value: 1 }; this.gain = gain; return { gain, connect() {} }; }
    createBufferSource() { const context = this; return { connect() {}, start() { context.startCount++; }, stop() { context.pauseCount++; } }; }
    async close() { this.closed = true; }
  };
  global.navigator = { clipboard: { async writeText() {} } };

  const invoke = async (name, args) => {
    calls.push({ name, args });
    switch (name) {
      case "get_media_library": return libraryResult;
      case "get_reactions": return reactions;
      case "get_message_status": return currentMessageStatus;
      case "pin_message": currentMessageStatus = { ...currentMessageStatus, pinned: true }; return currentMessageStatus;
      case "unpin_message": currentMessageStatus = { ...currentMessageStatus, pinned: false }; return currentMessageStatus;
      case "get_music_queue": return musicResult;
      case "preview_reaction_sound": return [1, 2, 3, 4];
      case "preview_library_asset":
        if (previewAssetResult && typeof previewAssetResult.then === "function") return previewAssetResult;
        return previewAssetResult;
      case "import_reaction_sound": return null;
      case "trigger_reaction": {
        const result = reactionTriggers.shift();
        if (result instanceof Error) throw result;
        return result || { position: 0 };
      }
      case "stop_reaction": return undefined;
      default: return undefined;
    }
  };
  const getBootstrap = () => ({ config: { port: 4590, musicMaxPendingPerUser: 3, musicRejectDuplicatePending: true } });
  const t = (key) => key === "modReactionQueued" ? "Queued #{position}" : key;
  const controls = initializeModules({ invoke, t, getBootstrap });

  return {
    roots,
    calls,
    revokedUrls,
    audioInstances,
    controls,
    setMessageStatus(value) { currentMessageStatus = value; },
    setPreviewAsset(value) { previewAssetResult = value; },
    setLibrary(value) { libraryResult = value; },
    setMusicQueue(value) { musicResult = value; },
    cleanup() {
      global.document = previous.document;
      global.window = previous.window;
      global.URL = previous.URL;
      global.Audio = previous.Audio;
      global.navigator = previous.navigator;
    },
  };
}

function elementsByI18n(root, key) {
  return root.querySelectorAll("[data-i18n]").filter((element) => element.dataset.i18n === key);
}

function buttonByI18n(root, key, index = 0) {
  return elementsByI18n(root, key).filter((element) => element.tagName === "BUTTON")[index];
}

function inputByI18n(root, key, index = 0) {
  return elementsByI18n(root, key).filter((element) => element.tagName === "INPUT")[index];
}

test("media library CRUD uses stable IDs and confirms deletion", async () => {
  const harness = createHarness();
  try {
    await flush();
    const root = harness.roots.get("library-module");
    const name = root.querySelector("ul").children[0].querySelector("input");
    name.value = "Renamed.gif";
    await buttonByI18n(root, "modRename").click();
    assert.deepEqual(harness.calls.find(({ name: method }) => method === "rename_library_media")?.args, {
      id: "asset-1",
      name: "Renamed.gif",
    });

    await buttonByI18n(root, "modPlay").click();
    assert.deepEqual(harness.calls.find(({ name: method }) => method === "play_library_media")?.args, {
      id: "asset-1",
    });

    const deleteButton = buttonByI18n(root, "modDelete");
    const deletePromise = deleteButton.click();
    await flush();
    const confirmation = root.querySelector('[role="group"]');
    assert.ok(confirmation, "deletion requires an inline confirmation");
    await buttonByI18n(confirmation, "modDelete").click();
    await deletePromise;
    assert.deepEqual(harness.calls.find(({ name: method }) => method === "delete_library_media")?.args, {
      id: "asset-1",
    });
    assert.ok(harness.calls.some(({ name }) => name === "get_media_library"));
  } finally {
    harness.cleanup();
  }
});

test("music queue controls send the selected ID and direction", async () => {
  const harness = createHarness({
    musicQueue: [
      { id: "song-1", title: "First", author: "Alice" },
      { id: "song-2", title: "Second", author: "Bob" },
    ],
  });
  try {
    await flush();
    const root = harness.roots.get("music-queue-module");
    const firstRow = root.querySelector("ol").children[0];
    const secondRow = root.querySelector("ol").children[1];
    await buttonByI18n(firstRow, "modDown").click();
    assert.deepEqual(harness.calls.find(({ name }) => name === "move_music_queue")?.args, {
      id: "song-1",
      direction: "down",
    });

    await buttonByI18n(secondRow, "modUp").click();
    const moves = harness.calls.filter(({ name }) => name === "move_music_queue");
    assert.deepEqual(moves[1]?.args, { id: "song-2", direction: "up" });
  } finally {
    harness.cleanup();
  }
});

test("pinning sends the currently displayed message ID", async () => {
  const harness = createHarness({
    messageStatus: {
      current: { id: "message-42", text: "Keep this", author: { username: "Alice" } },
      pinned: false,
    },
  });
  try {
    await flush();
    const root = harness.roots.get("message-module");
    const pinButton = buttonByI18n(root, "modPin");
    await pinButton.click();
    assert.deepEqual(harness.calls.find(({ name }) => name === "pin_message")?.args, {
      messageId: "message-42",
    });
    assert.equal(pinButton.disabled, true);
  } finally {
    harness.cleanup();
  }
});

test("local reaction preview never triggers live playback and releases resources on stop", async () => {
  const harness = createHarness({
    reactions: {
      enabled: true,
      widgetEnabled: false,
      globalCooldownSeconds: 10,
      memberCooldownSeconds: 30,
      musicPercent: 25,
      allowedChannelIds: [],
      allowedRoleIds: [],
      geometry: { anchor: "legacy", marginX: 0, marginY: 0, contentScale: 100 },
      definitions: [{ id: "reaction-1", name: "Applause", soundId: "sound-1", visualId: "asset-1", volume: 70, enabled: true }],
    },
  });
  try {
    await flush();
    const root = harness.roots.get("reaction-module");
    await buttonByI18n(root, "modTestLocal").click();
    assert.deepEqual(harness.calls.find(({ name }) => name === "preview_reaction_sound")?.args, { id: "sound-1" });
    assert.deepEqual(harness.calls.find(({ name }) => name === "preview_library_asset")?.args, { id: "asset-1" });
    assert.equal(harness.calls.some(({ name }) => name === "trigger_reaction"), false);
    assert.equal(harness.audioInstances.length, 1);
    assert.equal(harness.audioInstances[0].gain.value, 0.7);
    assert.equal(root.querySelector(".reaction-local-preview").hidden, false);

    await buttonByI18n(root, "modStop").click();
    assert.equal(harness.audioInstances[0].pauseCount, 1);
    assert.deepEqual(harness.calls.find(({ name }) => name === "stop_reaction")?.args, undefined);
    assert.equal(root.querySelector(".reaction-local-preview").hidden, true);
    assert.equal(root.querySelector(".reaction-local-preview").children.length, 0);
    assert.equal(harness.revokedUrls.length, 1);
  } finally {
    harness.cleanup();
  }
});

test("stopping a pending reaction preview prevents late visual IPC from reviving it", async () => {
  const pendingAsset = deferred();
  const harness = createHarness({
    previewLibraryAsset: pendingAsset.promise,
    reactions: {
      enabled: true,
      widgetEnabled: false,
      globalCooldownSeconds: 10,
      memberCooldownSeconds: 30,
      musicPercent: 25,
      allowedChannelIds: [],
      allowedRoleIds: [],
      geometry: { anchor: "legacy", marginX: 0, marginY: 0, contentScale: 100 },
      definitions: [{ id: "reaction-1", name: "Applause", soundId: "sound-1", visualId: "asset-1", volume: 70, enabled: true }],
    },
  });
  try {
    await flush();
    const root = harness.roots.get("reaction-module");
    const previewPromise = buttonByI18n(root, "modTestLocal").click();
    await flush();
    await buttonByI18n(root, "modStop").click();
    pendingAsset.resolve({ bytes: [1, 2, 3], contentType: "image/png" });
    await previewPromise;
    assert.equal(root.querySelector(".reaction-local-preview").hidden, true);
    assert.equal(root.querySelector(".reaction-local-preview").children.length, 0);
    assert.equal(harness.audioInstances.length, 1);
    assert.equal(harness.audioInstances[0].startCount, 0);
    assert.equal(harness.audioInstances[0].closed, true);
  } finally {
    harness.cleanup();
  }
});


test("reaction disclosures retain expanded state and draft settings after library refresh", async () => {
  const harness = createHarness();
  try {
    await flush();
    const root = harness.roots.get("reaction-module");
    const general = elementsByI18n(root, "modReactionGeneral")[0].parentElement;
    assert.equal(general.open, false);
    general.open = true;
    await general.dispatch("toggle");
    const cooldown = elementsByI18n(root, "modGlobalCooldown")[0].parentElement.querySelector("input");
    await cooldown.input("42");
    await harness.controls.loadLibrary();
    assert.equal(elementsByI18n(root, "modReactionGeneral")[0].parentElement.open, true);
    assert.equal(elementsByI18n(root, "modGlobalCooldown")[0].parentElement.querySelector("input").value, 42);
    assert.equal(elementsByI18n(root, "modReactionAccess")[0].parentElement.open, false);
    // Leaving the field saves the reactions without a Save button.
    const editor = root.children.find((child) => child.className === "reaction-editor");
    const edited = elementsByI18n(root, "modGlobalCooldown")[0].parentElement.querySelector("input");
    await editor.dispatch("change", { target: edited });
    await new Promise((resolve) => setTimeout(resolve, 5));
    await flush();
    assert.equal(harness.calls.find(({ name }) => name === "save_reactions").args.settings.globalCooldownSeconds, 42);
  } finally { harness.cleanup(); }
});


test("reaction play distinguishes immediate playback, FIFO position and queue-full refusal", async () => {
  const harness = createHarness({
    reactionTriggers: [{ position: 0 }, { position: 2 }, new Error("Reaction queue is full.")],
    reactions: { enabled: true, globalCooldownSeconds: 10, memberCooldownSeconds: 30, musicPercent: 25,
      allowedChannelIds: [], allowedRoleIds: [], geometry: { anchor: "legacy", marginX: 0, marginY: 0, contentScale: 100 },
      definitions: [{ id: "one", name: "One", soundId: "sound", volume: 70, enabled: true }] },
  });
  try {
    await flush();
    const root = harness.roots.get("reaction-module");
    const play = buttonByI18n(root, "modPlay");
    await play.click();
    assert.equal(root.querySelector('[role="status"]').textContent, "modReactionStarted");
    await play.click();
    assert.equal(root.querySelector('[role="status"]').textContent, "Queued #2");
    await play.click();
    assert.equal(root.querySelector('[role="status"]').textContent, "modErrorQueueFull");
  } finally { harness.cleanup(); }
});
