const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");

const panelHtml = fs.readFileSync(__dirname + "/../gui/panel.html", "utf8");
const panelSource = require("./test-source.cjs").panelSource();
// One entry per language since the English and Spanish regional variants were merged.
const locales = [
  "en-US", "fr-FR", "de-DE", "es-ES", "it-IT", "pt-BR", "pl-PL", "tr-TR",
  "ru-RU", "zh-CN", "ko-KR", "ja-JP", "id-ID",
];
const flags = ["us", "fr", "de", "es", "it", "br", "pl", "tr", "ru", "cn", "kr", "jp", "id"];

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
    const path = `${__dirname}/../gui/assets/flags/${flag}.svg`;
    assert.ok(fs.existsSync(path), path);
    assert.match(fs.readFileSync(path, "utf8"), /<svg[^>]+viewBox="0 0 24 16"/);
    assert.match(panelSource, new RegExp(`flag: "${flag}"`), flag);
  }
});

test("merged regional variants keep the user's language and every language has a dictionary", () => {
  for (const merged of ["en-GB", "en-IN", "es-419"]) {
    assert.doesNotMatch(panelSource, new RegExp(`locale: "${merged}"`), merged);
  }
  // A locale saved by an older version falls back to its language's single entry.
  assert.ok(panelSource.includes('if (!languageOptionByLocale.has(locale)) locale = defaultLocaleByLanguage[locale.split("-")[0]] || "en-US";'));
  for (const [locale, language] of [
    ["ru-RU", "ru"], ["zh-CN", "zh"], ["ko-KR", "ko"], ["ja-JP", "ja"], ["id-ID", "id"],
    ["it-IT", "it"], ["pt-BR", "pt"], ["pl-PL", "pl"], ["tr-TR", "tr"],
  ]) {
    assert.match(panelSource, new RegExp(`locale: "${locale}", language: "${language}"`), locale);
    assert.match(panelSource, new RegExp(`translations\\.${language} =`), language);
  }
});
