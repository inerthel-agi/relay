const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

const {
  applyAdvancedEdits, capturePresetFields, changedPresetFields, describePresetChanges, describeTestResult, explainReason, filterLogEntries,
  AI_MODELS, describeAiAdvice, describeAiTest, describeSettingsAnswer, describeTroubleshootAnswer, logActionLabel, mergePresetFields, parseDiscordIds, queueReasonLabel, readOwnPresets, reasonLabel, wordsFromInput,
} = require("../gui/moderation-ui.mjs");
const { translations } = require("../gui/translations.mjs");

const t = (key) => key;
const format = (key, values) => `${key}:${JSON.stringify(values)}`;
const panelHtml = fs.readFileSync(__dirname + "/../gui/panel.html", "utf8");

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

// Reason codes Relay writes for held, ignored or blocked items (moderation.rs, privacy.rs, state.rs, bot.rs).
const EXPLAINED_REASONS = [
  "cooldown", "duplicate", "mention_spam", "caps_spam", "emoji_spam", "repeated_characters", "blocked_user",
  "new_account", "new_member", "raid", "safety_delay", "manual_review", "expired", "queue_full", "honeypot",
  "escalation", "forbidden_concept", "forbidden_regex", "forbidden_similarity", "similarity_score",
  "discord_invite", "link_shortener", "scam_domain", "email", "phone", "person_phone_combination", "ip_address",
  "gps", "coordinates", "location_profile", "postal_address", "partial_address", "address_or_visual_context",
  "person_address_combination", "iban", "payment_card", "license_plate", "sensitive_url", "custom_pattern",
  "exif_metadata", "ocr_text", "document", "scan_incomplete", "image_scan_unavailable", "image_fetch_unavailable",
  "scan_config_changed", "image_limits", "mime_mismatch",
];

test("a held or ignored item says why, with the current setting that changes it", () => {
  const settings = { userCooldownSeconds: 30, safetyDelaySeconds: 8, minAccountAgeDays: 7, raidLimitPerMinute: 5 };
  assert.equal(
    explainReason("cooldown", settings, t, format),
    'whyCooldown:{"cooldown":30,"window":60,"delay":8,"accountDays":7,"memberDays":0,"raid":5,"expiry":0} whySetting:{"setting":"userCooldown"}',
  );
  assert.match(explainReason("scam_domain", settings, t, format), /^whyLink:.* whySetting:\{"setting":"blockScams"\}$/);
  assert.match(explainReason("partial_address", settings, t, format), /^whyPersonalData:.* whySetting:\{"setting":"privacyAllowlist"\}$/);
  assert.match(explainReason("scan_incomplete", undefined, t, format), /^whyScanIncomplete:\{[^}]*\}$/);
  assert.equal(explainReason("reject_all", settings, t, format), "");
  assert.equal(explainReason("custom_code", settings, t, format), "");
});

test("every reason Relay writes has a translated label and explanation in all languages", () => {
  const settings = { userCooldownSeconds: 30, safetyDelaySeconds: 8, minAccountAgeDays: 7, minMemberAgeDays: 3, raidLimitPerMinute: 5, pendingExpiryMinutes: 15 };
  for (const [language, dictionary] of Object.entries(translations)) {
    const translate = (key) => dictionary[key];
    const formatIn = (key, values) => Object.entries(values).reduce((text, [name, value]) => text.replaceAll(`{${name}}`, value), translate(key));
    for (const reason of EXPLAINED_REASONS) {
      const labelKey = reasonLabel(reason, (key) => `#${key}`);
      assert.ok(labelKey.startsWith("#") && dictionary[labelKey.slice(1)], `${language}.${reason} label`);
      const why = explainReason(reason, settings, translate, formatIn);
      assert.ok(why && !/undefined|\{\w+\}/.test(why), `${language}.${reason}: ${why}`);
    }
  }
  assert.match(
    explainReason("cooldown", settings, (key) => translations.fr[key], (key, values) => Object.entries(values).reduce((text, [name, value]) => text.replaceAll(`{${name}}`, value), translations.fr[key])),
    /30 s\)\. Réglage : Délai entre deux messages\.$/,
  );
});

