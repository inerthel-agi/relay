const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

const traySource = fs.readFileSync(__dirname + "/tray.js", "utf8");
const dictionarySource = traySource.slice(
  traySource.indexOf("const trayTranslations ="),
  traySource.indexOf("let language ="),
);
const context = vm.createContext({});
vm.runInContext(`${dictionarySource}\nglobalThis.trayTranslationsForTest = trayTranslations;\nglobalThis.trayRegionalTranslationsForTest = trayRegionalTranslations;`, context);
const translations = context.trayTranslationsForTest;
const regionalTranslations = context.trayRegionalTranslationsForTest;

test("tray translations cover every interface language", () => {
  const expectedKeys = Object.keys(translations.en).sort();
  for (const language of ["fr", "es", "de", "ru", "zh", "ko", "ja", "id"]) {
    assert.deepEqual(Object.keys(translations[language]).sort(), expectedKeys, language);
  }
});

test("tray refresh reads the language shared by the control panel", () => {
  assert.match(traySource, /localStorage\.getItem\("relay-language"\)/);
  assert.match(traySource, /async function refreshTray\(\) \{\s+applyTrayLanguage\(\)/);
  assert.match(traySource, /invoke\("get_start_with_windows"\)[\s\S]*"set_start_with_windows", \{ enabled \}/);
});

test("English regions use their own tray vocabulary", () => {
  assert.deepEqual({ ...regionalTranslations["en-US"] }, {
    displayWidgets: "Display widgets",
    visibleMovable: "Visible · Movable",
    startWithWindows: "Launch with Windows",
  });
  assert.deepEqual({ ...regionalTranslations["en-GB"] }, {
    displayWidgets: "Show widgets",
    visibleMovable: "Visible · Can be moved",
    startWithWindows: "Start with Windows",
  });
  assert.deepEqual({ ...regionalTranslations["en-IN"] }, {
    displayWidgets: "Show widgets",
    visibleMovable: "Visible · Can be moved",
    localRelay: "Local Relay service",
  });
  assert.match(traySource, /trayRegionalTranslations\[locale\]\?\.\[key\]/);
  assert.match(traySource, /locale = .*storedLocale/);
});

test("the Russian tray stays localized", () => {
  assert.equal(translations.ru.openPanel, "Открыть панель управления");
  assert.notEqual(translations.ru.quitRelay, translations.en.quitRelay);
});

test("the added Asian and Indonesian tray locales stay localized", () => {
  const expectedOpenPanel = {
    zh: "打开控制面板",
    ko: "제어 패널 열기",
    ja: "コントロール パネルを開く",
    id: "Buka panel kontrol",
  };

  for (const [language, label] of Object.entries(expectedOpenPanel)) {
    assert.equal(translations[language].openPanel, label, `${language}.openPanel`);
    assert.notEqual(translations[language].quitRelay, translations.en.quitRelay, `${language}.quitRelay`);
  }
});
