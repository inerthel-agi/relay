// Safety → Moderation: presets, message tester, word lists, advanced words,
// decision log, held messages, Discord actions and keyboard review.

const wordPackIds = ["hate", "scams", "sexual", "harassment", "doxxing"];
const OWN_PRESETS_KEY = "relay-moderation-presets";
const OWN_PRESET_LIMIT = 12;
const UNDO_WINDOW_MS = 30_000;
const REFRESH_MS = 3_000;
/// Model behind each AI tier, per provider (shown in the panel).
export const AI_MODELS = {
  openai: { fast: "gpt-6-luna", precise: "gpt-6.1-sol" },
  anthropic: { fast: "claude-haiku-5-5", precise: "claude-sonnet-5-5" },
};
/// Config fields a preset (built-in or personal) controls, besides `moderation`.
const PRESET_CONFIG_KEYS = [
  "moderationEnabled", "moderationAllowImages", "moderationAllowVideos", "moderationAllowAudio",
  "privacyScanEnabled", "privacyProtectionLevel", "privacyBlockThreshold", "privacyReviewIntermediate",
  "privacyAutoDeleteBlockedMessages", "privacyEnabledCategories",
];
const PRESET_MODERATION_KEYS = [
  "wordPacks", "reviewOnlySelected", "userCooldownSeconds", "raidLimitPerMinute", "blockDuplicates",
  "blockTextSpam", "minAccountAgeDays", "blockInvites", "blockShorteners", "blockScamDomains",
];
const REASON_KEYS = {
  forbidden_concept: "reasonFilterWord", forbidden_regex: "reasonRegex",
  forbidden_similarity: "reasonSimilar", similarity_score: "reasonSimilar",
  discord_invite: "reasonInvite", link_shortener: "reasonShortener", scam_domain: "reasonScam",
  cooldown: "reasonCooldown", duplicate: "reasonDuplicate", mention_spam: "reasonMentions",
  caps_spam: "reasonCaps", emoji_spam: "reasonEmoji", repeated_characters: "reasonRepeated",
  blocked_user: "reasonBlockedUser", new_account: "reasonNewAccount", new_member: "reasonNewMember",
  raid: "reasonRaid", safety_delay: "reasonSafetyDelay", manual_review: "reasonManual",
  expired: "reasonExpired", queue_full: "reasonQueueFull", honeypot: "reasonHoneypot",
  escalation: "reasonEscalation", block_warning: "reasonWarning", reject_all: "reasonRejectAll",
  live_preset_on: "reasonLiveOn", live_preset_off: "reasonLiveOff", privacy: "reasonPrivacy",
  ai_image: "reasonAiImage", ai_personal_data: "reasonAiPersonalData", ai_image_blocked: "reasonAiImageBlocked",
  ai_image_unchecked: "reasonAiImageUnchecked",
  ai_image_not_sent: "reasonAiImageNotSent",
  // Privacy scan reasons reuse the labels of their category.
  email: "privacyCategoryEmail", phone: "privacyCategoryPhone", person_phone_combination: "privacyCategoryPhone",
  ip_address: "privacyCategoryIp", gps: "privacyCategoryGps", coordinates: "privacyCategoryGps",
  location_profile: "privacyCategoryGps", postal_address: "privacyCategoryAddress",
  partial_address: "privacyCategoryAddress", address_or_visual_context: "privacyCategoryAddress",
  person_address_combination: "privacyCategoryAddress", iban: "privacyCategoryFinancial",
  payment_card: "privacyCategoryFinancial", license_plate: "privacyCategoryPlate", sensitive_url: "privacyCategoryUrl",
  custom_pattern: "privacyCategoryCustom", exif_metadata: "privacyCategoryMetadata", ocr_text: "privacyCategoryOcr",
  document: "privacyCategoryDocument", scan_incomplete: "reasonScanIncomplete",
  image_scan_unavailable: "reasonScanUnavailable", image_fetch_unavailable: "reasonScanUnavailable",
  scan_config_changed: "reasonRulesChanged", image_limits: "reasonMediaLimits", mime_mismatch: "reasonMediaLimits",
  ai_flagged: "reasonAiFlagged",
};
/// Why Relay took a decision: [sentence key, label key of the setting that changes it].
const REASON_EXPLANATIONS = {
  cooldown: ["whyCooldown", "userCooldown"], duplicate: ["whyDuplicate", "blockDuplicates"],
  mention_spam: ["whyTextSpam", "blockTextSpam"], caps_spam: ["whyTextSpam", "blockTextSpam"],
  emoji_spam: ["whyTextSpam", "blockTextSpam"], repeated_characters: ["whyTextSpam", "blockTextSpam"],
  blocked_user: ["whyBlockedUser", "blockedUsers"], new_account: ["whyNewAccount", "minAccountAge"],
  new_member: ["whyNewMember", "minMemberAge"], raid: ["whyRaid", "raidLimit"],
  safety_delay: ["whySafetyDelay", "safetyDelay"], manual_review: ["whyManualReview", "enableModeration"],
  expired: ["whyExpired", "pendingExpiry"], queue_full: ["whyQueueFull"],
  honeypot: ["whyHoneypot", "honeypotChannel"], escalation: ["whyEscalation"],
  forbidden_concept: ["whyFilterWord", "packExclusions"], forbidden_regex: ["whyFilterWord", "packExclusions"],
  forbidden_similarity: ["whyFilterWord", "packExclusions"], similarity_score: ["whyFilterWord", "packExclusions"],
  discord_invite: ["whyLink", "blockInvites"], link_shortener: ["whyLink", "blockShorteners"],
  scam_domain: ["whyLink", "blockScams"],
  gps: ["whyLocation"], coordinates: ["whyLocation"], location_profile: ["whyLocation"], exif_metadata: ["whyLocation"],
  scan_incomplete: ["whyScanIncomplete"], image_scan_unavailable: ["whyScanUnavailable"],
  image_fetch_unavailable: ["whyScanUnavailable"], scan_config_changed: ["whyRulesChanged"],
  image_limits: ["whyMediaLimits"], mime_mismatch: ["whyMediaLimits"],
  ai_flagged: ["whyAiFlagged", "aiCheckBefore"],
  ai_image: ["whyAiImage", "aiImageGuard"], ai_image_blocked: ["whyAiImageBlocked", "aiImageGuard"],
  ai_image_unchecked: ["whyAiImageUnchecked", "aiImageGuard"],
  ai_image_not_sent: ["whyAiImageNotSent", "aiImageGuard"],
  ai_personal_data: ["whyAiPersonalData", "aiPersonalRelay"],
};
for (const reason of [
  "email", "phone", "person_phone_combination", "ip_address", "postal_address", "partial_address",
  "address_or_visual_context", "person_address_combination", "iban", "payment_card", "license_plate",
  "sensitive_url", "custom_pattern", "document", "ocr_text",
]) REASON_EXPLANATIONS[reason] = ["whyPersonalData", "privacyAllowlist"];
const LOG_ACTION_KEYS = {
  blocked: "logBlocked", held: "logHeld", ignored: "logIgnored", approved: "logApproved",
  rejected: "logRejected", expired: "logExpired", evicted: "logEvicted", released: "logReleased",
  warned: "logWarned", timedOut: "logTimedOut", banned: "logBanned", kicked: "logKicked",
  messageDeleted: "logMessageDeleted",
};

