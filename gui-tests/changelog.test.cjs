const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

const panelSource = require("./test-source.cjs").panelSource();
const panelHtml = fs.readFileSync(__dirname + "/../gui/panel.html", "utf8");
const changelogMarkdown = fs.readFileSync(__dirname + "/../CHANGELOG.md", "utf8");

function sourceBetween(source, start, end) {
  const startIndex = source.indexOf(start);
  const endIndex = source.indexOf(end, startIndex);
  assert.notEqual(startIndex, -1, `missing source marker: ${start}`);
  assert.notEqual(endIndex, -1, `missing source marker: ${end}`);
  return source.slice(startIndex, endIndex);
}

const parserSource = sourceBetween(
  panelSource,
  "function parseChangelogReleases",
  "function appendInlineChangelogText",
);
const context = vm.createContext({});
vm.runInContext(
  `${parserSource}\nglobalThis.parseChangelogReleases = parseChangelogReleases;\nglobalThis.changelogBodyForLanguage = changelogBodyForLanguage;`,
  context,
);

test("the sidebar exposes a Changelog page after Personalization", () => {
  assert.match(panelHtml, /data-page-target="changelog"/);
  assert.match(panelHtml, /data-page="changelog"/);
  assert.match(panelHtml, /id="changelog-releases"/);
  assert.match(panelSource, /changelog: \{ title: "navChangelog"/);
  assert.match(panelSource, /invoke\("get_changelog_markdown"\)/);
});

test("parses published changelog sections and skips Unreleased", () => {
  const releases = context.parseChangelogReleases(changelogMarkdown);
  assert.ok(releases.length >= 2);
  const version = panelSource.match(/let currentAppVersion = "([^"]+)"/)?.[1];
  const pending = changelogMarkdown.split("## [Unreleased]")[1]?.split(/\n## \[/)[0];
  const target = pending?.match(/Target version: (\d+\.\d+\.\d+)\./)?.[1];
  assert.equal(target || releases[0].version, version);
  assert.equal(panelHtml.split(`data-app-version>${version}<`).length - 1, 2);
  assert.match(releases[0].date, /^\d{4}-\d{2}-\d{2}$/);
  assert.match(releases[0].body, /### English/);
  assert.ok([...changelogMarkdown.matchAll(/^### (.+)$/gm)]
    .every((match) => match[1].trim() === "English"));
  assert.doesNotMatch(releases[0].body, /Pending change|Unreleased/);
  assert.equal(releases.some((release) => release.version === "Unreleased"), false);
});

test("selects every Relay interface language from a multilingual section", () => {
  const body = [
    "### English\n\n#### Added\n\n- New feature.",
    "### Français\n\n#### Ajouté\n\n- Nouvelle fonctionnalité.",
    "### Español\n\n#### Añadido\n\n- Nueva función.",
    "### Deutsch\n\n#### Hinzugefügt\n\n- Neue Funktion.",
    "### Русский\n\n#### Добавлено\n\n- Новая функция.",
    "### 简体中文\n\n#### 新增\n\n- 新功能。",
    "### 한국어\n\n#### 추가\n\n- 새 기능.",
    "### 日本語\n\n#### 追加\n\n- 新機能。",
    "### Bahasa Indonesia\n\n#### Ditambahkan\n\n- Fitur baru.",
  ].join("\n\n");
  assert.match(context.changelogBodyForLanguage(body, "en"), /New feature/);
  assert.doesNotMatch(context.changelogBodyForLanguage(body, "en"), /Nouvelle fonctionnalité/);
  assert.match(context.changelogBodyForLanguage(body, "fr"), /Nouvelle fonctionnalité/);
  assert.match(context.changelogBodyForLanguage(body, "es"), /Nueva función/);
  assert.match(context.changelogBodyForLanguage(body, "de"), /Neue Funktion/);
  assert.match(context.changelogBodyForLanguage(body, "ru"), /Новая функция/);
  assert.match(context.changelogBodyForLanguage(body, "zh"), /新功能/);
  assert.match(context.changelogBodyForLanguage(body, "ko"), /새 기능/);
  assert.match(context.changelogBodyForLanguage(body, "ja"), /新機能/);
  assert.match(context.changelogBodyForLanguage(body, "id"), /Fitur baru/);
  assert.match(context.changelogBodyForLanguage(body, "pt"), /New feature/);
});

test("English release notes are available in every interface language", () => {
  const releases = context.parseChangelogReleases(changelogMarkdown);
  const body = releases.find((release) => release.version === "1.3.1").body;
  const english = context.changelogBodyForLanguage(body, "en");
  assert.ok(english.length > 0);
  for (const language of ["fr", "es", "de", "ru", "zh", "ko", "ja", "id"]) {
    assert.equal(context.changelogBodyForLanguage(body, language), english);
  }
});

test("an up-to-date local build shows its installed version, not the older GitHub release", () => {
  const control = () => ({ classList: { toggle() {} } });
  const status = { textContent: "" };
  const local = vm.createContext({
    currentAppVersion: "1.3.6g", updateUiState: { kind: "current", version: "1.3.6" },
    latestUpdate: { updateAvailable: false }, updateStatusElement: status,
    updateCheckButton: control(), installUpdateButton: control(), updateAvailableDot: control(),
    t: (key) => key, formatTranslation: (key, values) => `${key}: ${values.version}`,
  });
  vm.runInContext(sourceBetween(panelSource, "function renderUpdateStatus", "function setUpdateMenuOpen")
    + "\nrenderUpdateStatus();", local);
  assert.equal(status.textContent, "upToDate: 1.3.6g");
});
