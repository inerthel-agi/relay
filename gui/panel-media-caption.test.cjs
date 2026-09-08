const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");

const panelSource = require("./test-source.cjs").panelSource();
const commandsSource = fs.readFileSync(__dirname + "/../src-tauri/src/commands.rs", "utf8");
const librarySource = fs.readFileSync(__dirname + "/../src-tauri/src/lib.rs", "utf8");

test("media caption switches persist independently from the full playback form", () => {
  assert.match(panelSource, /async function saveMediaCaptionVisibility\(\)/);
  assert.match(panelSource, /invoke\("set_media_caption_visibility", \{/);
  assert.match(panelSource, /showMediaTextObs: showMediaTextObsElement\.checked/);
  assert.match(panelSource, /showMediaTextWidget: showMediaTextWidgetElement\.checked/);
  assert.match(panelSource, /input\.addEventListener\("change", \(\) => void saveMediaCaptionVisibility\(\)\)/);
  assert.match(commandsSource, /pub async fn set_media_caption_visibility\(/);
  assert.match(commandsSource, /config\.show_media_text_obs = show_media_text_obs/);
  assert.match(commandsSource, /config\.show_media_text_widget = show_media_text_widget/);
  assert.match(librarySource, /set_media_caption_visibility,/);
});
