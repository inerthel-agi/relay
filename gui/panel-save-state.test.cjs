const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

const source = require("./test-source.cjs").panelSource();
const html = fs.readFileSync(__dirname + "/panel.html", "utf8");
const helpers = source.slice(source.indexOf("function formSaveState"), source.indexOf("function setCredentials"));
const saveSource = source.slice(source.indexOf("function readConfigDraft"), source.indexOf("async function saveMediaCaptionVisibility"));
const commandSource = source.slice(source.indexOf('commandsForm.addEventListener("submit"'), source.indexOf("\n// Page modules"));

function fixture() {
  const elements = new Map();
  const forms = new Map();
  const element = (id) => {
    if (!elements.has(id)) elements.set(id, { id, value: "", checked: false, type: "text", dataset: {}, textContent: "" });
    return elements.get(id);
  };
  let form;
  for (const match of html.matchAll(/<form\b[^>]*>|<\/form>|<(?:input|select|textarea)\b[^>]*>/g)) {
    const tag = match[0];
    const id = tag.match(/id="([^"]+)"/)?.[1];
    if (tag.startsWith("</form")) form = undefined;
    else if (tag.startsWith("<form")) {
      form = { id, controls: [], handlers: {}, addEventListener(event, fn) { this.handlers[event] = fn; }, querySelectorAll() { return this.controls; } };
      forms.set(id, form);
    } else if (id) {
      const input = element(id);
      input.type = tag.match(/type="([^"]+)"/)?.[1] || "text";
      input.value = tag.match(/value="([^"]*)"/)?.[1] || "";
      input.form = form;
      form?.controls.push(input);
    }
  }
  const context = {
    document: { getElementById: element },
    dirtyForms: new Set(), formRevisions: new WeakMap(),
    t: (key) => key, language: "en", errorCategory: () => null,
    filterWordsToConcepts: (value) => value ? [{ canonical: value, aliases: [], regexes: [] }] : [],
    filterConceptsToLines: (value) => (value || []).map((item) => item.canonical).join(", "),
    filterRoleIds: (value) => value ? value.split(",") : [],
    filterRoleIdsToInput: (value) => (value || []).join(","),
    privacyListFromInput: (value) => value ? value.split("\n") : [],
    privacyListToInput: (value) => (value || []).join("\n"),
    populateChannels: (input, channels, selected) => { input.value = selected || ""; },
    applyNotificationSoundConfig() {}, updateBotActivityAvailability() {},
    customCommandsUi: undefined,
    applyOutputGeometryConfig() {}, updateSkipShortcutDisplay() {}, updatePanicShortcutDisplay() {}, moderationUi: { applyAdvancedEdits: (concepts) => concepts, readDraft: (settings) => settings, fillForm() {}, pendingTextCount: () => 0, decorateQueueItem() {}, renderTexts() {} }, obsSetup: undefined,
  };
  for (const match of source.matchAll(/const (\w+) = \$\("#([^"]+)"\);/g)) {
    context[match[1]] = forms.get(match[2]) || element(match[2]);
  }
  context.privacyCategoryElements = [{ form: forms.get("moderation-form"), checked: true, value: "contact" }];
  context.commandInputs = Object.fromEntries(["channel", "url", "show", "status", "test", "regenerate", "clear", "nuke", "lock", "changelog"].map((name) => [name, element("command-" + name)]));
  const config = {
    watchedChannelId: "media", ttsChannelId: "", musicChannelId: "", honeypotChannelId: "",
    mediaCleanupEnabled: false, ttsCleanupEnabled: false, musicCleanupEnabled: false,
    mediaWelcomeMessageId: "", ttsWelcomeMessageId: "", musicWelcomeMessageId: "",
    moderationEnabled: false, privacyScanEnabled: false, showMediaTextObs: false,
    showMediaTextWidget: false, widgetSoundEnabled: false,
    port: 4590, displayDurationMs: 8000, gifDurationMs: 8000, stickerDurationMs: 8000,
    notificationDurationMs: 8000, mediaVolume: 50, ttsCharacterLimit: 0, ttsQueueLimit: 50,
    ttsNotificationsObsEnabled: false, showAuthor: true,
    botOnlineStatus: "online", botActivityType: "custom", botActivityText: "",
    privacyConcepts: [], privacyFilterExemptRoleIds: [], privacyAllowlist: [], privacyCustomPatterns: [],
    privacyProtectionLevel: "balanced", privacyEnabledCategories: ["contact"], privacyBlockThreshold: "high",
    privacyReviewIntermediate: true, privacyAutoDeleteBlockedMessages: true, privacySimilarityBoost: 4,
    moderationAllowImages: true, moderationAllowVideos: true, moderationAllowAudio: true,
    honeypotAction: "kick", customCommands: [],
  };
  let persisted = structuredClone(config);
  const calls = [];
  context.bootstrap = { config: structuredClone(config), channels: [] };
  context.invoke = async (command, args) => {
    calls.push({ command, args });
    if (command === "get_bootstrap") return { config: structuredClone(persisted), channels: [] };
    if (command === "apply_config") {
      persisted = structuredClone(args.config);
      return { config: structuredClone(persisted), channels: [] };
    }
    if (command === "save_command_settings") return structuredClone(persisted);
    throw Error(command);
  };
  vm.createContext(context);
  vm.runInContext(helpers + saveSource + commandSource + '\nfunction applyBootstrap(next) { bootstrap = next; applyConfig(next.config); }', context);
  context.applyConfig(config);
  function edit(id, value) {
    const input = element(id);
    if (typeof value === "boolean") input.checked = value;
    else input.value = value;
    context.dirtyForms.add(input.form);
    context.formRevisions.set(input.form, (context.formRevisions.get(input.form) || 0) + 1);
  }
  return { context, forms, element, edit, calls };
}

