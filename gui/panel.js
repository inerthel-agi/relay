import { initializePresetControls } from "./output-presets.mjs";
import { initializeModules } from "./modules.mjs";
const { invoke } = window.__TAURI__.core;
let moduleControls;

import { translations, regionalTranslations } from "./translations.mjs";

const pageMetadata = {
  messages: { title: "moduleMessages", kicker: "playback" },
  reactions: { title: "moduleReactions", kicker: "playback" },
  overview: { title: "navOverview", kicker: "system" },
  media: { title: "navMedia", kicker: "playback" },
  music: { title: "navMusic", kicker: "jukebox" },
  overlay: { title: "navOverlay", kicker: "output" },
  moderation: { title: "navModeration", kicker: "safety" },
  commands: { title: "navCommands", kicker: "commandsKicker" },
  history: { title: "navHistory", kicker: "archive" },
  help: { title: "navHelp", kicker: "guide" },
  personalization: { title: "navPersonalization", kicker: "personalizationKicker" },
  changelog: { title: "navChangelog", kicker: "changelogKicker" },
  about: { title: "navAbout", kicker: "about" },
};

const $ = (selector) => document.querySelector(selector);
const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];

const botStatusElement = $("#bot-status");
const botAvatarElement = $("#bot-avatar");
const botLabelElement = $("#bot-label");
const serverStatusElement = $("#server-status");
const serverLabelElement = $("#server-label");
const clientCountElement = $("#client-count");
const credentialForm = $("#credential-form");
const botPresenceForm = $("#bot-presence-form");
const routingForm = $("#routing-form");
const musicForm = $("#music-form");
const mediaForm = $("#media-form");
const messagesForm = $("#messages-form");
const moderationForm = $("#moderation-form");
const commandsForm = $("#commands-form");
const dirtyForms = new Set();
const formRevisions = new WeakMap();
for (const form of [botPresenceForm, routingForm, musicForm, mediaForm, messagesForm, moderationForm, commandsForm]) {
  form.addEventListener("input", () => {
    dirtyForms.add(form);
    formRevisions.set(form, (formRevisions.get(form) || 0) + 1);
    setSaveState(formSaveState(form), "unsaved");
  });
}
const commandsSaveStateElement = $("#commands-save-state");
const channelLockStateElement = $("#channel-lock-state");
const commandInputs = {
  channel: $("#command-channel"), url: $("#command-url"), show: $("#command-show"),
  status: $("#command-status"), test: $("#command-test"), regenerate: $("#command-regenerate"), clear: $("#command-clear"), nuke: $("#command-nuke"), lock: $("#command-lock"),
  changelog: $("#command-changelog"),
};
const customCommandForm = $("#custom-command-form");
const customCommandListElement = $("#custom-command-list");
const customCommandsEmptyElement = $("#custom-commands-empty");
const customCommandCountElement = $("#custom-command-count");
const customCommandsSaveStateElement = $("#custom-commands-save-state");
const customCommandEditorStateElement = $("#custom-command-editor-state");
const customCommandPreviewElement = $("#custom-command-preview");
const customCommandNameElement = $("#custom-command-name");
const customCommandDescriptionElement = $("#custom-command-description");
const customCommandActionElement = $("#custom-command-action");
const customCommandEnabledElement = $("#custom-command-enabled");
const customActionFieldsElement = $("#custom-action-fields");
const customCommandAdminOnlyElement = $("#custom-command-admin-only");
const customCommandUsersElement = $("#custom-command-users");
const customCommandRolesElement = $("#custom-command-roles");
const customCommandChannelsElement = $("#custom-command-channels");
const customRequiredPermissionsElement = $("#custom-required-permissions");
const customPermissionInputs = $$('input[name="custom-permission"]');
const addCustomCommandButton = $("#add-custom-command");
const cancelCustomCommandButton = $("#cancel-custom-command");
const syncCustomCommandsButton = $("#sync-custom-commands");
const defaultRelayCommandNames = new Set([
  "channel", "url", "show", "status", "test", "regenerate", "clear", "nuke", "lock", "changelog",
]);
let customCommands = [];
let customCommandsDirty = false;
let editingCustomCommandIndex = null;
const clientIdElement = $("#client-id");
const tokenElement = $("#discord-token");
const youtubeApiKeyElement = $("#youtube-api-key");
const youtubeKeyStatusElement = $("#youtube-key-status");
const musicSaveStateElement = $("#music-save-state");
const credentialStateElement = $("#credential-state");
const botOnlineStatusElement = $("#bot-online-status");
const botActivityTypeElement = $("#bot-activity-type");
const botActivityTextElement = $("#bot-activity-text");
const botPresenceSaveStateElement = $("#bot-presence-save-state");
const inviteRowElement = $("#invite-row");
const inviteUrlElement = $("#invite-url");
const openInviteButton = $("#open-invite");
const channelElement = $("#channel");
const refreshChannelsButton = $("#refresh-channels");
const ttsChannelElement = $("#tts-channel");
const mediaCleanupEnabledElement = $("#media-cleanup-enabled");
const mediaWelcomeMessageElement = $("#media-welcome-message");
const ttsCleanupEnabledElement = $("#tts-cleanup-enabled");
const ttsWelcomeMessageElement = $("#tts-welcome-message");
const musicChannelElement = $("#music-channel");
const musicWelcomeElement = $("#music-welcome-message");
const musicCleanupEnabledElement = $("#music-cleanup-enabled");
const honeypotChannelElement = $("#honeypot-channel");
const honeypotActionElement = $("#honeypot-action");
const durationElement = $("#duration");
const gifDurationElement = $("#gif-duration");
const stickerDurationElement = $("#sticker-duration");
const portElement = $("#port");
const mediaVolumeElement = $("#media-volume");
const mediaVolumeValueElement = $("#media-volume-value");
const widgetSoundEnabledElement = $("#widget-sound-enabled");
const ttsCharacterLimitElement = $("#tts-character-limit");
const ttsQueueLimitElement = $("#tts-queue-limit");
const notificationDurationElement = $("#notification-duration");
const ttsNotificationsObsElement = $("#tts-notifications-obs");
const showAuthorElement = $("#show-author");
const showMediaTextObsElement = $("#show-media-text-obs");
const showMediaTextWidgetElement = $("#show-media-text-widget");
const moderationEnabledElement = $("#moderation-enabled");
const moderationAllowImagesElement = $("#moderation-allow-images");
const moderationAllowVideosElement = $("#moderation-allow-videos");
const moderationAllowAudioElement = $("#moderation-allow-audio");
const privacyScanEnabledElement = $("#privacy-scan-enabled");
const privacyProtectionLevelElement = $("#privacy-protection-level");
const privacyBlockThresholdElement = $("#privacy-block-threshold");
const privacyReviewIntermediateElement = $("#privacy-review-intermediate");
const privacyAutoDeleteBlockedMessagesElement = $("#privacy-auto-delete-blocked-messages");
const privacyCategoryElements = $$('input[name="privacy-category"]');
const privacyCustomPatternsElement = $("#privacy-custom-patterns");
const privacyAllowlistElement = $("#privacy-allowlist");
const privacyConceptsElement = $("#privacy-concepts");
const privacyExemptRoleIdsElement = $("#privacy-exempt-role-ids");
const moderationSaveStateElement = $("#moderation-save-state");
const moderationCountElement = $("#moderation-count");
const moderationListElement = $("#moderation-list");
const moderationEmptyElement = $("#moderation-empty");
const moderationItemTemplate = $("#moderation-item-template");
const clearPendingMediaButton = $("#clear-pending-media");
const saveStateElement = $("#save-state");
const mediaSaveStateElement = $("#media-save-state");
const messagesSaveStateElement = $("#messages-save-state");
const overlayUrlElement = $("#overlay-url");
const copyUrlButton = $("#copy-url");
const audioUrlElement = $("#audio-url");
const copyAudioUrlButton = $("#copy-audio-url");
const youtubeUrlElement = $("#youtube-url");
const copyYoutubeUrlButton = $("#copy-youtube-url");
const musicWidgetWidthElement = $("#music-widget-width");
const musicWidgetHeightElement = $("#music-widget-height");
const saveMusicOverlayButton = $("#save-music-overlay");
const musicOverlaySaveStateElement = $("#music-overlay-save-state");
if (musicWidgetHeightElement) musicWidgetHeightElement.min = "90";
const ttsUrlElement = $("#tts-url");
const copyTtsUrlButton = $("#copy-tts-url");
const notificationUrlElement = $("#notification-url");
const copyNotificationUrlButton = $("#copy-notification-url");
const stickerUrlElement = $("#sticker-url");
const copyStickerUrlButton = $("#copy-sticker-url");
const outputReadinessCards = new Map(
  $$('[data-output-card]').map((element) => [element.dataset.outputCard, element]),
);
const outputStateElements = new Map(
  $$('[data-output-state]').map((element) => [element.dataset.outputState, element]),
);
const outputLastConnectedElements = new Map(
  $$('[data-output-last-connected]').map(
    (element) => [element.dataset.outputLastConnected, element],
  ),
);
const outputTestButtons = new Map(
  $$('[data-test-output]').map((element) => [element.dataset.testOutput, element]),
);
const regenerateSecretButton = $("#regenerate-secret");
const widgetStateElement = $("#widget-state");
const toggleWidgetButton = $("#toggle-widget");
const lockWidgetButton = $("#lock-widget");
const notificationWidgetStateElement = $("#notification-widget-state");
const notificationWidgetEnabledElement = $("#notification-widget-enabled");
const lockNotificationWidgetButton = $("#lock-notification-widget");
const notificationSoundEnabledElement = $("#notification-sound-enabled");
const notificationSoundObsElement = $("#notification-sound-obs");
const pickNotificationSoundButton = $("#pick-notification-sound");
const clearNotificationSoundButton = $("#clear-notification-sound");
const notificationSoundStateElement = $("#notification-sound-state");
const previewElement = $("#preview");
const outputGeometryGridElement = $("#output-geometry-grid");
const interfaceLanguageElement = $("#interface-language");
const interfaceLanguageButton = $("#interface-language-button");
const interfaceLanguageOptionsElement = $("#interface-language-options");
const interfaceLanguageLabelElement = $("#interface-language-label");
const interfaceLanguageFlagElement = $("#interface-language-flag");
const sidebarLanguagePickerElement = $("#sidebar-language-picker");
const sidebarLanguageOptionsElement = $("#sidebar-language-options");
const interfaceThemeElement = $("#interface-theme");
const interfaceFontElement = $("#interface-font");
const sidebarLayoutElement = $("#sidebar-layout");
const sidebarElement = $(".sidebar");
const designPickerElement = $("#design-picker");
const designPickerSelectedElement = $("#design-picker-selected");
const designInputs = $$("input[name='interface-design']");
const accentInputs = [$("#accent-r"), $("#accent-g"), $("#accent-b")];
const accentPickerElement = $("#accent-picker");
const fontScaleElement = $("#font-scale");
const fontScaleValueElement = $("#font-scale-value");
const personalizationStateElement = $("#personalization-state");
const resetPersonalizationButton = $("#reset-personalization");
const historyListElement = $("#history-list");
const historyEmptyElement = $("#history-empty");
const changelogReleasesElement = $("#changelog-releases");
const changelogEmptyElement = $("#changelog-empty");
const historyItemTemplate = $("#history-item-template");
const clearOverlayButton = $("#clear-overlay");
const skipMediaButton = $("#skip-media");
const skipShortcutKeyElement = $("#skip-shortcut-key");
const skipShortcutCaptureButton = $("#skip-shortcut-capture");
const skipShortcutValueElement = $("#skip-shortcut-value");
const languageToggleButton = $("#language-toggle");
const languageValueElement = $("#language-value");
const languageFlagElement = $("#language-flag");
const themeToggleButton = $("#theme-toggle");
const themeValueElement = $("#theme-value");
const pageTitleElement = $("#page-title");
const pageKickerElement = $("#page-kicker");
const navigationBackButton = $("#navigation-back");
const navigationForwardButton = $("#navigation-forward");
const settingsSearchControl = $("#settings-search-control");
const settingsSearchElement = $("#settings-search");
const settingsSearchClearButton = $("#settings-search-clear");
const settingsSearchResultsElement = $("#settings-search-results");
const nowPlayingElement = $("#now-playing");
const nowPlayingArtworkElement = $("#now-playing-artwork");
const nowPlayingTitleElement = $("#now-playing-title");
const nowPlayingArtistElement = $("#now-playing-artist");
const previousAudioButton = $("#previous-audio");
const toggleAudioButton = $("#toggle-audio");
const skipAudioButton = $("#skip-audio");
const pauseAudioIcon = $("#pause-audio-icon");
const playAudioIcon = $("#play-audio-icon");
const updateControlElement = $("#update-control");
const updateCheckButton = $("#update-check");
const updateAvailableDot = $("#update-available-dot");
const updateMenuElement = $("#update-menu");
const updateMenuCloseButton = $("#update-menu-close");
const updateStatusElement = $("#update-status");
const installUpdateButton = $("#install-update");