test("every moderation label is translated in all interface languages", () => {
  const keys = [
    "moderationProtectionLevel", "presetStandard", "presetChanges", "testBlock", "wordPacks", "packHate", "spamAndLinks",
    "modPartSanctions", "modPartLive", "moderationLog", "reasonNewAccount", "logTimedOut", "countdownRelease",
    "actionTrust", "honeypotTimeout",
  ];
  for (const [language, dictionary] of Object.entries(translations)) {
    for (const key of keys) assert.ok(dictionary[key], `${language}.${key}`);
    assert.match(dictionary.presetChanges, /\{count\}[\s\S]*\{list\}|\{list\}[\s\S]*\{count\}/, language);
  }
});

test("AI settings name the same providers as Relay and keep keys out of the page", () => {
  const rust = fs.readFileSync(__dirname + "/../src-tauri/src/ai.rs", "utf8");
  assert.match(rust, /#\[serde\(rename = "openai"\)\]\s*OpenAi/);
  assert.match(rust, /#\[serde\(rename_all = "camelCase"\)\]\s*pub enum AiProvider/);
  assert.deepEqual(Object.keys(AI_MODELS), ["openai", "anthropic"]);
  for (const provider of Object.keys(AI_MODELS)) {
    assert.match(panelHtml, new RegExp(`<option value="${provider}">`));
    assert.match(panelHtml, new RegExp(`id="moderation-ai-key-${provider}" type="password" autocomplete="off"`));
    assert.match(panelHtml, new RegExp(`data-ai-key-save="${provider}"`));
    assert.match(panelHtml, new RegExp(`data-ai-key-forget="${provider}"`));
    assert.deepEqual(Object.keys(AI_MODELS[provider]), ["fast", "precise"]);
  }
  for (const id of ["moderation-ai-provider", "moderation-ai-tier", "moderation-ai-hour", "moderation-ai-day", "moderation-ai-model-name"]) {
    assert.match(panelHtml, new RegExp(`id="${id}"`), id);
  }
  // Limits in the page match the limits Relay validates.
  assert.match(rust, /MAX_CALLS_PER_HOUR: u16 = 600;/);
  assert.match(rust, /MAX_CALLS_PER_DAY: u16 = 10_000;/);
  assert.match(panelHtml, /id="moderation-ai-hour" type="number" min="1" max="600"/);
  assert.match(panelHtml, /id="moderation-ai-day" type="number" min="1" max="10000"/);
  const panel = fs.readFileSync(__dirname + "/../gui/panel.js", "utf8");
  assert.match(panel, /ai: moderationUi\.readAiDraft\(bootstrap\.config\.ai\)/);
});

test("every AI failure cause Relay can return has a translated message", () => {
  const rust = fs.readFileSync(__dirname + "/../src-tauri/src/ai/client.rs", "utf8");
  const variants = rust.match(/pub enum AiError \{([^}]*)\}/)[1].match(/^\s*([A-Z]\w+),/gm).map((line) => line.trim().slice(0, -1));
  assert.ok(variants.length >= 10);
  for (const [language, dictionary] of Object.entries(translations)) {
    const translate = (key) => dictionary[key];
    for (const variant of variants) {
      const code = variant[0].toLowerCase() + variant.slice(1);
      const message = describeAiTest({ error: code }, (key) => `#${key}`);
      assert.match(message, /^#ai(Test|HelpPace)[A-Z]/, code);
      assert.ok(translate(message.slice(1)), `${language}.${code}`);
    }
  }
  const format = (key, values) => `${key}:${JSON.stringify(values)}`;
  assert.equal(
    describeAiTest({ error: null, model: "claude-haiku-5-5", latencyMs: 812, inputTokens: 40, outputTokens: 5 }, t, format),
    'aiTestOk:{"model":"claude-haiku-5-5","ms":812,"tokens":45}',
  );
  // An unknown cause never shows raw text from Relay or the provider.
  assert.equal(describeAiTest({ error: "Incorrect API key provided: sk-abc" }, t), "aiTestBadRequest");
  assert.match(panelHtml, /id="moderation-ai-test"/);
  assert.match(panelHtml, /id="moderation-ai-test-result"/);
});

test("AI advice reads as a hint for every verdict, category and state Relay can send", () => {
  const rust = fs.readFileSync(__dirname + "/../src-tauri/src/ai/advice.rs", "utf8");
  // Variant lines only: doc comments inside the enum hold capitalised words too.
  const variants = (name) => rust.match(new RegExp(`pub enum ${name} \{([^}]*)\}`))[1].match(/^\s*[A-Z]\w+(?=,)/gm)
    .map((line) => line.trim()).map((variant) => variant[0].toLowerCase() + variant.slice(1));
  const verdicts = variants("AiVerdict");
  const categories = variants("AiCategory");
  assert.deepEqual(verdicts, ["clearProblem", "ambiguous", "noIssueSeen"]);
  assert.equal(categories.length, 10);
  for (const [language, dictionary] of Object.entries(translations)) {
    const translate = (key) => dictionary[key];
    for (const verdict of verdicts) {
      const line = describeAiAdvice({ state: "done", verdict, categories, reason: "Raison." }, translate);
      assert.ok(!/undefined|aiVerdict|aiCategory/.test(line), `${language}.${verdict}: ${line}`);
    }
    for (const advice of [{ state: "pending" }, { state: "notSent", cause: "personalData" }, { state: "notSent", cause: "empty" }, { state: "failed", cause: "timeout" }]) {
      const line = describeAiAdvice(advice, translate);
      assert.ok(line && !/undefined|aiAdvice|aiTest/.test(line), `${language}.${advice.state}: ${line}`);
    }
  }
  assert.equal(
    describeAiAdvice({ state: "done", verdict: "ambiguous", categories: ["threat", "unknown"], reason: "  Ton ironique.  " }, t),
    "aiAdviceLabel · aiVerdictAmbiguous (aiCategoryThreat) — Ton ironique.",
  );
  assert.equal(describeAiAdvice({ state: "done", verdict: "noIssueSeen", categories: [], reason: "" }, t), "aiAdviceLabel · aiVerdictNoIssueSeen");
  assert.equal(describeAiAdvice({ state: "failed", cause: "spendLimit" }, t), "aiAdviceLabel · aiTestSpendLimit");
  assert.equal(describeAiAdvice(undefined, t), "");
  assert.match(panelHtml, /id="moderation-ai-message-advice" type="checkbox"/);
  assert.match(panelHtml, /id="moderation-test-ai" [^>]*hidden/);
  const ui = fs.readFileSync(__dirname + "/../gui/moderation-ui.mjs", "utf8");
  assert.match(ui, /messageAdvice: field\("ai-message-advice"\)\.checked/);
  // Model output is only ever written as text.
  assert.doesNotMatch(ui, /innerHTML/);
});

test("every reason an image was not sent reads in plain words, animations included", () => {
  const rust = fs.readFileSync(__dirname + "/../src-tauri/src/ai/advice.rs", "utf8");
  const causes = rust.match(/pub enum NotSentCause \{([^}]*)\}/)[1].match(/^\s*([A-Z]\w+),/gm)
    .map((line) => line.trim().replace(",", "")).map((variant) => variant[0].toLowerCase() + variant.slice(1));
  assert.deepEqual(causes, ["personalData", "empty", "notAnImage", "animated", "location", "tooLarge", "unverified"]);
  for (const [language, dictionary] of Object.entries(translations)) {
    const translate = (key) => dictionary[key];
    for (const cause of causes) {
      const line = describeAiAdvice({ state: "notSent", cause }, translate);
      assert.ok(line && !/undefined|aiAdvice/.test(line), `${language}.${cause}: ${line}`);
    }
  }
  assert.equal(describeAiAdvice({ state: "notSent", cause: "animated" }, t), "aiAdviceLabel · aiAdviceNotSentAnimated");
  // Animations are never sent, so no advice is ever a partial check.
  assert.doesNotMatch(fs.readFileSync(__dirname + "/../gui/moderation-ui.mjs", "utf8"), /firstFrameOnly|aiAdvicePartial/);
  // The queue offers the AI per media only once a provider is chosen.
  assert.match(panelHtml, /class="button button--quiet button--small moderation-item__ai" type="button" data-i18n="aiTestAdvice" hidden/);
});

test("a settings proposal is described by Relay and only touches preset settings", () => {
  const done = {
    state: "done",
    changes: ["reviewedTypes", "privacyScan"],
    weakensProtection: true,
    note: "  Images pass; the scan is off.  ",
    config: {},
  };
  assert.equal(
    describeSettingsAnswer(done, t, format),
    'presetChanges:{"count":2,"list":"changeReviewedTypes, changePrivacyScan"} aiSettingsWeakens aiSettingsNote Images pass; the scan is off.',
  );
  assert.equal(
    describeSettingsAnswer({ ...done, changes: [], weakensProtection: false, note: "" }, t, format),
    "presetNoChange",
  );
  assert.equal(describeSettingsAnswer({ state: "notSent", cause: "personalData" }, t, format), "aiSettingsLabel · aiAdviceNotSentPersonalData");
  assert.equal(describeSettingsAnswer({ state: "failed", cause: "keyRefused" }, t, format), "aiSettingsLabel · aiTestKeyRefused");
  assert.equal(describeSettingsAnswer({ state: "failed", cause: "somethingNew" }, t, format), "aiSettingsLabel · aiTestBadRequest");

  // Even a proposal carrying other values only brings the preset settings.
  const current = {
    watchedChannelId: "123456789012345678", moderationEnabled: false, privacyScanEnabled: true,
    moderation: { reviewOnlySelected: false, trustedRoleIds: ["223456789012345678"], userCooldownSeconds: 0 },
  };
  const proposed = {
    watchedChannelId: "999999999999999999", moderationEnabled: true, privacyScanEnabled: true,
    moderation: { reviewOnlySelected: true, trustedRoleIds: [], userCooldownSeconds: 10 },
  };
  const applied = mergePresetFields(current, capturePresetFields(proposed));
  assert.equal(applied.watchedChannelId, "123456789012345678");
  assert.deepEqual(applied.moderation.trustedRoleIds, ["223456789012345678"]);
  assert.equal(applied.moderationEnabled, true);
  assert.equal(applied.moderation.reviewOnlySelected, true);
  assert.equal(applied.moderation.userCooldownSeconds, 10);
});

test("the settings assistant is a bounded, hidden-by-default box outside the autosaved form", () => {
  const box = panelHtml.match(/<div id="moderation-ai-settings"[\s\S]*?<\/div>\s*<\/div>/)?.[0] || "";
  assert.match(box, /hidden/);
  assert.match(box, /<textarea id="moderation-ai-wish"[^>]*maxlength="500"/);
  assert.match(box, /id="moderation-ai-suggest"/);
  assert.ok(panelHtml.indexOf('id="moderation-ai-settings"') < panelHtml.indexOf('id="moderation-form"'));
});

test("troubleshooting findings carry Relay's label: from the report or a guess", () => {
  assert.deepEqual(
    describeTroubleshootAnswer({
      state: "done",
      findings: [
        { text: "  Ton GIF attend une validation.  ", fromReport: true },
        { text: "OBS a peut-être été relancé.", fromReport: false },
        { text: "   ", fromReport: true },
      ],
      nextStep: " Ouvre la page Modération. ",
      report: ["Discord bot connected: yes."],
    }, t),
    [
      "aiHelpFromReport Ton GIF attend une validation.",
      "aiHelpGuess OBS a peut-être été relancé.",
      "aiHelpNextStep Ouvre la page Modération.",
    ],
  );
  assert.deepEqual(describeTroubleshootAnswer({ state: "done", findings: [], nextStep: "", report: [] }, t), ["aiHelpNothing"]);
  assert.deepEqual(describeTroubleshootAnswer({ state: "notSent", cause: "personalData" }, t), ["aiHelpLabel · aiAdviceNotSentPersonalData"]);
  assert.deepEqual(describeTroubleshootAnswer({ state: "failed", cause: "noProvider" }, t), ["aiHelpLabel · aiTestNoProvider"]);
});

test("the troubleshooting box is bounded, says what it sends and shows the sent report", () => {
  const box = panelHtml.match(/<div id="ai-help"[\s\S]*?<\/details>/)?.[0] || "";
  assert.match(box, /<textarea id="ai-help-question"[^>]*maxlength="500"/);
  assert.match(box, /data-i18n="aiHelpSends"/);
  assert.match(box, /id="ai-help-ask"/);
  assert.match(box, /<ul id="ai-help-answer"[^>]*hidden/);
  assert.match(box, /<details id="ai-help-report"[^>]*hidden/);
  assert.match(box, /<pre id="ai-help-report-text">/);
});

test("a message held by the AI watch says so and names the switch that turns it off", () => {
  assert.equal(reasonLabel("ai_flagged", t), "reasonAiFlagged");
  assert.equal(explainReason("ai_flagged", {}, t, format), 'whyAiFlagged:{"cooldown":0,"window":60,"delay":0,"accountDays":0,"memberDays":0,"raid":0,"expiry":0} whySetting:{"setting":"aiCheckBefore"}');
  // The switch lives in the autosaved moderation form, off unless checked.
  const form = panelHtml.slice(panelHtml.indexOf('id="moderation-form"'));
  assert.match(form, /<input id="moderation-ai-watch" type="checkbox"(?![^>]*checked)[^>]*>/);
  assert.match(form, /data-i18n="aiCheckBeforeHelp"/);
});

test("applying an AI proposal only writes the settings the AI changed", () => {
  const asked = {
    moderationEnabled: false, privacyScanEnabled: true, privacyEnabledCategories: ["email", "phone"],
    moderation: { reviewOnlySelected: false, blockShorteners: false, userCooldownSeconds: 0 },
  };
  const proposal = {
    ...asked, moderationEnabled: true, privacyEnabledCategories: ["phone", "email"],
    moderation: { ...asked.moderation, reviewOnlySelected: true },
  };
  const fields = changedPresetFields(asked, proposal);
  // Same categories in another order are not a change.
  assert.deepEqual(fields, { moderationEnabled: true, moderation: { reviewOnlySelected: true } });
  // A setting the streamer changed while waiting is kept.
  const now = { ...asked, moderation: { ...asked.moderation, blockShorteners: true } };
  const applied = mergePresetFields(now, fields);
  assert.equal(applied.moderation.blockShorteners, true);
  assert.equal(applied.moderation.reviewOnlySelected, true);
  assert.equal(applied.moderationEnabled, true);
  assert.deepEqual(applied.privacyEnabledCategories, ["email", "phone"]);
});

test("the AI image check and the personal-data relay save every setting Relay reads", () => {
  const { AI_GUARD_CATEGORIES } = require("../gui/moderation-ui.mjs");
  const rust = fs.readFileSync(__dirname + "/../src-tauri/src/ai.rs", "utf8");
  // Same category names and order as `ImageGuardCategory::ALL` (camelCase).
  const variants = rust.match(/pub enum ImageGuardCategory \{([^}]*)\}/)[1].match(/[A-Z]\w+/g)
    .map((name) => name[0].toLowerCase() + name.slice(1));
  assert.deepEqual(AI_GUARD_CATEGORIES, variants);
  const form = panelHtml.slice(panelHtml.indexOf('id="moderation-form"'), panelHtml.indexOf("</form>", panelHtml.indexOf('id="moderation-form"')));
  for (const value of AI_GUARD_CATEGORIES) assert.match(form, new RegExp(`name="ai-guard-category" value="${value}"`));
  // Off by default in the page; the saved settings decide.
  for (const id of ["image-guard", "image-delete", "image-animated", "personal-relay", "personal-photos", "personal-screenshots", "personal-videos", "personal-incomplete"]) {
    assert.match(form, new RegExp(`<input id="moderation-ai-${id}" type="checkbox"(?![^>]*checked)[^>]*>`), id);
  }
  // The relay sits in Personal data protection.
  const privacy = panelHtml.slice(panelHtml.indexOf('id="moderation-privacy"'), panelHtml.indexOf('id="moderation-spam"'));
  assert.match(privacy, /id="moderation-ai-personal-relay"/);
  const ui = fs.readFileSync(__dirname + "/../gui/moderation-ui.mjs", "utf8");
  const serde = [...rust.matchAll(/^\s+pub (image_guard\w*|personal_data_\w+):/gm)]
    .map(([, name]) => name.replace(/_(\w)/g, (_, letter) => letter.toUpperCase()));
  assert.equal(serde.length, 9);
  for (const name of serde) assert.match(ui, new RegExp(`${name}: `), name);
  // Free and paid badges: three each.
  assert.equal((form.match(/data-i18n="aiBadgeFree"/g) || []).length, 3);
  assert.equal((form.match(/data-i18n="aiBadgePaid"/g) || []).length, 3);
});

