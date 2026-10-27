const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

const panelSource = require("./test-source.cjs").panelSource().replace(/\r\n/g, "\n");
const panelHtml = fs.readFileSync(__dirname + "/../gui/panel.html", "utf8");
const commandsSource = fs.readFileSync(__dirname + "/../src-tauri/src/commands.rs", "utf8");
const libSource = fs.readFileSync(__dirname + "/../src-tauri/src/lib.rs", "utf8");

test("music output settings live on the Outputs page instead of being duplicated", () => {
  assert.doesNotMatch(panelHtml, /id="music-widget-(?:width|height)"|id="youtube-url"|id="notification-url"/);
  const outputsPage = panelHtml.slice(panelHtml.indexOf('data-page="overlay"'), panelHtml.indexOf('data-page="moderation"'));
  assert.match(outputsPage, /id="disclosure-music-3"/);
  assert.doesNotMatch(panelHtml, /data-i18n="openOverlayPage"|data-i18n="outputSettingsOnOverlay"/);
  assert.match(panelSource, /for \(const button of \$\$\("\[data-go-to-page\]"\)\)/);
  assert.match(panelSource, /showPage\(button\.dataset\.goToPage, \{ moveFocus: true \}\)/);
  assert.doesNotMatch(panelSource, /set_music_widget_size|applyMusicOverlaySize/);
  assert.doesNotMatch(commandsSource, /set_music_widget_size/);
  assert.doesNotMatch(libSource, /set_music_widget_size/);
});

test("the Overlay media widget card remains the single size control", () => {
  assert.match(panelSource, /if \(target === "mediaWidget"\) \{\s*const width = card\.querySelector\('\[data-size-field="width"\]'\);/);
  assert.match(panelSource, /if \(!typing\(width\)\) width\.value = String\(Math\.round\(config\.widgetWidth \?\? 640\)\);/);
  assert.match(panelSource, /payload\.width = Math\.round\(clamp\(card\.querySelector\('\[data-size-field="width"\]'\)\.value, 160, 16384\)\);/);
  assert.match(panelSource, /payload\.height = Math\.round\(clamp\(card\.querySelector\('\[data-size-field="height"\]'\)\.value, 90, 16384\)\);/);
});

test("music overlay copy distinguishes media and notification widgets in every locale", () => {
  const dictionarySource = panelSource.slice(
    panelSource.indexOf("const translations ="),
    panelSource.indexOf("const pageMetadata ="),
  );
  const context = vm.createContext({});
  vm.runInContext(
    `${dictionarySource}\nglobalThis.translationsForTest = translations;`,
    context,
  );
  const translations = context.translationsForTest;
  const expectedCopy = {
    en: "Windows Now Playing uses the 16:9 media floating widget. Message notifications use the separate compact notification widget.",
    fr: "Windows Now Playing utilise le widget flottant multimédia 16:9. Les notifications de messages utilisent le widget compact séparé.",
    es: "Windows Now Playing usa el widget flotante multimedia 16:9. Las notificaciones de mensajes usan el widget compacto independiente.",
    de: "Windows Now Playing verwendet das schwebende 16:9-Medienwidget. Nachrichtenbenachrichtigungen verwenden das separate kompakte Widget.",
    ru: "Windows Now Playing использует плавающий медиавиджет 16:9. Уведомления сообщений используют отдельный компактный виджет.",
    zh: "Windows Now Playing 使用 16:9 媒体浮动小组件。消息通知使用独立的紧凑小组件。",
    ko: "Windows Now Playing은 16:9 미디어 플로팅 위젯을 사용합니다. 메시지 알림은 별도의 소형 위젯을 사용합니다.",
    ja: "Windows Now Playing は 16:9 のメディアフローティングウィジェットを使用します。メッセージ通知は別のコンパクトなウィジェットを使用します。",
    id: "Windows Now Playing menggunakan widget mengambang media 16:9. Notifikasi pesan menggunakan widget ringkas yang terpisah.",
  };

  for (const [language, copy] of Object.entries(expectedCopy)) {
    assert.equal(translations[language].musicOverlayCopy, copy, language);
  }
  assert.match(panelHtml, />Windows Now Playing uses the 16:9 media floating widget\. Message notifications use the separate compact notification widget\.</);
});

test("only the two media outputs offer a maximum media size, saved within Rust's bounds", () => {
  const targets = panelSource.slice(panelSource.indexOf("const outputGeometryTargets = {"), panelSource.indexOf("const outputGeometryTimers"));
  const withMaxSize = [...targets.matchAll(/^\s+(\w+): \{[^\n]*maxSize: true/gm)].map((match) => match[1]);
  assert.deepEqual(withMaxSize, ["mediaObs", "mediaWidget"]);
  assert.match(panelSource, /metadata\.maxSize \? geometryControl\("maxSize", "mediaMaxSize", MIN_MEDIA_MAX_SIZE, 100\) : ""/);
  assert.match(panelSource, /maxSize: outputGeometryTargets\[target\]\.maxSize \? value\("maxSize", MIN_MEDIA_MAX_SIZE, 100\) : 100/);
  const rust = require("node:fs").readFileSync(__dirname + "/../src-tauri/src/config.rs", "utf8");
  assert.match(rust, /pub const MIN_MEDIA_MAX_SIZE: u8 = 10;/);
  assert.match(panelSource, /const MIN_MEDIA_MAX_SIZE = 10;/);
});

test("output geometry saves whole numbers and never rewrites the field being typed in", () => {
  assert.match(panelSource, /const value = \(field, minimum, maximum\) => Math\.round\(clamp\(/);
  assert.match(panelSource, /const typing = \(input\) => input === document\.activeElement && input\.type === "number";/);
  assert.match(panelSource, /if \(!typing\(input\)\) input\.value = String\(values\[input\.dataset\.geometryField\]\);/);
  assert.match(panelSource, /if \(input\.type === "number"\) input\.addEventListener\("change", \(\) => queueOutputGeometrySave\(target\)\);/);
  assert.match(panelSource, /payload\.width = Math\.round\(clamp\(/);
});