const history = [];
let bootstrap;
let socket;
let reconnectTimer;
let reconnectDelayMs = 1000;
let statusTimer;
let mediaCaptionSaveGeneration = 0;
let isUnloading = false;
let shortcutCaptureActive = false;
let currentPage = "overview";
const navigationHistory = ["overview"];
let navigationHistoryIndex = 0;
let settingsSearchIndex = [];
let settingsSearchHighlightTimer;
const languageOptions = [
  { locale: "en-US", language: "en", label: "English (US)", short: "EN-US", flag: "us" },
  { locale: "en-GB", language: "en", label: "English (UK)", short: "EN-UK", flag: "gb" },
  { locale: "en-IN", language: "en", label: "English (India)", short: "EN-IN", flag: "in" },
  { locale: "fr-FR", language: "fr", label: "Français", short: "FR", flag: "fr" },
  { locale: "de-DE", language: "de", label: "Deutsch", short: "DE", flag: "de" },
  { locale: "es-ES", language: "es", label: "Español", short: "ES", flag: "es" },
  { locale: "es-419", language: "es", label: "Español (Latinoamérica)", short: "ES-LATAM", flag: "mx" },
  { locale: "ru-RU", language: "ru", label: "Русский", short: "RU", flag: "ru" },
  { locale: "zh-CN", language: "zh", label: "简体中文", short: "ZH", flag: "cn" },
  { locale: "ko-KR", language: "ko", label: "한국어", short: "KO", flag: "kr" },
  { locale: "ja-JP", language: "ja", label: "日本語", short: "JA", flag: "jp" },
  { locale: "id-ID", language: "id", label: "Bahasa Indonesia", short: "ID", flag: "id" },
];
const languageOptionByLocale = new Map(languageOptions.map((option) => [option.locale, option]));
const defaultLocaleByLanguage = {
  en: "en-US", fr: "fr-FR", es: "es-ES", de: "de-DE", ru: "ru-RU",
  zh: "zh-CN", ko: "ko-KR", ja: "ja-JP", id: "id-ID",
};
const supportedDesigns = ["openai", "anthropic", "neo-brutalism", "gridline", "lumen"];
const supportedSidebarLayouts = ["fixed", "compact", "dynamic"];
const supportedInterfaceFonts = [
  "design", "bricolage", "dm-sans", "figtree", "inter",
  "jetbrains-mono", "manrope", "poppins", "space-grotesk",
];
const storedLanguage = localStorage.getItem("relay-language") || "en";
let locale = localStorage.getItem("relay-locale") || defaultLocaleByLanguage[storedLanguage] || "en-US";
if (!languageOptionByLocale.has(locale)) locale = "en-US";
let language = languageOptionByLocale.get(locale).language;
let design = localStorage.getItem("relay-design") || "openai";
if (!supportedDesigns.includes(design)) design = "openai";
let interfaceFont = localStorage.getItem("relay-interface-font") || "design";
if (!supportedInterfaceFonts.includes(interfaceFont)) interfaceFont = "design";
let sidebarLayout = localStorage.getItem("relay-sidebar-layout") || "fixed";
if (!supportedSidebarLayouts.includes(sidebarLayout)) sidebarLayout = "fixed";
let sidebarExpanded = false;
let theme = localStorage.getItem("relay-theme")
  || (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light");
let accentRgb = parseStoredAccent();
let fontScale = clamp(Number(localStorage.getItem("relay-font-scale")) || 100, 80, 140);
let personalizationTimer;
let privacyFilterSaveTimer;
let privacyFilterSaveGeneration = 0;
let privacyFilterDraft = "";
const audioPlaybackTargets = new Map();
let currentAudioPlayback;
let nowPlayingArtworkRequest = 0;
const artworkCache = new Map();
let currentAppVersion = "1.3.6";
let bundledChangelogMarkdown = "";
let latestUpdate;
let updateUiState = { kind: "idle" };
let titlebarThemeFrame;

function t(key) {
  return regionalTranslations[locale]?.[key] || translations[language][key] || translations.en[key] || key;
}

function applyTranslations(root = document) {
  for (const element of $$("[data-i18n]", root)) {
    element.textContent = t(element.dataset.i18n);
  }
  for (const element of $$("[data-i18n-placeholder]", root)) {
    element.placeholder = t(element.dataset.i18nPlaceholder);
  }
}

function formatTranslation(key, values = {}) {
  return Object.entries(values).reduce(
    (message, [name, value]) => message.replaceAll(`{${name}}`, value),
    t(key),
  );
}

function setAppVersion(version) {
  const normalized = String(version).replace(/^v/, "");
  if (!/^\d+\.\d+\.\d+$/.test(normalized)) return;
  currentAppVersion = normalized;
  for (const element of $$("[data-app-version]")) element.textContent = normalized;
  updateCheckButton.setAttribute("aria-label", `${t("checkUpdates")}. Relay v${normalized}`);
  renderChangelog();
}

function parseChangelogReleases(markdown) {
  const releases = [];
  let current;
  const flush = () => {
    if (!current) return;
    const body = current.lines.join("\n").trim();
    if (body) {
      releases.push({ version: current.version, date: current.date, body });
    }
    current = undefined;
  };
  for (const line of String(markdown).split(/\r?\n/)) {
    if (line.startsWith("## [")) {
      flush();
      const match = line.match(/^## \[([^\]]+)\](?:\s*-\s*(.+))?$/);
      if (!match || match[1].trim().toLowerCase() === "unreleased") continue;
      current = {
        version: match[1].trim(),
        date: match[2]?.trim() || null,
        lines: [],
      };
      continue;
    }
    if (line.startsWith("[") && line.includes("]: http")) continue;
    current?.lines.push(line);
  }
  flush();
  return releases;
}

function changelogBodyForLanguage(body, languageCode) {
  const buckets = { default: [] };
  let current = "default";
  for (const line of String(body).split(/\r?\n/)) {
    const heading = line.match(/^###\s+(.+)$/);
    if (heading) {
      current = heading[1].trim().toLowerCase();
      buckets[current] ??= [];
      continue;
    }
    buckets[current].push(line);
  }
  const aliases = {
    en: ["english"],
    fr: ["français", "francais"],
    es: ["español", "espanol", "spanish"],
    de: ["deutsch", "german"],
    ru: ["русский", "russian"],
    zh: ["简体中文", "chinese"],
    ko: ["한국어", "korean"],
    ja: ["日本語", "japanese"],
    id: ["bahasa indonesia", "indonesian"],
  };
  const preferred = [...(aliases[languageCode] || aliases.en)];
  if (languageCode !== "en") preferred.push("english");
  for (const key of preferred) {
    const text = (buckets[key] || []).join("\n").trim();
    if (text) return text;
  }
  return String(body).trim();
}

function appendInlineChangelogText(parent, text) {
  const parts = String(text).split(/(\*\*[^*]+\*\*)/g);
  for (const part of parts) {
    if (part.startsWith("**") && part.endsWith("**") && part.length > 4) {
      const strong = document.createElement("strong");
      strong.textContent = part.slice(2, -2);
      parent.append(strong);
      continue;
    }
    parent.append(document.createTextNode(part));
  }
}

function appendChangelogMarkdown(parent, markdown) {
  let list;
  const closeList = () => {
    list = undefined;
  };
  for (const line of String(markdown).split(/\r?\n/)) {
    if (line.startsWith("#### ")) {
      closeList();
      const heading = document.createElement("h4");
      heading.textContent = line.slice(5).trim();
      parent.append(heading);
      continue;
    }
    if (line.startsWith("### ")) {
      closeList();
      const heading = document.createElement("h3");
      heading.textContent = line.slice(4).trim();
      parent.append(heading);
      continue;
    }
    if (line.startsWith("- ")) {
      if (!list) {
        list = document.createElement("ul");
        parent.append(list);
      }
      const item = document.createElement("li");
      appendInlineChangelogText(item, line.slice(2));
      list.append(item);
      continue;
    }
    if (!line.trim()) {
      closeList();
      continue;
    }
    closeList();
    const paragraph = document.createElement("p");
    appendInlineChangelogText(paragraph, line);
    parent.append(paragraph);
  }
}

function renderChangelog() {
  if (!changelogReleasesElement || !changelogEmptyElement) return;
  const releases = parseChangelogReleases(bundledChangelogMarkdown);
  changelogReleasesElement.replaceChildren();
  changelogEmptyElement.hidden = releases.length > 0;
  if (!releases.length) return;

  const currentIndex = Math.max(0, releases.findIndex((release) => release.version === currentAppVersion));
  for (const [index, release] of releases.entries()) {
    const details = document.createElement("details");
    details.className = "changelog-release";
    details.open = index === currentIndex;
    const summary = document.createElement("summary");
    const title = document.createElement("strong");
    title.className = "changelog-release__version";
    const badge = index === currentIndex ? `${t("changelogLatest")} · ` : "";
    title.textContent = `${badge}Relay ${release.version}`;
    const date = document.createElement("span");
    date.className = "changelog-release__date";
    date.textContent = release.date
      ? formatTranslation(index === currentIndex ? "changelogUpdated" : "changelogReleased", { date: release.date })
      : t("changelogPrevious");
    summary.append(title, date);
    const body = document.createElement("div");
    body.className = "changelog-release__body";
    appendChangelogMarkdown(body, changelogBodyForLanguage(release.body, language));
    details.append(summary, body);
    changelogReleasesElement.append(details);
  }
}

function renderUpdateStatus() {
  const version = updateUiState.version || currentAppVersion;
  const messages = {
    idle: () => t("checkUpdatesPrompt"),
    checking: () => t("checkingUpdates"),
    available: () => formatTranslation("updateAvailable", { version }),
    current: () => formatTranslation("upToDate", { version: currentAppVersion }),
    installing: () => formatTranslation("downloadingUpdate", { version }),
    error: () => `${t(updateUiState.errorKey)} ${updateUiState.error}`,
  };
  updateStatusElement.textContent = (messages[updateUiState.kind] || messages.idle)();
  const busy = updateUiState.kind === "checking" || updateUiState.kind === "installing";
  updateCheckButton.classList.toggle("is-checking", busy);
  updateCheckButton.disabled = busy;
  installUpdateButton.disabled = busy;
  installUpdateButton.hidden = !latestUpdate?.updateAvailable;
  updateAvailableDot.hidden = !latestUpdate?.updateAvailable;
}

function setUpdateMenuOpen(open) {
  updateMenuElement.hidden = !open;
  updateCheckButton.setAttribute("aria-expanded", String(open));
  if (open) window.requestAnimationFrame(() => updateMenuCloseButton.focus());
}

function activeLanguageOption() {
  return languageOptionByLocale.get(locale) || languageOptions[0];
}

function setLanguageMenuOpen(open) {
  interfaceLanguageOptionsElement.hidden = !open;
  interfaceLanguageButton.setAttribute("aria-expanded", String(open));
  if (open) setSidebarLanguageMenuOpen(false);
}

function setSidebarLanguageMenuOpen(open) {
  sidebarLanguageOptionsElement.hidden = !open;
  languageToggleButton.setAttribute("aria-expanded", String(open));
  if (open) setLanguageMenuOpen(false);
}

function renderLanguagePicker() {
  const selected = activeLanguageOption();
  const flagUrl = `./assets/flags/${selected.flag}.svg`;
  interfaceLanguageLabelElement.textContent = selected.label;
  interfaceLanguageFlagElement.src = flagUrl;
  languageFlagElement.src = flagUrl;
  languageValueElement.textContent = selected.short;
  interfaceLanguageButton.setAttribute("aria-label", `${t("language")}: ${selected.label}`);
  languageToggleButton.setAttribute("aria-label", `${t("language")}: ${selected.label}`);
  languageToggleButton.title = selected.label;
  for (const option of $$("[data-locale]", interfaceLanguageOptionsElement)) {
    const isSelected = option.dataset.locale === selected.locale;
    option.setAttribute("aria-selected", String(isSelected));
  }
  sidebarLanguageOptionsElement.replaceChildren(...languageOptions.map((option) => {
    const button = document.createElement("button");
    const isSelected = option.locale === selected.locale;
    button.className = "language-picker__option sidebar-language-picker__option";
    button.type = "button";
    button.dataset.locale = option.locale;
    button.setAttribute("role", "option");
    button.setAttribute("aria-selected", String(isSelected));
    button.innerHTML = `<img class="flag-icon" src="./assets/flags/${option.flag}.svg" alt=""><span>${option.label}</span><i aria-hidden="true">✓</i>`;
    return button;
  }));
}

function selectInterfaceLanguage(nextLocale, focusTarget) {
  const option = languageOptionByLocale.get(nextLocale);
  if (!option) return;
  locale = option.locale;
  language = option.language;
  setLanguageMenuOpen(false);
  setSidebarLanguageMenuOpen(false);
  applyLanguage();
  applyTheme();
  applyPersonalization();
  focusTarget.focus();
}

function applyLanguage() {
  window.dispatchEvent(new Event("relay-language-change"));
  document.documentElement.lang = locale;
  localStorage.setItem("relay-language", language);
  localStorage.setItem("relay-locale", locale);
  applyTranslations();
  renderLanguagePicker();
  updateCheckButton.setAttribute("aria-label", `${t("checkUpdates")}. Relay v${currentAppVersion}`);
  updateMenuCloseButton.setAttribute("aria-label", t("closeUpdateMenu"));
  navigationBackButton.title = t("navigationBack");
  navigationBackButton.setAttribute("aria-label", t("navigationBack"));
  navigationForwardButton.title = t("navigationForward");
  navigationForwardButton.setAttribute("aria-label", t("navigationForward"));
  settingsSearchElement.setAttribute("aria-label", t("searchLabel"));
  settingsSearchClearButton.title = t("clearSearch");
  settingsSearchClearButton.setAttribute("aria-label", t("clearSearch"));
  for (const input of [overlayUrlElement, audioUrlElement, youtubeUrlElement, notificationUrlElement, ttsUrlElement, stickerUrlElement]) {
    if (input) input.setAttribute("aria-label", input.closest("label")?.querySelector("[data-i18n]")?.textContent || input.getAttribute("aria-label"));
  }
  buildSettingsSearchIndex();
  renderSettingsSearchResults();
  renderUpdateStatus();
  updatePageHeading();
  renderNowPlaying();
  renderChangelog();
  renderCustomCommands();
  if (!customCommandForm.hidden) {
    let action;
    try {
      action = readCustomAction();
    } catch {
      action = defaultCustomAction(customCommandActionElement.value);
    }
    renderCustomActionFields(action);
  }
  if (bootstrap) {
    setBotStatus(bootstrap.bot);
    setServerStatus(bootstrap.server);
    setCredentials(bootstrap.credentials);
    setWidgetState(bootstrap.widget);
    setNotificationWidgetState(bootstrap.notificationWidget);
    populateChannels(channelElement, bootstrap.channels, channelElement.value, t("selectChannel"));
    populateChannels(ttsChannelElement, bootstrap.channels, ttsChannelElement.value, t("ttsDisabled"));
    populateChannels(musicChannelElement, bootstrap.channels, musicChannelElement.value, t("musicDisabled"));
    populateChannels(honeypotChannelElement, bootstrap.channels, honeypotChannelElement.value, t("honeypotDisabled"));
    renderHistory();
    renderModeration();
  }
}

function resolveTitlebarColor(property) {
  const probe = document.createElement("span");
  probe.style.color = `var(${property})`;
  probe.hidden = true;
  document.body.append(probe);
  const resolved = window.getComputedStyle(probe).color;
  probe.remove();

  const canvas = document.createElement("canvas");
  canvas.width = 1;
  canvas.height = 1;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  context.fillStyle = resolved;
  context.fillRect(0, 0, 1, 1);
  return [...context.getImageData(0, 0, 1, 1).data.slice(0, 3)];
}

function syncWindowTheme() {
  window.cancelAnimationFrame(titlebarThemeFrame);
  titlebarThemeFrame = window.requestAnimationFrame(() => {
    const caption = resolveTitlebarColor("--titlebar-bg");
    const text = resolveTitlebarColor("--titlebar-ink");
    const border = resolveTitlebarColor("--titlebar-border");
    invoke("set_window_theme", { theme, caption, text, border }).catch(() => {});
  });
}

function applyTheme() {
  document.documentElement.dataset.theme = theme;
  localStorage.setItem("relay-theme", theme);
  themeValueElement.textContent = t(theme);
  syncWindowTheme();
}

function applyDesign() {
  document.documentElement.dataset.design = design;
  localStorage.setItem("relay-design", design);
  for (const input of designInputs) input.checked = input.value === design;
  const selectedDesign = designInputs.find((input) => input.value === design);
  designPickerSelectedElement.textContent = selectedDesign
    ?.closest(".design-choice")
    ?.querySelector(".design-choice__copy strong")
    ?.textContent || design;
  for (const element of $$('[data-relay-base-font-size]')) {
    element.style.removeProperty("font-size");
    delete element.dataset.relayBaseFontSize;
  }
  syncWindowTheme();
}

function applySidebarLayout() {
  const appliedLayout = sidebarLayout === "dynamic"
    ? (sidebarExpanded ? "fixed" : "compact")
    : sidebarLayout;
  document.documentElement.dataset.sidebarLayout = appliedLayout;
  document.documentElement.dataset.sidebarBehavior = sidebarLayout;
  localStorage.setItem("relay-sidebar-layout", sidebarLayout);
  sidebarLayoutElement.value = sidebarLayout;
}

function setDynamicSidebarExpanded(expanded) {
  const nextExpanded = sidebarLayout === "dynamic" && expanded;
  if (sidebarExpanded === nextExpanded) return;
  sidebarExpanded = nextExpanded;
  applySidebarLayout();
}

function applyInterfaceFont() {
  document.documentElement.dataset.interfaceFont = interfaceFont;
  localStorage.setItem("relay-interface-font", interfaceFont);
  interfaceFontElement.value = interfaceFont;
}

function activeAudioPlayback() {
  const states = [...audioPlaybackTargets.values()];
  return states.find(({ status }) => status === "playing")
    || states.find(({ status }) => status === "paused");
}

function releaseNowPlayingArtwork() {
  nowPlayingArtworkRequest += 1;
}

function loadArtwork(artworkId) {
  if (!artworkCache.has(artworkId)) {
    const pending = invoke("get_media_artwork", { artworkId }).then((bytes) => new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result);
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(new Blob([Array.isArray(bytes) ? new Uint8Array(bytes) : bytes]));
    }));
    artworkCache.set(artworkId, pending);
    if (artworkCache.size > 50) artworkCache.delete(artworkCache.keys().next().value);
    pending.catch(() => {
      if (artworkCache.get(artworkId) === pending) artworkCache.delete(artworkId);
    });
  }
  return artworkCache.get(artworkId);
}

async function loadNowPlayingArtwork(media) {
  releaseNowPlayingArtwork();
  const request = nowPlayingArtworkRequest;
  nowPlayingArtworkElement.onerror = () => {
    nowPlayingArtworkElement.onerror = null;
    nowPlayingArtworkElement.src = "./assets/relay-radar.png";
  };
  nowPlayingArtworkElement.src = media.author?.displayAvatarUrl || "./assets/relay-radar.png";
  if (!media.artworkId) return;
  try {
    const source = await loadArtwork(media.artworkId);
    if (request !== nowPlayingArtworkRequest || currentAudioPlayback?.media.artworkId !== media.artworkId) return;
    nowPlayingArtworkElement.src = source;
  } catch {}
}

function renderNowPlaying() {
  const playback = activeAudioPlayback();
  if (!playback) {
    currentAudioPlayback = undefined;
    nowPlayingElement.hidden = true;
    releaseNowPlayingArtwork();
    return;
  }
  const mediaChanged = currentAudioPlayback?.media.url !== playback.media.url
    || currentAudioPlayback?.media.artworkId !== playback.media.artworkId;
  currentAudioPlayback = playback;
  nowPlayingElement.hidden = false;
  nowPlayingTitleElement.textContent = playback.media.title || playback.media.filename || "Discord audio";
  nowPlayingArtistElement.textContent = playback.media.artist || playback.media.author?.username || "Discord";
  const paused = playback.status === "paused";
  pauseAudioIcon.hidden = paused;
  playAudioIcon.hidden = !paused;
  const toggleLabel = t(paused ? "resumeAudio" : "pauseAudio");
  toggleAudioButton.title = toggleLabel;
  toggleAudioButton.setAttribute("aria-label", toggleLabel);
  previousAudioButton.title = t("previousAudio");
  previousAudioButton.setAttribute("aria-label", t("previousAudio"));
  skipAudioButton.title = t("skipAudio");
  skipAudioButton.setAttribute("aria-label", t("skipAudio"));
  const audioHistory = history.filter(({ kind }) => kind === "audio");
  const historyIndex = audioHistory.findIndex(({ url }) => url === playback.media.url);
  previousAudioButton.disabled = historyIndex < 0 || historyIndex >= audioHistory.length - 1;
  if (mediaChanged) loadNowPlayingArtwork(playback.media);
}

