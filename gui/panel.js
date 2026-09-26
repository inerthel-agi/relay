import { initializePresetControls } from "./output-presets.mjs";
import { initializeModules } from "./modules.mjs";
import { filterConceptsToLines, filterRoleIds, filterRoleIdsToInput, filterWordsAreSaveable, filterWordsToConcepts, privacyListFromInput, privacyListToInput } from "./privacy-filters.mjs";
import { appendChangelogMarkdown, changelogBodyForLanguage, parseChangelogReleases } from "./changelog-markdown.mjs";
import { buildDiagnosticReport, errorCategory } from "./diagnostics.mjs";
import { initializeCustomCommands } from "./custom-commands.mjs";
import { initializeOverview, readStorage } from "./overview.mjs";
import { initializeSettingsSearch } from "./settings-search.mjs";
import { initializeAutosave, initializeStartWithWindows } from "./autosave.mjs";
import { initializePanic } from "./panic.mjs";
import { initializeDiscordCheck } from "./discord-check.mjs";
import { initializeObsSetup } from "./obs-setup.mjs";
import { initializeModerationUi } from "./moderation-ui.mjs";
import { initializeStreamStyle, normalizeStreamBackground, normalizeStreamStyle } from "./stream-style.mjs";
const { invoke } = window.__TAURI__.core;
let moduleControls;

import { translations, regionalTranslations } from "./translations.mjs";

