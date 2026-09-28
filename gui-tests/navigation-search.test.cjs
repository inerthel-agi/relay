const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");

const panelHtml = fs.readFileSync(__dirname + "/../gui/panel.html", "utf8");
const panelSource = require("./test-source.cjs").panelSource();

test("the top bar exposes local settings search and page history controls", () => {
  for (const id of [
    "navigation-back", "navigation-forward", "settings-search",
    "settings-search-clear", "settings-search-results",
  ]) {
    assert.match(panelHtml, new RegExp(`id="${id}"`), id);
  }

  assert.match(panelSource, /const navigationHistory = \["overview"\]/);
  assert.match(panelSource, /function navigateHistory\(offset\)/);
  assert.match(panelSource, /function buildSettingsSearchIndex\(\)/);
  assert.match(panelSource, /\[data-page\] \[data-i18n\]/);
});

test("settings search indexes labels only and never form values", () => {
  const searchSource = panelSource.slice(
    panelSource.indexOf("function buildSettingsSearchIndex"),
    panelSource.indexOf("function closeSettingsSearch"),
  );
  assert.match(searchSource, /element\.dataset\.i18n/);
  assert.doesNotMatch(searchSource, /\.value|textContent/);
});
