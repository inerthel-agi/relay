const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");

const { buildDiagnosticReport, errorCategory, sanitizeDiagnosticText } = require("../gui/diagnostics.mjs");
const translationsSource = fs.readFileSync(__dirname + "/../gui/translations.mjs", "utf8");

test("common backend errors map to translated categories", () => {
  const cases = {
    "the Discord bot is not connected": "errBotOffline",
    "Connect the Discord bot to verify the protected message.": "errBotOffline",
    "Enter a valid Discord welcome message ID or link.": "errInvalidDiscordId",
    "The welcome message link must belong to the selected music channel.": "errInvalidDiscordId",
    "Privacy deletion failed. Verify Manage Messages in this channel.": "errDiscordPermission",
    "The media is no longer in history.": "errNoLongerAvailable",
    "YouTube is not configured.": "errYoutubeKey",
    "YouTube search failed.": "errYoutubeSearch",
    "FFmpeg could not be extracted.": "errDownloadTool",
    "the installer download is incomplete": "errUpdate",
    "Reaction sound must be under 15 MB.": "errFileTooLarge",
    "Unsupported reaction audio format.": "errUnsupportedFormat",
    "The sticker duration must be between 1 and 60 seconds.": "errOutOfRange",
    "The selected shortcut is already in use.": "errShortcutInUse",
    "Windows refused to update the startup setting.": "errStartup",
  };
  for (const [message, key] of Object.entries(cases)) assert.equal(errorCategory(message), key, message);
  assert.equal(errorCategory("Something entirely new happened"), null);
});

test("every error category is translated in all interface languages", () => {
  const { translations } = require("../gui/translations.mjs");
  const keys = [...fs.readFileSync(__dirname + "/../gui/diagnostics.mjs", "utf8").matchAll(/"(err[A-Z]\w+)"/g)].map((match) => match[1]);
  for (const language of ["en", "fr", "es", "de", "ru", "zh", "ko", "ja", "id"]) {
    for (const key of keys) assert.ok(translations[language][key], `${language} ${key}`);
  }
  assert.equal(errorCategory("Relay is paused."), "errPaused");
  assert.equal(errorCategory("The OBS WebSocket password is incorrect."), "errObsPassword");
  assert.equal(errorCategory("OBS is not reachable. In OBS, open Tools"), "errObsUnreachable");
});

test("sanitizing removes tokens, private link secrets, Discord IDs and the Windows user name", () => {
  const text = sanitizeDiagnosticText(
    "token fake-discord-token-value.abcdef.not-a-real-token-only-for-tests at http://localhost:4590/obs/visual?secret=abc123 "
    + "channel 123456789012345678 in C:\\Users\\inerthel\\AppData",
  );
  assert.doesNotMatch(text, /fake-discord-token|abc123|123456789012345678|inerthel/);
  assert.match(text, /\[discord-token\]/);
  assert.match(text, /secret=\[redacted\]/);
  assert.match(text, /C:\\Users\\\[user\]/);
});

test("the report describes the setup without secrets or identifiers", () => {
  const report = buildDiagnosticReport({
    version: "1.3.7",
    now: new Date("2026-09-26T00:00:00Z"),
    bootstrap: {
      config: { port: 4590, watchedChannelId: "123456789012345678", formerMessageChannelId: "", moderationEnabled: true },
      credentials: { configured: true, clientId: "987654321098765432", youtubeConfigured: false },
      bot: { connected: false, error: "Disallowed gateway intents" },
      server: { connected: true, overlayClients: 2, outputs: { visual: { obsClients: 1, lastConnectedAt: 1 } } },
      overlayUrl: "http://localhost:4590/obs/visual?secret=private-secret",
      wsUrl: "ws://127.0.0.1:4590/ws?token=panel-token",
      inviteUrl: "https://discord.com/oauth2/authorize?client_id=987654321098765432",
      channels: [{ id: "123456789012345678", name: "media" }],
      history: [{}, {}],
    },
    interfaceState: { locale: "fr-FR", design: "graphite", theme: "dark", fontScale: 100 },
    setup: { bot: false, channel: true, reactions: null },
    recentErrors: [{ at: 0, message: "Invalid media URL http://127.0.0.1:4590/x?token=abc" }],
  });
  assert.match(report, /Version: 1\.3\.7/);
  assert.match(report, /Relay channel set: yes/);
  assert.match(report, /Relay channel choice pending: no/);
  assert.match(report, /bot error: Disallowed gateway intents/);
  assert.match(report, /visual: obs 1, widget 0/);
  assert.match(report, /bot: to do/);
  assert.doesNotMatch(report, /reactions:/);
  assert.match(report, /token=\[redacted\]/);
  for (const secret of ["private-secret", "panel-token", "987654321098765432", "123456789012345678", "media\n"]) {
    assert.equal(report.includes(secret), false, secret);
  }
});
