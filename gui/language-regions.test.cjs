const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");

const panelHtml = fs.readFileSync(__dirname + "/panel.html", "utf8");
const panelSource = require("./test-source.cjs").panelSource();
const locales = [
  "en-US", "en-GB", "en-IN", "fr-FR", "de-DE", "es-ES", "es-419",
  "ru-RU", "zh-CN", "ko-KR", "ja-JP", "id-ID",
];
const flags = ["us", "gb", "in", "fr", "de", "es", "mx", "ru", "cn", "kr", "jp", "id"];

test("the language picker exposes every supported regional locale", () => {
  for (const locale of locales) {
    assert.match(panelSource, new RegExp(`locale: "${locale}"`), locale);
  }
  assert.match(panelSource, /interfaceLanguageOptionsElement\.replaceChildren\(\.\.\.languageOptionButtons\(/);
  assert.match(panelSource, /sidebarLanguageOptionsElement\.replaceChildren\(\s*\.\.\.languageOptionButtons\(/);
  assert.match(panelSource, /language = option\.language/);
  assert.match(panelSource, /localStorage\.setItem\("relay-locale", locale\)/);
});

test("the sidebar language control uses an inline regional picker", () => {
  assert.match(panelHtml, /id="sidebar-language-picker"/);
  assert.match(panelHtml, /id="sidebar-language-options"[^>]*role="listbox"/);
  assert.match(panelSource, /function setSidebarLanguageMenuOpen\(open\)/);
  assert.match(panelSource, /sidebarLanguageOptionsElement\.replaceChildren/);
  assert.match(panelSource, /languageToggleButton\.addEventListener\("click", \(\) => \{\s*setSidebarLanguageMenuOpen/s);
  assert.doesNotMatch(panelSource, /languageToggleButton\.addEventListener\([\s\S]*?showPage\("personalization"\)/);
});

test("every regional locale uses a bundled SVG flag", () => {
  for (const flag of flags) {
    const path = `${__dirname}/assets/flags/${flag}.svg`;
    assert.ok(fs.existsSync(path), path);
    assert.match(fs.readFileSync(path, "utf8"), /<svg[^>]+viewBox="0 0 24 16"/);
    assert.match(panelSource, new RegExp(`flag: "${flag}"`), flag);
  }
});

test("regional variants keep a local dictionary and regional locale", () => {
  assert.match(panelSource, /locale: "en-GB", language: "en"/);
  assert.match(panelSource, /locale: "en-IN", language: "en"/);
  assert.match(panelSource, /locale: "es-419", language: "es"/);
  for (const [locale, language] of [
    ["ru-RU", "ru"], ["zh-CN", "zh"], ["ko-KR", "ko"], ["ja-JP", "ja"], ["id-ID", "id"],
  ]) {
    assert.match(panelSource, new RegExp(`locale: "${locale}", language: "${language}"`), locale);
    assert.match(panelSource, new RegExp(`translations\\.${language} =`), language);
  }
});
