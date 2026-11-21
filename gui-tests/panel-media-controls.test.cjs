const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");

const panelHtml = fs.readFileSync(__dirname + "/../gui/panel.html", "utf8");
const panelSource = require("./test-source.cjs").panelSource();
const commandsSource = fs.readFileSync(__dirname + "/../src-tauri/src/commands.rs", "utf8");

test("media settings expose a captured global shortcut", () => {
  assert.match(panelHtml, /id="skip-shortcut-capture"/);
  assert.match(panelHtml, /id="skip-shortcut-key"/);
  assert.match(panelSource, /invoke\("set_skip_shortcut"/);
  assert.match(panelSource, /shortcutTokenFromEvent/);
  assert.match(commandsSource, /pub async fn set_skip_shortcut\(/);
});

test("history media exposes a download action backed by the native save dialog", () => {
  assert.match(panelHtml, /history-item__download/);
  assert.match(panelSource, /invoke\("download_history_media"/);
  assert.match(commandsSource, /AsyncFileDialog::new\(\)/);
  assert.match(commandsSource, /pub async fn download_history_media\(/);
});

test("leaving shortcut capture restores the button and saves nothing", () => {
  const vm = require("node:vm");
  const panel = fs.readFileSync(__dirname + "/../gui/panel.js", "utf8");
  function element() {
    return {
      listeners: {}, focus() {}, contains(node) { return node === this; },
      addEventListener(name, fn) { this.listeners[name] = fn; },
      setAttribute(name, value) { this[name] = value; },
    };
  }
  const win = element();
  const panic = element();
  const value = {};
  const saves = [];
  const context = vm.createContext({
    window: win, skipShortcutCaptureButton: element(), panicShortcutCaptureButton: panic,
    skipShortcutKeyElement: {}, skipShortcutValueElement: {}, panicShortcutValueElement: value,
    panicUi: null, bootstrap: { config: { panicShortcut: "control+KeyP" } },
    mediaSaveStateElement: {}, t: (key) => key, setSaveState() {},
    invoke: (...args) => { saves.push(args); return Promise.resolve({}); },
  });
  vm.runInContext("let shortcutCaptureActive = false;\n" +
    panel.slice(panel.indexOf("function formatShortcutLabel"), panel.indexOf("function updateBotActivityAvailability")) +
    panel.slice(panel.indexOf('skipShortcutCaptureButton.addEventListener("click"'), panel.indexOf('previousAudioButton.addEventListener("click"')), context);
  function key(code, ctrlKey = false) {
    const event = { code, key: code, ctrlKey, preventDefault() { this.prevented = true; }, stopPropagation() {} };
    win.listeners.keydown(event);
    return event.prevented;
  }
  panic.listeners.click();
  panic.listeners.blur();
  assert.equal(panic["aria-pressed"], "false");
  assert.match(value.textContent, /P$/);
  assert.equal(key("KeyP", true), undefined);
  panic.listeners.click();
  win.listeners.pointerdown({ target: {} });
  assert.equal(panic["aria-pressed"], "false");
  panic.listeners.click();
  key("KeyP");
  assert.equal(key("Tab"), undefined);
  assert.equal(panic["aria-pressed"], "false");
  assert.deepEqual(saves, []);
  panic.listeners.click();
  key("F24");
  assert.equal(saves[0][1].shortcut, "F24");
});