function updateAudioPlayback(playback) {
  const existing = audioPlaybackTargets.get(playback.target);
  if (playback.status === "idle") {
    if (!existing || existing.media.url === playback.media.url) audioPlaybackTargets.delete(playback.target);
  } else {
    audioPlaybackTargets.delete(playback.target);
    audioPlaybackTargets.set(playback.target, playback);
  }
  renderNowPlaying();
}

async function controlCurrentAudio(action) {
  if (!currentAudioPlayback) return;
  for (const button of [previousAudioButton, toggleAudioButton, skipAudioButton]) button.disabled = true;
  try {
    await invoke("control_audio", { action, currentUrl: currentAudioPlayback.media.url });
  } catch (error) {
    setSaveState(mediaSaveStateElement, "error", String(error));
  } finally {
    renderNowPlaying();
  }
}

function clamp(value, minimum, maximum) {
  return Math.min(maximum, Math.max(minimum, Number(value) || 0));
}

const outputGeometryTargets = {
  mediaObs: { configKey: "mediaObsGeometry", titleKey: "mediaObsOutput", previewKey: "overlayUrl" },
  mediaWidget: { configKey: "mediaWidgetGeometry", titleKey: "mediaWidgetOutput", previewKey: "overlayUrl", widget: "media" },
  notificationObs: { configKey: "notificationObsGeometry", titleKey: "notificationObsOutput", previewKey: "notificationUrl" },
  notificationWidget: { configKey: "notificationWidgetGeometry", titleKey: "notificationWidgetOutput", previewKey: "notificationUrl", widget: "notification" },
};
const outputGeometryTimers = new Map();

function geometryControl(field, labelKey, minimum, maximum) {
  return `
    <label class="geometry-control">
      <span data-i18n="${labelKey}"></span>
      <input data-geometry-field="${field}" data-geometry-kind="range" type="range" min="${minimum}" max="${maximum}" step="1">
      <input data-geometry-field="${field}" data-geometry-kind="number" type="number" min="${minimum}" max="${maximum}" step="1">
    </label>`;
}

function initializePresets(card, target) {
  initializePresetControls(card, target, { payload: outputGeometryPayload, persist: persistOutputGeometry,
    state: (kind, message) => setSaveState(card.querySelector("[data-geometry-state]"), kind, message), storage: localStorage });
}

function initializeOutputGeometryControls() {
  outputGeometryGridElement.innerHTML = Object.entries(outputGeometryTargets).map(([target, metadata]) => {
    const sizeControls = metadata.widget ? `
      <div class="geometry-size-row">
        <label><span data-i18n="outputWidth"></span><input data-size-field="width" type="number" min="160" max="16384" step="1"></label>
        <label><span data-i18n="outputHeight"></span><input data-size-field="height" type="number" min="90" max="16384" step="1"></label>
      </div>` : "";
    const ratioControl = metadata.widget === "media" ? `
      <label class="inline-switch geometry-ratio">
        <span data-i18n="keepAspectRatio"></span>
        <span class="switch"><input data-keep-aspect-ratio type="checkbox"><span class="switch__track" aria-hidden="true"></span></span>
      </label>` : "";
    return `
      <details class="output-geometry-card geometry-disclosure" data-geometry-target="${target}">
        <summary><h4 data-i18n="${metadata.titleKey}"></h4><span class="save-state" data-geometry-state role="status"></span></summary>
        <div class="geometry-preview">
          <span data-i18n="geometryPreview"></span>
          <iframe data-geometry-preview title="Relay output preview"></iframe>
        </div>
        ${sizeControls}
        ${ratioControl}
<div class="preset-controls"><label class="field"><span data-i18n="presets"></span><select data-preset></select></label><label class="field"><span data-i18n="presetName"></span><input data-preset-name maxlength="64"></label><div class="section-actions"><button type="button" class="button button--quiet" data-save-preset data-i18n="savePreset"></button><button type="button" class="button button--quiet" data-apply-preset data-i18n="applyPreset"></button><button type="button" class="button button--quiet" data-delete-preset data-i18n="deletePreset"></button></div></div>
        <div class="geometry-controls">
          <label class="field"><span data-i18n="outputAnchor"></span><select data-output-anchor><option value="legacy" data-i18n="anchorLegacy"></option><option value="topLeft" data-i18n="anchorTopLeft"></option><option value="topCenter" data-i18n="anchorTopCenter"></option><option value="topRight" data-i18n="anchorTopRight"></option><option value="center" data-i18n="anchorCenter"></option><option value="bottomLeft" data-i18n="anchorBottomLeft"></option><option value="bottomCenter" data-i18n="anchorBottomCenter"></option><option value="bottomRight" data-i18n="anchorBottomRight"></option></select></label>
${geometryControl("marginX", "marginX", 0, 200)}
${geometryControl("marginY", "marginY", 0, 200)}
${geometryControl("contentScale", "contentScale", 50, 200)}
          ${geometryControl("cropTop", "cropTop", 0, 40)}
          ${geometryControl("cropRight", "cropRight", 0, 40)}
          ${geometryControl("cropBottom", "cropBottom", 0, 40)}
          ${geometryControl("cropLeft", "cropLeft", 0, 40)}
        </div>
        <button class="button button--quiet" data-reset-geometry type="button" data-i18n="resetOutput"></button>
      </details>`;
  }).join("");

  for (const card of $$("[data-geometry-target]", outputGeometryGridElement)) {
    const target = card.dataset.geometryTarget;
    initializePresets(card, target);
    card.querySelector("[data-output-anchor]").addEventListener("change", () => queueOutputGeometrySave(target));
    for (const input of $$('[data-geometry-field]', card)) {
      input.addEventListener("input", () => {
        const peer = card.querySelector(
          `[data-geometry-field="${input.dataset.geometryField}"][data-geometry-kind="${input.dataset.geometryKind === "range" ? "number" : "range"}"]`,
        );
        peer.value = input.value;
        queueOutputGeometrySave(target);
      });
    }
    for (const input of $$('[data-size-field], [data-keep-aspect-ratio]', card)) {
      input.addEventListener("input", () => queueOutputGeometrySave(target));
    }
    card.querySelector("[data-reset-geometry]").addEventListener("click", () => {
      setOutputGeometryDefaults(target);
      persistOutputGeometry(target);
    });
  }
}

function applyMusicOverlaySize(config, force = false) {
  if (!musicWidgetWidthElement || !musicWidgetHeightElement) return;
  if (
    !force
    && (document.activeElement === musicWidgetWidthElement
      || document.activeElement === musicWidgetHeightElement)
  ) {
    return;
  }
  musicWidgetWidthElement.value = String(Math.round(config.widgetWidth ?? 640));
  musicWidgetHeightElement.value = String(Math.round(config.widgetHeight ?? 360));
}

function applyOutputGeometryTarget(config, target, force = false) {
  const card = outputGeometryGridElement.querySelector(`[data-geometry-target="${target}"]`);
  if (!card || (!force && card.contains(document.activeElement))) return;
  const geometry = config[outputGeometryTargets[target].configKey] || {};
  card.querySelector("[data-output-anchor]").value = geometry.anchor || "legacy";
  const values = {
    contentScale: geometry.contentScale ?? 100,
    marginX: geometry.marginX ?? 12,
    marginY: geometry.marginY ?? 16,
    cropTop: geometry.cropTop ?? 0,
    cropRight: geometry.cropRight ?? 0,
    cropBottom: geometry.cropBottom ?? 0,
    cropLeft: geometry.cropLeft ?? 0,
  };
  for (const input of $$('[data-geometry-field]', card)) {
    input.value = String(values[input.dataset.geometryField]);
  }
  if (target === "mediaWidget") {
    card.querySelector('[data-size-field="width"]').value = String(Math.round(config.widgetWidth ?? 640));
    card.querySelector('[data-size-field="height"]').value = String(Math.round(config.widgetHeight ?? 360));
    card.querySelector("[data-keep-aspect-ratio]").checked = config.widgetKeepAspectRatio !== false;
  } else if (target === "notificationWidget") {
    card.querySelector('[data-size-field="width"]').value = String(Math.round(config.notificationWidgetWidth ?? 400));
    card.querySelector('[data-size-field="height"]').value = String(Math.round(config.notificationWidgetHeight ?? 104));
  }
  applyMusicOverlaySize(config, force);
}

function applyOutputGeometryConfig(config, force = false) {
  for (const target of Object.keys(outputGeometryTargets)) {
    applyOutputGeometryTarget(config, target, force);
  }
}

function outputGeometryPayload(target) {
  const card = outputGeometryGridElement.querySelector(`[data-geometry-target="${target}"]`);
  const value = (field, minimum, maximum) => clamp(
    card.querySelector(`[data-geometry-field="${field}"][data-geometry-kind="number"]`).value,
    minimum,
    maximum,
  );
  const payload = {
    target,
    geometry: {
      contentScale: value("contentScale", 50, 200),
      anchor: card.querySelector("[data-output-anchor]").value,
      marginX: value("marginX", 0, 200),
      marginY: value("marginY", 0, 200),
      cropTop: value("cropTop", 0, 40),
      cropRight: value("cropRight", 0, 40),
      cropBottom: value("cropBottom", 0, 40),
      cropLeft: value("cropLeft", 0, 40),
    },
  };
  if (outputGeometryTargets[target].widget) {
    payload.width = clamp(card.querySelector('[data-size-field="width"]').value, 160, 16384);
    payload.height = clamp(card.querySelector('[data-size-field="height"]').value, 90, 16384);
  }
  if (target === "mediaWidget") {
    payload.keepAspectRatio = card.querySelector("[data-keep-aspect-ratio]").checked;
  }
  return payload;
}

function queueOutputGeometrySave(target) {
  outputGeometryGridElement.querySelector(`[data-geometry-target="${target}"] [data-geometry-state]`).textContent = t("saving");
  if (outputGeometryTimers.has(target)) return;
  outputGeometryTimers.set(target, window.setTimeout(() => {
    outputGeometryTimers.delete(target);
    persistOutputGeometry(target);
  }, 80));
}

async function persistOutputGeometry(target) {
  if (outputGeometryTimers.has(target)) {
    window.clearTimeout(outputGeometryTimers.get(target));
    outputGeometryTimers.delete(target);
  }
  const state = outputGeometryGridElement.querySelector(`[data-geometry-target="${target}"] [data-geometry-state]`);
  state.textContent = t("saving");
  try {
    const config = await invoke("set_output_geometry", outputGeometryPayload(target));
    bootstrap.config = config;
    applyOutputGeometryTarget(config, target, true);
    state.textContent = t("geometrySaved");
  } catch (error) {
    state.textContent = String(error);
  }
}

function setOutputGeometryDefaults(target) {
  const card = outputGeometryGridElement.querySelector(`[data-geometry-target="${target}"]`);
  card.querySelector("[data-output-anchor]").value = "legacy";
  for (const input of $$('[data-geometry-field]', card)) {
    input.value = ({ contentScale: "100", marginX: "12", marginY: "16" })[input.dataset.geometryField] || "0";
  }
  if (target === "mediaWidget") {
    card.querySelector('[data-size-field="width"]').value = "640";
    card.querySelector('[data-size-field="height"]').value = "360";
    card.querySelector("[data-keep-aspect-ratio]").checked = true;
  } else if (target === "notificationWidget") {
    card.querySelector('[data-size-field="width"]').value = "400";
    card.querySelector('[data-size-field="height"]').value = "104";
  }
}

function setOutputGeometryPreviewUrls() {
  const port = bootstrap.config.port;
  for (const [target, metadata] of Object.entries(outputGeometryTargets)) {
    const iframe = outputGeometryGridElement.querySelector(`[data-geometry-target="${target}"] [data-geometry-preview]`);
    // Use 127.0.0.1 short pages (not /obs/visual) so panel CSP + preview=1 work without
    // nested iframes or localhost frame-src blocks ("This content is blocked").
    const path = metadata.previewKey === "notificationUrl" ? "/notifications" : "/medias";
    const url = new URL(`http://127.0.0.1:${port}${path}`);
    url.searchParams.set("preview", "1");
    if (metadata.widget === "media") {
      url.searchParams.set("widget", "1");
      url.searchParams.set("locked", "1");
    } else if (metadata.widget === "notification") {
      url.searchParams.set("target", "widget");
      url.searchParams.set("locked", "1");
    }
    if (iframe.src !== url.href) iframe.src = url.href;
  }
}

function parseStoredAccent() {
  try {
    const value = JSON.parse(localStorage.getItem("relay-accent-rgb") || "null");
    if (Array.isArray(value) && value.length === 3) {
      return value.map((channel) => clamp(channel, 0, 255));
    }
  } catch {}
  return [88, 185, 137];
}

function accentInk([red, green, blue]) {
  return (red * 299 + green * 587 + blue * 114) / 1000 > 145 ? "#07110b" : "#ffffff";
}

function rgbToHex(rgb) {
  return `#${rgb.map((channel) => clamp(channel, 0, 255).toString(16).padStart(2, "0")).join("")}`;
}

function hexToRgb(hex) {
  return [1, 3, 5].map((index) => Number.parseInt(hex.slice(index, index + 2), 16));
}

function scaleInterfaceText() {
  const elements = $$('body *:not(svg):not(path)');
  for (const element of elements) element.style.removeProperty("font-size");
  for (const element of elements) {
    if (!element.dataset.relayBaseFontSize) {
      const size = Number.parseFloat(window.getComputedStyle(element).fontSize);
      if (Number.isFinite(size) && size > 0) element.dataset.relayBaseFontSize = String(size);
    }
  }
  for (const element of elements) {
    if (element.dataset.relayBaseFontSize) {
      element.style.fontSize = `${Number(element.dataset.relayBaseFontSize) * fontScale / 100}px`;
    }
  }
}

function syncInterfacePreferences() {
  window.clearTimeout(personalizationTimer);
  personalizationTimer = window.setTimeout(async () => {
    try {
      await invoke("set_interface_preferences", {
        language, theme, accentRgb, fontScale,
      });
      personalizationStateElement.textContent = t("personalizationSaved");
    } catch (error) {
      personalizationStateElement.textContent = String(error);
    }
  }, 120);
}

function applyPersonalization(sync = true) {
  const color = `rgb(${accentRgb.join(" ")})`;
  document.documentElement.style.setProperty("--accent", color);
  document.documentElement.style.setProperty("--accent-ink", accentInk(accentRgb));
  syncWindowTheme();
  localStorage.setItem("relay-accent-rgb", JSON.stringify(accentRgb));
  localStorage.setItem("relay-font-scale", String(fontScale));
  renderLanguagePicker();
  interfaceThemeElement.value = theme;
  accentInputs.forEach((input, index) => { input.value = String(accentRgb[index]); });
  accentPickerElement.value = rgbToHex(accentRgb);
  fontScaleElement.value = String(fontScale);
  fontScaleValueElement.textContent = `${fontScale}%`;
  applyInterfaceFont();
  scaleInterfaceText();
  if (sync) syncInterfacePreferences();
}

function updatePageHeading() {
  const metadata = pageMetadata[currentPage];
  pageTitleElement.textContent = t(metadata.title);
  pageKickerElement.textContent = t(metadata.kicker);
}

function updateNavigationControls() {
  navigationBackButton.disabled = navigationHistoryIndex <= 0;
  navigationForwardButton.disabled = navigationHistoryIndex >= navigationHistory.length - 1;
}

function showPage(page, { recordHistory = true } = {}) {
  if (!pageMetadata[page]) return;
  if (recordHistory && page !== currentPage) {
    navigationHistory.splice(navigationHistoryIndex + 1);
    navigationHistory.push(page);
    navigationHistoryIndex = navigationHistory.length - 1;
  }
  currentPage = page;
  for (const element of $$("[data-page]")) {
    const active = element.dataset.page === page;
    element.hidden = !active;
    element.classList.toggle("is-active", active);
  }
  for (const button of $$("[data-page-target]")) {
    button.classList.toggle("is-active", button.dataset.pageTarget === page);
  }
  if (page === "history") {
    window.requestAnimationFrame(loadHistoryVideoThumbnails);
  }
  updatePageHeading();
  updateNavigationControls();
}

function navigateHistory(offset) {
  const nextIndex = navigationHistoryIndex + offset;
  if (nextIndex < 0 || nextIndex >= navigationHistory.length) return;
  navigationHistoryIndex = nextIndex;
  showPage(navigationHistory[navigationHistoryIndex], { recordHistory: false });
}