test("saving commands preserves a media draft and its dirty state", async () => {
  const f = fixture();
  f.edit("duration", "17");
  await f.forms.get("commands-form").handlers.submit({ preventDefault() {} });
  assert.equal(f.element("duration").value, "17");
  assert.equal(f.context.dirtyForms.has(f.forms.get("media-form")), true);
});

test("saving routing excludes unrelated drafts, even invalid privacy input", async () => {
  const f = fixture();
  f.edit("duration", "17");
  f.edit("media-cleanup-enabled", true);
  f.context.filterWordsToConcepts = () => { throw Error("unfinished filter"); };
  assert.equal(await f.context.saveConfig(f.element("save-state"), f.forms.get("routing-form")), true);
  const saved = f.calls.find((call) => call.command === "apply_config").args.config;
  assert.equal(saved.displayDurationMs, 8000);
  assert.equal(saved.mediaCleanupEnabled, true);
  assert.equal(f.element("duration").value, "17");
});

test("saving messages excludes media and channel drafts", async () => {
  const f = fixture();
  f.edit("duration", "17");
  f.edit("tts-channel", "messages");
  f.edit("notification-duration", "12");
  f.edit("tts-character-limit", "180");
  f.edit("tts-queue-limit", "5");
  f.edit("tts-notifications-obs", true);

  assert.equal(
    await f.context.saveConfig(f.element("messages-save-state"), f.forms.get("messages-form")),
    true,
  );
  const saved = f.calls.find((call) => call.command === "apply_config").args.config;
  assert.equal(saved.ttsChannelId, "");
  assert.equal(saved.notificationDurationMs, 12000);
  assert.equal(saved.ttsCharacterLimit, 180);
  assert.equal(saved.ttsQueueLimit, 5);
  assert.equal("ttsSpeechEnabled" in saved, false);
  assert.equal(saved.ttsNotificationsObsEnabled, true);
  assert.equal(saved.displayDurationMs, 8000);
  assert.equal(f.element("tts-channel").value, "messages");
  assert.equal(f.context.dirtyForms.has(f.forms.get("media-form")), true);
});

test("the Discord channels form saves every channel together", async () => {
  const f = fixture();
  f.edit("channel", "media-2");
  f.edit("tts-channel", "messages");
  f.edit("tts-cleanup-enabled", true);
  f.edit("tts-welcome-message", "welcome");
  f.edit("music-channel", "music");
  f.edit("music-cleanup-enabled", true);
  f.edit("honeypot-channel", "trap");
  f.edit("honeypot-action", "ban");
  f.edit("duration", "17");

  assert.equal(await f.context.saveConfig(f.element("save-state"), f.forms.get("routing-form")), true);
  const saved = f.calls.find((call) => call.command === "apply_config").args.config;
  assert.equal(saved.watchedChannelId, "media-2");
  assert.equal(saved.ttsChannelId, "messages");
  assert.equal(saved.ttsCleanupEnabled, true);
  assert.equal(saved.ttsWelcomeMessageId, "welcome");
  assert.equal(saved.musicChannelId, "music");
  assert.equal(saved.musicCleanupEnabled, true);
  assert.equal(saved.honeypotChannelId, "trap");
  assert.equal(saved.honeypotAction, "ban");
  assert.equal(saved.displayDurationMs, 8000);
});

test("automatic filters never save a cleanup or moderation draft", async () => {
  const f = fixture();
  f.edit("media-cleanup-enabled", true);
  f.edit("privacy-scan-enabled", true);
  f.edit("privacy-concepts", "example");
  await f.context.saveConfig(f.element("moderation-save-state"), f.forms.get("moderation-form"), true);
  const saved = f.calls.find((call) => call.command === "apply_config").args.config;
  assert.equal(saved.mediaCleanupEnabled, false);
  assert.equal(saved.privacyScanEnabled, false);
  assert.equal(saved.privacyConcepts[0].canonical, "example");
  assert.equal(f.element("privacy-scan-enabled").checked, true);
  assert.equal(f.element("moderation-save-state").dataset.state, "unsaved");
});