test("images held or blocked by the AI say why and name the switch", () => {
  assert.equal(reasonLabel("ai_image", t), "reasonAiImage");
  assert.equal(reasonLabel("ai_personal_data", t), "reasonAiPersonalData");
  assert.equal(reasonLabel("ai_image_blocked", t), "reasonAiImageBlocked");
  assert.equal(reasonLabel("ai_image_blocked_hate", t), "reasonAiImageBlocked · aiGuardHate");
  assert.equal(reasonLabel("ai_image_blocked_self_harm", t), "reasonAiImageBlocked · aiGuardSelfHarm");
  assert.equal(reasonLabel("ai_image_unchecked", t), "reasonAiImageUnchecked");
  assert.equal(reasonLabel("ai_image_not_sent", t), "reasonAiImageNotSent");
  assert.match(explainReason("ai_image_not_sent", {}, t, format), /^whyAiImageNotSent:.*"aiImageGuard"/);
  assert.match(explainReason("ai_image_blocked_hate", {}, t, format), /^whyAiImageBlocked:/);
  assert.match(explainReason("ai_image_unchecked", {}, t, format), /^whyAiImageUnchecked:/);
  assert.match(explainReason("ai_personal_data", {}, t, format), /^whyAiPersonalData:.* whySetting:\{"setting":"aiPersonalRelay"\}$/);
  const rust = fs.readFileSync(__dirname + "/../src-tauri/src/ai/image_guard.rs", "utf8");
  for (const reason of rust.match(/pub const (?:HOLD_\w+|BLOCKED): &str = "(\w+)"/g).map((line) => line.match(/"(\w+)"/)[1])) {
    assert.notEqual(reasonLabel(reason, t), reason.replace(/_/g, " "), reason);
  }
  // The help pace refusal reads in plain words.
  assert.equal(describeTroubleshootAnswer({ state: "failed", cause: "helpCooldown" }, t).join(" ").includes("aiHelpPaceRefused"), true);
});