function normalizeSettingsSearch(value) {
  return String(value)
    .normalize("NFKD")
    .replace(/\p{Mark}/gu, "")
    .toLocaleLowerCase(locale);
}

function buildSettingsSearchIndex() {
  const seen = new Set();
  settingsSearchIndex = $$('[data-page] [data-i18n]').flatMap((element) => {
    const page = element.closest("[data-page]")?.dataset.page;
    const key = element.dataset.i18n;
    const identity = `${page}:${key}`;
    if (!pageMetadata[page] || seen.has(identity)) return [];
    seen.add(identity);
    const label = t(key);
    const pageLabel = t(pageMetadata[page].title);
    const target = element.closest("label, .setting-row, details, fieldset, .panel-section, .help-step") || element;
    return [{
      label, page, pageLabel, target,
      searchable: normalizeSettingsSearch(`${label} ${pageLabel}`),
    }];
  });
}

function closeSettingsSearch() {
  settingsSearchResultsElement.hidden = true;
}

function openSettingsSearchResult(entry) {
  settingsSearchElement.value = "";
  settingsSearchClearButton.hidden = true;
  closeSettingsSearch();
  showPage(entry.page);
  for (let parent = entry.target.closest("details"); parent; parent = parent.parentElement?.closest("details")) {
    parent.open = true;
  }
  window.requestAnimationFrame(() => {
    entry.target.scrollIntoView({ behavior: "smooth", block: "center" });
    window.clearTimeout(settingsSearchHighlightTimer);
    entry.target.classList.remove("settings-search-target");
    window.requestAnimationFrame(() => entry.target.classList.add("settings-search-target"));
    settingsSearchHighlightTimer = window.setTimeout(
      () => entry.target.classList.remove("settings-search-target"),
      1400,
    );
  });
}

function renderSettingsSearchResults() {
  const query = normalizeSettingsSearch(settingsSearchElement.value.trim());
  settingsSearchClearButton.hidden = !query;
  settingsSearchResultsElement.replaceChildren();
  if (!query) {
    closeSettingsSearch();
    return;
  }
  const terms = query.split(/\s+/).filter(Boolean);
  const results = settingsSearchIndex
    .filter((entry) => terms.every((term) => entry.searchable.includes(term)))
    .sort((left, right) => {
      const leftStarts = normalizeSettingsSearch(left.label).startsWith(query);
      const rightStarts = normalizeSettingsSearch(right.label).startsWith(query);
      return Number(rightStarts) - Number(leftStarts) || left.label.localeCompare(right.label, locale);
    })
    .slice(0, 8);
  settingsSearchResultsElement.hidden = false;
  if (!results.length) {
    const empty = document.createElement("p");
    empty.className = "settings-search__empty";
    empty.textContent = t("searchNoResults");
    settingsSearchResultsElement.append(empty);
    return;
  }
  for (const entry of results) {
    const button = document.createElement("button");
    button.className = "settings-search__result";
    button.type = "button";
    button.setAttribute("role", "option");
    const label = document.createElement("strong");
    label.textContent = entry.label;
    const page = document.createElement("small");
    page.textContent = entry.pageLabel;
    button.append(label, page);
    button.addEventListener("click", () => openSettingsSearchResult(entry));
    settingsSearchResultsElement.append(button);
  }
}

function setBotStatus(status) {
  botStatusElement.classList.toggle("is-online", status.connected);
  botLabelElement.textContent = status.connected ? status.username : status.error || t("botOffline");
  botAvatarElement.hidden = !status.displayAvatarUrl;
  if (status.displayAvatarUrl) {
    botAvatarElement.src = status.displayAvatarUrl;
  } else {
    botAvatarElement.removeAttribute("src");
  }
}

function outputClientCount(value) {
  return Math.max(0, Number(value) || 0);
}

function formatOutputLastConnected(timestamp) {
  const value = Number(timestamp);
  if (!Number.isFinite(value) || value <= 0) {
    return t("outputNeverConnected");
  }
  const formattingLocale = typeof locale === "string" ? locale : language;
  return `${t("outputLastConnected")} ${new Date(value).toLocaleTimeString(formattingLocale, {
    hour: "2-digit", minute: "2-digit", second: "2-digit",
  })}`;
}

let queueSignature;
function renderQueue(items = []) {
  const signature = JSON.stringify([language, items]);
  if (signature === queueSignature) return;
  queueSignature = signature;
  const list = document.querySelector("#queue-list");
  const state = document.querySelector("#queue-state");
  state.textContent = items.length ? String(items.length) : t("queueEmpty");
  list.replaceChildren(...items.map((item) => {
    const row = document.createElement("li");
    const label = document.createElement("span");
    label.textContent = [item.ready ? item.title : t("queuePreparing"), item.author, item.kind].filter(Boolean).join(" · ");
    const button = document.createElement("button");
    button.type = "button"; button.className = "button button--quiet"; button.textContent = t("removeQueue"); button.disabled = !item.ready;
    button.addEventListener("click", async () => {
      button.disabled = true;
      try { await invoke("remove_queued_media", { id: item.id }); queueSignature = undefined; await refreshRuntimeStatus(); }
      catch (error) { state.textContent = String(error); button.disabled = false; }
    });
    row.append(label, button); return row;
  }));
}

function renderOutputReadiness(status = {}) {
  const outputs = status.outputs || {};
  for (const target of ["visual", "audio", "notification", "sticker"]) {
    const output = outputs[target] || {};
    const obsClients = outputClientCount(output.obsClients);
    const previewClients = outputClientCount(output.previewClients);
    const widgetClients = outputClientCount(output.widgetClients);
    const clients = [
      [t("outputObs"), obsClients],
      [t("outputPreview"), previewClients],
      [t("outputWidget"), widgetClients],
    ].filter(([, count]) => count > 0);
    const liveOutputConnected = obsClients + widgetClients > 0;
    const stateElement = outputStateElements.get(target);
    const lastConnectedElement = outputLastConnectedElements.get(target);
    const card = outputReadinessCards.get(target);
    const testButton = outputTestButtons.get(target);

    if (stateElement) {
      stateElement.textContent = clients.length > 0
        ? clients.map(([name, count]) => `${name} ${count}`).join(" · ")
        : t("outputDisconnected");
    }
    if (lastConnectedElement) {
      lastConnectedElement.textContent = formatOutputLastConnected(output.lastConnectedAt);
    }
    if (card) {
      let diagnostic = card.querySelector(".output-diagnostic");
      if (!diagnostic) { diagnostic = document.createElement("p"); diagnostic.className = "output-diagnostic"; card.append(diagnostic); }
      const disabled = target === "notification" && !bootstrap?.config?.ttsNotificationsObsEnabled && widgetClients === 0;
      diagnostic.textContent = !status.connected ? t("serverOffline") : disabled ? t("diagnosticDisabled")
        : !liveOutputConnected ? t("diagnosticWaiting") : t("diagnosticReady");
      card.classList.toggle("is-live", liveOutputConnected);
      card.classList.toggle("is-preview-only", !liveOutputConnected && previewClients > 0);
    }
    if (testButton) {
      testButton.disabled = !liveOutputConnected;
      testButton.title = liveOutputConnected ? "" : t("outputTestNeedsLiveOutput");
    }
  }
}

function setServerStatus(status) {
  serverStatusElement.classList.toggle("is-online", status.connected);
  serverLabelElement.textContent = status.connected ? t("serverOnline") : status.error || t("serverOffline");
  clientCountElement.textContent = String(status.overlayClients || 0);
  renderOutputReadiness(status);
  updateSampleTestButton();
  if (!status.connected) {
    audioPlaybackTargets.clear();
    renderNowPlaying();
  }
}

function filterWordKey(value) {
  return String(value || "")
    .normalize("NFKC")
    .toLocaleLowerCase()
    .trim()
    .replace(/[\s._-]+/g, "");
}

function filterConceptsToLines(concepts) {
  return (Array.isArray(concepts) ? concepts : [])
    .map((concept) => typeof concept?.canonical === "string" ? concept.canonical.trim() : "")
    .filter(Boolean)
    .join(", ");
}

function filterRoleIdsToInput(roleIds) {
  return (Array.isArray(roleIds) ? roleIds : [])
    .filter((roleId) => typeof roleId === "string" && /^\d{17,20}$/.test(roleId))
    .join(", ");
}

function filterRoleIds(value) {
  const seen = new Set();
  return String(value || "")
    .split(/[\r\n,]+/)
    .map((entry) => entry.trim())
    .map((entry) => entry.match(/^<@&(\d{17,20})>$/)?.[1] || entry)
    .reduce((roleIds, entry) => {
      if (entry && !seen.has(entry)) {
        seen.add(entry);
        roleIds.push(entry);
      }
      return roleIds;
    }, []);
}

function filterWordsToConcepts(value, existingConcepts) {
  const existing = new Map(
    (Array.isArray(existingConcepts) ? existingConcepts : [])
      .filter((concept) => concept && typeof concept.canonical === "string")
      .map((concept) => [filterWordKey(concept.canonical), concept]),
  );
  const seen = new Set();
  const words = String(value || "")
    .split(/[\r\n,]+/)
    .map((word) => word.trim())
    .filter(Boolean);
  return words.reduce((concepts, word) => {
    const key = filterWordKey(word);
    if (!key || seen.has(key)) {
      return concepts;
    }
    seen.add(key);
    const previous = existing.get(key);
    concepts.push({
      canonical: word,
      aliases: Array.isArray(previous?.aliases)
        ? previous.aliases.filter((alias) => typeof alias === "string")
        : [],
      regexes: Array.isArray(previous?.regexes)
        ? previous.regexes.filter((pattern) => typeof pattern === "string")
        : [],
    });
    return concepts;
  }, []);
}

function filterWordsAreSaveable(value) {
  return String(value || "")
    .split(/[\r\n,]+/)
    .map((word) => word.trim())
    .filter(Boolean)
    .every((word) => {
      const normalized = filterWordKey(word);
      return /\p{L}/u.test(word)
        && normalized.length >= 3
        && normalized.length <= 64;
    });
}

function privacyListToInput(values) {
  return (Array.isArray(values) ? values : [])
    .filter((value) => typeof value === "string" && value.trim())
    .join("\n");
}

function privacyListFromInput(value) {
  const seen = new Set();
  return String(value || "")
    .split(/[\r\n]+/)
    .map((entry) => entry.trim())
    .filter((entry) => {
      const key = entry.normalize("NFKC").toLocaleLowerCase();
      if (!entry || seen.has(key)) {
        return false;
      }
      seen.add(key);
      return true;
    });
}

function cloneCustomCommands(commands) {
  return JSON.parse(JSON.stringify(Array.isArray(commands) ? commands : []));
}

function defaultCustomAction(type) {
  const reason = () => ({ mode: "optional", fixedValue: "" });
  const entity = () => ({ mode: "required", fixedValue: "" });
  switch (type) {
    case "ban": return { type, reason: reason(), deleteMessageDays: { mode: "fixed", fixedValue: 0 } };
    case "unban": return { type, reason: reason() };
    case "kick": return { type, reason: reason() };
    case "timeout": return { type, durationMinutes: { mode: "fixed", fixedValue: 60 }, reason: reason() };
    case "removeTimeout": return { type, reason: reason() };
    case "clearMessages": return {
      type,
      channel: { mode: "optional", fixedValue: "" },
      count: { mode: "fixed", fixedValue: 10 },
    };
    case "addRole": return { type, role: entity(), reason: reason() };
    case "removeRole": return { type, role: entity(), reason: reason() };
    case "reply": return { type, text: "Relay", ephemeral: true };
    default: return defaultCustomAction("ban");
  }
}

function customActionTranslationKey(type) {
  return {
    ban: "customActionBan", unban: "customActionUnban", kick: "customActionKick",
    timeout: "customActionTimeout", removeTimeout: "customActionRemoveTimeout",
    clearMessages: "customActionClearMessages", addRole: "customActionAddRole",
    removeRole: "customActionRemoveRole", reply: "customActionReply",
  }[type] || "customActionReply";
}

function customActionPermissionKey(type) {
  return {
    ban: "permissionBanMembers", unban: "permissionBanMembers", kick: "permissionKickMembers",
    timeout: "permissionModerateMembers", removeTimeout: "permissionModerateMembers",
    clearMessages: "permissionManageMessages", addRole: "permissionManageRoles",
    removeRole: "permissionManageRoles",
  }[type];
}

function customParameterMarkup(key, labelKey, kind, minimum = "", maximum = "") {
  const inputAttributes = kind === "integer"
    ? `type="number" min="${minimum}" max="${maximum}" step="1"`
    : `type="text" maxlength="512" autocomplete="off" spellcheck="false"`;
  return `
    <div class="custom-parameter" data-custom-parameter="${key}" data-kind="${kind}" data-min="${minimum}" data-max="${maximum}">
      <strong>${t(labelKey)}</strong>
      <label class="field">
        <span>${t("customParameterMode")}</span>
        <select data-custom-parameter-mode>
          <option value="required">${t("parameterRequired")}</option>
          <option value="optional">${t("parameterOptional")}</option>
          <option value="fixed">${t("parameterFixed")}</option>
        </select>
      </label>
      <label class="field">
        <span>${t("customParameterValue")}</span>
        <input data-custom-parameter-value ${inputAttributes}>
      </label>
    </div>`;
}

function setCustomParameterValue(key, parameter) {
  const root = customActionFieldsElement.querySelector(`[data-custom-parameter="${key}"]`);
  if (!root) return;
  root.querySelector("[data-custom-parameter-mode]").value = parameter?.mode || "optional";
  root.querySelector("[data-custom-parameter-value]").value = String(parameter?.fixedValue ?? "");
}

function updateCustomParameterAvailability(root = customActionFieldsElement) {
  const parameters = root.matches?.("[data-custom-parameter]")
    ? [root]
    : $$('[data-custom-parameter]', root);
  for (const parameter of parameters) {
    const mode = parameter.querySelector("[data-custom-parameter-mode]").value;
    const value = parameter.querySelector("[data-custom-parameter-value]");
    value.disabled = mode === "required";
    value.required = mode === "fixed" || (mode === "optional" && parameter.dataset.kind === "entity-role");
  }
}

function renderCustomRequiredPermissions() {
  const permissionKey = customActionPermissionKey(customCommandActionElement.value);
  const permission = permissionKey ? t(permissionKey) : "—";
  customRequiredPermissionsElement.textContent = formatTranslation("customRequiredPermission", { permission });
}

function renderCustomActionFields(action = defaultCustomAction(customCommandActionElement.value)) {
  const type = customCommandActionElement.value;
  if (action.type !== type) action = defaultCustomAction(type);
  switch (type) {
    case "ban":
      customActionFieldsElement.innerHTML = customParameterMarkup("reason", "customReason", "text")
        + customParameterMarkup("deleteMessageDays", "customDeleteDays", "integer", 0, 7);
      setCustomParameterValue("reason", action.reason);
      setCustomParameterValue("deleteMessageDays", action.deleteMessageDays);
      break;
    case "unban":
    case "kick":
    case "removeTimeout":
      customActionFieldsElement.innerHTML = customParameterMarkup("reason", "customReason", "text");
      setCustomParameterValue("reason", action.reason);
      break;
    case "timeout":
      customActionFieldsElement.innerHTML = customParameterMarkup("durationMinutes", "customDurationMinutes", "integer", 1, 40320)
        + customParameterMarkup("reason", "customReason", "text");
      setCustomParameterValue("durationMinutes", action.durationMinutes);
      setCustomParameterValue("reason", action.reason);
      break;
    case "clearMessages":
      customActionFieldsElement.innerHTML = customParameterMarkup("channel", "customChannelId", "entity-channel")
        + customParameterMarkup("count", "customMessageCount", "integer", 1, 1000);
      setCustomParameterValue("channel", action.channel);
      setCustomParameterValue("count", action.count);
      break;
    case "addRole":
    case "removeRole":
      customActionFieldsElement.innerHTML = customParameterMarkup("role", "customRoleId", "entity-role")
        + customParameterMarkup("reason", "customReason", "text");
      setCustomParameterValue("role", action.role);
      setCustomParameterValue("reason", action.reason);
      break;
    case "reply":
      customActionFieldsElement.innerHTML = `
        <label class="field field--full">
          <span>${t("customReplyText")}</span>
          <textarea id="custom-reply-text" minlength="1" maxlength="1900" required></textarea>
        </label>
        <label class="field field--full">
          <span>${t("customReplyVisibility")}</span>
          <select id="custom-reply-visibility">
            <option value="ephemeral">${t("customReplyEphemeral")}</option>
            <option value="public">${t("customReplyPublic")}</option>
          </select>
        </label>`;
      $("#custom-reply-text").value = action.text || "";
      $("#custom-reply-visibility").value = action.ephemeral === false ? "public" : "ephemeral";
      break;
  }
  updateCustomParameterAvailability();
  renderCustomRequiredPermissions();
}

