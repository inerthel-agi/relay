const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");

const panelHtml = fs.readFileSync(__dirname + "/../gui/panel.html", "utf8");
const panelSource = require("./test-source.cjs").panelSource();
const panelCss = require("./test-source.cjs").panelStyles();
const traySource = fs.readFileSync(__dirname + "/../gui/tray.js", "utf8");
const trayCss = fs.readFileSync(__dirname + "/../gui/tray.css", "utf8");
const designs = ["graphite", "paper", "neo-brutalism", "gridline", "lumen", "signal", "midnight", "ember", "dawn", "cloud", "titanium", "aether", "orbital", "oled"];

test("personalization exposes five accessible design choices in a retractable picker", () => {
  assert.match(panelHtml, /<details id="design-picker" class="design-picker">/);
  assert.match(panelHtml, /<summary>/);
  assert.match(panelHtml, /id="design-picker-selected"/);
  assert.match(panelHtml, /<html[^>]+data-design="graphite"/);
  for (const design of designs) {
    assert.match(panelHtml, new RegExp(`name="interface-design" value="${design}"`));
  }
});

test("the selected design is persisted, applied and shared with the outputs", () => {
  assert.match(panelSource, /const supportedDesigns = \["graphite", "paper", "neo-brutalism", "gridline", "lumen", "signal", "midnight", "ember", "dawn", "cloud", "titanium", "aether", "orbital", "oled"\]/);
  assert.match(panelSource, /localStorage\.getItem\("relay-design"\)/);
  assert.match(panelSource, /localStorage\.setItem\("relay-design", design\)/);
  assert.match(panelSource, /document\.documentElement\.dataset\.design = design/);
  assert.match(panelSource, /designPickerElement\.open = false/);
  assert.match(panelSource, /designPickerSelectedElement\.textContent/);
  // Outputs follow the design, so it now travels with the interface preferences.
  assert.match(panelSource, /preferences: \{ language, theme, accentRgb, fontScale, design, \.\.\.streamPreferences \}/);
});

test("brand-named designs stored by earlier releases migrate to neutral names", () => {
  const legacy = /const legacyDesignNames = (\{[^}]+\})/;
  for (const source of [panelSource, traySource]) {
    const names = Function(`return ${source.match(legacy)[1]}`)();
    assert.deepEqual(names, { openai: "graphite", anthropic: "paper" });
  }
  assert.match(panelSource, /design = legacyDesignNames\[design\] \|\| design;/);
  assert.match(traySource, /legacyDesignNames\[rawDesign\] \|\| rawDesign/);
});

test("each art direction covers light, dark, focus, responsive and reduced-motion states", () => {
  for (const design of designs.slice(1)) {
    assert.match(panelCss, new RegExp(`data-design="${design}"`));
    assert.match(trayCss, new RegExp(`data-design="${design}"`));
  }
  assert.match(panelCss, /data-design="paper"\]\[data-theme="dark"\]/);
  assert.match(panelCss, /data-design="neo-brutalism"\]\[data-theme="dark"\]/);
  assert.match(panelCss, /data-design="gridline"\]\[data-theme="dark"\]/);
  assert.match(panelCss, /data-design="lumen"\]\[data-theme="dark"\]/);
  assert.match(panelCss, /design-choice:has\(input:focus-visible\)/);
  assert.match(panelCss, /@media \(prefers-reduced-motion: reduce\)/);
});

test("tray refresh reads the design and theme shared by personalization", () => {
  assert.match(traySource, /localStorage\.getItem\("relay-design"\)/);
  assert.match(traySource, /localStorage\.getItem\("relay-theme"\)/);
  assert.match(traySource, /document\.documentElement\.dataset\.design/);
  assert.match(traySource, /document\.documentElement\.dataset\.theme/);
  assert.match(traySource, /applyTrayAccent/);
  assert.match(traySource, /"gridline", "lumen"/);
  assert.match(trayCss, /--tray-accent/);
});