test("held media show a translated reason, with the scan's risk level when there is one", () => {
  assert.equal(queueReasonLabel({}, t), "reasonManual");
  assert.equal(queueReasonLabel({ privacyClassification: "low", privacyReason: "manual_review" }, t), "low · reasonManual");
  assert.equal(queueReasonLabel({ privacyClassification: "medium", privacyCategories: ["postalAddress", "phone"], privacyReason: "postal_address" }, t), "medium · privacyCategoryAddress");
  assert.equal(queueReasonLabel({ privacyClassification: "medium", privacyReason: "ocr_text", holdReason: "new_account" }, t), "reasonNewAccount");
  // The queue row uses it: a manual-review item once showed the raw code MANUAL_REVIEW.
  const panelSource = require("./test-source.cjs").panelSource();
  const nodes = {};
  const item = { querySelector: (selector) => (nodes[selector] ??= { addEventListener() {} }) };
  const context = vm.createContext({
    bootstrap: { config: {}, pendingMedia: [{ id: 1, media: { kind: "image", timestamp: 0 } }] },
    moderationUi: undefined, moderationListElement: { replaceChildren() {}, append() {} },
    moderationCountElement: {}, moderationEmptyElement: {}, clearPendingMediaButton: {},
    moderationItemTemplate: { content: { cloneNode: () => item } },
    setMediaThumbnail() {}, t, locale: "en", queueReasonLabel,
  });
  const render = panelSource.slice(panelSource.indexOf("function renderModeration"), panelSource.indexOf("function populateChannels"));
  vm.runInContext(`${render}\nrenderModeration();`, context);
  assert.equal(nodes[".moderation-item__privacy"].textContent, "reasonManual");
});