function normalizeDiscordId(value) {
  const match = String(value || "").trim().match(/^(?:\d{17,20}|<@!?(\d{17,20})>|<@&(\d{17,20})>|<#(\d{17,20})>)$/);
  if (!match) return null;
  return match[1] || match[2] || match[3] || match[0];
}

function discordIdListFromInput(value) {
  const tokens = String(value || "").split(/[\s,]+/).filter(Boolean);
  const ids = tokens.map(normalizeDiscordId);
  if (ids.some((id) => !id) || ids.length > 100) throw new Error(t("customInvalidIds"));
  return [...new Set(ids)];
}

function readCustomParameter(key) {
  const root = customActionFieldsElement.querySelector(`[data-custom-parameter="${key}"]`);
  const mode = root.querySelector("[data-custom-parameter-mode]").value;
  const input = root.querySelector("[data-custom-parameter-value]");
  let fixedValue = input.value.trim();
  if (root.dataset.kind === "integer") {
    fixedValue = Number(fixedValue || root.dataset.min || 0);
    if (mode !== "required"
      && (!Number.isInteger(fixedValue)
        || fixedValue < Number(root.dataset.min)
        || fixedValue > Number(root.dataset.max))) {
      input.setCustomValidity(t("customParameterValue"));
      input.reportValidity();
      input.setCustomValidity("");
      throw new Error(t("customParameterValue"));
    }
  } else if (root.dataset.kind.startsWith("entity") && fixedValue) {
    const id = normalizeDiscordId(fixedValue);
    if (!id) throw new Error(t("customInvalidIds"));
    fixedValue = id;
  }
  if (root.dataset.kind === "entity-role" && mode !== "required" && !fixedValue) {
    throw new Error(t("customInvalidIds"));
  }
  if (root.dataset.kind === "entity-channel" && mode === "fixed" && !fixedValue) {
    throw new Error(t("customInvalidIds"));
  }
  return { mode, fixedValue };
}

function readCustomAction() {
  const type = customCommandActionElement.value;
  switch (type) {
    case "ban": return { type, reason: readCustomParameter("reason"), deleteMessageDays: readCustomParameter("deleteMessageDays") };
    case "unban": return { type, reason: readCustomParameter("reason") };
    case "kick": return { type, reason: readCustomParameter("reason") };
    case "timeout": return { type, durationMinutes: readCustomParameter("durationMinutes"), reason: readCustomParameter("reason") };
    case "removeTimeout": return { type, reason: readCustomParameter("reason") };
    case "clearMessages": return { type, channel: readCustomParameter("channel"), count: readCustomParameter("count") };
    case "addRole": return { type, role: readCustomParameter("role"), reason: readCustomParameter("reason") };
    case "removeRole": return { type, role: readCustomParameter("role"), reason: readCustomParameter("reason") };
    case "reply": return {
      type,
      text: $("#custom-reply-text").value,
      ephemeral: $("#custom-reply-visibility").value !== "public",
    };
    default: throw new Error(t("customCommandAction"));
  }
}

function collectCustomCommandDraft() {
  if (!customCommandForm.reportValidity()) return null;
  const name = customCommandNameElement.value.trim().toLowerCase();
  if (defaultRelayCommandNames.has(name)
    || customCommands.some((command, index) => command.name === name && index !== editingCustomCommandIndex)) {
    throw new Error(t("customDuplicateName"));
  }
  return {
    name,
    description: customCommandDescriptionElement.value.trim(),
    enabled: customCommandEnabledElement.checked,
    action: readCustomAction(),
    access: {
      administratorOnly: customCommandAdminOnlyElement.checked,
      requiredPermissions: customPermissionInputs.filter((input) => input.checked).map((input) => input.value),
      allowedUserIds: discordIdListFromInput(customCommandUsersElement.value),
      allowedRoleIds: discordIdListFromInput(customCommandRolesElement.value),
      allowedChannelIds: discordIdListFromInput(customCommandChannelsElement.value),
    },
  };
}

function renderCustomCommands() {
  customCommandListElement.replaceChildren();
  customCommandCountElement.textContent = `${customCommands.length} / 16`;
  customCommandsEmptyElement.hidden = customCommands.length !== 0;
  addCustomCommandButton.disabled = customCommands.length >= 16;
  for (const [index, command] of customCommands.entries()) {
    const item = document.createElement("li");
    item.className = "custom-command-card";
    const identity = document.createElement("div");
    identity.className = "custom-command-card__identity";
    const title = document.createElement("span");
    const code = document.createElement("strong");
    code.className = "command-code";
    code.textContent = `/relay ${command.name}`;
    const badge = document.createElement("span");
    badge.className = "custom-command-card__badge";
    badge.dataset.active = String(command.enabled !== false);
    badge.textContent = t(command.enabled !== false ? "active" : "disabled");
    title.append(code, badge);
    const details = document.createElement("small");
    details.textContent = `${t(customActionTranslationKey(command.action?.type))} · ${command.description}`;
    identity.append(title, details);
    const actions = document.createElement("div");
    actions.className = "custom-command-card__actions";
    for (const [action, key] of [["edit", "edit"], ["delete", "delete"]]) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "button button--quiet";
      button.dataset.customCommandAction = action;
      button.dataset.customCommandIndex = String(index);
      button.textContent = t(key);
      actions.append(button);
    }
    item.append(identity, actions);
    customCommandListElement.append(item);
  }
}

function closeCustomCommandEditor() {
  customCommandForm.hidden = true;
  editingCustomCommandIndex = null;
  customCommandEditorStateElement.textContent = "";
  syncCustomCommandsButton.disabled = false;
}

function openCustomCommandEditor(index = null) {
  if (index === null && customCommands.length >= 16) {
    setSaveState(customCommandsSaveStateElement, "error", t("customMaxReached"));
    return;
  }
  editingCustomCommandIndex = index;
  const definition = index === null ? {
    name: "",
    description: "",
    enabled: true,
    action: defaultCustomAction("ban"),
    access: { administratorOnly: true, requiredPermissions: [], allowedUserIds: [], allowedRoleIds: [], allowedChannelIds: [] },
  } : cloneCustomCommands([customCommands[index]])[0];
  customCommandNameElement.value = definition.name;
  customCommandDescriptionElement.value = definition.description;
  customCommandEnabledElement.checked = definition.enabled !== false;
  customCommandActionElement.value = definition.action.type;
  customCommandAdminOnlyElement.checked = definition.access?.administratorOnly !== false;
  const requiredPermissions = new Set(definition.access?.requiredPermissions || []);
  for (const input of customPermissionInputs) input.checked = requiredPermissions.has(input.value);
  customCommandUsersElement.value = (definition.access?.allowedUserIds || []).join("\n");
  customCommandRolesElement.value = (definition.access?.allowedRoleIds || []).join("\n");
  customCommandChannelsElement.value = (definition.access?.allowedChannelIds || []).join("\n");
  customCommandPreviewElement.textContent = `/relay ${definition.name || "command"}`;
  renderCustomActionFields(definition.action);
  customCommandForm.hidden = false;
  syncCustomCommandsButton.disabled = true;
  customCommandNameElement.focus();
}

function formSaveState(form) {
  const ids = {
    "bot-presence-form": "bot-presence-save-state", "routing-form": "save-state",
    "music-form": "music-save-state", "media-form": "media-save-state",
    "messages-form": "messages-save-state",
    "moderation-form": "moderation-save-state", "commands-form": "commands-save-state",
  };
  return document.getElementById(ids[form.id]);
}

function setSaveState(element, state, message = t(state)) {
  if (!element) return;
  element.dataset.state = state;
  element.textContent = message;
}

function captureFormDraft(form) {
  return [...form.querySelectorAll("input, select, textarea")]
    .filter((element) => element.type !== "password" && element.type !== "file")
    .map((element) => ({
      element, value: element.value, checked: element.checked,
      option: element.selectedOptions?.[0]?.cloneNode(true),
    }));
}

function restoreFormDraft(draft) {
  for (const { element, value, checked, option } of draft) {
    if (option && ![...element.options].some((item) => item.value === value)) {
      element.append(option);
    }
    element.value = value;
    if (typeof checked === "boolean") element.checked = checked;
  }
}

let configSaveQueue = Promise.resolve();
function queueConfigSave(save) {
  const pending = configSaveQueue.then(save);
  configSaveQueue = pending.catch(() => {});
  return pending;
}

function finishFormSave(form, revision, stateElement) {
  if ((formRevisions.get(form) || 0) === revision) {
    dirtyForms.delete(form);
    setSaveState(stateElement, "saved");
  } else {
    setSaveState(stateElement, "unsaved");
  }
}

function applyConfig(config) {
  const drafts = [...dirtyForms].map(captureFormDraft);
  mediaCleanupEnabledElement.checked = Boolean(config.mediaCleanupEnabled);
  mediaWelcomeMessageElement.value = config.mediaWelcomeMessageId || "";
  ttsCleanupEnabledElement.checked = Boolean(config.ttsCleanupEnabled);
  ttsWelcomeMessageElement.value = config.ttsWelcomeMessageId || "";
  durationElement.value = String(config.displayDurationMs / 1000);
  gifDurationElement.value = String((config.gifDurationMs ?? config.displayDurationMs) / 1000);
  stickerDurationElement.value = String((config.stickerDurationMs ?? 8000) / 1000);
  portElement.value = String(config.port);
  mediaVolumeElement.value = String(config.mediaVolume ?? 50);
  mediaVolumeValueElement.value = `${mediaVolumeElement.value}%`;
  mediaVolumeValueElement.textContent = `${mediaVolumeElement.value}%`;
  ttsCharacterLimitElement.value = String(config.ttsCharacterLimit ?? 0);
  ttsQueueLimitElement.value = String(config.ttsQueueLimit ?? 50);
  notificationDurationElement.value = String((config.notificationDurationMs ?? 8000) / 1000);
  widgetSoundEnabledElement.checked = Boolean(config.widgetSoundEnabled);
  applyNotificationSoundConfig(config);
  ttsNotificationsObsElement.checked = Boolean(config.ttsNotificationsObsEnabled);
  botOnlineStatusElement.value = config.botOnlineStatus || "online";
  botActivityTypeElement.value = config.botActivityType || "custom";
  botActivityTextElement.value = config.botActivityText || "";
  updateBotActivityAvailability();
  showAuthorElement.checked = config.showAuthor;
  showMediaTextObsElement.checked = Boolean(config.showMediaTextObs);
  showMediaTextWidgetElement.checked = Boolean(config.showMediaTextWidget);
  moderationEnabledElement.checked = Boolean(config.moderationEnabled);
  moderationAllowImagesElement.checked = config.moderationAllowImages !== false;
  moderationAllowVideosElement.checked = config.moderationAllowVideos !== false;
  moderationAllowAudioElement.checked = config.moderationAllowAudio !== false;
  honeypotActionElement.value = config.honeypotAction || "kick";
  privacyScanEnabledElement.checked = Boolean(config.privacyScanEnabled);
  privacyProtectionLevelElement.value = config.privacyProtectionLevel || "balanced";
  privacyBlockThresholdElement.value = config.privacyBlockThreshold || "high";
  privacyReviewIntermediateElement.checked = config.privacyReviewIntermediate !== false;
  privacyAutoDeleteBlockedMessagesElement.checked = config.privacyAutoDeleteBlockedMessages !== false;
  const enabledCategories = new Set(
    config.privacyEnabledCategories || privacyCategoryElements.map((input) => input.value),
  );
  for (const input of privacyCategoryElements) {
    input.checked = enabledCategories.has(input.value);
  }
  privacyCustomPatternsElement.value = privacyListToInput(config.privacyCustomPatterns);
  privacyAllowlistElement.value = privacyListToInput(config.privacyAllowlist);
  privacyConceptsElement.value = filterConceptsToLines(config.privacyConcepts);
  privacyExemptRoleIdsElement.value = filterRoleIdsToInput(config.privacyFilterExemptRoleIds);
  populateChannels(channelElement, bootstrap?.channels || [], config.watchedChannelId, t("selectChannel"));
  populateChannels(ttsChannelElement, bootstrap?.channels || [], config.ttsChannelId, t("ttsDisabled"));
  musicWelcomeElement.value = config.musicWelcomeMessageId || "";
  musicCleanupEnabledElement.checked = Boolean(config.musicCleanupEnabled);
  populateChannels(musicChannelElement, bootstrap?.channels || [], config.musicChannelId, t("musicDisabled"));
  populateChannels(honeypotChannelElement, bootstrap?.channels || [], config.honeypotChannelId, t("honeypotDisabled"));
  commandInputs.channel.checked = config.commandChannelEnabled !== false;
  commandInputs.url.checked = config.commandUrlEnabled !== false;
  commandInputs.show.checked = config.commandShowEnabled !== false;
  commandInputs.status.checked = config.commandStatusEnabled !== false;
  commandInputs.test.checked = config.commandTestEnabled !== false;
  commandInputs.regenerate.checked = config.commandRegenerateEnabled !== false;
  commandInputs.clear.checked = config.commandClearEnabled !== false;
  commandInputs.nuke.checked = config.commandNukeEnabled !== false;
  commandInputs.lock.checked = config.commandLockEnabled !== false;
  commandInputs.changelog.checked = config.commandChangelogEnabled !== false;
  commandInputs.lock.disabled = Boolean(config.channelLock);
  channelLockStateElement.dataset.i18n = config.channelLock ? "commandLockActive" : "commandLockInactive";
  channelLockStateElement.textContent = t(channelLockStateElement.dataset.i18n);
  if (!customCommandsDirty) {
    customCommands = cloneCustomCommands(config.customCommands);
    renderCustomCommands();
  }
  applyOutputGeometryConfig(config);
  updateSkipShortcutDisplay(config.skipShortcut);
  drafts.forEach(restoreFormDraft);
  mediaVolumeValueElement.value = `${mediaVolumeElement.value}%`;
  mediaVolumeValueElement.textContent = `${mediaVolumeElement.value}%`;
  updateBotActivityAvailability();
}

function setCredentials(status) {
  setSaveState(credentialStateElement, "idle", status.configured
    ? `${t("savedVia")} ${status.source}`
    : t("notConfigured"));
  if (!clientIdElement.value) clientIdElement.value = status.clientId || "";
  updateYoutubeKeyStatus(status);
}

function updateYoutubeKeyStatus(status = bootstrap?.credentials) {
  if (!youtubeKeyStatusElement) return;
  youtubeKeyStatusElement.textContent = status?.youtubeConfigured
    ? t("youtubeKeyConfigured")
    : t("youtubeKeyMissing");
}

function formatShortcutLabel(shortcut) {
  return String(shortcut || "control+alt+KeyS")
    .split("+")
    .map((token) => {
      const normalized = token.trim();
      const lower = normalized.toLowerCase();
      if (lower === "control" || lower === "ctrl") return "Ctrl";
      if (lower === "alt" || lower === "option") return "Alt";
      if (lower === "shift") return "Shift";
      if (lower === "super" || lower === "command" || lower === "cmd") return "Win";
      if (/^key[a-z]$/i.test(normalized)) return normalized.slice(-1).toUpperCase();
      if (/^digit\d$/i.test(normalized)) return normalized.slice(-1);
      return normalized
        .replace(/^Arrow/, "")
        .replace(/^Numpad/, "Num ");
    })
    .join(" ");
}

function updateSkipShortcutDisplay(shortcut) {
  const label = formatShortcutLabel(shortcut);
  skipShortcutKeyElement.textContent = label;
  skipShortcutValueElement.textContent = label;
}

function shortcutTokenFromEvent(event) {
  const modifierCodes = new Set(["ControlLeft", "ControlRight", "AltLeft", "AltRight", "ShiftLeft", "ShiftRight", "MetaLeft", "MetaRight"]);
  if (modifierCodes.has(event.code)) return "";
  const supportedCode = /^(Key[A-Z]|Digit\d|F(?:[1-9]|1\d|2[0-4])|Arrow(?:Up|Down|Left|Right)|Numpad(?:\d|Add|Subtract|Multiply|Divide|Decimal|Enter|Equal)|(?:Backquote|Backslash|BracketLeft|BracketRight|Comma|Equal|Minus|Period|Quote|Semicolon|Slash|Backspace|CapsLock|Delete|End|Enter|Escape|Home|Insert|PageDown|PageUp|Pause|PrintScreen|ScrollLock|Space|Tab))$/;
  return supportedCode.test(event.code) ? event.code : "";
}

function beginShortcutCapture() {
  shortcutCaptureActive = true;
  skipShortcutCaptureButton.setAttribute("aria-pressed", "true");
  skipShortcutValueElement.textContent = t("pressShortcut");
  skipShortcutCaptureButton.focus();
}

function cancelShortcutCapture() {
  shortcutCaptureActive = false;
  skipShortcutCaptureButton.setAttribute("aria-pressed", "false");
  updateSkipShortcutDisplay(bootstrap?.config?.skipShortcut);
}

async function saveCapturedShortcut(shortcut) {
  const previousLabel = formatShortcutLabel(bootstrap?.config?.skipShortcut);
  shortcutCaptureActive = false;
  skipShortcutCaptureButton.setAttribute("aria-pressed", "false");
  skipShortcutValueElement.textContent = formatShortcutLabel(shortcut);
  setSaveState(mediaSaveStateElement, "saving");
  try {
    const config = await invoke("set_skip_shortcut", { shortcut });
    bootstrap.config = config;
    updateSkipShortcutDisplay(config.skipShortcut);
    setSaveState(mediaSaveStateElement, "saved", t("shortcutSaved"));
  } catch (error) {
    skipShortcutValueElement.textContent = previousLabel;
    setSaveState(mediaSaveStateElement, "error", String(error) || t("shortcutInvalid"));
  }
}

function updateBotActivityAvailability() {
  botActivityTextElement.disabled = botActivityTypeElement.value === "none";
}

function setWidgetState(state) {
  if (bootstrap) {
    bootstrap.widget = state;
  }
  widgetStateElement.textContent = state.visible
    ? state.locked ? t("widgetVisibleLocked") : t("widgetVisibleMovable")
    : t("widgetHidden");
  toggleWidgetButton.textContent = state.visible ? t("hideWidget") : t("showWidget");
  lockWidgetButton.textContent = state.locked ? t("unlockMove") : t("lockDisplay");
}

function setNotificationWidgetState(state) {
  if (bootstrap) {
    bootstrap.notificationWidget = state;
  }
  notificationWidgetEnabledElement.checked = state.visible;
  notificationWidgetStateElement.textContent = state.visible
    ? state.locked ? t("widgetVisibleLocked") : t("widgetVisibleMovable")
    : t("widgetHidden");
  lockNotificationWidgetButton.textContent = state.locked ? t("unlockMove") : t("lockDisplay");
  lockNotificationWidgetButton.disabled = !state.visible;
}

function renderHistory() {
  historyListElement.replaceChildren();
  const query = (document.querySelector("#history-search")?.value || "").trim().toLocaleLowerCase();
  const kindFilter = document.querySelector("#history-kind")?.value || "";
  const visibleHistory = history.filter((item) => (!kindFilter || item.kind === kindFilter)
    && [item.filename, item.title, item.artist, item.author?.username].some((value) => String(value || "").toLocaleLowerCase().includes(query)));
  historyEmptyElement.hidden = visibleHistory.length > 0;
  historyEmptyElement.textContent = t(history.length ? "noHistoryResults" : "historyEmpty");
  for (const mediaEvent of visibleHistory) {
    const item = historyItemTemplate.content.cloneNode(true);
    const kind = mediaEvent.kind || "image";
    setMediaThumbnail(item, mediaEvent, kind);
    item.querySelector(".history-item__type").textContent = kind.toUpperCase();
    item.querySelector(".history-item__filename").textContent = mediaEvent.filename || kind;
    item.querySelector(".history-item__author").textContent = mediaEvent.author?.username || t("unknownAuthor");
    const time = item.querySelector(".history-item__time");
    time.dateTime = new Date(mediaEvent.timestamp).toISOString();
    time.textContent = new Date(mediaEvent.timestamp).toLocaleTimeString(locale, {
      hour: "2-digit", minute: "2-digit", second: "2-digit",
    });
    const replayButton = item.querySelector(".history-item__replay");
    replayButton.textContent = t("replay");
    replayButton.addEventListener("click", async () => {
      try {
        await invoke("replay_media", { messageId: mediaEvent.messageId });
      } catch (error) {
        setSaveState(saveStateElement, "error", String(error));
      }
    });
    const downloadButton = item.querySelector(".history-item__download");
    if (["image", "gif", "video"].includes(kind)) {
      const saveLibrary = document.createElement("button");
      saveLibrary.type = "button";
      saveLibrary.className = "button button--quiet";
      saveLibrary.textContent = t("modSaveLibrary");
      downloadButton.parentElement.append(saveLibrary);
      saveLibrary.addEventListener("click", async () => {
        saveLibrary.disabled = true;
        try {
          await invoke("save_history_to_library", { messageId: mediaEvent.messageId, mediaUrl: mediaEvent.url });
          await moduleControls?.loadLibrary();
          setSaveState(saveStateElement, "saved", t("modSaved"));
        } catch (error) { setSaveState(saveStateElement, "error", String(error)); }
        finally { saveLibrary.disabled = false; }
      });
    }
    downloadButton.textContent = t("download");
    downloadButton.addEventListener("click", async () => {
      downloadButton.disabled = true;
      setSaveState(saveStateElement, "saving", t("downloading"));
      try {
        const saved = await invoke("download_history_media", {
          messageId: mediaEvent.messageId,
          mediaUrl: mediaEvent.url,
        });
        setSaveState(saveStateElement, saved ? "saved" : "idle", saved ? t("downloaded") : t("downloadCanceled"));
      } catch (error) {
        setSaveState(saveStateElement, "error", String(error));
      } finally {
        downloadButton.disabled = false;
      }
    });
    historyListElement.append(item);
  }
  loadHistoryVideoThumbnails();
}

function isVideoThumbnail(kind, contentType) {
  return kind === "video" || (kind === "gif" && contentType?.startsWith("video/"));
}

function loadHistoryVideoThumbnails() {
  for (const video of historyListElement.querySelectorAll("video[data-thumbnail-source]")) {
    if (!video.src) {
      video.src = video.dataset.thumbnailSource;
      video.load();
    }
  }
}

function setMediaThumbnail(item, mediaEvent, kind) {
  const thumbnail = item.querySelector(".history-item__thumb");
  const panelToken = bootstrap?.wsUrl
    ? new URL(bootstrap.wsUrl).searchParams.get("token")
    : "";
  const source = mediaEvent.cachedMediaId
    ? `http://127.0.0.1:${bootstrap.config.port}/media-cache/${encodeURIComponent(mediaEvent.cachedMediaId)}?token=${encodeURIComponent(panelToken)}`
    : mediaEvent.url || mediaEvent.proxyUrl;
  if (isVideoThumbnail(kind, mediaEvent.contentType) && source) {
    const video = document.createElement("video");
    video.className = thumbnail.className;
    video.poster = "./assets/relay-radar.png";
    video.preload = "none";
    video.muted = true;
    video.playsInline = true;
    video.dataset.thumbnailSource = source;
    video.addEventListener("loadeddata", () => {
      const time = Number.isFinite(video.duration) ? Math.min(0.1, video.duration / 2) : 0;
      if (time > 0) {
        video.addEventListener("seeked", () => video.pause(), { once: true });
        video.currentTime = time;
      } else {
        video.pause();
      }
    }, { once: true });
    video.addEventListener("error", () => {
      video.removeAttribute("src");
      video.poster = "./assets/relay-radar.png";
    }, { once: true });
    thumbnail.replaceWith(video);
    return;
  }
  thumbnail.src = kind === "image" || kind === "gif"
    ? source
    : "./assets/relay-radar.png";
  thumbnail.alt = mediaEvent.filename || kind;
  if (kind === "audio" && mediaEvent.artworkId) {
    thumbnail.onerror = () => {
      thumbnail.onerror = null;
      thumbnail.src = "./assets/relay-radar.png";
    };
    loadArtwork(mediaEvent.artworkId).then((source) => { thumbnail.src = source; }).catch(() => {});
  }
}

function replaceHistory(mediaEvents) {
  history.splice(0, history.length, ...mediaEvents.slice(0, 50));
  renderHistory();
}

function sameHistoryMedia(left, right) {
  return left?.messageId === right?.messageId && left?.url === right?.url;
}

function rememberMedia(mediaEvent) {
  // Replay rebroadcasts the same media over WS without updating server history.
  // Skip local duplicates so Replay does not create a second row.
  if (history.some((item) => sameHistoryMedia(item, mediaEvent))) {
    return;
  }
  history.unshift(mediaEvent);
  history.length = Math.min(history.length, 50);
  renderHistory();
}

function renderModeration() {
  const pending = bootstrap?.pendingMedia || [];
  moderationListElement.replaceChildren();
  moderationCountElement.textContent = `${pending.length} / 50`;
  moderationEmptyElement.hidden = pending.length > 0;
  const config = bootstrap?.config;
  const filterWordsActive = Array.isArray(config?.privacyConcepts)
    && config.privacyConcepts.length > 0;
  const privacyReviewQueue = config?.privacyScanEnabled || filterWordsActive;
  moderationEmptyElement.textContent = t(
    config?.moderationEnabled
      ? "moderationEmpty"
      : privacyReviewQueue
        ? "privacyReviewQueueEmpty"
        : "moderationDisabled",
  );
  clearPendingMediaButton.disabled = pending.length === 0;

  for (const pendingItem of pending) {
    const mediaEvent = pendingItem.media;
    const item = moderationItemTemplate.content.cloneNode(true);
    const kind = mediaEvent.kind || "image";
    setMediaThumbnail(item, mediaEvent, kind);
    item.querySelector(".history-item__type").textContent = kind.toUpperCase();
    item.querySelector(".history-item__filename").textContent = mediaEvent.filename || kind;
    item.querySelector(".history-item__author").textContent = mediaEvent.author?.username || t("unknownAuthor");
    const classification = pendingItem.privacyClassification
      || t("privacyPendingManual");
    const categories = Array.isArray(pendingItem.privacyCategories)
      ? pendingItem.privacyCategories.map((category) => String(category)
        .replace(/([A-Z])/g, " $1")
        .trim()
        .toUpperCase())
      : [];
    const detected = categories.length > 0
      ? categories.join(" + ")
      : (pendingItem.privacyReason || "manual_review").toUpperCase();
    item.querySelector(".moderation-item__privacy").textContent = `${String(classification).toUpperCase()} · ${detected}`;
    const time = item.querySelector(".history-item__time");
    time.dateTime = new Date(mediaEvent.timestamp).toISOString();
    time.textContent = new Date(mediaEvent.timestamp).toLocaleTimeString(locale, {
      hour: "2-digit", minute: "2-digit", second: "2-digit",
    });
    const approveButton = item.querySelector(".moderation-item__approve");
    const rejectButton = item.querySelector(".moderation-item__reject");
    approveButton.textContent = t("approve");
    rejectButton.textContent = t("reject");
    const decide = async (command) => {
      approveButton.disabled = true;
      rejectButton.disabled = true;
      try {
        await invoke(command, { id: pendingItem.id });
        bootstrap.pendingMedia = bootstrap.pendingMedia.filter(({ id }) => id !== pendingItem.id);
        renderModeration();
      } catch (error) {
        setSaveState(moderationSaveStateElement, "error", String(error));
        approveButton.disabled = false;
        rejectButton.disabled = false;
      }
    };
    approveButton.addEventListener("click", () => decide("approve_pending_media"));
    rejectButton.addEventListener("click", () => decide("reject_pending_media"));
    moderationListElement.append(item);
  }
}

function populateChannels(element, channels, selectedChannelId, placeholderText) {
  element.replaceChildren();
  const placeholder = document.createElement("option");
  placeholder.value = "";
  placeholder.textContent = placeholderText;
  element.append(placeholder);
  const groups = new Map();
  for (const channel of channels) {
    if (!groups.has(channel.guildName)) {
      groups.set(channel.guildName, []);
    }
    groups.get(channel.guildName).push(channel);
  }
  for (const [guildName, guildChannels] of groups) {
    const group = document.createElement("optgroup");
    group.label = guildName;
    for (const channel of guildChannels) {
      const option = document.createElement("option");
      option.value = channel.id;
      option.textContent = `# ${channel.name}`;
      group.append(option);
    }
    element.append(group);
  }
  if (selectedChannelId && !channels.some((channel) => channel.id === selectedChannelId)) {
    const option = document.createElement("option");
    option.value = selectedChannelId;
    option.textContent = `${t("unavailableChannel")} (${selectedChannelId})`;
    element.append(option);
  }
  element.value = selectedChannelId;
}

function handleServerMessage(event) {
  let message;
  try {
    message = JSON.parse(event.data);
  } catch {
    return;
  }
  if (message.type === "config") {
    bootstrap.config = message.payload;
    applyConfig(message.payload);
  } else if (message.type === "history") {
    replaceHistory(message.payload);
  } else if (message.type === "media") {
    if (message.payload) rememberMedia(message.payload);
  } else if (message.type === "audioPlayback") {
    if (message.payload?.media?.kind === "audio") updateAudioPlayback(message.payload);
  } else if (message.type === "clear") {
    history.length = 0;
    renderHistory();
  }
}

function scheduleReconnect() {
  if (isUnloading || reconnectTimer) {
    return;
  }
  reconnectTimer = window.setTimeout(() => {
    reconnectTimer = undefined;
    connectPanelSocket();
  }, reconnectDelayMs);
  reconnectDelayMs = Math.min(reconnectDelayMs * 2, 10000);
}

function connectPanelSocket() {
  window.clearTimeout(reconnectTimer);
  reconnectTimer = undefined;
  const previousSocket = socket;
  const nextSocket = new WebSocket(bootstrap.wsUrl);
  socket = nextSocket;
  previousSocket?.close();
  nextSocket.addEventListener("open", () => {
    reconnectDelayMs = 1000;
  });
  nextSocket.addEventListener("message", handleServerMessage);
  nextSocket.addEventListener("close", () => {
    if (socket === nextSocket) {
      scheduleReconnect();
    }
  });
  nextSocket.addEventListener("error", () => nextSocket.close());
}

function applyBootstrap(nextBootstrap, reconnect = false) {
  bootstrap = nextBootstrap;
  setBotStatus(bootstrap.bot);
  setServerStatus(bootstrap.server);
  setCredentials(bootstrap.credentials);
  setWidgetState(bootstrap.widget);
  setNotificationWidgetState(bootstrap.notificationWidget);
  applyConfig(bootstrap.config);
  replaceHistory(bootstrap.history);
  renderModeration();
  overlayUrlElement.value = bootstrap.overlayUrl;
  audioUrlElement.value = bootstrap.audioUrl;
  if (youtubeUrlElement) youtubeUrlElement.value = bootstrap.youtubeUrl || bootstrap.overlayUrl || "";
  if (ttsUrlElement) ttsUrlElement.value = bootstrap.ttsUrl;
  if (notificationUrlElement) notificationUrlElement.value = bootstrap.notificationUrl || bootstrap.overlayUrl || "";
  if (stickerUrlElement) stickerUrlElement.value = bootstrap.stickerUrl;
  applyMusicOverlaySize(bootstrap.config, true);
  inviteRowElement.hidden = !bootstrap.inviteUrl;
  inviteUrlElement.value = bootstrap.inviteUrl || "";
  // Live preview stays on /medias (not the OBS composite) so preview=1 works.
  const previewUrl = new URL(`http://127.0.0.1:${bootstrap.config.port}/medias`);
  previewUrl.searchParams.set("preview", "1");
  previewUrl.searchParams.set("sample", document.querySelector("#preview-sample")?.value || "portrait");
  if (previewElement.src !== previewUrl.href) {
    previewElement.src = previewUrl.href;
  }
  setOutputGeometryPreviewUrls();
  if (reconnect) {
    connectPanelSocket();
  }
}

function readConfigDraft(form, filterOnly = false) {
  if (filterOnly) {
    return { privacyConcepts: filterWordsToConcepts(privacyConceptsElement.value, bootstrap.config.privacyConcepts) };
  }
  if (form === messagesForm) {
    return {
      ttsChannelId: ttsChannelElement.value,
      ttsCleanupEnabled: ttsCleanupEnabledElement.checked,
      ttsWelcomeMessageId: ttsWelcomeMessageElement.value.trim(),
      notificationDurationMs: Number(notificationDurationElement.value) * 1000,
      ttsCharacterLimit: Number(ttsCharacterLimitElement.value),
      ttsQueueLimit: Number(ttsQueueLimitElement.value),
      ttsSpeechEnabled: false,
      ttsNotificationsObsEnabled: ttsNotificationsObsElement.checked,
    };
  }
  if (form === routingForm) {
    return {
      watchedChannelId: channelElement.value,
      mediaCleanupEnabled: mediaCleanupEnabledElement.checked,
      mediaWelcomeMessageId: mediaWelcomeMessageElement.value.trim(),
      port: Number(portElement.value),
    };
  }
  if (form === musicForm) {
    return {
      musicChannelId: musicChannelElement.value,
      musicWelcomeMessageId: musicWelcomeElement.value.trim(),
      musicCleanupEnabled: musicCleanupEnabledElement.checked,
    };
  }
  if (form === moderationForm) {
    return {
      honeypotChannelId: honeypotChannelElement.value,
      honeypotAction: honeypotActionElement.value,
      moderationEnabled: moderationEnabledElement.checked,
      moderationAllowImages: moderationAllowImagesElement.checked,
      moderationAllowVideos: moderationAllowVideosElement.checked,
      moderationAllowAudio: moderationAllowAudioElement.checked,
      privacyScanEnabled: privacyScanEnabledElement.checked,
      privacyConcepts: filterWordsToConcepts(privacyConceptsElement.value, bootstrap.config.privacyConcepts),
      privacyFilterExemptRoleIds: filterRoleIds(privacyExemptRoleIdsElement.value),
      privacyProtectionLevel: privacyProtectionLevelElement.value,
      privacyEnabledCategories: privacyCategoryElements.filter((input) => input.checked).map((input) => input.value),
      privacyBlockThreshold: privacyBlockThresholdElement.value,
      privacyReviewIntermediate: privacyReviewIntermediateElement.checked,
      privacyAutoDeleteBlockedMessages: privacyAutoDeleteBlockedMessagesElement.checked,
      privacyAllowlist: privacyListFromInput(privacyAllowlistElement.value),
      privacyCustomPatterns: privacyListFromInput(privacyCustomPatternsElement.value),
    };
  }
  if (form === mediaForm) {
    return {
      displayDurationMs: Number(durationElement.value) * 1000,
      gifDurationMs: Number(gifDurationElement.value) * 1000,
      stickerDurationMs: Number(stickerDurationElement.value) * 1000,
      mediaVolume: Number(mediaVolumeElement.value),
      showAuthor: showAuthorElement.checked,
      showMediaTextObs: showMediaTextObsElement.checked,
      showMediaTextWidget: showMediaTextWidgetElement.checked,
      widgetSoundEnabled: widgetSoundEnabledElement.checked,
    };
  }
  if (form === botPresenceForm) {
    return {
      botOnlineStatus: botOnlineStatusElement.value,
      botActivityType: botActivityTypeElement.value,
      botActivityText: botActivityTextElement.value,
    };
  }
  throw new Error("Unknown settings form");
}

async function saveConfig(stateElement, form, filterOnly = false) {
  const revision = formRevisions.get(form) || 0;
  setSaveState(stateElement, "saving");
  try {
    const draft = readConfigDraft(form, filterOnly);
    await queueConfigSave(async () => {
      // The backend accepts a full config. Merge only this form's captured fields.
      const current = await invoke("get_bootstrap");
      const nextBootstrap = await invoke("apply_config", {
        config: { ...current.config, ...draft },
      });
      let complete = !filterOnly;
      if (filterOnly) {
        try {
          complete = Object.entries(readConfigDraft(form)).every(([key, value]) =>
            JSON.stringify(value) === JSON.stringify(nextBootstrap.config[key]));
        } catch { /* An unfinished field remains a draft. */ }
      }
      if (complete) finishFormSave(form, revision, stateElement);
      else setSaveState(stateElement, "unsaved");
      applyBootstrap(nextBootstrap, current.config.port !== nextBootstrap.config.port);
    });
    return true;
  } catch (error) {
    setSaveState(stateElement, "error", String(error));
    return false;
  }
}

async function saveMediaCaptionVisibility() {
  const generation = ++mediaCaptionSaveGeneration;
  setSaveState(mediaSaveStateElement, "saving");
  try {
    const config = await invoke("set_media_caption_visibility", {
      showMediaTextObs: showMediaTextObsElement.checked,
      showMediaTextWidget: showMediaTextWidgetElement.checked,
    });
    if (generation !== mediaCaptionSaveGeneration) return;
    bootstrap.config = config;
    setSaveState(mediaSaveStateElement, "saved");
  } catch (error) {
    if (generation === mediaCaptionSaveGeneration) {
      setSaveState(mediaSaveStateElement, "error", String(error));
    }
  }
}

function schedulePrivacyFilterSave() {
  privacyFilterDraft = privacyConceptsElement.value;
  const generation = ++privacyFilterSaveGeneration;
  window.clearTimeout(privacyFilterSaveTimer);
  privacyFilterSaveTimer = window.setTimeout(() => {
    privacyFilterSaveTimer = undefined;
    void savePrivacyFiltersAutomatically(generation);
  }, 750);
}

async function savePrivacyFiltersAutomatically(generation) {
  if (generation !== privacyFilterSaveGeneration
    || !bootstrap?.config
    || !filterWordsAreSaveable(privacyFilterDraft)) {
    return;
  }

  const saved = await saveConfig(moderationSaveStateElement, moderationForm, true);
  if (!saved || generation === privacyFilterSaveGeneration) {
    return;
  }

  privacyConceptsElement.value = privacyFilterDraft;
  schedulePrivacyFilterSave();
}

let statusRefreshInFlight = false;
let lastChannelsSignature;
let lastPendingSignature;

function applyNotificationSoundConfig(config) {
  notificationSoundEnabledElement.checked = Boolean(config.notificationSoundEnabled);
  notificationSoundObsElement.checked = Boolean(config.notificationSoundObsEnabled);
  const soundPath = config.notificationSoundPath || "";
  notificationSoundStateElement.textContent = soundPath
    ? soundPath.split(/[\\/]/).pop()
    : t("noNotificationSound");
}

async function refreshRuntimeStatus() {
  if (statusRefreshInFlight) {
    return;
  }
  statusRefreshInFlight = true;
  try {
    const status = await invoke("get_runtime_status");
    void moduleControls?.refresh();
    bootstrap.bot = status.bot;
    bootstrap.server = status.server;
    bootstrap.widget = status.widget;
    bootstrap.notificationWidget = status.notificationWidget;
    bootstrap.channels = status.channels;
    bootstrap.pendingMedia = status.pendingMedia;
    renderQueue(status.queue || []);
    setBotStatus(status.bot);
    setServerStatus(status.server);
    setWidgetState(status.widget);
    setNotificationWidgetState(status.notificationWidget);
    const pendingSignature = JSON.stringify(status.pendingMedia.map((item) => item.id));
    if (pendingSignature !== lastPendingSignature) {
      lastPendingSignature = pendingSignature;
      renderModeration();
    }
    const channelsSignature = JSON.stringify(status.channels);
    const selectingChannel = document.activeElement === channelElement
      || document.activeElement === ttsChannelElement
      || document.activeElement === musicChannelElement
      || document.activeElement === honeypotChannelElement;
    if (channelsSignature !== lastChannelsSignature && !selectingChannel) {
      lastChannelsSignature = channelsSignature;
      populateChannels(channelElement, status.channels, channelElement.value, t("selectChannel"));
      populateChannels(ttsChannelElement, status.channels, ttsChannelElement.value, t("ttsDisabled"));
      populateChannels(musicChannelElement, status.channels, musicChannelElement.value, t("musicDisabled"));
      populateChannels(honeypotChannelElement, status.channels, honeypotChannelElement.value, t("honeypotDisabled"));
    }
  } catch {
    setServerStatus({ connected: false, overlayClients: 0 });
    setBotStatus({ connected: false });
  } finally {
    statusRefreshInFlight = false;
  }
}

for (const button of $$("[data-page-target]")) {
  button.addEventListener("click", () => showPage(button.dataset.pageTarget));
}

navigationBackButton.addEventListener("click", () => navigateHistory(-1));
navigationForwardButton.addEventListener("click", () => navigateHistory(1));

settingsSearchElement.addEventListener("input", renderSettingsSearchResults);
settingsSearchElement.addEventListener("focus", renderSettingsSearchResults);
settingsSearchElement.addEventListener("keydown", (event) => {
  const results = $$(".settings-search__result", settingsSearchResultsElement);
  if (event.key === "ArrowDown" && results.length) {
    event.preventDefault();
    results[0].focus();
  }
});
settingsSearchResultsElement.addEventListener("keydown", (event) => {
  const results = $$(".settings-search__result", settingsSearchResultsElement);
  const index = results.indexOf(document.activeElement);
  if (event.key === "ArrowDown" && index < results.length - 1) {
    event.preventDefault();
    results[index + 1].focus();
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    if (index > 0) results[index - 1].focus();
    else settingsSearchElement.focus();
  }
});
settingsSearchClearButton.addEventListener("click", () => {
  settingsSearchElement.value = "";
  renderSettingsSearchResults();
  settingsSearchElement.focus();
});

for (const button of $$("[data-help-link]")) {
  button.addEventListener("click", () => invoke("open_help_link", { link: button.dataset.helpLink }));
}

updateCheckButton.addEventListener("click", async () => {
  setUpdateMenuOpen(true);
  updateUiState = { kind: "checking" };
  renderUpdateStatus();
  try {
    latestUpdate = await invoke("check_for_updates");
    setAppVersion(latestUpdate.currentVersion);
    updateUiState = {
      kind: latestUpdate.updateAvailable ? "available" : "current",
      version: latestUpdate.latestVersion,
    };
  } catch (error) {
    latestUpdate = undefined;
    updateUiState = { kind: "error", errorKey: "updateCheckFailed", error: String(error) };
  }
  renderUpdateStatus();
});

installUpdateButton.addEventListener("click", async () => {
  updateUiState = { kind: "installing", version: latestUpdate.latestVersion };
  renderUpdateStatus();
  try {
    await invoke("download_and_install_update");
  } catch (error) {
    updateUiState = { kind: "error", errorKey: "updateInstallFailed", error: String(error) };
    renderUpdateStatus();
  }
});

updateMenuCloseButton.addEventListener("click", () => {
  setUpdateMenuOpen(false);
  updateCheckButton.focus();
});

document.addEventListener("pointerdown", (event) => {
  if (!updateMenuElement.hidden && !updateControlElement.contains(event.target)) {
    setUpdateMenuOpen(false);
  }
  if (!settingsSearchResultsElement.hidden && !settingsSearchControl.contains(event.target)) {
    closeSettingsSearch();
  }
  if (!interfaceLanguageOptionsElement.hidden && !interfaceLanguageElement.contains(event.target)) {
    setLanguageMenuOpen(false);
  }
  if (!sidebarLanguageOptionsElement.hidden && !sidebarLanguagePickerElement.contains(event.target)) {
    setSidebarLanguageMenuOpen(false);
  }
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    if (!updateMenuElement.hidden) {
      setUpdateMenuOpen(false);
      updateCheckButton.focus();
    }
    if (!settingsSearchResultsElement.hidden) {
      closeSettingsSearch();
      settingsSearchElement.focus();
    }
    if (!interfaceLanguageOptionsElement.hidden) {
      setLanguageMenuOpen(false);
      interfaceLanguageButton.focus();
    }
    if (!sidebarLanguageOptionsElement.hidden) {
      setSidebarLanguageMenuOpen(false);
      languageToggleButton.focus();
    }
  }
  if ((event.ctrlKey || event.metaKey) && event.key.toLocaleLowerCase() === "k") {
    event.preventDefault();
    settingsSearchElement.focus();
    settingsSearchElement.select();
  }
  if (event.altKey && !event.ctrlKey && !event.metaKey && ["ArrowLeft", "ArrowRight"].includes(event.key)) {
    event.preventDefault();
    navigateHistory(event.key === "ArrowLeft" ? -1 : 1);
  }
});

