const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");

const panelHtml = fs.readFileSync(__dirname + "/panel.html", "utf8");
const panelSource = require("./test-source.cjs").panelSource();
const panelCss = require("./test-source.cjs").panelStyles();
const traySource = fs.readFileSync(__dirname + "/tray.js", "utf8");
const trayCss = fs.readFileSync(__dirname + "/tray.css", "utf8");
const designs = ["openai", "anthropic", "neo-brutalism", "gridline", "lumen"];

test("personalization exposes five accessible design choices in a retractable picker", () => {
  assert.match(panelHtml, /<details id="design-picker" class="design-picker">/);
  assert.match(panelHtml, /<summary>/);
  assert.match(panelHtml, /id="design-picker-selected"/);
  assert.match(panelHtml, /<html[^>]+data-design="openai"/);
  for (const design of designs) {
    assert.match(panelHtml, new RegExp(`name="interface-design" value="${design}"`));
  }
});

test("the selected design is persisted and applied without changing the native preference schema", () => {
  assert.match(panelSource, /const supportedDesigns = \["openai", "anthropic", "neo-brutalism", "gridline", "lumen"\]/);
  assert.match(panelSource, /localStorage\.getItem\("relay-design"\)/);
  assert.match(panelSource, /localStorage\.setItem\("relay-design", design\)/);
  assert.match(panelSource, /document\.documentElement\.dataset\.design = design/);
  assert.match(panelSource, /designPickerElement\.open = false/);
  assert.match(panelSource, /designPickerSelectedElement\.textContent/);
  assert.match(panelSource, /language, theme, accentRgb, fontScale,/);
});

test("each art direction covers light, dark, focus, responsive and reduced-motion states", () => {
  for (const design of designs.slice(1)) {
    assert.match(panelCss, new RegExp(`data-design="${design}"`));
    assert.match(trayCss, new RegExp(`data-design="${design}"`));
  }
  assert.match(panelCss, /data-design="anthropic"\]\[data-theme="dark"\]/);
  assert.match(panelCss, /data-design="neo-brutalism"\]\[data-theme="dark"\]/);
  assert.match(panelCss, /data-design="gridline"\]\[data-theme="dark"\]/);
  assert.match(panelCss, /data-design="lumen"\]\[data-theme="dark"\]/);
  assert.match(panelCss, /design-choice:has\(input:focus-visible\)/);
  assert.match(panelCss, /@media \(max-width: 700px\)/);
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

test("text scaling measures the unscaled tree before applying the factor", () => {
  const scalingSource = panelSource.slice(
    panelSource.indexOf("function scaleInterfaceText"),
    panelSource.indexOf("function syncInterfacePreferences"),
  );
  assert.ok(
    scalingSource.indexOf('style.removeProperty("font-size")')
      < scalingSource.indexOf("window.getComputedStyle(element).fontSize"),
  );
  assert.ok(
    scalingSource.indexOf("window.getComputedStyle(element).fontSize")
      < scalingSource.lastIndexOf("element.style.fontSize ="),
  );
});