test("the decision log shows the Discord name Relay found and keeps the ID for the filter", () => {
  const ui = fs.readFileSync(__dirname + "/../gui/moderation-ui.mjs", "utf8");
  const rust = fs.readFileSync(__dirname + "/../src-tauri/src/commands/moderation.rs", "utf8");
  assert.match(rust, /\n\s+author_names: HashMap<String, String>,/);
  assert.match(ui, /const name = overview\.authorNames\?\.\[entry\.authorId\];/);
  assert.match(ui, /author\.textContent = name \|\| entry\.authorId;/);
  assert.match(ui, /logFilter\.value = entry\.authorId;/);
});

test("the cooldown notice has its switch next to the delay and is saved under its Rust name", () => {
  const html = fs.readFileSync(__dirname + "/../gui/panel.html", "utf8");
  const ui = fs.readFileSync(__dirname + "/../gui/moderation-ui.mjs", "utf8");
  const rust = fs.readFileSync(__dirname + "/../src-tauri/src/moderation.rs", "utf8");
  const cooldown = html.indexOf('id="moderation-cooldown"');
  const notice = html.indexOf('id="moderation-cooldown-notice"');
  assert.ok(cooldown > 0 && notice > cooldown && notice < html.indexOf('id="moderation-raid-limit"'));
  assert.match(ui, /"cooldown-notice": "cooldownNotice"/);
  assert.match(rust, /\n\s+pub cooldown_notice: bool,/);
});