$("#privacy-reference").addEventListener("click", () => {
  showPage("help");
  const privacyDetails = $("#privacy-details");
  privacyDetails.open = true;
  window.requestAnimationFrame(() => privacyDetails.scrollIntoView({ behavior: "smooth", block: "start" }));
});

languageToggleButton.addEventListener("click", () => {
  setSidebarLanguageMenuOpen(sidebarLanguageOptionsElement.hidden);
});

themeToggleButton.addEventListener("click", () => {
  theme = theme === "light" ? "dark" : "light";
  applyTheme();
  applyPersonalization();
});

interfaceLanguageButton.addEventListener("click", () => {
  setLanguageMenuOpen(interfaceLanguageOptionsElement.hidden);
});

for (const option of $$("[data-locale]", interfaceLanguageOptionsElement)) {
  option.addEventListener("click", () => {
    selectInterfaceLanguage(option.dataset.locale, interfaceLanguageButton);
  });
}

sidebarLanguageOptionsElement.addEventListener("click", (event) => {
  const option = event.target.closest("[data-locale]");
  if (option) selectInterfaceLanguage(option.dataset.locale, languageToggleButton);
});

interfaceThemeElement.addEventListener("change", () => {
  theme = interfaceThemeElement.value;
  applyTheme();
  applyPersonalization();
});