const pageMetadata = {
  overview: { title: "navOverview", kicker: "navGroupStart" },
  discord: { title: "navDiscord", kicker: "navGroupStart" },
  help: { title: "navHelp", kicker: "navGroupStart" },
  messages: { title: "moduleMessages", kicker: "navGroupContent" },
  media: { title: "navMedia", kicker: "navGroupContent" },
  music: { title: "navMusic", kicker: "navGroupContent" },
  reactions: { title: "moduleReactions", kicker: "navGroupContent" },
  overlay: { title: "navOverlay", kicker: "navGroupBroadcast" },
  history: { title: "navHistory", kicker: "navGroupBroadcast" },
  moderation: { title: "navModeration", kicker: "navGroupSafety" },
  commands: { title: "navCommands", kicker: "navGroupSafety" },
  personalization: { title: "navPersonalization", kicker: "navGroupApp" },
  changelog: { title: "navChangelog", kicker: "navGroupApp" },
  about: { title: "navAbout", kicker: "navGroupApp" },
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
const systemForm = $("#system-form");
const musicForm = $("#music-form");
const mediaForm = $("#media-form");
const messagesForm = $("#messages-form");
const moderationForm = $("#moderation-form");
const commandsForm = $("#commands-form");
const dirtyForms = new Set();
const formRevisions = new WeakMap();
for (const form of [botPresenceForm, routingForm, systemForm, musicForm, mediaForm, messagesForm, moderationForm, commandsForm]) {
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
const panicShortcutCaptureButton = $("#panic-shortcut-capture");
const panicShortcutValueElement = $("#panic-shortcut-value");
const languageToggleButton = $("#language-toggle");
const languageValueElement = $("#language-value");
const languageFlagElement = $("#language-flag");
const themeToggleButton = $("#theme-toggle");
const themeValueElement = $("#theme-value");
const pageTitleElement = $("#page-title");
const pageKickerElement = $("#page-kicker");
const navigationBackButton = $("#navigation-back");
const navigationForwardButton = $("#navigation-forward");
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
const supportedDesigns = ["graphite", "paper", "neo-brutalism", "gridline", "lumen", "signal"];
// Earlier releases stored brand-named design identifiers.
const legacyDesignNames = { openai: "graphite", anthropic: "paper" };
const supportedSidebarLayouts = ["fixed", "compact", "dynamic"];
const supportedInterfaceFonts = [
  "design", "bricolage", "dm-sans", "figtree", "inter",
  "jetbrains-mono", "manrope", "poppins", "space-grotesk",
];
const storedLanguage = localStorage.getItem("relay-language") || "en";
let locale = localStorage.getItem("relay-locale") || defaultLocaleByLanguage[storedLanguage] || "en-US";
if (!languageOptionByLocale.has(locale)) locale = "en-US";
let language = languageOptionByLocale.get(locale).language;
let design = localStorage.getItem("relay-design") || "graphite";
design = legacyDesignNames[design] || design;
if (!supportedDesigns.includes(design)) design = "graphite";
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
let currentAppVersion = "1.4.0";
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
  for (const element of $$("[data-i18n-aria-label]", root)) {
    element.setAttribute("aria-label", t(element.dataset.i18nAriaLabel));
  }
  for (const element of $$("[data-i18n-title]", root)) {
    element.title = t(element.dataset.i18nTitle);
  }
  // Compact sidebars show icons only; keep each page name available on hover.
  for (const button of $$(".navigation__item", root)) {
    button.title = button.querySelector(".navigation__label")?.textContent || "";
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
  if (open) {
    setSidebarLanguageMenuOpen(false);
    focusSelectedOption(interfaceLanguageOptionsElement);
  }
}

function setSidebarLanguageMenuOpen(open) {
  sidebarLanguageOptionsElement.hidden = !open;
  languageToggleButton.setAttribute("aria-expanded", String(open));
  if (open) {
    setLanguageMenuOpen(false);
    focusSelectedOption(sidebarLanguageOptionsElement);
  }
}

function focusSelectedOption(listbox) {
  const options = $$('[role="option"]', listbox);
  (options.find((option) => option.getAttribute("aria-selected") === "true") || options[0])?.focus();
}

// Arrow, Home and End keys move between options of an open listbox.
function handleListboxKeys(listbox, event) {
  const options = $$('[role="option"]', listbox);
  const index = options.indexOf(document.activeElement);
  const target = {
    ArrowDown: Math.min(index + 1, options.length - 1),
    ArrowUp: Math.max(index - 1, 0),
    Home: 0,
    End: options.length - 1,
  }[event.key];
  if (target === undefined || !options.length) return;
  event.preventDefault();
  options[target].focus();
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
  // Both pickers render the same option list from languageOptions.
  interfaceLanguageOptionsElement.replaceChildren(...languageOptionButtons(selected, "language-picker__option"));
  sidebarLanguageOptionsElement.replaceChildren(
    ...languageOptionButtons(selected, "language-picker__option sidebar-language-picker__option"),
  );
}

function languageOptionButtons(selected, className) {
  return languageOptions.map((option) => {
    const button = document.createElement("button");
    button.className = className;
    button.type = "button";
    button.dataset.locale = option.locale;
    button.setAttribute("role", "option");
    button.setAttribute("aria-selected", String(option.locale === selected.locale));
    button.innerHTML = `<img class="flag-icon" src="./assets/flags/${option.flag}.svg" alt=""><span>${option.label}</span><i aria-hidden="true">✓</i>`;
    return button;
  });
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
  settingsSearch.applyLanguage();
  renderUpdateStatus();
  updatePageHeading();
  renderNowPlaying();
  renderChangelog();
  customCommandsUi?.applyLanguage();
  panicUi?.applyLanguage();
  discordCheck?.render();
  obsSetup?.applyLanguage();
  streamStyleUi?.applyLanguage();
  moderationUi?.applyLanguage();
  if (bootstrap?.config?.port) streamStyleUi?.setPreviewPort(bootstrap.config.port, language);
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
  streamStyleUi?.render();
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
  syncWindowTheme();
  streamStyleUi?.render();
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
  nowPlayingTitleElement.textContent = playback.media.title || playback.media.filename || t("nowPlayingFallbackTitle");
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
    notify("error", String(error));
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

// Font sizes in panel.css are multiplied by --text-scale, so content rendered
// later (history rows, module cards) follows the preference automatically.
function scaleInterfaceText() {
  document.documentElement.style.setProperty("--text-scale", String(fontScale / 100));
}

function syncInterfacePreferences() {
  window.clearTimeout(personalizationTimer);
  personalizationTimer = window.setTimeout(async () => {
    try {
      // Outputs follow the design unless a notification style is chosen.
      const streamPreferences = streamStyleUi?.preferences() ?? {
        outputStyle: normalizeStreamStyle(readStorage("relay-output-style")),
        outputBackground: normalizeStreamBackground(readStorage("relay-output-background")),
      };
      await invoke("set_interface_preferences", {
        preferences: { language, theme, accentRgb, fontScale, design, ...streamPreferences },
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

function showPage(page, { recordHistory = true, moveFocus = false } = {}) {
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
    const active = button.dataset.pageTarget === page;
    button.classList.toggle("is-active", active);
    if (active) button.setAttribute("aria-current", "page");
    else button.removeAttribute("aria-current");
  }
  if (page === "history") {
    window.requestAnimationFrame(loadHistoryVideoThumbnails);
  }
  if (page === "discord") discordCheck?.runIfStale();
  updatePageHeading();
  updateNavigationControls();
  try { localStorage.setItem("relay-last-page", page); } catch { /* Storage is optional. */ }
  // Announce the new page to keyboard and screen reader users.
  if (moveFocus) pageTitleElement.focus({ preventScroll: true });
}

function navigateHistory(offset) {
  const nextIndex = navigationHistoryIndex + offset;
  if (nextIndex < 0 || nextIndex >= navigationHistory.length) return;
  navigationHistoryIndex = nextIndex;
  showPage(navigationHistory[navigationHistoryIndex], { recordHistory: false });
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
  for (const target of ["visual", "audio", "notification", "sticker", "reaction"]) {
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

function formSaveState(form) {
  const ids = {
    "bot-presence-form": "bot-presence-save-state", "routing-form": "save-state", "system-form": "system-save-state",
    "music-form": "music-save-state", "media-form": "media-save-state",
    "messages-form": "messages-save-state",
    "moderation-form": "moderation-save-state", "commands-form": "commands-save-state",
  };
  return document.getElementById(ids[form.id]);
}

// Shows the outcome of an action in a toast visible from any page.
function notify(state, message = t(state)) {
  const toast = document.getElementById("toast");
  if (!toast) return;
  window.clearTimeout(notify.timer);
  setSaveState(toast, state, message);
  toast.hidden = !message;
  if (state !== "saving") notify.timer = window.setTimeout(() => { toast.hidden = true; }, 5000);
}

// Last errors shown in the panel, kept for the diagnostic report.
const recentErrors = [];

// Backend errors are English; other languages get a translated summary and keep the original as a tooltip.
function describeError(message) {
  if (language === "en") return message;
  const key = errorCategory(message);
  return key ? t(key) : message;
}

function setSaveState(element, state, message = t(state)) {
  if (!element) return;
  element.dataset.state = state;
  if (state === "error") {
    const original = String(message ?? "");
    recentErrors.push({ at: Date.now(), message: original });
    if (recentErrors.length > 10) recentErrors.shift();
    element.textContent = describeError(original);
    element.title = original;
    return;
  }
  element.title = "";
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
  moderationUi?.fillForm(config);
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
  customCommandsUi?.syncFromConfig(config);
  applyOutputGeometryConfig(config);
  updateSkipShortcutDisplay(config.skipShortcut);
  updatePanicShortcutDisplay(config.panicShortcut);
  drafts.forEach(restoreFormDraft);
  mediaVolumeValueElement.value = `${mediaVolumeElement.value}%`;
  mediaVolumeValueElement.textContent = `${mediaVolumeElement.value}%`;
  updateBotActivityAvailability();
}

function setCredentials(status) {
  if (!status.configured) $("#disclosure-discord-connection").open = true;
  setSaveState(credentialStateElement, "idle", status.configured
    ? `${t("savedVia")} ${status.source}`
    : t("notConfigured"));
  if (!clientIdElement.value) clientIdElement.value = status.clientId || "";
  updateYoutubeKeyStatus(status);
  obsSetup?.updateCredentials(status);
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

// Both global shortcuts share one capture flow; each target keeps its own save command.
const shortcutTargets = {
  skip: {
    button: skipShortcutCaptureButton,
    value: skipShortcutValueElement,
    configKey: "skipShortcut",
    save: (shortcut) => invoke("set_skip_shortcut", { shortcut }),
  },
  panic: {
    button: panicShortcutCaptureButton,
    value: panicShortcutValueElement,
    configKey: "panicShortcut",
    save: (shortcut) => invoke("set_panic_shortcut", { shortcut }),
  },
};
let shortcutCaptureTarget = "skip";

function updatePanicShortcutDisplay(shortcut) {
  panicShortcutValueElement.textContent = formatShortcutLabel(shortcut || "control+alt+KeyP");
  panicUi?.applyLanguage();
}

function beginShortcutCapture(target = "skip") {
  shortcutCaptureTarget = target;
  shortcutCaptureActive = true;
  const { button, value } = shortcutTargets[target];
  button.setAttribute("aria-pressed", "true");
  value.textContent = t("pressShortcut");
  button.focus();
}

function cancelShortcutCapture() {
  shortcutCaptureActive = false;
  for (const { button } of Object.values(shortcutTargets)) button.setAttribute("aria-pressed", "false");
  updateSkipShortcutDisplay(bootstrap?.config?.skipShortcut);
  updatePanicShortcutDisplay(bootstrap?.config?.panicShortcut);
}

async function saveCapturedShortcut(shortcut) {
  const target = shortcutTargets[shortcutCaptureTarget];
  const previous = bootstrap?.config?.[target.configKey];
  shortcutCaptureActive = false;
  target.button.setAttribute("aria-pressed", "false");
  target.value.textContent = formatShortcutLabel(shortcut);
  setSaveState(mediaSaveStateElement, "saving");
  try {
    const config = await target.save(shortcut);
    bootstrap.config = config;
    updateSkipShortcutDisplay(config.skipShortcut);
    updatePanicShortcutDisplay(config.panicShortcut);
    setSaveState(mediaSaveStateElement, "saved", t("shortcutSaved"));
  } catch (error) {
    target.value.textContent = formatShortcutLabel(previous);
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
        notify("error", String(error));
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
          notify("saved", t("modSaved"));
        } catch (error) { notify("error", String(error)); }
        finally { saveLibrary.disabled = false; }
      });
    }
    downloadButton.textContent = t("download");
    const download = async (format) => {
      downloadButton.disabled = true;
      notify("saving", t("downloading"));
      try {
        const saved = await invoke("download_history_media", {
          messageId: mediaEvent.messageId,
          mediaUrl: mediaEvent.url,
          format,
        });
        notify(saved ? "saved" : "idle", saved ? t("downloaded") : t("downloadCanceled"));
      } catch (error) {
        notify("error", String(error));
      } finally {
        downloadButton.disabled = false;
      }
    };
    if (kind === "youtube") {
      attachDownloadFormatMenu(downloadButton, download);
    } else {
      downloadButton.addEventListener("click", () => download(null));
    }
    historyListElement.append(item);
  }
  loadHistoryVideoThumbnails();
  updateOverview();
}

// YouTube downloads pick MP3 or MP4 in the panel before any native dialog opens.
function attachDownloadFormatMenu(button, download) {
  const menu = document.createElement("div");
  menu.className = "download-menu";
  menu.setAttribute("role", "menu");
  menu.hidden = true;
  const close = () => {
    menu.hidden = true;
    button.setAttribute("aria-expanded", "false");
  };
  for (const [format, key] of [["mp3", "downloadAudioMp3"], ["mp4", "downloadVideoMp4"]]) {
    const choice = document.createElement("button");
    choice.type = "button";
    choice.className = "download-menu__item";
    choice.setAttribute("role", "menuitem");
    choice.textContent = t(key);
    choice.addEventListener("click", () => {
      close();
      download(format);
    });
    menu.append(choice);
  }
  menu.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") return;
    close();
    button.focus();
  });
  menu.addEventListener("focusout", (event) => {
    if (!menu.contains(event.relatedTarget) && event.relatedTarget !== button) close();
  });
  button.setAttribute("aria-haspopup", "menu");
  button.setAttribute("aria-expanded", "false");
  button.addEventListener("click", () => {
    const opening = menu.hidden;
    menu.hidden = !opening;
    button.setAttribute("aria-expanded", String(opening));
    if (opening) menu.querySelector("button")?.focus();
  });
  button.parentElement.classList.add("history-item__actions--menu");
  button.after(menu);
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
  if (kind === "youtube") {
    thumbnail.alt = mediaEvent.filename;
    thumbnail.onerror = () => {
      thumbnail.onerror = null;
      thumbnail.src = "./assets/relay-radar.png";
    };
    thumbnail.src = youtubeThumbnail(mediaEvent) || "./assets/relay-radar.png";
    return;
  }
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

// Only YouTube's image host is allowed by the panel CSP.
function youtubeThumbnail(mediaEvent) {
  try {
    const url = new URL(mediaEvent.thumbnail);
    if (url.protocol === "https:" && url.hostname === "i.ytimg.com") return url.href;
  } catch {}
  return /^[\w-]{11}$/.test(mediaEvent.videoId || "")
    ? `https://i.ytimg.com/vi/${mediaEvent.videoId}/mqdefault.jpg`
    : "";
}

function replaceHistory(mediaEvents) {
  history.splice(0, history.length, ...mediaEvents.slice(0, 50).map(historyMedia));
  renderHistory();
}

function sameHistoryMedia(left, right) {
  return left?.messageId === right?.messageId && left?.url === right?.url;
}

function historyMedia(entry) {
  if (!entry.music) return entry;
  const music = entry.music;
  return {
    ...entry,
    kind: "youtube",
    messageId: `youtube-${music.playbackId}`,
    url: `https://www.youtube.com/watch?v=${music.videoId}`,
    filename: music.title,
    title: music.title,
    thumbnail: music.thumbnail,
    videoId: music.videoId,
    artist: music.channelTitle,
    author: { username: music.requestedBy },
  };
}

function rememberMedia(mediaEvent) {
  mediaEvent = historyMedia(mediaEvent);
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
  const pendingTexts = moderationUi?.pendingTextCount() || 0;
  moderationListElement.replaceChildren();
  moderationCountElement.textContent = `${pending.length + pendingTexts} / 50`;
  moderationEmptyElement.hidden = pending.length + pendingTexts > 0;
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
  clearPendingMediaButton.disabled = pending.length + pendingTexts === 0;

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
    moderationUi?.decorateQueueItem(item, pendingItem);
    moderationListElement.append(item);
  }
  moderationUi?.renderTexts(moderationListElement, moderationItemTemplate);
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
  } else if (message.type === "musicHistory") {
    if (message.payload) rememberMedia(message.payload);
  } else if (message.type === "audioPlayback") {
    if (message.payload?.media?.kind === "audio") updateAudioPlayback(message.payload);
  } else if (message.type === "outputsPaused") {
    panicUi?.setPaused(message.payload);
  } else if (message.type === "clear") {
    if (panicUi?.isPaused()) return;
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
  $("#reactions-url").value = `http://127.0.0.1:${bootstrap.config.port}/reactions`;
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
  streamStyleUi?.setPreviewPort(bootstrap.config.port, language);
  if (reconnect) {
    connectPanelSocket();
  }
  updateOverview();
}

// The Overview module is created before startup; these wrappers keep call sites simple.
function updateOverview() { overviewUi?.update(); }
function markSetupTested() { overviewUi?.markTested(); }
function setupStatus() { return overviewUi.setupStatus(); }

function readConfigDraft(form, filterOnly = false) {
  if (filterOnly) {
    return { privacyConcepts: moderationUi.applyAdvancedEdits(filterWordsToConcepts(privacyConceptsElement.value, bootstrap.config.privacyConcepts)) };
  }
  if (form === messagesForm) {
    return {
      notificationDurationMs: Number(notificationDurationElement.value) * 1000,
      ttsCharacterLimit: Number(ttsCharacterLimitElement.value),
      ttsQueueLimit: Number(ttsQueueLimitElement.value),
      ttsNotificationsObsEnabled: ttsNotificationsObsElement.checked,
    };
  }
  if (form === routingForm) {
    return {
      watchedChannelId: channelElement.value,
      mediaCleanupEnabled: mediaCleanupEnabledElement.checked,
      mediaWelcomeMessageId: mediaWelcomeMessageElement.value.trim(),
      ttsChannelId: ttsChannelElement.value,
      ttsCleanupEnabled: ttsCleanupEnabledElement.checked,
      ttsWelcomeMessageId: ttsWelcomeMessageElement.value.trim(),
      musicChannelId: musicChannelElement.value,
      musicWelcomeMessageId: musicWelcomeElement.value.trim(),
      musicCleanupEnabled: musicCleanupEnabledElement.checked,
      honeypotChannelId: honeypotChannelElement.value,
      honeypotAction: honeypotActionElement.value,
    };
  }
  if (form === systemForm) {
    return { port: Number(portElement.value) };
  }
  if (form === musicForm) {
    // The music form only stores the YouTube key; its channel lives on the Discord page.
    return {};
  }
  if (form === moderationForm) {
    return {
      moderationEnabled: moderationEnabledElement.checked,
      moderationAllowImages: moderationAllowImagesElement.checked,
      moderationAllowVideos: moderationAllowVideosElement.checked,
      moderationAllowAudio: moderationAllowAudioElement.checked,
      privacyScanEnabled: privacyScanEnabledElement.checked,
      privacyConcepts: moderationUi.applyAdvancedEdits(filterWordsToConcepts(privacyConceptsElement.value, bootstrap.config.privacyConcepts)),
      privacyFilterExemptRoleIds: filterRoleIds(privacyExemptRoleIdsElement.value),
      privacyProtectionLevel: privacyProtectionLevelElement.value,
      privacyEnabledCategories: privacyCategoryElements.filter((input) => input.checked).map((input) => input.value),
      privacyBlockThreshold: privacyBlockThresholdElement.value,
      privacyReviewIntermediate: privacyReviewIntermediateElement.checked,
      privacyAutoDeleteBlockedMessages: privacyAutoDeleteBlockedMessagesElement.checked,
      privacyAllowlist: privacyListFromInput(privacyAllowlistElement.value),
      privacyCustomPatterns: privacyListFromInput(privacyCustomPatternsElement.value),
      moderation: moderationUi.readDraft(bootstrap.config.moderation),
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

/** JSON with sorted object keys, so field order never makes two settings differ. */
function stableJson(value) {
  return JSON.stringify(value, (_, item) => (item && typeof item === "object" && !Array.isArray(item)
    ? Object.fromEntries(Object.keys(item).sort().map((key) => [key, item[key]]))
    : item));
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
            stableJson(value) === stableJson(nextBootstrap.config[key]));
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
    panicUi?.setPaused(status.outputsPaused);
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
    updateOverview();
  } catch {
    setServerStatus({ connected: false, overlayClients: 0 });
    setBotStatus({ connected: false });
  } finally {
    statusRefreshInFlight = false;
  }
}

for (const button of $$("[data-page-target]")) {
  button.addEventListener("click", () => showPage(button.dataset.pageTarget, { moveFocus: true }));
}

navigationBackButton.addEventListener("click", () => navigateHistory(-1));
navigationForwardButton.addEventListener("click", () => navigateHistory(1));

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
  settingsSearch.closeIfOutside(event.target);
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
    settingsSearch.closeAndFocus();
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
    settingsSearch.focus();
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

interfaceLanguageOptionsElement.addEventListener("click", (event) => {
  const option = event.target.closest("[data-locale]");
  if (option) selectInterfaceLanguage(option.dataset.locale, interfaceLanguageButton);
});

interfaceLanguageOptionsElement.addEventListener("keydown", (event) => handleListboxKeys(interfaceLanguageOptionsElement, event));
sidebarLanguageOptionsElement.addEventListener("keydown", (event) => handleListboxKeys(sidebarLanguageOptionsElement, event));

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
  theme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  design = "graphite";
  interfaceFont = "design";
  sidebarLayout = "fixed";
  sidebarExpanded = false;
  accentRgb = [88, 185, 137];
  fontScale = 100;
  streamStyleUi?.reset();
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

systemForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  await saveConfig(formSaveState(systemForm), systemForm);
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
routingForm.addEventListener("input", clearMusicCleanupPreview);
$("#music-cleanup-cancel").addEventListener("click", () => {
  clearMusicCleanupPreview();
  musicCleanupStatus.textContent = "";
});
cleanupPreviewButton.addEventListener("click", async () => {
  clearMusicCleanupPreview();
  if (dirtyForms.has(routingForm)) {
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

initializeAutosave({
  forms: [botPresenceForm, routingForm, systemForm, mediaForm, messagesForm, moderationForm, commandsForm],
  dirtyForms, formSaveState, setSaveState, t, skipTarget: privacyConceptsElement,
});
initializeStartWithWindows({ $, invoke, t, setSaveState });

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

// Page modules: each owns its DOM and receives the shared panel helpers it needs.
const panicUi = initializePanic({
  $, t, invoke, notify,
  getShortcutLabel: () => formatShortcutLabel(bootstrap?.config?.panicShortcut || "control+alt+KeyP"),
});
const discordCheck = initializeDiscordCheck({
  $, t, formatTranslation, invoke, setSaveState, getBootstrap: () => bootstrap,
});
const moderationUi = initializeModerationUi({
  $, $$, t, formatTranslation, invoke, setSaveState, readStorage,
  getBootstrap: () => bootstrap,
  getLocale: () => locale,
  applyConfig,
  saveModeration: () => saveConfig(moderationSaveStateElement, moderationForm),
  onQueueChanged: () => renderModeration(),
});
const streamStyleUi = initializeStreamStyle({
  $, $$, t, formatTranslation, readStorage,
  getDesign: () => design, getTheme: () => theme,
  onChange: () => syncInterfacePreferences(),
});
const obsSetup = initializeObsSetup({
  $, t, formatTranslation, invoke, setSaveState, readStorage,
  onInstalled: () => void refreshRuntimeStatus(),
});

const settingsSearch = initializeSettingsSearch({
  $, $$, t, pageMetadata, showPage, getLocale: () => locale,
});

const overviewUi = initializeOverview({
  $, $$, t, invoke, formatTranslation, setSaveState, setWidgetState,
  getBootstrap: () => bootstrap, getHistory: () => history,
});

const customCommandsUi = initializeCustomCommands({
  $, $$, t, invoke, formatTranslation, setSaveState,
  getBootstrap: () => bootstrap, applyConfig, applyBootstrap,
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
openInviteButton.addEventListener("click", () => invoke("open_help_link", { link: inviteUrlElement.value }));

for (const button of $$("[data-go-to-page]")) {
  button.addEventListener("click", () => {
    showPage(button.dataset.goToPage, { moveFocus: true });
    if (button.dataset.goToTarget) settingsSearch.reveal(document.getElementById(button.dataset.goToTarget));
  });
}

// The tray can ask the panel to open a specific page (for example Moderation).
window.__TAURI__.event?.listen("relay-open-page", ({ payload }) => {
  if (pageMetadata[payload]) showPage(payload, { moveFocus: true });
});

$("#setup-invite-button")?.addEventListener("click", () => {
  if (bootstrap?.inviteUrl) void invoke("open_help_link", { link: bootstrap.inviteUrl });
});

const copyReactionsUrlButton = $("#copy-reactions-url");
copyReactionsUrlButton.addEventListener("click", () => copyValue(copyReactionsUrlButton, $("#reactions-url").value));

for (const [target, button] of outputTestButtons) {
  button.addEventListener("click", async () => {
    if (button.disabled) return;
    button.disabled = true;
    try {
      await invoke("test_output", { target });
      button.textContent = t("outputTestSent");
      markSetupTested();
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
  notify("saving", t("regenerating"));
  try {
    applyBootstrap(await invoke("regenerate_secret"), true);
    notify("saved", t("secretRegenerated"));
  } catch (error) {
    notify("error", String(error));
  }
});

$("#copy-diagnostic").addEventListener("click", async (event) => {
  const button = event.currentTarget;
  const state = $("#diagnostic-state");
  button.disabled = true;
  try {
    const status = await invoke("get_runtime_status").catch(() => null);
    const report = buildDiagnosticReport({
      version: currentAppVersion,
      bootstrap: { ...bootstrap, ...(status || {}), history },
      interfaceState: { locale, design, theme, fontScale },
      setup: bootstrap ? setupStatus() : {},
      recentErrors,
    });
    await navigator.clipboard.writeText(report);
    setSaveState(state, "saved", t("diagnosticCopied"));
  } catch {
    setSaveState(state, "error", t("copyFailed"));
  } finally {
    button.disabled = false;
  }
});

toggleWidgetButton.addEventListener("click", async () => {
  try {
    setWidgetState(await invoke("toggle_widget"));
  } catch (error) {
    notify("error", String(error));
  }
});

lockWidgetButton.addEventListener("click", async () => {
  try {
    setWidgetState(await invoke("set_widget_locked", { locked: !bootstrap.widget.locked }));
  } catch (error) {
    notify("error", String(error));
  }
});

notificationWidgetEnabledElement.addEventListener("change", async () => {
  try {
    setNotificationWidgetState(await invoke("set_notification_widget_visible", {
      visible: notificationWidgetEnabledElement.checked,
    }));
  } catch (error) {
    notificationWidgetEnabledElement.checked = !notificationWidgetEnabledElement.checked;
    notify("error", String(error));
  }
});

lockNotificationWidgetButton.addEventListener("click", async () => {
  try {
    setNotificationWidgetState(await invoke("set_notification_widget_locked", {
      locked: !bootstrap.notificationWidget.locked,
    }));
  } catch (error) {
    notify("error", String(error));
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

skipShortcutCaptureButton.addEventListener("click", () => beginShortcutCapture("skip"));
panicShortcutCaptureButton.addEventListener("click", () => beginShortcutCapture("panic"));
// No browser menu (Back, Refresh, Print…) on right click; text fields keep Copy and Paste.
window.addEventListener("contextmenu", (event) => {
  if (!event.target?.closest?.("input, textarea, [contenteditable='true']")) event.preventDefault();
});
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
    notify("error", String(error));
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
  try { await invoke("preview_output_sample", { sample: document.querySelector("#preview-sample").value }); state.textContent = t("outputTestSent"); markSetupTested(); }
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
  const lastPage = readStorage("relay-last-page");
  const setup = setupStatus();
  const setupComplete = ["bot", "channel"].every((step) => setup[step]);
  if (setupComplete && lastPage && pageMetadata[lastPage] && lastPage !== currentPage) showPage(lastPage);
  moduleControls = initializeModules({ invoke, t, getBootstrap: () => bootstrap });
  connectPanelSocket();
  statusTimer = window.setInterval(refreshRuntimeStatus, 1500);
} catch (error) {
  setSaveState(saveStateElement, "error", String(error));
  setSaveState(credentialStateElement, "error", String(error));
}
