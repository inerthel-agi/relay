const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");

const {
  applyAdvancedEdits, capturePresetFields, describePresetChanges, describeTestResult, filterLogEntries,
  logActionLabel, mergePresetFields, parseDiscordIds, readOwnPresets, reasonLabel, wordsFromInput,
} = require("./moderation-ui.mjs");
const { translations } = require("./translations.mjs");

const t = (key) => key;
const format = (key, values) => `${key}:${JSON.stringify(values)}`;
const panelHtml = fs.readFileSync(__dirname + "/panel.html", "utf8");

test("member IDs are read from raw IDs and mentions without duplicates", () => {
  assert.deepEqual(
    parseDiscordIds("<@123456789012345678>, <@!223456789012345678> 123456789012345678\n<@&323456789012345678> 12"),
    ["123456789012345678", "223456789012345678", "323456789012345678"],
  );
  assert.deepEqual(wordsFromInput("free nitro, , porn\nfree nitro"), ["free nitro", "porn"]);
});

test("preset changes and tester results are explained in plain words", () => {
  assert.equal(describePresetChanges([], t, format), "presetNoChange");
  assert.equal(
    describePresetChanges(["moderationEnabled", "wordPacks"], t, format),
    'presetChanges:{"count":2,"list":"changeModerationEnabled, changeWordPacks"}',
  );
  assert.equal(
    describeTestResult({ action: "block", reasons: ["forbidden_concept", "custom_code"], nameHidden: true }, t, format),
    'testBlock testReasons:{"reasons":"reasonFilterWord, custom code"} testNameHidden',
  );
  assert.equal(reasonLabel("new_account", t), "reasonNewAccount");
  assert.equal(logActionLabel("timedOut", t), "logTimedOut");
});

test("the advanced editor replaces variants of existing words only", () => {
  const concepts = [
    { canonical: "Hitler", aliases: ["h1tler"], regexes: [] },
    { canonical: "fdp", aliases: [], regexes: ["f+d+p+"] },
  ];
  const edits = new Map([["hitler", { aliases: "h!tler, hitl3r", regexes: ["h.?i.?t.?l.?e.?r"] }]]);
  const edited = applyAdvancedEdits(concepts, edits);
  assert.deepEqual(edited[0], { canonical: "Hitler", aliases: ["h!tler", "hitl3r"], regexes: ["h.?i.?t.?l.?e.?r"] });
  assert.deepEqual(edited[1], concepts[1]);
});

test("personal presets capture only the settings a preset controls", () => {
  const config = {
    watchedChannelId: "1", moderationEnabled: true, privacyScanEnabled: true, privacyEnabledCategories: ["email"],
    moderation: { wordPacks: ["hate"], userCooldownSeconds: 10, trustedUserIds: ["123456789012345678"] },
  };
  const fields = capturePresetFields(config);
  assert.equal(fields.watchedChannelId, undefined);
  assert.equal(fields.moderation.trustedUserIds, undefined);
  const merged = mergePresetFields({ ...config, moderationEnabled: false, moderation: { ...config.moderation, wordPacks: [] } }, fields);
  assert.equal(merged.moderationEnabled, true);
  assert.deepEqual(merged.moderation.wordPacks, ["hate"]);
  assert.deepEqual(merged.moderation.trustedUserIds, ["123456789012345678"]);
  assert.equal(merged.watchedChannelId, "1");
  assert.deepEqual(readOwnPresets(() => "not json"), []);
  assert.deepEqual(readOwnPresets(() => JSON.stringify([{ name: "Soir", fields }, { nope: true }])).length, 1);
});

test("the decision log can be narrowed to one member", () => {
  const entries = [{ authorId: "111111111111111111" }, { authorId: "222222222222222222" }, {}];
  assert.equal(filterLogEntries(entries, "").length, 3);
  assert.deepEqual(filterLogEntries(entries, "2222"), [entries[1]]);
});

test("the moderation page exposes presets, tester, queue shortcuts, log and every new setting", () => {
  for (const preset of ["relaxed", "standard", "strict", "event"]) {
    assert.match(panelHtml, new RegExp(`data-moderation-preset="${preset}"`));
  }
  for (const id of [
    "moderation-test-text", "moderation-log-list", "moderation-pack-list", "moderation-advanced-list",
    "moderation-cooldown", "moderation-raid-limit", "moderation-safety-delay", "moderation-live-preset",
    "moderation-max-media", "moderation-loudness", "moderation-automod-sync", "moderation-trusted-roles",
    "moderation-blocked-users", "moderation-honeypot-exempt", "moderation-review-only-selected",
  ]) {
    assert.match(panelHtml, new RegExp(`id="${id}"`), id);
  }
  assert.match(panelHtml, /class="moderation-item__more"/);
  assert.match(panelHtml, /id="honeypot-action"[\s\S]*value="timeout" data-i18n="honeypotTimeout"/);
  // The queue sits above the settings, the log below them.
  assert.ok(panelHtml.indexOf('id="moderation-list"') < panelHtml.indexOf('id="moderation-form"'));
  assert.ok(panelHtml.indexOf('id="moderation-log-list"') > panelHtml.indexOf('id="moderation-form"'));
});

test("every moderation label is translated in all interface languages", () => {
  const keys = [
    "moderationPresets", "presetStandard", "presetChanges", "testBlock", "wordPacks", "packHate", "spamAndLinks",
    "sanctionsAndTrap", "liveProtection", "moderationLog", "reasonNewAccount", "logTimedOut", "countdownRelease",
    "actionTrust", "honeypotTimeout",
  ];
  for (const [language, dictionary] of Object.entries(translations)) {
    for (const key of keys) assert.ok(dictionary[key], `${language}.${key}`);
    assert.match(dictionary.presetChanges, /\{count\}[\s\S]*\{list\}|\{list\}[\s\S]*\{count\}/, language);
  }
});