interfaceFontElement.addEventListener("change", () => {
  interfaceFont = interfaceFontElement.value;
  applyPersonalization();
});

sidebarLayoutElement.addEventListener("change", () => {
  sidebarLayout = sidebarLayoutElement.value;
  sidebarExpanded = false;
  applySidebarLayout();
});

sidebarElement.addEventListener("pointerenter", () => setDynamicSidebarExpanded(true));
sidebarElement.addEventListener("pointerleave", () => setDynamicSidebarExpanded(false));
sidebarElement.addEventListener("focusin", () => setDynamicSidebarExpanded(true));
sidebarElement.addEventListener("focusout", () => {
  window.requestAnimationFrame(() => {
    if (!sidebarElement.contains(document.activeElement)) setDynamicSidebarExpanded(false);
  });
});

for (const input of designInputs) {
  input.addEventListener("change", () => {
    design = input.value;
    applyDesign();
    applyPersonalization();
    designPickerElement.open = false;
  });
}

for (const [index, input] of accentInputs.entries()) {
  input.addEventListener("input", () => {
    accentRgb[index] = clamp(input.value, 0, 255);
    applyPersonalization();
  });
}

accentPickerElement.addEventListener("input", () => {
  accentRgb = hexToRgb(accentPickerElement.value);
  applyPersonalization();
});

fontScaleElement.addEventListener("input", () => {
  fontScale = clamp(fontScaleElement.value, 80, 140);
  applyPersonalization();
});

resetPersonalizationButton.addEventListener("click", () => {
  locale = "en-US";
  language = "en";
  theme = "dark";
  design = "openai";
  interfaceFont = "design";
  sidebarLayout = "fixed";
  sidebarExpanded = false;
  accentRgb = [88, 185, 137];
  fontScale = 100;
  applyLanguage();
  applyTheme();
  applyDesign();
  applySidebarLayout();
  applyPersonalization();
});

mediaVolumeElement.addEventListener("input", () => {
  mediaVolumeValueElement.value = `${mediaVolumeElement.value}%`;
  mediaVolumeValueElement.textContent = `${mediaVolumeElement.value}%`;
});

credentialForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  setSaveState(credentialStateElement, "saving", t("encrypting"));
  try {
    applyBootstrap(await invoke("save_credentials", {
      clientId: clientIdElement.value.trim(),
      token: tokenElement.value.trim(),
    }));
    setSaveState(credentialStateElement, "saved", t("encryptedStarting"));
  } catch (error) {
    setSaveState(credentialStateElement, "error", String(error));
  } finally {
    tokenElement.value = "";
  }
});

botActivityTypeElement.addEventListener("change", updateBotActivityAvailability);

botPresenceForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  await saveConfig(botPresenceSaveStateElement, botPresenceForm);
});

routingForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  await saveConfig(saveStateElement, routingForm);
});

messagesForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  await saveConfig(messagesSaveStateElement, messagesForm);
});

musicForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  setSaveState(musicSaveStateElement, "saving");
  try {
    const youtubeApiKey = youtubeApiKeyElement.value.trim();
    if (youtubeApiKey) {
      const credentials = await invoke("store_youtube_api_key", { youtubeApiKey });
      if (bootstrap) bootstrap.credentials = credentials;
      updateYoutubeKeyStatus(credentials);
    }
    await saveConfig(musicSaveStateElement, musicForm);
  } catch (error) {
    setSaveState(musicSaveStateElement, "error", String(error));
  } finally {
    youtubeApiKeyElement.value = "";
  }
});

let musicCleanupToken;
const musicCleanupStatus = $("#music-cleanup-status");
const musicCleanupConfirmation = $("#music-cleanup-confirmation");
const cleanupPreviewButton = $("#music-cleanup-preview");
const cleanupConfirmButton = $("#music-cleanup-confirm");
function clearMusicCleanupPreview() {
  musicCleanupToken = undefined;
  musicCleanupConfirmation.hidden = true;
}
musicForm.addEventListener("input", clearMusicCleanupPreview);
$("#music-cleanup-cancel").addEventListener("click", () => {
  clearMusicCleanupPreview();
  musicCleanupStatus.textContent = "";
});
cleanupPreviewButton.addEventListener("click", async () => {
  clearMusicCleanupPreview();
  if (dirtyForms.has(musicForm)) {
    musicCleanupStatus.textContent = t("musicCleanupSaveFirst");
    return;
  }
  cleanupPreviewButton.disabled = true;
  try {
    const preview = await invoke("preview_music_cleanup");
    musicCleanupToken = preview.token;
    musicCleanupStatus.textContent = t("musicCleanupCount").replace("{count}", preview.count)
      + (preview.limitReached ? " " + t("musicCleanupLimit") : "");
    musicCleanupConfirmation.hidden = preview.count === 0;
  } catch (error) {
    musicCleanupStatus.textContent = String(error);
  } finally { cleanupPreviewButton.disabled = false; }
});
cleanupConfirmButton.addEventListener("click", async () => {
  if (!musicCleanupToken) return;
  const token = musicCleanupToken;
  clearMusicCleanupPreview();
  cleanupPreviewButton.disabled = true;
  cleanupConfirmButton.disabled = true;
  try {
    const result = await invoke("confirm_music_cleanup", { token });
    musicCleanupStatus.textContent = t("musicCleanupDone")
      .replace("{deleted}", result.deleted).replace("{failed}", result.failed).replace("{skipped}", result.skipped);
  } catch (error) {
    musicCleanupStatus.textContent = String(error);
  } finally {
    cleanupPreviewButton.disabled = false;
    cleanupConfirmButton.disabled = false;
  }
});

