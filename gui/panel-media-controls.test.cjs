const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");

const panelHtml = fs.readFileSync(__dirname + "/panel.html", "utf8");
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