test("an emptied moderation number keeps its saved value instead of sending 0", () => {
  const ui = fs.readFileSync(__dirname + "/../gui/moderation-ui.mjs", "utf8");
  // The CI checkout uses CRLF line endings.
  const start = ui.search(/ {4}const number = \(id, fallback\) => \{\r?\n {6}const text/);
  assert.ok(start > 0);
  const body = ui.slice(start, ui.indexOf("};", start) + 2).replace("const number =", "globalThis.number =");
  const values = { "duplicate-window": "", cooldown: " 12 ", raid: "abc" };
  const context = vm.createContext({ field: (id) => ({ value: values[id] }) });
  vm.runInContext(body, context);
  assert.equal(context.number("duplicate-window", 60), 60);
  assert.equal(context.number("cooldown", 0), 12);
  assert.equal(context.number("raid", 5), 5);
});

test("the repeated media window is a number next to its switch, explained with its value", () => {
  const html = fs.readFileSync(__dirname + "/../gui/panel.html", "utf8");
  const ui = fs.readFileSync(__dirname + "/../gui/moderation-ui.mjs", "utf8");
  const rust = fs.readFileSync(__dirname + "/../src-tauri/src/moderation.rs", "utf8");
  const toggle = html.indexOf('id="moderation-duplicates"');
  const window = html.indexOf('id="moderation-duplicate-window"');
  assert.ok(toggle > 0 && window > toggle);
  assert.match(html, /id="moderation-duplicate-window" type="number" min="5" max="3600"/);
  assert.match(ui, /"duplicate-window": "duplicateWindowSeconds"/);
  assert.match(rust, /\n\s+pub duplicate_window_seconds: u16,/);
  assert.match(rust, /duplicate_window_seconds: 60,/);
  assert.match(explainReason("duplicate", { duplicateWindowSeconds: 45 }, t, format), /^whyDuplicate:\{.*"window":45/);
});