test("typing while a save is pending retains the newer value", async () => {
  const f = fixture();
  f.edit("duration", "17");
  let release;
  const original = f.context.invoke;
  f.context.invoke = async (command, args) => {
    if (command === "apply_config") await new Promise((resolve) => { release = resolve; });
    return original(command, args);
  };
  const pending = f.context.saveConfig(f.element("media-save-state"), f.forms.get("media-form"));
  while (!release) await new Promise(setImmediate);
  f.edit("duration", "23");
  release();
  await pending;
  assert.equal(f.element("duration").value, "23");
  assert.equal(f.element("media-save-state").dataset.state, "unsaved");
});

test("queued form saves merge the latest persisted values", async () => {
  const f = fixture();
  f.edit("duration", "17");
  f.edit("port", "4591");
  await Promise.all([
    f.context.saveConfig(f.element("media-save-state"), f.forms.get("media-form")),
    f.context.saveConfig(f.element("system-save-state"), f.forms.get("system-form")),
  ]);
  const writes = f.calls.filter((call) => call.command === "apply_config");
  assert.equal(writes[1].args.config.displayDurationMs, 17000);
  assert.equal(writes[1].args.config.port, 4591);
});

test("failed saves retain drafts and do not poison subsequent saves", async () => {
  const f = fixture();
  f.edit("duration", "17");
  const original = f.context.invoke;
  f.context.invoke = async () => { throw Error("unavailable"); };
  assert.equal(await f.context.saveConfig(f.element("media-save-state"), f.forms.get("media-form")), false);
  assert.equal(f.element("media-save-state").dataset.state, "error");
  assert.equal(f.element("duration").value, "17");
  assert.equal(f.context.dirtyForms.has(f.forms.get("media-form")), true);
  f.context.invoke = original;
  assert.equal(await f.context.saveConfig(f.element("media-save-state"), f.forms.get("media-form")), true);
  assert.equal(f.context.dirtyForms.has(f.forms.get("media-form")), false);
});

test("draft snapshots exclude credentials", () => {
  const f = fixture();
  f.element("youtube-api-key").value = "test-only-value";
  assert.equal(f.context.captureFormDraft(f.forms.get("music-form")).some(({ element }) => element.type === "password"), false);
});

test("a filter-only save clears the dirty state when no other setting changed", async () => {
  const f = fixture();
  f.edit("privacy-concepts", "example");
  await f.context.saveConfig(f.element("moderation-save-state"), f.forms.get("moderation-form"), true);
  assert.equal(f.context.dirtyForms.has(f.forms.get("moderation-form")), false);
  assert.equal(f.element("moderation-save-state").dataset.state, "saved");
});

test("live configuration updates refresh clean forms without replacing drafts", () => {
  const f = fixture();
  f.edit("duration", "17");
  f.context.applyConfig({ ...f.context.bootstrap.config, port: 4591, displayDurationMs: 9000 });
  assert.equal(f.element("port").value, "4591");
  assert.equal(f.element("duration").value, "17");
});

test("clipboard rejection is reported and the copy action becomes available again", async () => {
  let reset;
  const button = { textContent: "Copy", dataset: {}, disabled: false };
  const context = vm.createContext({
    navigator: { clipboard: { writeText: async () => { throw Error("denied"); } } },
    window: { setTimeout: (fn) => { reset = fn; } }, t: (key) => key,
  });
  const copySource = source.slice(source.indexOf("async function copyValue"), source.indexOf('copyUrlButton.addEventListener'));
  vm.runInContext(copySource, context);
  await context.copyValue(button, "demo");
  assert.equal(button.dataset.state, "error");
  assert.equal(button.textContent, "copyFailed");
  reset();
  assert.equal(button.disabled, false);
  assert.equal(button.textContent, "copy");
});

test("credential status refreshes do not erase unsubmitted input", () => {
  const token = { value: "test-only-draft" };
  const key = { value: "test-only-draft" };
  const client = { value: "draft-client" };
  const context = vm.createContext({
    tokenElement: token, youtubeApiKeyElement: key, clientIdElement: client,
    credentialStateElement: {}, setSaveState() {}, updateYoutubeKeyStatus() {}, t: (value) => value,
    moderationUi: { applyAdvancedEdits: (concepts) => concepts, readDraft: (settings) => settings, fillForm() {}, pendingTextCount: () => 0, decorateQueueItem() {}, renderTexts() {} }, obsSetup: undefined, $: () => ({}),
  });
  vm.runInContext(source.slice(source.indexOf("function setCredentials"), source.indexOf("function updateYoutubeKeyStatus")), context);
  context.setCredentials({ configured: true, source: "Windows", clientId: "saved-client" });
  assert.equal(token.value, "test-only-draft");
  assert.equal(key.value, "test-only-draft");
  assert.equal(client.value, "draft-client");
});