/** Discord IDs from raw IDs or mentions (<@id>, <@!id>, <@&id>), deduplicated. */
export function parseDiscordIds(value) {
  const ids = [];
  for (const match of String(value || "").matchAll(/(?:<@[!&]?)?(\d{17,20})>?/g)) {
    if (!ids.includes(match[1])) ids.push(match[1]);
  }
  return ids;
}

export function wordsFromInput(value) {
  const words = [];
  for (const word of String(value || "").split(/[,\n]/)) {
    const trimmed = word.trim();
    if (trimmed && trimmed.length <= 64 && !words.includes(trimmed)) words.push(trimmed);
  }
  return words;
}

/** `ai_image_blocked_<category>`: the AI image check block, with the category that matched. */
const AI_BLOCK_CATEGORY_KEYS = {
  sexual: "aiGuardSexual", violence: "aiGuardViolence", self_harm: "aiGuardSelfHarm",
  hate: "aiGuardHate", harassment: "aiGuardHarassment", spam: "aiGuardSpam",
};
const AI_BLOCK_PREFIX = "ai_image_blocked_";
const aiBlockCategory = (reason) => String(reason || "").startsWith(AI_BLOCK_PREFIX)
  ? AI_BLOCK_CATEGORY_KEYS[reason.slice(AI_BLOCK_PREFIX.length)] : undefined;

export function reasonLabel(reason, t) {
  const category = aiBlockCategory(reason);
  if (category) return `${t("reasonAiImageBlocked")} · ${t(category)}`;
  const key = REASON_KEYS[reason];
  return key ? t(key) : String(reason || "").replace(/_/g, " ");
}

/** Image check categories, in the order Relay saves them (`ImageGuardCategory`). */
export const AI_GUARD_CATEGORIES = ["sexual", "violence", "selfHarm", "hate", "harassment", "spam"];
const aiGuardBoxes = () => [...document.querySelectorAll('input[name="ai-guard-category"]')];

const AI_ERROR_KEYS = {
  noProvider: "aiTestNoProvider", noKey: "aiTestNoKey", hourlyLimit: "aiTestHourlyLimit",
  dailyLimit: "aiTestDailyLimit", paused: "aiTestPaused", keyRefused: "aiTestKeyRefused",
  spendLimit: "aiTestSpendLimit", rateLimited: "aiTestRateLimited", unavailable: "aiTestUnavailable",
  timeout: "aiTestTimeout", refused: "aiTestRefused", invalidOutput: "aiTestInvalidOutput",
  badRequest: "aiTestBadRequest", helpCooldown: "aiHelpPaceRefused", helpDailyLimit: "aiHelpPaceRefused",
};

/** The connection test result, from the cause code Relay returns (never provider text). */
export function describeAiTest(result, t, formatTranslation) {
  if (!result?.error) {
    return formatTranslation("aiTestOk", {
      model: result?.model || "?",
      ms: result?.latencyMs ?? 0,
      tokens: (result?.inputTokens ?? 0) + (result?.outputTokens ?? 0),
    });
  }
  return t(AI_ERROR_KEYS[result.error] || "aiTestBadRequest");
}

const AI_VERDICT_KEYS = {
  clearProblem: "aiVerdictClearProblem", ambiguous: "aiVerdictAmbiguous", noIssueSeen: "aiVerdictNoIssueSeen",
};
const AI_CATEGORY_KEYS = {
  harassment: "aiCategoryHarassment", threat: "aiCategoryThreat", hate: "aiCategoryHate", sexual: "aiCategorySexual",
  spam: "aiCategorySpam", evasion: "aiCategoryEvasion", selfHarm: "aiCategorySelfHarm", other: "aiCategoryOther",
  privateConversation: "aiCategoryPrivateConversation", personalData: "aiCategoryPersonalData",
};
const AI_NOT_SENT_KEYS = {
  personalData: "aiAdviceNotSentPersonalData", empty: "aiAdviceNotSentEmpty", notAnImage: "aiAdviceNotSentNotAnImage",
  animated: "aiAdviceNotSentAnimated",
  location: "aiAdviceNotSentLocation", tooLarge: "aiAdviceNotSentTooLarge", unverified: "aiAdviceNotSentUnverified",
};

/**
 * One line for the moderator. The AI's reason is model output: it is shown as
 * plain text after Relay's own words, never as a decision.
 */
export function describeAiAdvice(advice, t) {
  if (!advice) return "";
  const label = t("aiAdviceLabel");
  if (advice.state === "done") {
    const categories = (advice.categories || []).map((category) => AI_CATEGORY_KEYS[category]).filter(Boolean).map(t);
    const verdict = t(AI_VERDICT_KEYS[advice.verdict] || "aiVerdictAmbiguous");
    const reason = String(advice.reason || "").trim();
    return `${label} · ${verdict}${categories.length ? ` (${categories.join(", ")})` : ""}${reason ? ` — ${reason}` : ""}`;
  }
  return ["pending", "notSent", "failed"].includes(advice.state) ? noAnswerLine(label, advice, t) : "";
}

/** "label · why" for a request without an answer: waiting, not sent or failed. */
function noAnswerLine(label, answer, t) {
  const key = answer?.state === "pending"
    ? "aiAdvicePending"
    : answer?.state === "notSent"
      ? AI_NOT_SENT_KEYS[answer.cause] || "aiAdviceNotSentUnverified"
      : AI_ERROR_KEYS[answer?.cause] || "aiTestBadRequest";
  return `${label} · ${t(key)}`;
}

/**
 * The settings assistant's proposal: the changes and the warning are Relay's
 * own words; the AI's note follows as plain text.
 */
export function describeSettingsAnswer(answer, t, formatTranslation) {
  if (answer?.state !== "done") return noAnswerLine(t("aiSettingsLabel"), answer, t);
  const note = String(answer.note || "").trim();
  return [
    describePresetChanges(answer.changes || [], t, formatTranslation),
    answer.weakensProtection ? t("aiSettingsWeakens") : "",
    note ? `${t("aiSettingsNote")} ${note}` : "",
  ].filter(Boolean).join(" ");
}

/**
 * Troubleshooting answer, one line per finding. Relay sets the label: "from
 * the report" only when the AI cited report lines that exist, a guess otherwise.
 */
