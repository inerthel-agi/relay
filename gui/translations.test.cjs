const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

const panelSource = require("./test-source.cjs").panelSource();
const panelHtml = fs.readFileSync(__dirname + "/panel.html", "utf8");
const dictionarySource = panelSource.slice(
  panelSource.indexOf("const translations ="),
  panelSource.indexOf("const pageMetadata ="),
);
const context = vm.createContext({});
vm.runInContext(`${dictionarySource}\nglobalThis.translationsForTest = translations;\nglobalThis.regionalTranslationsForTest = regionalTranslations;`, context);
const translations = context.translationsForTest;
const regionalTranslations = context.regionalTranslationsForTest;

test("every supported language provides every interface translation key", () => {
  const expectedKeys = Object.keys(translations.en).sort();
  for (const language of ["fr", "es", "de", "ru", "zh", "ko", "ja", "id"]) {
    assert.deepEqual(Object.keys(translations[language]).sort(), expectedKeys, language);
  }
});

test("English regions use their own Relay vocabulary", () => {
  assert.deepEqual({ ...regionalTranslations["en-US"] }, {
    personalizationTitle: "Customize Relay your way.",
    accentColor: "Accent color",
    previewCopy: "Readable text with your chosen accent color.",
    privacyCategoryPhone: "Phone numbers",
    privacyCategoryAddress: "ZIP and postal addresses",
    customActionClearMessages: "Clear messages",
    customAllowedChannels: "Allowed command channel IDs",
  });
  assert.deepEqual({ ...regionalTranslations["en-GB"] }, {
    personalizationTitle: "Personalise Relay your way.",
    accentColor: "Accent colour",
    previewCopy: "Readable text with your chosen accent colour.",
    privacyCategoryPhone: "Telephone numbers",
    privacyCategoryAddress: "Postal addresses",
    customActionClearMessages: "Delete messages",
    customAllowedChannels: "Allowed command channel IDs",
  });
  assert.deepEqual({ ...regionalTranslations["en-IN"] }, {
    personalizationTitle: "Personalise Relay for your workspace.",
    accentColor: "Accent colour",
    previewCopy: "Readable text with your chosen accent colour.",
    privacyCategoryPhone: "Mobile and telephone numbers",
    privacyCategoryAddress: "Postal addresses and PIN codes",
    customActionClearMessages: "Delete messages",
    customAllowedChannels: "Permitted command channel IDs",
  });
  assert.match(panelSource, /regionalTranslations\[locale\]\?\.\[key\]/);
});

test("the Discord invitation action opens the authorization URL", () => {
  assert.match(panelHtml, /id="open-invite"[\s\S]*data-i18n="openInvite"/);
  assert.doesNotMatch(panelHtml, /id="copy-invite"/);
  assert.match(
    panelSource,
    /openInviteButton\.addEventListener\("click", \(\) => invoke\("open_help_link", \{ link: inviteUrlElement\.value \}\)\)/,
  );
});

test("reported Spanish and German pages no longer use English fallback copy", () => {
  for (const language of ["es", "de"]) {
    for (const key of [
      "overviewTitle", "credentialsTitle", "routingTitle", "moderationTitle",
      "moderationSettings", "historyKicker", "historyTitle", "historyCopy",
    ]) {
      assert.notEqual(translations[language][key], translations.en[key], `${language}.${key}`);
    }
  }
});

test("Russian localizes the primary Relay controls", () => {
  for (const key of [
    "navOverview", "overviewTitle", "credentialsTitle", "mediaTitle", "moderationTitle",
    "commandsTitle", "privacyProtection", "searchPlaceholder", "fontFamily",
  ]) {
    assert.notEqual(translations.ru[key], translations.en[key], `ru.${key}`);
  }
  assert.equal(translations.ru.navOverview, "Обзор");
  assert.equal(translations.ru.privacyProfileBalanced, "Сбалансированный");
});

