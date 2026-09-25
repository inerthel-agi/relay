// Error categories for translated messages, and a shareable diagnostic without secrets.

// Ordered: the first matching pattern wins. Keys live in translations.mjs.
const errorCategories = [
  [/discord bot is not connected|connect the discord bot|bot offline/i, "errBotOffline"],
  [/shortcut is already in use/i, "errShortcutInUse"],
  [/file with that name already exists/i, "errFileExists"],
  [/startup setting|windows startup/i, "errStartup"],
  [/manage messages|manage roles|manage channels|missing permissions|permission/i, "errDiscordPermission"],
  [/synchroni[sz]e .*discord|discord (command )?schema/i, "errDiscordSync"],
  [/(message|channel|role|welcome)[^.]*(id|link)[^.]*(invalid|must belong)|invalid discord message id|enter a valid discord/i, "errInvalidDiscordId"],
  [/no longer (in history|pending|available|waiting|visible|exists)|is no longer|unavailable\.$/i, "errNoLongerAvailable"],
  [/youtube (is not configured|api key)|saved youtube api key/i, "errYoutubeKey"],
  [/youtube (search|duration lookup)/i, "errYoutubeSearch"],
  [/helper|ffmpeg|yt-dlp|youtube download/i, "errDownloadTool"],
  [/installer|updater signature|release (version|asset)|github (response|release)/i, "errUpdate"],
  [/timed out|timeout|error sending request|connection (refused|reset)|dns|network/i, "errNetwork"],
  [/too large|must be under|exceeds|size limit|size is outside/i, "errFileTooLarge"],
  [/unsupported|not h\.264|invalid hevc|format/i, "errUnsupportedFormat"],
  [/must be between|outside the supported bounds|must contain at most|may contain at most|within the source/i, "errOutOfRange"],
];

export function errorCategory(message) {
  const text = String(message ?? "");
  return errorCategories.find(([pattern]) => pattern.test(text))?.[1] || null;
}

// Removes anything that could identify or unlock the user's setup.
export function sanitizeDiagnosticText(value) {
  return String(value ?? "")
    .replace(/[\w-]{23,28}\.[\w-]{6,7}\.[\w-]{27,}/g, "[discord-token]")
    .replace(/([?&](?:secret|token|key|client_id)=)[^&\s]+/gi, "$1[redacted]")
    .replace(/(?<![?&])\b(?:secret|token|api[_ -]?key)\s*[:=]\s*(?!\[redacted\])\S+/gi, "[redacted]")
    .replace(/\b[A-Za-z0-9+/_-]{32,}={0,2}/g, "[redacted]")
    .replace(/\b\d{17,20}\b/g, "[discord-id]")
    .replace(/([A-Za-z]:\\Users\\)[^\\\s]+/gi, "$1[user]")
    .replace(/(\/(?:home|Users)\/)[^/\s]+/g, "$1[user]");
}

const yesNo = (value) => (value ? "yes" : "no");

function outputLine(name, output = {}) {
  const count = (value) => Math.max(0, Number(value) || 0);
  const last = Number(output.lastConnectedAt) > 0 ? new Date(Number(output.lastConnectedAt)).toISOString() : "never";
  return `  ${name}: obs ${count(output.obsClients)}, widget ${count(output.widgetClients)}, preview ${count(output.previewClients)}, last connected ${last}`;
}

// Plain English on purpose: the report is meant to be pasted into a support request.
export function buildDiagnosticReport({ version, bootstrap = {}, interfaceState = {}, setup = {}, recentErrors = [], now = new Date() }) {
  const config = bootstrap.config || {};
  const outputs = bootstrap.server?.outputs || {};
  const lines = [
    "Relay diagnostic",
    `Generated: ${now.toISOString()}`,
    `Version: ${version || "unknown"} (Windows)`,
    `Interface: language ${interfaceState.locale || "?"}, design ${interfaceState.design || "?"}, theme ${interfaceState.theme || "?"}, text ${interfaceState.fontScale || 100}%`,
    "",
    "Discord",
    `  credentials saved: ${yesNo(bootstrap.credentials?.configured)}`,
    `  bot connected: ${yesNo(bootstrap.bot?.connected)}`,
    `  bot error: ${sanitizeDiagnosticText(bootstrap.bot?.error) || "none"}`,
    `  visible channels: ${(bootstrap.channels || []).length}`,
    `  media channel set: ${yesNo(config.watchedChannelId)}`,
    `  notification channel set: ${yesNo(config.ttsChannelId)}`,
    `  music channel set: ${yesNo(config.musicChannelId)}`,
    `  security trap set: ${yesNo(config.honeypotChannelId)}`,
    `  YouTube key saved: ${yesNo(bootstrap.credentials?.youtubeConfigured)}`,
    "",
    "Local server",
    `  online: ${yesNo(bootstrap.server?.connected)}`,
    `  port: ${config.port ?? "?"}`,
    `  error: ${sanitizeDiagnosticText(bootstrap.server?.error) || "none"}`,
    `  connected outputs: ${Math.max(0, Number(bootstrap.server?.overlayClients) || 0)}`,
    outputLine("visual", outputs.visual),
    outputLine("audio", outputs.audio),
    outputLine("notification", outputs.notification),
    outputLine("sticker", outputs.sticker),
    outputLine("reaction", outputs.reaction),
    "",
    "Widgets",
    `  media: visible ${yesNo(bootstrap.widget?.visible)}, locked ${yesNo(bootstrap.widget?.locked)}`,
    `  notifications: visible ${yesNo(bootstrap.notificationWidget?.visible)}, locked ${yesNo(bootstrap.notificationWidget?.locked)}`,
    "",
    "Features",
    `  manual moderation: ${yesNo(config.moderationEnabled)}`,
    `  privacy scan: ${yesNo(config.privacyScanEnabled)}`,
    `  OBS message notifications: ${yesNo(config.ttsNotificationsObsEnabled)}`,
    `  show author: ${yesNo(config.showAuthor)}`,
    `  queued media: ${(bootstrap.queue || []).length}`,
    `  awaiting moderation: ${(bootstrap.pendingMedia || []).length}`,
    `  history items: ${(bootstrap.history || []).length}`,
    "",
    "Setup checklist",
    ...Object.entries(setup).filter(([, value]) => value !== null).map(([step, value]) => `  ${step}: ${value ? "done" : "to do"}`),
    "",
    "Recent panel errors",
    ...(recentErrors.length
      ? recentErrors.map(({ at, message }) => `  ${new Date(at).toISOString()} ${sanitizeDiagnosticText(message)}`)
      : ["  none"]),
  ];
  return lines.join("\n");
}
