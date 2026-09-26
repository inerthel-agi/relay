// Safety → Moderation: presets, message tester, word lists, advanced words,
// decision log, held messages, Discord actions and keyboard review.

export const moderationPresets = ["relaxed", "standard", "strict", "event"];
export const wordPackIds = ["hate", "scams", "sexual", "harassment", "doxxing"];
const OWN_PRESETS_KEY = "relay-moderation-presets";
const OWN_PRESET_LIMIT = 12;
const UNDO_WINDOW_MS = 30_000;
const REFRESH_MS = 3_000;
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
};
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

export function reasonLabel(reason, t) {
  const key = REASON_KEYS[reason];
  return key ? t(key) : String(reason || "").replace(/_/g, " ");
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
  let undoConfig = null;
  let undoTimer;
  let visiblePackWords = null;
  let selectedIndex = 0;

  const switches = {
    "filter-usernames": "filterUsernames", "filter-music-titles": "filterMusicTitles",
    "review-text": "reviewText", duplicates: "blockDuplicates", "text-spam": "blockTextSpam",
    "block-invites": "blockInvites", "block-shorteners": "blockShorteners", "block-scams": "blockScamDomains",
    warn: "warnOnBlock", loudness: "loudnessLimiter",
  };
  const numbers = {
    "min-account-age": "minAccountAgeDays", "min-member-age": "minMemberAgeDays",
    "pending-expiry": "pendingExpiryMinutes", "safety-delay": "safetyDelaySeconds",
    cooldown: "userCooldownSeconds", "raid-limit": "raidLimitPerMinute",
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
  }

  function readDraft(current = {}) {
    const number = (id, fallback) => {
      const value = Number(field(id).value);
      return Number.isFinite(value) ? Math.round(value) : fallback;
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
      item.append(time, action, reason);
      if (entry.authorId) {
        const author = document.createElement("button");
        author.type = "button";
        author.className = "text-link moderation-log__author";
        author.textContent = entry.authorId;
        author.title = t("moderationLogFilter");
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
    const countdown = item.querySelector(".moderation-item__countdown");
    if (pendingItem.releaseAt) {
      countdown.hidden = false;
      countdown.dataset.releaseAt = String(pendingItem.releaseAt);
      updateCountdown(countdown);
    }
    wireMoreActions(item.querySelector(".moderation-item__more"), pendingItem.id);
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
    try {
      const keys = await invoke("preview_moderation_preset", { preset: pendingPreset });
      confirmText.textContent = describePresetChanges(keys, t, formatTranslation);
      confirmBox.hidden = false;
      $("#moderation-preset-apply").disabled = keys.length === 0;
    } catch (error) {
      setSaveState(saveState, "error", String(error));
    }
  }));
  $("#moderation-preset-cancel").addEventListener("click", () => {
    confirmBox.hidden = true;
    pendingPreset = null;
  });
  $("#moderation-preset-apply").addEventListener("click", async () => {
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
    applyAdvancedEdits: (concepts) => applyAdvancedEdits(concepts, advancedEdits()),
    decorateQueueItem,
    renderTexts,
    refresh,
    pendingTextCount: () => (overview.pendingTexts || []).length,
    applyLanguage: () => {
      renderOwnPresets();
      renderLog();
      const config = getBootstrap()?.config;
      if (config) {
        renderPacks(readDraft(config.moderation || {}));
        renderAdvanced(config.privacyConcepts);
      }
    },
  };
}
