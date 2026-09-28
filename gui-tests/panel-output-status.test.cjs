const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

const panelSource = require("./test-source.cjs").panelSource();
const panelHtml = fs.readFileSync(__dirname + "/../gui/panel.html", "utf8");
const readinessSource = panelSource.slice(
  panelSource.indexOf("function outputClientCount"),
  panelSource.indexOf("function setServerStatus"),
);
const targets = ["visual", "audio", "notification", "sticker"];

function classList() {
  const values = new Set();
  return {
    toggle(name, enabled) {
      if (enabled) values.add(name);
      else values.delete(name);
    },
    contains(name) { return values.has(name); },
  };
}

function createHarness() {
  const cards = new Map();
  const states = new Map();
  const lastConnected = new Map();
  const testButtons = new Map();
  for (const target of targets) {
    cards.set(target, { classList: classList(), querySelector: () => ({ textContent: "" }), append() {} });
    states.set(target, { textContent: "" });
    lastConnected.set(target, { textContent: "" });
    testButtons.set(target, { disabled: false, title: "" });
  }
  const translations = {
    outputObs: "OBS", outputPreview: "Preview", outputWidget: "Widget",
    outputDisconnected: "Not connected", outputLastConnected: "Last connected",
    outputNeverConnected: "Never connected", outputTestNeedsLiveOutput: "Connect a live output first.",
  };
  const context = vm.createContext({
    Date,
    Math,
    Number,
    language: "en",
    bootstrap: { config: {} },
    outputReadinessCards: cards,
    outputStateElements: states,
    outputLastConnectedElements: lastConnected,
    outputTestButtons: testButtons,
    t: (key) => translations[key],
  });
  vm.runInContext(`${readinessSource}\nglobalThis.renderOutputReadinessForTest = renderOutputReadiness;`, context);
  return {
    cards,
    lastConnected,
    render: context.renderOutputReadinessForTest,
    states,
    testButtons,
  };
}

test("output readiness distinguishes live OBS, previews, widgets, and missing outputs", () => {
  const harness = createHarness();
  harness.render({
    outputs: {
      visual: {
        obsClients: 2,
        previewClients: 1,
        widgetClients: 3,
        lastConnectedAt: Date.UTC(2026, 6, 12, 10, 30, 0),
      },
      notification: { previewClients: 1 },
    },
  });

  assert.equal(harness.states.get("visual").textContent, "OBS 2 · Preview 1 · Widget 3");
  assert.match(harness.lastConnected.get("visual").textContent, /^Last connected /);
  assert.equal(harness.cards.get("visual").classList.contains("is-live"), true);
  assert.equal(harness.testButtons.get("visual").disabled, false);

  assert.equal(harness.states.get("notification").textContent, "Preview 1");
  assert.equal(harness.cards.get("notification").classList.contains("is-preview-only"), true);
  assert.equal(harness.testButtons.get("notification").disabled, true);

  assert.equal(harness.states.get("audio").textContent, "Not connected");
  assert.equal(harness.lastConnected.get("audio").textContent, "Never connected");
  assert.equal(harness.testButtons.get("audio").disabled, true);
});

test("output readiness center exposes one local test control per source", () => {
  for (const target of targets) {
    assert.match(panelHtml, new RegExp(`data-output-card="${target}"`));
    assert.match(panelHtml, new RegExp(`data-test-output="${target}"`));
  }
  assert.match(panelSource, /invoke\("test_output", \{ target \}\)/);
});

test("voice controls are removed and one Relay channel feeds messages and media", () => {
  assert.doesNotMatch(panelHtml, /id="tts-speech-enabled"|data-output-card="tts"/);
  assert.doesNotMatch(panelHtml, /id="tts-channel"|id="tts-cleanup-enabled"|id="tts-welcome-message"/);
  assert.equal(panelHtml.match(/name="watchedChannelId"/g).length, 1);
  assert.match(panelHtml, /id="relay-channel-choice"[^>]*hidden/);
  assert.match(panelHtml, /data-channel-name="channel"[\s\S]*?data-go-to-target="channel"/);
  assert.match(panelSource, /invoke\("choose_relay_channel", \{ channelId \}\)/);
  assert.doesNotMatch(panelSource, /messageNotificationsEnabled|ttsChannelElement/);
});