export function describeTroubleshootAnswer(answer, t) {
  if (answer?.state !== "done") return [noAnswerLine(t("aiHelpLabel"), answer, t)];
  const lines = (answer.findings || [])
    .map((finding) => [finding.fromReport, String(finding.text || "").trim()])
    .filter(([, text]) => text)
    .map(([fromReport, text]) => `${t(fromReport ? "aiHelpFromReport" : "aiHelpGuess")} ${text}`);
  const next = String(answer.nextStep || "").trim();
  if (next) lines.push(`${t("aiHelpNextStep")} ${next}`);
  return lines.length ? lines : [t("aiHelpNothing")];
}

function showAdvice(element, advice, t) {
  element.textContent = describeAiAdvice(advice, t);
  element.dataset.verdict = advice?.state === "done" ? advice.verdict : advice?.state || "";
  element.hidden = !element.textContent;
}

/**
 * One plain sentence on why Relay took a decision, then the setting that
 * changes it. Numbers come from the current settings, not from the moment of
 * the decision. Unknown reasons return "".
 */
export function explainReason(reason, settings, t, formatTranslation) {
  const [key, setting] = REASON_EXPLANATIONS[aiBlockCategory(reason) ? "ai_image_blocked" : reason] || [];
  if (!key) return "";
  const why = formatTranslation(key, {
    cooldown: settings?.userCooldownSeconds ?? 0,
    window: settings?.duplicateWindowSeconds ?? 60,
    delay: settings?.safetyDelaySeconds ?? 0,
    accountDays: settings?.minAccountAgeDays ?? 0,
    memberDays: settings?.minMemberAgeDays ?? 0,
    raid: settings?.raidLimitPerMinute ?? 0,
    expiry: settings?.pendingExpiryMinutes ?? 0,
  });
  return setting ? `${why} ${formatTranslation("whySetting", { setting: t(setting) })}` : why;
}

/** Adds the explanation under a queue item or a log entry, when there is one. */
function appendExplanation(container, text) {
  if (!text) return;
  const why = document.createElement("small");
  why.className = "moderation-why";
  why.textContent = text;
  container.append(why);
}

export function logActionLabel(action, t) {
  return t(LOG_ACTION_KEYS[action] || "logBlocked");
}

/** "3 settings will change: manual review, word lists, links." */
export function describePresetChanges(keys, t, formatTranslation) {
  if (!keys.length) return t("presetNoChange");
  const labels = keys.map((key) => t(`change${key[0].toUpperCase()}${key.slice(1)}`));
  return formatTranslation("presetChanges", { count: keys.length, list: labels.join(", ") });
}

export function describeTestResult(result, t, formatTranslation) {
  const verdict = t({ allow: "testAllow", review: "testReview", block: "testBlock" }[result.action] || "testAllow");
  const parts = [verdict];
  if (result.reasons?.length) {
    parts.push(formatTranslation("testReasons", { reasons: result.reasons.map((reason) => reasonLabel(reason, t)).join(", ") }));
  }
  if (result.nameHidden) parts.push(t("testNameHidden"));
  return parts.join(" ");
}

/** Replaces aliases and regexes of each concept with the advanced editor's values. */
export function applyAdvancedEdits(concepts, edits) {
  return concepts.map((concept) => {
    const edit = edits.get(concept.canonical.toLowerCase());
    if (!edit) return concept;
    return { ...concept, aliases: wordsFromInput(edit.aliases).slice(0, 50), regexes: edit.regexes };
  });
}

export function filterLogEntries(entries, authorFilter) {
  const filter = String(authorFilter || "").trim();
  return filter ? entries.filter((entry) => String(entry.authorId || "").includes(filter)) : entries;
}

/** Snapshot of the settings a personal preset restores. */
export function capturePresetFields(config) {
  const fields = { moderation: {} };
  for (const key of PRESET_CONFIG_KEYS) fields[key] = structuredClone(config[key]);
  for (const key of PRESET_MODERATION_KEYS) fields.moderation[key] = structuredClone(config.moderation?.[key]);
  return fields;
}

/**
 * Preset settings that differ between two configs (lists compared as sets), so
 * an AI proposal only writes what it changed, not values captured earlier.
 */
export function changedPresetFields(before, after) {
  const same = (left, right) => {
    const normal = (value) => JSON.stringify(Array.isArray(value) ? [...value].sort() : value);
    return normal(left) === normal(right);
  };
  const from = capturePresetFields(before);
  const to = capturePresetFields(after);
  const fields = {};
  for (const key of PRESET_CONFIG_KEYS) if (!same(from[key], to[key])) fields[key] = to[key];
  const moderation = {};
  for (const key of PRESET_MODERATION_KEYS) {
    if (!same(from.moderation[key], to.moderation[key])) moderation[key] = to.moderation[key];
  }
  if (Object.keys(moderation).length) fields.moderation = moderation;
  return fields;
}

export function mergePresetFields(config, fields) {
  const merged = { ...config, moderation: { ...config.moderation } };
  for (const key of PRESET_CONFIG_KEYS) if (key in fields) merged[key] = structuredClone(fields[key]);
  for (const key of PRESET_MODERATION_KEYS) if (key in (fields.moderation || {})) merged.moderation[key] = structuredClone(fields.moderation[key]);
  return merged;
}

export function readOwnPresets(readStorage) {
  try {
    const presets = JSON.parse(readStorage(OWN_PRESETS_KEY) || "[]");
    return Array.isArray(presets)
      ? presets.filter((preset) => typeof preset?.name === "string" && preset.fields && typeof preset.fields === "object").slice(0, OWN_PRESET_LIMIT)
      : [];
  } catch {
    return [];
  }
}

