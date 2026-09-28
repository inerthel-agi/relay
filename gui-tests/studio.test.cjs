const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");
test("presets validate stored data and preserve safe geometry", async () => {
  const { normalizePreset, readPresets } = await import("../gui/output-presets.mjs");
  assert.equal(normalizePreset({ name: "" }), null);
  const preset = normalizePreset({ name: "OBS", geometry: { contentScale: 500, marginX: -5, anchor: "invalid" } });
  assert.equal(preset.geometry.contentScale, 400);
  assert.equal(preset.geometry.marginX, 0);
  assert.equal(preset.geometry.anchor, "legacy");
  assert.deepEqual(readPresets({ getItem: () => "invalid json" }, "presets"), []);
  assert.equal(readPresets({ getItem: () => JSON.stringify(Array.from({length: 20}, () => preset)) }, "presets").length, 12);
});
test("shared output placement supports every edge without clipping", () => {
  const context = vm.createContext({});
  vm.runInContext(fs.readFileSync(__dirname + "/../outputs/layout.js", "utf8") + ";globalThis.layout=RelayLayout", context);
  for (const anchor of context.layout.anchors) {
    const position = context.layout.position(300, 160, 640, 360, { anchor, marginX: 20, marginY: 16 });
    if (anchor === "legacy") { assert.equal(position, null); continue; }
    assert.ok(position.left >= 0 && position.left + 300 <= 640);
    assert.ok(position.top >= 0 && position.top + 160 <= 360);
  }
  assert.equal(context.layout.position(300,160,640,360,{anchor:"topRight",marginX:20,marginY:16}).left,320);
});
