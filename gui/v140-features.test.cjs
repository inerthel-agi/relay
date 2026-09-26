const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");

const { describeDiscordCheck } = require("./discord-check.mjs");
const { describeInstalledSources, parseObsPort } = require("./obs-setup.mjs");
const { initializePanic } = require("./panic.mjs");

const t = (key) => key;
const format = (key, values) => `${key}:${JSON.stringify(values)}`;
const panelHtml = fs.readFileSync(__dirname + "/panel.html", "utf8");
const trayHtml = fs.readFileSync(__dirname + "/tray.html", "utf8");

test("the bot check lists missing permissions with Discord's names", () => {
  const summary = describeDiscordCheck({
    botConnected: true,
    messageContentIntent: "enabled",
    checks: [
      { feature: "media", channelName: "media", status: "ok", missing: [] },
      { feature: "music", channelName: "music", status: "missingPermissions", missing: ["SEND_MESSAGES", "EMBED_LINKS"] },
      { feature: "honeypot", channelName: null, status: "notConfigured", missing: [] },
    ],
  }, t, format);
  assert.equal(summary.state, "error");
  assert.equal(summary.needsInvite, true);
  assert.equal(summary.intent, null);
  assert.equal(summary.rows[0].state, "saved");
  assert.equal(summary.rows[1].text, 'checkMissing:{"permissions":"permSendMessages, permEmbedLinks"}');
  assert.equal(summary.rows[2].state, "idle");
});

test("a disabled intent is reported even when the bot cannot connect", () => {
  const offline = describeDiscordCheck({ botConnected: false, messageContentIntent: "missing", checks: [] }, t, format);
  assert.equal(offline.state, "error");
  assert.equal(offline.intent, "intentMissing");
  const waiting = describeDiscordCheck({ botConnected: false, messageContentIntent: "unknown", checks: [] }, t, format);
  assert.equal(waiting.message, "checkBotOffline");
  const ready = describeDiscordCheck({ botConnected: true, messageContentIntent: "enabled", checks: [] }, t, format);
  assert.equal(ready.state, "saved");
  assert.equal(ready.message, "checkAllGood");
});

test("OBS setup validates the port and describes each installed source", () => {
  assert.equal(parseObsPort("4455"), 4455);
  assert.equal(parseObsPort("0"), null);
  assert.equal(parseObsPort("70000"), null);
  assert.equal(parseObsPort("abc"), null);
  assert.equal(
    describeInstalledSources([{ name: "Relay Visual", action: "created" }, { name: "Relay Audio", action: "added" }], t),
    "Relay Visual (obsCreated), Relay Audio (obsAdded)",
  );
});

test("the panic button pauses, shows the banner and resumes", async () => {
  const elements = {};
  const element = (id) => elements[id] ??= {
    hidden: false, disabled: false, title: "", attributes: {}, listeners: {},
    classList: { values: new Set(), toggle(name, on) { if (on) this.values.add(name); else this.values.delete(name); } },
    setAttribute(name, value) { this.attributes[name] = value; },
    addEventListener(type, listener) { this.listeners[type] = listener; },
  };
  const calls = [];
  global.document = { documentElement: element("root") };
  try {
    const panic = initializePanic({
      $: (selector) => element(selector), t, notify() {}, getShortcutLabel: () => "Ctrl Alt P",
      invoke: async (command) => { calls.push(command); },
    });
    panic.setPaused(false);
    assert.equal(elements["#paused-banner"].hidden, true);
    await elements["#panic-button"].listeners.click();
    assert.deepEqual(calls, ["panic_stop"]);
    assert.equal(panic.isPaused(), true);
    assert.equal(elements["#paused-banner"].hidden, false);
    await elements["#resume-button"].listeners.click();
    assert.deepEqual(calls, ["panic_stop", "resume_outputs"]);
    assert.equal(panic.isPaused(), false);
    panic.applyLanguage();
    assert.equal(elements["#panic-button"].title, "panicTitle (Ctrl Alt P)");
  } finally {
    delete global.document;
  }
});

test("the panel and tray expose the panic controls and the OBS password stays a password field", () => {
  assert.match(panelHtml, /id="panic-button"/);
  assert.match(panelHtml, /id="panic-shortcut-capture"/);
  assert.match(panelHtml, /id="obs-password" type="password"/);
  assert.match(trayHtml, /id="panic-toggle"/);
  assert.doesNotMatch(trayHtml, /widget-icon[^>]*>\s*[MT]\s*</);
});

test("right click never opens the browser menu, except in text fields of the panel", () => {
  for (const file of ["panel.js", "tray.js", "../notifications/notifications.js", "../overlay/overlay.js", "../stickers/stickers.js"]) {
    const source = fs.readFileSync(__dirname + "/" + file, "utf8");
    assert.match(source, /addEventListener\("contextmenu"[\s\S]{0,160}preventDefault\(\)/, file);
  }
  const panel = fs.readFileSync(__dirname + "/panel.js", "utf8");
  assert.match(panel, /closest\?\.\("input, textarea, \[contenteditable='true'\]"\)/);
});
