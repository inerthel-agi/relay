const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

const traySource = fs.readFileSync(__dirname + "/tray.js", "utf8");
const attentionSource = traySource.slice(traySource.indexOf("let attentionPage"), traySource.indexOf("async function refreshTray"));

function setup() {
  const elements = {
    "#tray-attention": { hidden: true },
    "#tray-attention-label": { textContent: "" },
    "#tray-attention-action": { textContent: "" },
  };
  const context = vm.createContext({
    document: { querySelector: (selector) => elements[selector] },
    translate: (key) => ({ pendingReview: "{count} waiting", reviewNow: "Review", discordNotReady: "Offline", openDiscordSettings: "Open Discord" })[key],
  });
  vm.runInContext(`${attentionSource}\nglobalThis.renderAttentionForTest = renderAttention;\nglobalThis.attentionPageForTest = () => attentionPage;`, context);
  return { elements, context };
}

test("pending moderation takes priority and opens the moderation page", () => {
  const { elements, context } = setup();
  context.renderAttentionForTest({ pendingMedia: [{}, {}], bot: { connected: false } });
  assert.equal(elements["#tray-attention"].hidden, false);
  assert.equal(elements["#tray-attention-label"].textContent, "2 waiting");
  assert.equal(context.attentionPageForTest(), "moderation");
});

test("a disconnected bot points to the Discord page", () => {
  const { elements, context } = setup();
  context.renderAttentionForTest({ pendingMedia: [], bot: { connected: false } });
  assert.equal(elements["#tray-attention-action"].textContent, "Open Discord");
  assert.equal(context.attentionPageForTest(), "discord");
});

test("nothing to do hides the attention row", () => {
  const { elements, context } = setup();
  context.renderAttentionForTest({ pendingMedia: [], bot: { connected: true } });
  assert.equal(elements["#tray-attention"].hidden, true);
  assert.equal(context.attentionPageForTest(), null);
});

test("the tray passes the page to the panel command", () => {
  assert.match(traySource, /invoke\("tray_open_control_panel", \{ page: attentionPage \}\)/);
});