test("Chinese, Korean, Japanese and Indonesian localize the primary Relay controls", () => {
  const expected = {
    zh: { navOverview: "概览", privacyProfileBalanced: "平衡" },
    ko: { navOverview: "개요", privacyProfileBalanced: "균형" },
    ja: { navOverview: "概要", privacyProfileBalanced: "バランス" },
    id: { navOverview: "Ringkasan", privacyProfileBalanced: "Seimbang" },
  };
  const keys = [
    "navOverview", "overviewTitle", "mediaTitle", "moderationTitle",
    "commandsTitle", "privacyProtection", "searchPlaceholder", "fontFamily",
  ];

  for (const [language, values] of Object.entries(expected)) {
    for (const key of keys) {
      assert.notEqual(translations[language][key], translations.en[key], `${language}.${key}`);
    }
    assert.equal(translations[language].navOverview, values.navOverview, `${language}.navOverview`);
    assert.equal(translations[language].privacyProfileBalanced, values.privacyProfileBalanced, `${language}.privacyProfileBalanced`);
  }
});

test("translation values contain no UTF-8 mojibake", () => {
  for (const language of Object.keys(translations)) {
    assert.doesNotMatch(Object.values(translations[language]).join("\n"), /Ã.|â€|Â./, language);
  }
});

test("moderation settings use three native disclosures without changing control IDs", () => {
  assert.equal([...panelHtml.matchAll(/<details id="moderation-/g)].length, 3);
  assert.match(panelHtml, /<details id="moderation-automod"[^>]* open>/);
  assert.match(panelHtml, /<details id="moderation-manual"[^>]*>/);
  assert.match(panelHtml, /<details id="moderation-privacy"[^>]*>/);

  for (const id of [
    "privacy-concepts", "privacy-exempt-role-ids", "moderation-enabled",
    "moderation-allow-images", "moderation-allow-videos", "moderation-allow-audio",
    "honeypot-channel", "honeypot-action",
    "privacy-scan-enabled", "privacy-protection-level", "privacy-block-threshold",
    "privacy-review-intermediate", "privacy-auto-delete-blocked-messages",
    "privacy-custom-patterns", "privacy-allowlist",
  ]) {
    assert.match(panelHtml, new RegExp(`id="${id}"`), id);
  }

  assert.match(panelHtml, /value="balanced" data-i18n="privacyProfileBalanced"/);
  assert.match(panelHtml, /id="honeypot-action"[\s\S]*value="kick" data-i18n="honeypotKick"[\s\S]*value="ban" data-i18n="honeypotBan"/);
  assert.match(panelHtml, /id="privacy-custom-patterns"[^>]*data-i18n-placeholder="privacyCustomPatternsPlaceholder"/);
  assert.match(panelHtml, /id="privacy-allowlist"[^>]*data-i18n-placeholder="privacyAllowlistPlaceholder"/);
  assert.match(panelSource, /honeypotChannelId: honeypotChannelElement\.value/);
  assert.match(panelSource, /honeypotAction: honeypotActionElement\.value/);
});

test("moderation translations are explicit and localized in every language", () => {
  const expected = {
    fr: ["Filtrage automatique", "Modération manuelle", "Activer l’analyse locale de confidentialité", "Équilibré", "Une valeur par ligne", "Piège à compte compromis"],
    es: ["Filtrado automático", "Moderación manual", "Activar el análisis local de privacidad", "Equilibrado", "Un valor por línea", "Trampa para cuentas comprometidas"],
    de: ["Automatische Filterung", "Manuelle Moderation", "Lokalen Datenschutzscan aktivieren", "Ausgewogen", "Ein Wert pro Zeile", "Falle für kompromittierte Konten"],
  };
  const keys = [
    "automaticFilterWords", "manualModeration", "privacyScanEnabled",
    "privacyProfileBalanced", "privacyCustomPatternsPlaceholder", "honeypotChannel",
  ];

  for (const [language, values] of Object.entries(expected)) {
    assert.deepEqual(keys.map((key) => translations[language][key]), values, language);
  }

  assert.equal(translations.fr.allowAudio, "Audio");
  assert.match(translations.fr.privacyProtectionHelp, /détectées/);
  assert.match(translations.es.privacyCategories, /Categorías/);
  assert.match(translations.de.privacyCategoryMetadata, /Metadaten/);
});

test("the top bar audio player exposes only previous, pause and skip controls", () => {
  assert.match(panelHtml, /id="now-playing"[\s\S]*id="previous-audio"[\s\S]*id="toggle-audio"[\s\S]*id="skip-audio"/);
  assert.doesNotMatch(panelHtml, /shuffle-audio|repeat-audio|next-audio/);
  assert.match(panelSource, /message\.type === "audioPlayback"/);
  assert.match(panelSource, /invoke\("control_audio"/);
});
