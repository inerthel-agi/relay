const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const { pathToFileURL } = require("node:url");

const gui = `${__dirname}/../gui`;
const panelHtml = fs.readFileSync(`${gui}/panel.html`, "utf8");
const panelCss = fs.readFileSync(`${gui}/panel.css`, "utf8");
const panelSource = require("./test-source.cjs").panelSource();
const traySource = fs.readFileSync(`${gui}/tray.js`, "utf8");
const trayHtml = fs.readFileSync(`${gui}/tray.html`, "utf8");
const load = (file) => import(pathToFileURL(`${gui}/${file}`).href);
const pageOf = (page) => {
  const start = panelHtml.indexOf(`data-page="${page}"`);
  const next = panelHtml.indexOf('<section class="page', start + 1);
  return panelHtml.slice(start, next < 0 ? undefined : next);
};

test("Simple mode only hides, and an unknown stored mode falls back to Simple", async () => {
  const { readInterfaceMode, readFoldedGroups, pageForMode } = await load("navigation-layout.mjs");
  assert.equal(readInterfaceMode("advanced"), "advanced");
  assert.equal(readInterfaceMode("expert"), "simple");
  assert.equal(readInterfaceMode(null), "simple");
  assert.deepEqual(readFoldedGroups('["live","nope"]', ["start", "live"]), ["live"]);
  assert.deepEqual(readFoldedGroups("{broken", ["live"]), []);
  assert.equal(pageForMode("simple", "history", ["history", "about"]), "overview");
  assert.equal(pageForMode("simple", "media", ["history"]), "media");
  assert.equal(pageForMode("advanced", "history", ["history"]), "history");
  assert.match(panelCss, /:root\[data-ui-mode="simple"\] \[data-advanced\] \{\s*display: none !important;/);
  // The mode buttons use data-interface-mode; the root must not, or it would be taken for a button.
  assert.match(fs.readFileSync(`${gui}/navigation-layout.mjs`, "utf8"), /querySelectorAll\("\.interface-mode__button\[data-interface-mode\]"\)/);
  assert.doesNotMatch(fs.readFileSync(`${gui}/navigation-layout.mjs`, "utf8"), /dataset\.interfaceMode = /);
  // A folded group never hides the page that is open.
  assert.match(panelCss, /\.navigation__item\[data-folded\]:not\(\.is-active\) \{\s*display: none;/);
});

test("every advanced menu entry is a real page and opening one switches to Advanced", () => {
  const advanced = [...panelHtml.matchAll(/data-page-target="([a-z]+)" data-advanced/g)].map((match) => match[1]);
  assert.deepEqual(advanced.sort(), ["about", "changelog", "commands", "history", "reactions"]);
  for (const page of advanced) assert.match(panelSource, new RegExp(`\\b${page}: \\{ title:`), page);
  assert.match(panelSource, /navigationLayout\?\.getMode\(\) === "simple" && navigationLayout\.advancedPages\.includes\(page\)\) navigationLayout\.setMode\("advanced"\)/);
  assert.match(panelSource, /target\.closest\("\[data-advanced\]"\) && navigationLayout\?\.getMode\(\) === "simple"/);
});

test("the Control room holds the live controls and the review queue", () => {
  const live = pageOf("live");
  for (const id of ["message-module", "skip-media", "queue-list", "moderation-list", "clear-pending-media", "moderation-count"]) {
    assert.match(live, new RegExp(`id="${id}"`), id);
    assert.equal(panelHtml.match(new RegExp(`id="${id}"`, "g")).length, 1, `${id} must exist once`);
  }
  assert.match(panelSource, /live: \{ title: "navLive", kicker: "navGroupLive" \}/);
});

test("the OBS message toggle lives on Outputs but still saves with the Messages form", () => {
  assert.match(pageOf("overlay"), /<input id="notifications-obs" name="notificationsObsEnabled" type="checkbox" form="messages-form">/);
  assert.doesNotMatch(pageOf("messages"), /id="notifications-obs"/);
  assert.match(panelSource, /notificationsObsElement\.addEventListener\(type, \(\) => messagesForm\.dispatchEvent\(new Event\(type\)\)\)/);
  assert.match(pageOf("overlay"), /id="stream-style"/);
  assert.match(pageOf("overlay"), /id="disclosure-music-3"/);
});

test("Ask the AI keeps a five-minute pace and ten questions a day", async () => {
  const { aiHelpQuota, formatWait, AI_HELP_COOLDOWN_MS, AI_HELP_DAILY_LIMIT } = await load("help-ui.mjs");
  const now = 1_000_000_000;
  assert.deepEqual(aiHelpQuota([], now), { recent: [], wait: 0, left: AI_HELP_DAILY_LIMIT });
  const justAsked = aiHelpQuota([now - 60_000], now);
  assert.equal(justAsked.wait, AI_HELP_COOLDOWN_MS - 60_000);
  assert.equal(justAsked.left, AI_HELP_DAILY_LIMIT - 1);
  assert.equal(aiHelpQuota([now - AI_HELP_COOLDOWN_MS], now).wait, 0);
  const full = Array.from({ length: 10 }, (_, index) => now - (index + 1) * 3_600_000);
  assert.equal(aiHelpQuota(full, now).left, 0);
  // Questions older than a day, from the future or not numbers do not count.
  assert.equal(aiHelpQuota([now - 86_400_000, now + 5_000, "x"], now).left, AI_HELP_DAILY_LIMIT);
  assert.equal(formatWait(245_100), "4:06");
  // Relay applies the pace; the panel reads Relay's count and re-reads it after each question.
  assert.match(panelSource, /initializeHelpUi\(\{ formatTranslation, loadAsks: \(\) => invoke\("ai_help_asks"\) \}\)/);
  assert.match(panelSource, /if \(!helpUi\.canAsk\(\)\) return;/);
  assert.match(panelSource, /await helpUi\.refresh\(\);/);
  assert.doesNotMatch(fs.readFileSync(`${gui}/help-ui.mjs`, "utf8"), /localStorage|setItem/);
});

test("each Help symptom has steps and a link to an existing page", () => {
  const help = pageOf("help");
  const tabs = [...help.matchAll(/data-help-symptom="([a-z]+)"/g)].map((match) => match[1]);
  assert.deepEqual(tabs, ["media", "message", "sound", "obs", "bot"]);
  for (const tab of tabs) {
    const panel = help.slice(help.indexOf(`id="help-fix-${tab}"`), help.indexOf("</div>", help.indexOf(`id="help-fix-${tab}"`)));
    assert.match(panel, /<li data-i18n="helpFix\w+\d">/, tab);
    const page = panel.match(/data-go-to-page="([a-z]+)"/)[1];
    assert.match(panelSource, new RegExp(`\\b${page}: \\{ title:`), `${tab} → ${page}`);
  }
  assert.doesNotMatch(help, /helpTroubleshooting|class="help-start"/);
});

test("accent colors convert without drift and the picker never uses inline styles", async () => {
  const { normalizeHex, hexToRgbArray, rgbArrayToHex, rgbToHsv, hsvToRgb, prefersDarkText } = await load("accent-picker.mjs");
  assert.equal(normalizeHex("58B989"), "#58b989");
  assert.equal(normalizeHex("#12345"), null);
  assert.equal(normalizeHex("#zzzzzz"), null);
  for (const hex of ["#58b989", "#3b82f6", "#000000", "#ffffff", "#ef4444", "#14b8a6"]) {
    assert.equal(rgbArrayToHex(hsvToRgb(rgbToHsv(hexToRgbArray(hex)))), hex, hex);
  }
  const { accentInk } = require("../outputs/theme.js");
  for (const hex of ["#58b989", "#3b82f6", "#6d5bff", "#eab308", "#ef4444"]) {
    assert.equal(prefersDarkText(hex), accentInk(hexToRgbArray(hex)) === "#07130d", hex);
  }
  // The panel CSP has no 'unsafe-inline': style attributes in the markup would be dropped.
  assert.doesNotMatch(panelHtml, /<[^>]+\sstyle="/);
  assert.match(panelHtml, /<input id="accent-picker"[^>]*type="color"[^>]*hidden/);
});

test("the tray skips media with the panel command and reads its version", () => {
  assert.match(trayHtml, /<button id="skip-media" class="quick-action" type="button">[\s\S]*data-i18n="skip"/);
  assert.match(traySource, /runAction\(event\.currentTarget, "skip_media"\)/);
  assert.match(traySource, /invoke\("get_app_version"\)/);
  assert.doesNotMatch(trayHtml, /\d+\.\d+\.\d+/);
  // Shortcuts are configurable: the tray shows the panel's labels, never a fixed one.
  assert.doesNotMatch(trayHtml, /<kbd>[^<]+<\/kbd>/);
  assert.match(panelSource, /rememberShortcutLabel\("skip", label\)/);
  assert.match(panelSource, /rememberShortcutLabel\("panic", /);
  assert.match(traySource, /localStorage\.getItem\("relay-shortcut-labels"\)/);
});