test("text scaling uses one root factor that later-rendered content inherits", () => {
  const scalingSource = panelSource.slice(
    panelSource.indexOf("function scaleInterfaceText"),
    panelSource.indexOf("function syncInterfacePreferences"),
  );
  assert.match(scalingSource, /setProperty\("--text-scale", String\(fontScale \/ 100\)\)/);
  assert.doesNotMatch(scalingSource, /element\.style\.fontSize/);
  assert.match(panelCss, /:root \{\s*--text-scale: 1;/);
  assert.doesNotMatch(panelCss, /font-size:\s*\d+(?:\.\d+)?px\s*;/);
});

const newDesigns = ["titanium", "aether", "orbital", "oled"];

test("Titanium, Aether, Orbital and Oled are wired into every surface", () => {
  const trayList = traySource.match(/dataset\.design = (\[[^\]]+\])/)[1];
  for (const design of newDesigns) {
    assert.ok(trayList.includes(`"${design}"`), `tray.js ${design}`);
    assert.match(panelHtml, new RegExp(`name="stream-style" value="${design}"`), `stream chip ${design}`);
    assert.match(panelHtml, new RegExp(`data-i18n="${design}DesignCopy"`), `copy key ${design}`);
    assert.match(panelCss, new RegExp(`\\.design-choice--${design} \\.design-choice__preview \\{`), `thumbnail ${design}`);
    assert.match(panelCss, new RegExp(`data-design="${design}"\\]\\[data-theme="dark"\\] \\{`), `panel dark ${design}`);
    assert.match(trayCss, new RegExp(`data-design="${design}"\\]\\[data-theme="light"\\] \\{`), `tray light ${design}`);
  }
  const streamStyleSource = fs.readFileSync(__dirname + "/../gui/stream-style.mjs", "utf8");
  for (const design of newDesigns) {
    assert.match(streamStyleSource, new RegExp(`"${design}"`));
    assert.match(streamStyleSource, new RegExp(`${design}: "`));
  }
});

test("the requested palettes and signature details are in place", () => {
  assert.match(panelCss, /data-design="titanium"\]\[data-theme="dark"\] \{[^}]*--canvas: #07080a;[^}]*--surface: #101319;/);
  assert.match(panelCss, /data-design="aether"\]\[data-theme="dark"\] \{[^}]*--canvas: #060913;[^}]*--surface: #11182d;/);
  assert.match(panelCss, /data-design="orbital"\]\[data-theme="dark"\] \{[^}]*--canvas: #05080d;[^}]*--surface: #0b121d;/);
  assert.match(panelCss, /data-design="oled"\]\[data-theme="dark"\] \{[^}]*--canvas: #000000;[^}]*--surface: #090a0c;/);
  assert.match(panelCss, /data-design="aether"\] \.sidebar \{[^}]*margin: 8px 0 8px 8px;[^}]*border-radius: 16px;[^}]*backdrop-filter: blur\(20px\)/);
  assert.match(panelCss, /data-design="orbital"\] \.navigation__item\.is-active \{ box-shadow: inset 2px 0 0 var\(--accent\); \}/);
  assert.match(panelCss, /data-design="oled"\] \.panel-section[^{]*\{[^}]*inset 0 1px 0 color-mix\(in srgb, var\(--accent\) 32%, transparent\)/);
});

test("the new design descriptions exist in all 13 languages", () => {
  const vm = require("node:vm");
  const dictionarySource = panelSource.slice(panelSource.indexOf("const translations ="), panelSource.indexOf("const pageMetadata ="));
  const context = vm.createContext({});
  vm.runInContext(`${dictionarySource}
globalThis.translationsForTest = translations;`, context);
  const languages = ["en", "fr", "es", "de", "it", "pt", "pl", "tr", "ru", "zh", "ko", "ja", "id"];
  for (const language of languages) {
    for (const design of newDesigns) {
      assert.ok(context.translationsForTest[language][`${design}DesignCopy`], `${language}.${design}`);
    }
  }
});