refreshChannelsButton.addEventListener("click", async () => {
  refreshChannelsButton.disabled = true;
  setSaveState(saveStateElement, "idle", "");
  try {
    const channels = await invoke("refresh_channels");
    bootstrap.channels = channels;
    populateChannels(channelElement, channels, channelElement.value, t("selectChannel"));
    populateChannels(ttsChannelElement, channels, ttsChannelElement.value, t("ttsDisabled"));
    populateChannels(musicChannelElement, channels, musicChannelElement.value, t("musicDisabled"));
    populateChannels(honeypotChannelElement, channels, honeypotChannelElement.value, t("honeypotDisabled"));
    setSaveState(saveStateElement, "saved", t("channelsRefreshed"));
  } catch (error) {
    setSaveState(saveStateElement, "error", String(error));
  } finally {
    refreshChannelsButton.disabled = false;
  }
});

mediaForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  await saveConfig(mediaSaveStateElement, mediaForm);
});

for (const input of [showMediaTextObsElement, showMediaTextWidgetElement]) {
  input.addEventListener("change", () => void saveMediaCaptionVisibility());
}

moderationForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  await saveConfig(moderationSaveStateElement, moderationForm);
});

privacyConceptsElement.addEventListener("input", schedulePrivacyFilterSave);

commandsForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const revision = formRevisions.get(commandsForm) || 0;
  const settings = Object.fromEntries(Object.entries(commandInputs).map(([name, input]) => [name, input.checked]));
  setSaveState(commandsSaveStateElement, "saving");
  try {
    await queueConfigSave(async () => {
      const config = await invoke("save_command_settings", { settings });
      bootstrap.config = config;
      finishFormSave(commandsForm, revision, commandsSaveStateElement);
      applyConfig(config);
    });
  } catch (error) {
    setSaveState(commandsSaveStateElement, "error", String(error));
  }
});

addCustomCommandButton.addEventListener("click", () => openCustomCommandEditor());
cancelCustomCommandButton.addEventListener("click", closeCustomCommandEditor);

customCommandNameElement.addEventListener("input", () => {
  const normalized = customCommandNameElement.value
    .toLowerCase()
    .replace(/\s+/g, "-")
    .replace(/[^a-z0-9_-]/g, "");
  if (normalized !== customCommandNameElement.value) customCommandNameElement.value = normalized;
  customCommandPreviewElement.textContent = `/relay ${normalized || "command"}`;
  customCommandEditorStateElement.textContent = "";
});

customCommandActionElement.addEventListener("change", () => {
  renderCustomActionFields(defaultCustomAction(customCommandActionElement.value));
  customCommandEditorStateElement.textContent = "";
});

customActionFieldsElement.addEventListener("change", (event) => {
  if (event.target.matches("[data-custom-parameter-mode]")) {
    updateCustomParameterAvailability(event.target.closest("[data-custom-parameter]"));
  }
  customCommandEditorStateElement.textContent = "";
});

customCommandForm.addEventListener("submit", (event) => {
  event.preventDefault();
  customCommandEditorStateElement.textContent = "";
  try {
    const definition = collectCustomCommandDraft();
    if (!definition) return;
    if (editingCustomCommandIndex === null) customCommands.push(definition);
    else customCommands[editingCustomCommandIndex] = definition;
    customCommandsDirty = true;
    renderCustomCommands();
    closeCustomCommandEditor();
    setSaveState(customCommandsSaveStateElement, "unsaved", t("customDraftSaved"));
  } catch (error) {
    customCommandEditorStateElement.textContent = String(error.message || error);
  }
});

customCommandListElement.addEventListener("click", (event) => {
  const button = event.target.closest("[data-custom-command-action]");
  if (!button) return;
  const index = Number(button.dataset.customCommandIndex);
  if (!Number.isInteger(index) || !customCommands[index]) return;
  if (button.dataset.customCommandAction === "edit") {
    openCustomCommandEditor(index);
    return;
  }
  customCommands.splice(index, 1);
  customCommandsDirty = true;
  closeCustomCommandEditor();
  renderCustomCommands();
  setSaveState(customCommandsSaveStateElement, "unsaved", t("customUnsaved"));
});

syncCustomCommandsButton.addEventListener("click", async () => {
  setSaveState(customCommandsSaveStateElement, "saving", t("customValidating"));
  const names = new Set();
  const invalidName = customCommands.some((command) => {
    if (defaultRelayCommandNames.has(command.name) || names.has(command.name)) return true;
    names.add(command.name);
    return false;
  });
  if (customCommands.length > 16 || invalidName) {
    setSaveState(customCommandsSaveStateElement, "error", t("customDuplicateName"));
    return;
  }
  syncCustomCommandsButton.disabled = true;
  addCustomCommandButton.disabled = true;
  setSaveState(customCommandsSaveStateElement, "saving", t("customSyncing"));
  try {
    const config = await invoke("save_custom_commands", { commands: cloneCustomCommands(customCommands) });
    customCommandsDirty = false;
    customCommands = cloneCustomCommands(config.customCommands);
    bootstrap.config = config;
    applyConfig(config);
    try {
      applyBootstrap(await invoke("get_bootstrap"));
    } catch {
      renderCustomCommands();
    }
    setSaveState(customCommandsSaveStateElement, "saved", t("customActive"));
  } catch (error) {
    customCommandsDirty = true;
    setSaveState(customCommandsSaveStateElement, "error", String(error));
  } finally {
    syncCustomCommandsButton.disabled = false;
    addCustomCommandButton.disabled = customCommands.length >= 16;
  }
});

clearPendingMediaButton.addEventListener("click", async () => {
  try {
    await invoke("clear_pending_media");
    bootstrap.pendingMedia = [];
    renderModeration();
  } catch (error) {
    setSaveState(moderationSaveStateElement, "error", String(error));
  }
});

async function copyValue(button, value) {
  button.disabled = true;
  try {
    await navigator.clipboard.writeText(value);
    button.textContent = t("copied");
    button.dataset.state = "saved";
  } catch {
    button.textContent = t("copyFailed");
    button.dataset.state = "error";
  } finally {
    window.setTimeout(() => {
      button.textContent = t("copy");
      delete button.dataset.state;
      button.disabled = false;
    }, 1200);
  }
}

copyUrlButton.addEventListener("click", () => copyValue(copyUrlButton, overlayUrlElement.value));
copyAudioUrlButton.addEventListener("click", () => copyValue(copyAudioUrlButton, audioUrlElement.value));
if (copyYoutubeUrlButton && youtubeUrlElement) {
  copyYoutubeUrlButton.addEventListener("click", () => copyValue(copyYoutubeUrlButton, youtubeUrlElement.value));
}
if (copyTtsUrlButton && ttsUrlElement) {
  copyTtsUrlButton.addEventListener("click", () => copyValue(copyTtsUrlButton, ttsUrlElement.value));
}
if (copyNotificationUrlButton && notificationUrlElement) {
  copyNotificationUrlButton.addEventListener("click", () => copyValue(copyNotificationUrlButton, notificationUrlElement.value));
}
if (copyStickerUrlButton && stickerUrlElement) {
  copyStickerUrlButton.addEventListener("click", () => copyValue(copyStickerUrlButton, stickerUrlElement.value));
}
openInviteButton.addEventListener("click", () => invoke("open_help_link", { link: inviteUrlElement.value }));

if (saveMusicOverlayButton) {
  saveMusicOverlayButton.addEventListener("click", async () => {
    if (musicOverlaySaveStateElement) musicOverlaySaveStateElement.textContent = t("saving");
    try {
      const config = await invoke("set_music_widget_size", {
        width: clamp(musicWidgetWidthElement.value, 160, 16384),
        height: clamp(musicWidgetHeightElement.value, 90, 16384),
      });
      bootstrap.config = config;
      applyOutputGeometryTarget(config, "mediaWidget", true);
      if (musicOverlaySaveStateElement) musicOverlaySaveStateElement.textContent = t("saved");
    } catch (error) {
      if (musicOverlaySaveStateElement) musicOverlaySaveStateElement.textContent = String(error);
    }
  });
}

for (const [target, button] of outputTestButtons) {
  button.addEventListener("click", async () => {
    if (button.disabled) return;
    button.disabled = true;
    try {
      await invoke("test_output", { target });
      button.textContent = t("outputTestSent");
    } catch (error) {
      button.textContent = t("outputTestFailed");
      button.title = String(error);
    }
    window.setTimeout(() => {
      button.textContent = t("testOutput");
      renderOutputReadiness(bootstrap?.server);
    }, 1600);
  });
}

regenerateSecretButton.addEventListener("click", async () => {
  setSaveState(saveStateElement, "saving", t("regenerating"));
  try {
    applyBootstrap(await invoke("regenerate_secret"), true);
    setSaveState(saveStateElement, "saved", t("secretRegenerated"));
  } catch (error) {
    setSaveState(saveStateElement, "error", String(error));
  }
});

toggleWidgetButton.addEventListener("click", async () => {
  try {
    setWidgetState(await invoke("toggle_widget"));
  } catch (error) {
    setSaveState(saveStateElement, "error", String(error));
  }
});

lockWidgetButton.addEventListener("click", async () => {
  try {
    setWidgetState(await invoke("set_widget_locked", { locked: !bootstrap.widget.locked }));
  } catch (error) {
    setSaveState(saveStateElement, "error", String(error));
  }
});

notificationWidgetEnabledElement.addEventListener("change", async () => {
  try {
    setNotificationWidgetState(await invoke("set_notification_widget_visible", {
      visible: notificationWidgetEnabledElement.checked,
    }));
  } catch (error) {
    notificationWidgetEnabledElement.checked = !notificationWidgetEnabledElement.checked;
    setSaveState(saveStateElement, "error", String(error));
  }
});

lockNotificationWidgetButton.addEventListener("click", async () => {
  try {
    setNotificationWidgetState(await invoke("set_notification_widget_locked", {
      locked: !bootstrap.notificationWidget.locked,
    }));
  } catch (error) {
    setSaveState(saveStateElement, "error", String(error));
  }
});

notificationSoundEnabledElement.addEventListener("change", async () => {
  try {
    const config = await invoke("set_notification_sound_enabled", {
      enabled: notificationSoundEnabledElement.checked,
    });
    bootstrap.config = config;
    applyNotificationSoundConfig(config);
  } catch (error) {
    notificationSoundEnabledElement.checked = !notificationSoundEnabledElement.checked;
    notificationSoundStateElement.textContent = String(error);
  }
});

notificationSoundObsElement.addEventListener("change", async () => {
  try {
    const config = await invoke("set_notification_sound_obs_enabled", {
      enabled: notificationSoundObsElement.checked,
    });
    bootstrap.config = config;
    applyNotificationSoundConfig(config);
  } catch (error) {
    notificationSoundObsElement.checked = !notificationSoundObsElement.checked;
    notificationSoundStateElement.textContent = String(error);
  }
});

pickNotificationSoundButton.addEventListener("click", async () => {
  pickNotificationSoundButton.disabled = true;
  try {
    const config = await invoke("pick_notification_sound");
    if (config) {
      bootstrap.config = config;
      applyNotificationSoundConfig(config);
    }
  } catch (error) {
    notificationSoundStateElement.textContent = String(error);
  } finally {
    pickNotificationSoundButton.disabled = false;
  }
});

clearNotificationSoundButton.addEventListener("click", async () => {
  try {
    const config = await invoke("clear_notification_sound");
    bootstrap.config = config;
    applyNotificationSoundConfig(config);
  } catch (error) {
    notificationSoundStateElement.textContent = String(error);
  }
});

skipMediaButton.addEventListener("click", async () => {
  try {
    await invoke("skip_media");
    setSaveState(mediaSaveStateElement, "saved", t("skipped"));
  } catch (error) {
    setSaveState(mediaSaveStateElement, "error", String(error));
  }
});

skipShortcutCaptureButton.addEventListener("click", beginShortcutCapture);
window.addEventListener("keydown", (event) => {
  if (!shortcutCaptureActive) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.key === "Escape") {
    cancelShortcutCapture();
    setSaveState(mediaSaveStateElement, "saved", t("shortcutCanceled"));
    return;
  }
  const token = shortcutTokenFromEvent(event);
  if (!token) return;
  const modifiers = [];
  if (event.ctrlKey) modifiers.push("control");
  if (event.altKey) modifiers.push("alt");
  if (event.shiftKey) modifiers.push("shift");
  if (event.metaKey) modifiers.push("super");
  void saveCapturedShortcut([...modifiers, token].join("+"));
});

previousAudioButton.addEventListener("click", () => controlCurrentAudio("previous"));
toggleAudioButton.addEventListener("click", () => controlCurrentAudio(
  currentAudioPlayback?.status === "paused" ? "resume" : "pause",
));
skipAudioButton.addEventListener("click", () => controlCurrentAudio("skip"));

clearOverlayButton.addEventListener("click", async () => {
  try {
    await invoke("clear_overlay");
  } catch (error) {
    setSaveState(mediaSaveStateElement, "error", String(error));
  }
});

window.addEventListener("beforeunload", () => {
  isUnloading = true;
  window.clearTimeout(reconnectTimer);
  window.clearInterval(statusTimer);
  const pendingGeometry = [...outputGeometryTimers.keys()];
  for (const target of pendingGeometry) {
    window.clearTimeout(outputGeometryTimers.get(target));
    outputGeometryTimers.delete(target);
    void persistOutputGeometry(target);
  }
  socket?.close();
  releaseNowPlayingArtwork();
});

function updateSampleTestButton() {
  const sample = document.querySelector("#preview-sample").value;
  const target = ["portrait", "landscape", "gif", "video"].includes(sample) ? "visual" : sample;
  const output = bootstrap?.server?.outputs?.[target] || {};
  const connected = Number(output.obsClients || 0) + Number(output.widgetClients || 0) > 0;
  const button = document.querySelector("#send-preview-sample");
  button.disabled = !connected;
  button.title = connected ? "" : t("outputTestNeedsLiveOutput");
}

function updateSamplePreview() {
  if (!bootstrap) return;
  updateSampleTestButton();
  const sample = document.querySelector("#preview-sample").value;
  const notification = sample === "notification" || sample === "sticker";
  const url = new URL("http://127.0.0.1:" + bootstrap.config.port + (notification ? "/notifications" : "/medias"));
  url.searchParams.set("preview", "1"); url.searchParams.set("sample", sample);
  previewElement.src = url.href;
}
document.querySelector("#preview-sample").addEventListener("change", updateSamplePreview);
document.querySelector("#send-preview-sample").addEventListener("click", async (event) => {
  const button = event.currentTarget; button.disabled = true;
  const state = document.querySelector("#preview-sample-state");
  try { await invoke("preview_output_sample", { sample: document.querySelector("#preview-sample").value }); state.textContent = t("outputTestSent"); }
  catch (error) { state.textContent = String(error); }
  finally { updateSampleTestButton(); }
});
document.querySelector("#history-search").addEventListener("input", renderHistory);
document.querySelector("#history-kind").addEventListener("change", renderHistory);
initializeOutputGeometryControls();
applyLanguage();
applyTheme();
applyDesign();
applySidebarLayout();
applyPersonalization();
showPage(currentPage);

try {
  invoke("get_app_version").then(setAppVersion).catch(() => {});
  invoke("get_changelog_markdown").then((markdown) => {
    bundledChangelogMarkdown = String(markdown || "");
    renderChangelog();
  }).catch(() => renderChangelog());
  applyBootstrap(await invoke("get_bootstrap"));
  moduleControls = initializeModules({ invoke, t, getBootstrap: () => bootstrap });
  connectPanelSocket();
  statusTimer = window.setInterval(refreshRuntimeStatus, 1500);
} catch (error) {
  setSaveState(saveStateElement, "error", String(error));
  setSaveState(credentialStateElement, "error", String(error));
}