export function initializeModerationUi({
  $, $$, t, formatTranslation, invoke, setSaveState, readStorage, getBootstrap, getLocale,
  applyConfig, saveModeration, onQueueChanged,
}) {
  const saveState = $("#moderation-save-state");
  const presetButtons = $$("[data-moderation-preset]");
  const presetCustom = $("#moderation-preset-custom");
  const confirmBox = $("#moderation-preset-confirm");
  const confirmText = $("#moderation-preset-changes");
  const undoBox = $("#moderation-preset-undo");
  const undoText = $("#moderation-preset-applied");
  const ownSelect = $("#moderation-own-preset");
  const ownName = $("#moderation-own-preset-name");
  const packList = $("#moderation-pack-list");
  const packWords = $("#moderation-pack-words");
  const advancedList = $("#moderation-advanced-list");
  const logList = $("#moderation-log-list");
  const logEmpty = $("#moderation-log-empty");
  const logFilter = $("#moderation-log-filter");
  const testResult = $("#moderation-test-result");
  const automodState = $("#moderation-automod-state");
  const liveState = $("#moderation-live-state");
  const moderationPage = $('[data-page="moderation"]');
  const field = (id) => $(`#moderation-${id}`);
  let overview = { activePreset: null, summary: {}, entries: [], pendingTexts: [], packs: [], liveStreaming: false };
  let pendingPreset = null;
  /// A settings assistant answer waiting in the same confirm box as presets.
  let pendingProposal = null;
  /// Bumped when a preset is opened or the box closed, so a late AI answer is dropped.
  let suggestRequest = 0;
  let undoConfig = null;
  let undoTimer;
  let visiblePackWords = null;
  let selectedIndex = 0;

  const switches = {
    "filter-usernames": "filterUsernames", "filter-music-titles": "filterMusicTitles",
    "review-text": "reviewText", duplicates: "blockDuplicates", "text-spam": "blockTextSpam",
    "block-invites": "blockInvites", "block-shorteners": "blockShorteners", "block-scams": "blockScamDomains",
    warn: "warnOnBlock", loudness: "loudnessLimiter", "cooldown-notice": "cooldownNotice",
  };
  const numbers = {
    "min-account-age": "minAccountAgeDays", "min-member-age": "minMemberAgeDays",
    "pending-expiry": "pendingExpiryMinutes", "safety-delay": "safetyDelaySeconds",
    cooldown: "userCooldownSeconds", "raid-limit": "raidLimitPerMinute",
    "duplicate-window": "duplicateWindowSeconds",
    "escalation-blocks": "escalationBlocks", "escalation-window": "escalationWindowMinutes",
    "escalation-timeout": "escalationTimeoutMinutes", "honeypot-timeout": "honeypotTimeoutMinutes",
    "live-port": "liveObsPort", "max-media": "maxMediaSeconds",
  };
  const idLists = {
    "trusted-roles": "trustedRoleIds", "trusted-users": "trustedUserIds",
    "blocked-users": "blockedUserIds", "honeypot-exempt": "honeypotExemptRoleIds",
  };

  function renderPacks(settings) {
    const enabled = new Set(settings.wordPacks || []);
    packList.replaceChildren(...wordPackIds.map((id) => {
      const row = document.createElement("div");
      row.className = "moderation-pack";
      const label = document.createElement("label");
      const input = document.createElement("input");
      input.type = "checkbox";
      input.name = "moderation-pack";
      input.value = id;
      input.checked = enabled.has(id);
      const name = document.createElement("span");
      const count = overview.packs.find((pack) => pack.id === id)?.words;
      name.textContent = count ? `${t(`pack${id[0].toUpperCase()}${id.slice(1)}`)} · ${formatTranslation("packWordCount", { count })}` : t(`pack${id[0].toUpperCase()}${id.slice(1)}`);
      label.append(input, name);
      const show = document.createElement("button");
      show.type = "button";
      show.className = "button button--quiet button--small";
      show.textContent = visiblePackWords === id ? t("packHide") : t("packShow");
      show.addEventListener("click", () => void togglePackWords(id));
      row.append(label, show);
      return row;
    }));
  }

  async function togglePackWords(id) {
    if (visiblePackWords === id) {
      visiblePackWords = null;
      packWords.hidden = true;
    } else {
      const words = await invoke("moderation_pack_words", { pack: id });
      visiblePackWords = id;
      $("#moderation-pack-words-title").textContent = t(`pack${id[0].toUpperCase()}${id.slice(1)}`);
      $("#moderation-pack-words-list").textContent = words.join(", ");
      packWords.hidden = false;
    }
    renderPacks(readDraft(getBootstrap()?.config?.moderation || {}));
  }

  function renderAdvanced(concepts) {
    advancedList.replaceChildren(...(concepts || []).map((concept) => {
      const row = document.createElement("div");
      row.className = "moderation-advanced__row";
      row.dataset.canonical = concept.canonical.toLowerCase();
      const title = document.createElement("strong");
      title.textContent = concept.canonical;
      const aliases = document.createElement("textarea");
      aliases.rows = 2;
      aliases.spellcheck = false;
      aliases.dataset.advanced = "aliases";
      aliases.setAttribute("aria-label", `${t("advancedAliases")} · ${concept.canonical}`);
      aliases.placeholder = t("advancedAliases");
      aliases.value = (concept.aliases || []).join("\n");
      const regexes = document.createElement("textarea");
      regexes.rows = 2;
      regexes.spellcheck = false;
      regexes.dataset.advanced = "regexes";
      regexes.setAttribute("aria-label", `${t("advancedRegexes")} · ${concept.canonical}`);
      regexes.placeholder = t("advancedRegexes");
      regexes.value = (concept.regexes || []).join("\n");
      row.append(title, aliases, regexes);
      return row;
    }));
    if (!concepts?.length) {
      const empty = document.createElement("p");
      empty.className = "empty-state";
      empty.textContent = t("advancedWordsEmpty");
      advancedList.append(empty);
    }
  }

  function advancedEdits() {
    const edits = new Map();
    for (const row of advancedList.querySelectorAll(".moderation-advanced__row")) {
      edits.set(row.dataset.canonical, {
        aliases: row.querySelector('[data-advanced="aliases"]').value,
        regexes: row.querySelector('[data-advanced="regexes"]').value.split("\n").map((line) => line.trim()).filter(Boolean).slice(0, 25),
      });
    }
    return edits;
  }

  /** Fills every control of the page from a config (existing ones included). */
  function fillForm(config) {
    const settings = config?.moderation || {};
    for (const [id, key] of Object.entries(switches)) field(id).checked = Boolean(settings[key]);
    for (const [id, key] of Object.entries(numbers)) field(id).value = String(settings[key] ?? 0);
    for (const [id, key] of Object.entries(idLists)) field(id).value = (settings[key] || []).join(", ");
    field("pack-exclusions").value = (settings.packExclusions || []).join(", ");
    field("scope-media").checked = settings.filterScopes?.media !== false;
    field("scope-notifications").checked = settings.filterScopes?.notifications !== false;
    field("scope-music").checked = settings.filterScopes?.music !== false;
    field("review-only-selected").value = settings.reviewOnlySelected ? "pass" : "drop";
    field("live-preset").value = settings.livePreset || "";
    renderPacks(settings);
    renderAdvanced(config?.privacyConcepts);
    renderPresetState(config);
    renderAi(config?.ai || {});
    // Log explanations quote current settings: redraw them when the config changes.
    renderLog();
    renderStateBadges();
  }

  function renderAiModel() {
    const models = AI_MODELS[field("ai-provider").value];
    field("ai-model-name").textContent = models
      ? formatTranslation("aiModelName", { model: models[field("ai-tier").value] || models.fast })
      : t("aiModelNone");
  }

  function renderAiKeys() {
    const credentials = getBootstrap()?.credentials || {};
    for (const provider of Object.keys(AI_MODELS)) {
      field(`ai-key-state-${provider}`).textContent = t(credentials[`${provider}Configured`] ? "aiKeySaved" : "aiKeyMissing");
    }
  }

  function renderAi(settings) {
    field("ai-provider").value = settings.provider || "";
    field("ai-tier").value = settings.modelTier || "fast";
    field("ai-hour").value = String(settings.maxCallsPerHour ?? 120);
    field("ai-day").value = String(settings.maxCallsPerDay ?? 1000);
    field("ai-message-advice").checked = Boolean(settings.messageAdvice);
    field("ai-watch").checked = Boolean(settings.watchMessages);
    field("ai-image-guard").checked = Boolean(settings.imageGuard);
    const categories = settings.imageGuardCategories ?? AI_GUARD_CATEGORIES;
    for (const box of aiGuardBoxes()) box.checked = categories.includes(box.value);
    field("ai-image-delete").checked = settings.imageGuardDeleteMessage ?? true;
    field("ai-image-animated").checked = Boolean(settings.imageGuardAnimated);
    field("ai-personal-relay").checked = Boolean(settings.personalDataRelay);
    field("ai-personal-photos").checked = settings.personalDataPhotos ?? true;
    field("ai-personal-screenshots").checked = Boolean(settings.personalDataScreenshots);
    field("ai-personal-videos").checked = settings.personalDataVideos ?? true;
    field("ai-personal-incomplete").checked = settings.personalDataIncomplete ?? true;
    // The tester and the settings assistant only appear once a provider is chosen.
    $("#moderation-test-ai").hidden = !settings.provider;
    $("#moderation-ai-settings").hidden = !settings.provider;
    renderAiModel();
    renderAiKeys();
    renderAiSteps();
  }

  /** Provider cards mirror the hidden select; key and use steps wait for a provider. */
  function renderAiSteps() {
    const provider = field("ai-provider").value;
    for (const card of document.querySelectorAll("[data-ai-provider]")) {
      card.setAttribute("aria-pressed", String(card.dataset.aiProvider === provider));
    }
    for (const key of document.querySelectorAll(".moderation-ai__key")) {
      key.hidden = !provider || !key.querySelector(`#moderation-ai-key-${provider}`);
    }
    for (const step of document.querySelectorAll('[data-ai-step="2"], [data-ai-step="3"]')) {
      step.toggleAttribute("data-waiting", !provider);
    }
    const advanced = document.querySelector(".ai-advanced");
    if (advanced) advanced.hidden = !provider;
    // With Anthropic, the OpenAI key stays available for the free image filter.
    if (provider === "anthropic") field("ai-key-openai").closest(".moderation-ai__key").hidden = false;
    field("ai-free-key-hint").hidden = provider !== "anthropic";
    field("ai-image-guard-options").hidden = !field("ai-image-guard").checked;
    field("ai-personal-scope").hidden = !field("ai-personal-relay").checked;
    field("ai-personal-provider").hidden = !field("ai-personal-relay").checked || Boolean(provider);
  }

  /** On/Off badges next to the parts that one switch turns on. */
  function renderStateBadges() {
    for (const badge of document.querySelectorAll("[data-state-for]")) {
      const on = Boolean(document.getElementById(badge.dataset.stateFor)?.checked);
      badge.textContent = t(on ? "stateOn" : "stateOff");
      badge.toggleAttribute("data-on", on);
    }
  }

  function readAiDraft(current = {}) {
    const number = (id, fallback) => {
      const value = Math.round(Number(field(id).value));
      return Number.isFinite(value) && value > 0 ? value : fallback;
    };
    return {
      ...current,
      provider: field("ai-provider").value || null,
      modelTier: field("ai-tier").value,
      maxCallsPerHour: number("ai-hour", current.maxCallsPerHour ?? 120),
      maxCallsPerDay: number("ai-day", current.maxCallsPerDay ?? 1000),
      messageAdvice: field("ai-message-advice").checked,
      watchMessages: field("ai-watch").checked,
      imageGuard: field("ai-image-guard").checked,
      imageGuardCategories: aiGuardBoxes().filter((box) => box.checked).map((box) => box.value),
      imageGuardDeleteMessage: field("ai-image-delete").checked,
      imageGuardAnimated: field("ai-image-animated").checked,
      personalDataRelay: field("ai-personal-relay").checked,
      personalDataPhotos: field("ai-personal-photos").checked,
      personalDataScreenshots: field("ai-personal-screenshots").checked,
      personalDataVideos: field("ai-personal-videos").checked,
      personalDataIncomplete: field("ai-personal-incomplete").checked,
    };
  }

  /** Keys go straight to Windows Credential Manager; they are never read back. */
  async function storeAiKey(provider, apiKey) {
    const result = $("#moderation-ai-key-result");
    setSaveState(result, "saving");
    try {
      const credentials = await invoke("store_ai_api_key", { provider, apiKey });
      const bootstrap = getBootstrap();
      if (bootstrap) bootstrap.credentials = credentials;
      renderAiKeys();
      setSaveState(result, "saved", t(apiKey ? "aiKeyStored" : "aiKeyForgotten"));
    } catch (error) {
      setSaveState(result, "error", String(error));
    } finally {
      field(`ai-key-${provider}`).value = "";
    }
  }

  function readDraft(current = {}) {
    const number = (id, fallback) => {
      const text = field(id).value.trim();
      const value = Number(text);
      // An emptied field keeps its saved value: 0 is refused for some settings.
      return text && Number.isFinite(value) ? Math.round(value) : fallback;
    };
    // liveRestore belongs to Relay; it is kept as-is and ignored when saving.
    const draft = { ...current };
    for (const [id, key] of Object.entries(switches)) draft[key] = field(id).checked;
    for (const [id, key] of Object.entries(numbers)) draft[key] = number(id, current[key] ?? 0);
    for (const [id, key] of Object.entries(idLists)) draft[key] = parseDiscordIds(field(id).value).slice(0, 200);
    draft.wordPacks = $$("input[name='moderation-pack']").filter((input) => input.checked).map((input) => input.value);
    if (!packList.childElementCount) draft.wordPacks = current.wordPacks || [];
    draft.packExclusions = wordsFromInput(field("pack-exclusions").value).slice(0, 200);
    draft.filterScopes = {
      media: field("scope-media").checked,
      notifications: field("scope-notifications").checked,
      music: field("scope-music").checked,
    };
    draft.reviewOnlySelected = field("review-only-selected").value === "pass";
    draft.livePreset = field("live-preset").value || null;
    return draft;
  }

  function renderPresetState(config = getBootstrap()?.config) {
    const active = overview.activePreset;
    for (const button of presetButtons) {
      const on = button.dataset.moderationPreset === active;
      button.classList.toggle("is-active", on);
      button.setAttribute("aria-pressed", String(on));
    }
    presetCustom.hidden = Boolean(active) || !config;
  }

  function renderOwnPresets() {
    const presets = readOwnPresets(readStorage);
    const options = presets.map((preset, index) => {
      const option = document.createElement("option");
      option.value = String(index);
      option.textContent = preset.name;
      return option;
    });
    if (!options.length) {
      const empty = document.createElement("option");
      empty.value = "";
      empty.textContent = t("ownPresetEmpty");
      options.push(empty);
    }
    ownSelect.replaceChildren(...options);
  }

  function storeOwnPresets(presets) {
    try {
      localStorage.setItem(OWN_PRESETS_KEY, JSON.stringify(presets.slice(0, OWN_PRESET_LIMIT)));
    } catch { /* Storage is optional. */ }
  }

  function applyConfigToForm(config) {
    applyConfig(config);
    fillForm(config);
  }

  function offerUndo(previous, message) {
    undoConfig = previous;
    undoText.textContent = message;
    undoBox.hidden = false;
    window.clearTimeout(undoTimer);
    undoTimer = window.setTimeout(() => {
      undoBox.hidden = true;
      undoConfig = null;
    }, UNDO_WINDOW_MS);
  }

  async function saveFrom(config) {
    applyConfigToForm(config);
    await saveModeration();
    await refresh();
  }

  function renderSummary() {
    const summary = overview.summary || {};
    $("#moderation-summary-blocked").textContent = String(summary.blocked || 0);
    $("#moderation-summary-held").textContent = String(summary.held || 0);
    $("#moderation-summary-ignored").textContent = String(summary.ignored || 0);
    $("#moderation-summary-flagged").textContent = String(summary.flaggedMembers || 0);
    liveState.hidden = !overview.liveStreaming || !getBootstrap()?.config?.moderation?.livePreset;
    $("#moderation-raid-state").hidden = !overview.raidActive;
  }

  function renderLog() {
    const entries = filterLogEntries(overview.entries || [], logFilter.value);
    logEmpty.hidden = entries.length > 0;
    logList.replaceChildren(...entries.slice(0, 200).map((entry) => {
      const item = document.createElement("li");
      item.className = "moderation-log__entry";
      item.dataset.action = entry.action;
      const time = document.createElement("time");
      time.dateTime = new Date(entry.at).toISOString();
      time.textContent = new Date(entry.at).toLocaleString(getLocale(), { dateStyle: "short", timeStyle: "medium" });
      const action = document.createElement("strong");
      action.textContent = logActionLabel(entry.action, t);
      const reason = document.createElement("span");
      reason.textContent = [entry.lane ? t(`lane${entry.lane[0].toUpperCase()}${entry.lane.slice(1)}`) : "", reasonLabel(entry.reason, t)].filter(Boolean).join(" · ");
      appendExplanation(reason, explainReason(entry.reason, getBootstrap()?.config?.moderation, t, formatTranslation));
      item.append(time, action, reason);
      if (entry.authorId) {
        const author = document.createElement("button");
        author.type = "button";
        author.className = "text-link moderation-log__author";
        // The log keeps only the ID; the Discord name is looked up for display.
        const name = overview.authorNames?.[entry.authorId];
        author.textContent = name || entry.authorId;
        author.title = name ? `${entry.authorId} · ${t("moderationLogFilter")}` : t("moderationLogFilter");
        author.addEventListener("click", () => {
          logFilter.value = entry.authorId;
          renderLog();
        });
        item.append(author);
      }
      return item;
    }));
  }

  async function refresh() {
    try {
      overview = await invoke("moderation_overview");
    } catch {
      return;
    }
    renderSummary();
    renderLog();
    renderPresetState();
    if (!packList.childElementCount || packList.dataset.counted !== "1") {
      packList.dataset.counted = "1";
      renderPacks(readDraft(getBootstrap()?.config?.moderation || {}));
    }
    onQueueChanged();
  }

  /** Extra controls for one media item of the queue (template clone). */
  function decorateQueueItem(item, pendingItem) {
    const row = item.querySelector(".moderation-item");
    row.dataset.pendingId = String(pendingItem.id);
    const sensitive = ["medium", "high", "critical"].includes(String(pendingItem.privacyClassification || "").toLowerCase());
    row.classList.toggle("is-sensitive", sensitive);
    if (pendingItem.holdReason) {
      item.querySelector(".moderation-item__privacy").textContent = reasonLabel(pendingItem.holdReason, t);
    }
    const reason = pendingItem.holdReason || pendingItem.privacyReason || "manual_review";
    appendExplanation(item.querySelector(".history-item__meta"), explainReason(reason, getBootstrap()?.config?.moderation, t, formatTranslation));
    const countdown = item.querySelector(".moderation-item__countdown");
    if (pendingItem.releaseAt) {
      countdown.hidden = false;
      countdown.dataset.releaseAt = String(pendingItem.releaseAt);
      updateCountdown(countdown);
    }
    wireMoreActions(item.querySelector(".moderation-item__more"), pendingItem.id);
    const meta = item.querySelector(".history-item__meta");
    if (pendingItem.aiAdvice) {
      const advice = document.createElement("small");
      advice.className = "moderation-ai-advice";
      showAdvice(advice, pendingItem.aiAdvice, t);
      meta.append(advice);
    }
    const askAi = item.querySelector(".moderation-item__ai");
    meta.append(askAi); // After the explanation and the advice it would update.
    askAi.textContent = t("aiTestAdvice");
    askAi.hidden = !getBootstrap()?.config?.ai?.provider
      || !["image", "gif"].includes(pendingItem.media?.kind)
      || pendingItem.aiAdvice?.state === "pending";
    // The periodic refresh only redraws when items come or go, so the answer is shown here.
    const showOnItem = (advice) => {
      const held = getBootstrap()?.pendingMedia?.find((candidate) => candidate.id === pendingItem.id);
      if (held) held.aiAdvice = advice;
      onQueueChanged();
    };
    askAi.addEventListener("click", async () => {
      askAi.disabled = true;
      showOnItem({ state: "pending" });
      try {
        showOnItem(await invoke("ai_review_media", { id: pendingItem.id }));
      } catch (error) {
        showOnItem(undefined);
        setSaveState(saveState, "error", String(error));
      }
    });
  }

  function updateCountdown(element) {
    const seconds = Math.max(0, Math.ceil((Number(element.dataset.releaseAt) - Date.now()) / 1000));
    element.textContent = formatTranslation("countdownRelease", { seconds });
  }

  function wireMoreActions(select, id) {
    // Template clones are not reached by the page translation pass.
    for (const option of select.options) if (option.dataset.i18n) option.textContent = t(option.dataset.i18n);
    select.setAttribute("aria-label", t("moreActions"));
    select.addEventListener("change", async () => {
      const action = select.value;
      if (!action) return;
      select.disabled = true;
      try {
        await invoke("moderate_pending", { id, action });
        await refresh();
      } catch (error) {
        setSaveState(saveState, "error", String(error));
      } finally {
        select.value = "";
        select.disabled = false;
      }
    });
  }

  /** Held notification messages, appended after the media items. */
  function renderTexts(list, template) {
    for (const pending of overview.pendingTexts || []) {
      const item = template.content.cloneNode(true);
      const row = item.querySelector(".moderation-item");
      row.classList.add("moderation-item--text");
      row.dataset.pendingId = String(pending.id);
      item.querySelector(".history-item__visual").hidden = true;
      item.querySelector(".history-item__filename").textContent = pending.text;
      item.querySelector(".history-item__author").textContent = pending.author?.username || t("unknownAuthor");
      item.querySelector(".moderation-item__privacy").textContent = `${t("heldText")} · ${reasonLabel(pending.reason, t)}`;
      appendExplanation(item.querySelector(".history-item__meta"), explainReason(pending.reason, getBootstrap()?.config?.moderation, t, formatTranslation));
      if (pending.aiAdvice) {
        const advice = document.createElement("small");
        advice.className = "moderation-ai-advice";
        showAdvice(advice, pending.aiAdvice, t);
        item.querySelector(".history-item__meta").append(advice);
      }
      const time = item.querySelector(".history-item__time");
      time.dateTime = new Date(pending.timestamp).toISOString();
      time.textContent = new Date(pending.timestamp).toLocaleTimeString(getLocale(), { hour: "2-digit", minute: "2-digit", second: "2-digit" });
      const approve = item.querySelector(".moderation-item__approve");
      const reject = item.querySelector(".moderation-item__reject");
      approve.textContent = t("approve");
      reject.textContent = t("reject");
      const decide = async (command) => {
        approve.disabled = true;
        reject.disabled = true;
        try {
          await invoke(command, { id: pending.id });
          await refresh();
        } catch (error) {
          setSaveState(saveState, "error", String(error));
          approve.disabled = false;
          reject.disabled = false;
        }
      };
      approve.addEventListener("click", () => decide("approve_pending_text"));
      reject.addEventListener("click", () => decide("reject_pending_text"));
      wireMoreActions(item.querySelector(".moderation-item__more"), pending.id);
      list.append(item);
    }
    return (overview.pendingTexts || []).length;
  }

  function selectItem(list, index) {
    const items = [...list.querySelectorAll(".moderation-item")];
    if (!items.length) return;
    selectedIndex = Math.min(Math.max(index, 0), items.length - 1);
    items.forEach((item, position) => item.classList.toggle("is-selected", position === selectedIndex));
    items[selectedIndex].scrollIntoView?.({ block: "nearest" });
  }

  /** A approves, R rejects, arrow keys move the selection. */
  function handleQueueKey(event, list) {
    if (event.target.closest?.("input, textarea, select")) return;
    const items = [...list.querySelectorAll(".moderation-item")];
    if (!items.length) return;
    const key = event.key.toLowerCase();
    if (key === "arrowdown" || key === "arrowup") {
      event.preventDefault();
      selectItem(list, selectedIndex + (key === "arrowdown" ? 1 : -1));
    } else if (key === "a" || key === "r") {
      event.preventDefault();
      const item = items[Math.min(selectedIndex, items.length - 1)];
      item.querySelector(key === "a" ? ".moderation-item__approve" : ".moderation-item__reject")?.click();
    }
  }

  presetButtons.forEach((button) => button.addEventListener("click", async () => {
    pendingPreset = button.dataset.moderationPreset;
    pendingProposal = null;
    suggestRequest += 1;
    try {
      const keys = await invoke("preview_moderation_preset", { preset: pendingPreset });
      confirmText.textContent = describePresetChanges(keys, t, formatTranslation);
      confirmBox.dataset.weakens = "false";
      confirmBox.hidden = false;
      $("#moderation-preset-apply").disabled = keys.length === 0;
    } catch (error) {
      setSaveState(saveState, "error", String(error));
    }
  }));
  $("#moderation-preset-cancel").addEventListener("click", () => {
    confirmBox.hidden = true;
    pendingPreset = null;
    pendingProposal = null;
    suggestRequest += 1;
  });
  $("#moderation-preset-apply").addEventListener("click", async () => {
    if (pendingProposal) {
      // Only the preset settings the AI changed, through the usual save queue.
      const { answer, asked } = pendingProposal;
      pendingProposal = null;
      confirmBox.hidden = true;
      const previous = structuredClone(getBootstrap().config);
      await saveFrom(mergePresetFields(getBootstrap().config, changedPresetFields(asked, answer.config)));
      offerUndo(previous, t("aiSettingsApplied"));
      return;
    }
    if (!pendingPreset) return;
    const previous = structuredClone(getBootstrap().config);
    try {
      const config = await invoke("apply_moderation_preset", { preset: pendingPreset });
      applyConfigToForm(config);
      confirmBox.hidden = true;
      offerUndo(previous, formatTranslation("presetApplied", { preset: t(`preset${pendingPreset[0].toUpperCase()}${pendingPreset.slice(1)}`) }));
      setSaveState(saveState, "saved");
      await refresh();
    } catch (error) {
      setSaveState(saveState, "error", String(error));
    }
  });
  $("#moderation-preset-undo-button").addEventListener("click", async () => {
    if (!undoConfig) return;
    const previous = undoConfig;
    undoBox.hidden = true;
    undoConfig = null;
    await saveFrom(mergePresetFields(getBootstrap().config, capturePresetFields(previous)));
  });

  $("#moderation-own-preset-save").addEventListener("click", () => {
    const name = ownName.value.trim().slice(0, 64);
    if (!name) {
      setSaveState(saveState, "error", t("presetNameRequired"));
      return;
    }
    const presets = readOwnPresets(readStorage).filter((preset) => preset.name !== name);
    presets.unshift({ name, fields: capturePresetFields(getBootstrap().config) });
    storeOwnPresets(presets);
    renderOwnPresets();
    setSaveState(saveState, "saved", t("ownPresetSaved"));
  });
  $("#moderation-own-preset-apply").addEventListener("click", async () => {
    const preset = readOwnPresets(readStorage)[Number(ownSelect.value)];
    if (!preset) return;
    const previous = structuredClone(getBootstrap().config);
    await saveFrom(mergePresetFields(getBootstrap().config, preset.fields));
    offerUndo(previous, formatTranslation("presetApplied", { preset: preset.name }));
  });
  $("#moderation-own-preset-delete").addEventListener("click", () => {
    const presets = readOwnPresets(readStorage);
    presets.splice(Number(ownSelect.value), 1);
    storeOwnPresets(presets);
    renderOwnPresets();
  });

  $("#moderation-test-button").addEventListener("click", async () => {
    const text = $("#moderation-test-text").value;
    if (!text.trim()) {
      testResult.textContent = t("testEmpty");
      return;
    }
    try {
      const result = await invoke("test_moderation_text", { text, lane: $("#moderation-test-lane").value });
      testResult.dataset.action = result.action;
      testResult.textContent = describeTestResult(result, t, formatTranslation);
    } catch (error) {
      testResult.textContent = String(error);
    }
  });

  field("ai-provider").addEventListener("input", renderAiModel);
  field("ai-provider").addEventListener("input", renderAiSteps);
  for (const card of document.querySelectorAll("[data-ai-provider]")) {
    card.addEventListener("click", () => {
      const select = field("ai-provider");
      select.value = card.dataset.aiProvider;
      // Same events as a user choice: the model line updates and the form autosaves.
      select.dispatchEvent(new Event("input", { bubbles: true }));
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
  }
  $("#moderation-form").addEventListener("change", renderStateBadges);
  field("ai-image-guard").addEventListener("change", renderAiSteps);
  field("ai-personal-relay").addEventListener("change", renderAiSteps);
  field("ai-tier").addEventListener("input", renderAiModel);
  for (const provider of Object.keys(AI_MODELS)) {
    const input = field(`ai-key-${provider}`);
    // A typed key must not trigger the settings autosave of the form.
    input.addEventListener("change", (event) => event.stopPropagation());
    $(`[data-ai-key-save="${provider}"]`).addEventListener("click", () => {
      const apiKey = input.value.trim();
      if (!apiKey) {
        setSaveState($("#moderation-ai-key-result"), "error", t("aiKeyEmpty"));
        return;
      }
      void storeAiKey(provider, apiKey);
    });
    $(`[data-ai-key-forget="${provider}"]`).addEventListener("click", () => void storeAiKey(provider, ""));
  }

  $("#moderation-test-ai").addEventListener("click", async (event) => {
    const button = event.currentTarget;
    const result = $("#moderation-test-ai-result");
    const text = $("#moderation-test-text").value;
    if (!text.trim()) {
      testResult.textContent = t("testEmpty");
      return;
    }
    button.disabled = true;
    showAdvice(result, { state: "pending" }, t);
    try {
      showAdvice(result, await invoke("ai_review_text", { text }), t);
    } catch (error) {
      result.hidden = false;
      result.dataset.verdict = "failed";
      result.textContent = String(error);
    } finally {
      button.disabled = false;
    }
  });

  $("#moderation-ai-suggest").addEventListener("click", async (event) => {
    const button = event.currentTarget;
    const result = $("#moderation-ai-suggest-result");
    const wish = $("#moderation-ai-wish").value;
    const show = (state, text) => {
      result.dataset.verdict = state;
      result.textContent = text;
      result.hidden = !text;
    };
    if (!wish.trim()) {
      show("notSent", t("aiSettingsWishEmpty"));
      return;
    }
    button.disabled = true;
    confirmBox.hidden = true;
    pendingPreset = null;
    pendingProposal = null;
    const request = ++suggestRequest;
    // The proposal will only write what differs from the settings asked about.
    const asked = structuredClone(getBootstrap().config);
    show("pending", `${t("aiSettingsLabel")} · ${t("aiAdvicePending")}`);
    try {
      const answer = await invoke("ai_suggest_settings", { wish });
      // A preset opened or the box closed meanwhile: this answer is no longer wanted.
      if (request !== suggestRequest) {
        show("", "");
        return;
      }
      if (answer?.state === "done") {
        show("", "");
        pendingProposal = { answer, asked };
        confirmText.textContent = describeSettingsAnswer(answer, t, formatTranslation);
        confirmBox.dataset.weakens = String(Boolean(answer.weakensProtection));
        confirmBox.hidden = false;
        $("#moderation-preset-apply").disabled = !answer.changes?.length;
      } else {
        show(answer?.state || "failed", describeSettingsAnswer(answer, t, formatTranslation));
      }
    } catch (error) {
      if (request === suggestRequest) show("failed", String(error));
    } finally {
      button.disabled = false;
    }
  });

  $("#moderation-ai-test").addEventListener("click", async (event) => {
    const button = event.currentTarget;
    const result = $("#moderation-ai-test-result");
    button.disabled = true;
    result.dataset.state = "saving";
    result.textContent = t("aiTestRunning");
    try {
      const outcome = await invoke("test_ai_connection");
      result.dataset.state = outcome?.error ? "error" : "saved";
      result.textContent = describeAiTest(outcome, t, formatTranslation);
    } catch (error) {
      setSaveState(result, "error", String(error));
    } finally {
      button.disabled = false;
    }
  });

  logFilter.addEventListener("input", renderLog);
  $("#moderation-log-clear").addEventListener("click", async () => {
    await invoke("clear_moderation_log");
    await refresh();
  });

  $("#moderation-automod-sync").addEventListener("click", async () => {
    setSaveState(automodState, "saving");
    try {
      const count = await invoke("sync_discord_automod");
      setSaveState(automodState, "saved", formatTranslation("automodSynced", { count }));
    } catch (error) {
      setSaveState(automodState, "error", String(error));
    }
  });
  $("#moderation-automod-remove").addEventListener("click", async () => {
    setSaveState(automodState, "saving");
    try {
      const removed = await invoke("remove_discord_automod");
      setSaveState(automodState, "saved", t(removed ? "automodRemoved" : "automodMissing"));
    } catch (error) {
      setSaveState(automodState, "error", String(error));
    }
  });

  const queueList = $("#moderation-list");
  queueList.addEventListener("keydown", (event) => handleQueueKey(event, queueList));
  queueList.addEventListener("focus", () => selectItem(queueList, selectedIndex));

  let ticks = 0;
  window.setInterval(() => {
    for (const countdown of document.querySelectorAll(".moderation-item__countdown[data-release-at]")) updateCountdown(countdown);
    ticks += 1;
    if (ticks % (REFRESH_MS / 1000) === 0 && !moderationPage.hidden && document.visibilityState === "visible") void refresh();
  }, 1000);

  renderOwnPresets();
  void refresh();

  return {
    fillForm,
    readDraft,
    readAiDraft,
    applyAdvancedEdits: (concepts) => applyAdvancedEdits(concepts, advancedEdits()),
    decorateQueueItem,
    renderTexts,
    refresh,
    pendingTextCount: () => (overview.pendingTexts || []).length,
    applyLanguage: () => {
      renderOwnPresets();
      renderLog();
      renderAiModel();
      renderAiKeys();
      renderStateBadges();
      const config = getBootstrap()?.config;
      if (config) {
        renderPacks(readDraft(config.moderation || {}));
        renderAdvanced(config.privacyConcepts);
      }
    },
  };
}
