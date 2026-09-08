const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

const panelSource = require("./test-source.cjs").panelSource().replace(/\r\n/g, "\n");
const panelHtml = fs.readFileSync(__dirname + "/panel.html", "utf8");
const commandsSource = fs.readFileSync(__dirname + "/../src-tauri/src/commands.rs", "utf8").replace(/\r\n/g, "\n");

function sourceBetween(source, start, end) {
  const startIndex = source.indexOf(start);
  const endIndex = source.indexOf(end, startIndex);
  assert.notEqual(startIndex, -1, `missing source marker: ${start}`);
  assert.notEqual(endIndex, -1, `missing source marker: ${end}`);
  return source.slice(startIndex, endIndex);
}

test("music size controls use the media widget config and limits", () => {
  const applySizeSource = sourceBetween(
    panelSource,
    "function applyMusicOverlaySize",
    "function applyOutputGeometryTarget",
  );
  const saveSizeSource = sourceBetween(
    panelSource,
    "if (saveMusicOverlayButton)",
    "for (const [target, button] of outputTestButtons)",
  );

  assert.match(applySizeSource, /config\.widgetWidth \?\? 640/);
  assert.match(applySizeSource, /config\.widgetHeight \?\? 360/);
  assert.doesNotMatch(applySizeSource, /config\.(?:notificationWidget|musicWidget)/);
  assert.match(saveSizeSource, /clamp\(musicWidgetWidthElement\.value, 160, 16384\)/);
  assert.match(saveSizeSource, /clamp\(musicWidgetHeightElement\.value, 90, 16384\)/);
  assert.match(panelHtml, /id="music-widget-width"[^>]*min="160"[^>]*max="16384"/);
  assert.match(panelHtml, /id="music-widget-height"[^>]*min="90"[^>]*max="16384"/);
  assert.match(
    panelSource,
    /if \(target === "mediaWidget"\)[\s\S]*?applyMusicOverlaySize\(config, force\)/,
  );
});

test("saving music size refreshes both inputs and media geometry from returned config", async () => {
  const applyMusicSource = sourceBetween(
    panelSource,
    "function applyMusicOverlaySize",
    "function applyOutputGeometryTarget",
  );
  const applyGeometrySource = sourceBetween(
    panelSource,
    "function applyOutputGeometryTarget",
    "function applyOutputGeometryConfig",
  );
  const handlerMarker = 'saveMusicOverlayButton.addEventListener("click", async () => {';
  const handlerSource = sourceBetween(panelSource, handlerMarker, "\n  });\n}");
  const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;

  const musicWidth = { value: "700" };
  const musicHeight = { value: "700" };
  const geometryWidth = { value: "640" };
  const geometryHeight = { value: "360" };
  const keepRatio = { checked: false };
  const card = {
    contains: () => false,
    querySelector(selector) {
      if (selector === '[data-size-field="width"]') return geometryWidth;
      if (selector === '[data-size-field="height"]') return geometryHeight;
      if (selector === "[data-keep-aspect-ratio]") return keepRatio;
      if (selector === "[data-output-anchor]") return { value: "legacy" };
      throw new Error(`unexpected selector: ${selector}`);
    },
  };
  const createRefreshFunctions = new Function(
    "document",
    "musicWidgetWidthElement",
    "musicWidgetHeightElement",
    "outputGeometryGridElement",
    "outputGeometryTargets",
    "$$",
    `${applyMusicSource}\n${applyGeometrySource}\nreturn { applyMusicOverlaySize, applyOutputGeometryTarget };`,
  );
  const refreshFunctions = createRefreshFunctions(
    { activeElement: null },
    musicWidth,
    musicHeight,
    { querySelector: () => card },
    { mediaWidget: { configKey: "mediaWidgetGeometry" } },
    () => [],
  );
  const returnedConfig = {
    widgetWidth: 800,
    widgetHeight: 450,
    widgetKeepAspectRatio: true,
    mediaWidgetGeometry: {},
  };
  const bootstrap = { config: null };
  let directMusicRefreshes = 0;
  let geometryRefreshes = 0;
  const handler = new AsyncFunction(
    "invoke",
    "clamp",
    "musicWidgetWidthElement",
    "musicWidgetHeightElement",
    "musicOverlaySaveStateElement",
    "bootstrap",
    "applyMusicOverlaySize",
    "applyOutputGeometryTarget",
    "t",
    handlerSource.slice(handlerMarker.length),
  );

  await handler(
    async (command, payload) => {
      assert.equal(command, "set_music_widget_size");
      assert.deepEqual(payload, { width: 700, height: 700 });
      return returnedConfig;
    },
    (value, minimum, maximum) => Math.min(maximum, Math.max(minimum, Number(value))),
    musicWidth,
    musicHeight,
    { textContent: "" },
    bootstrap,
    (...args) => {
      directMusicRefreshes += 1;
      return refreshFunctions.applyMusicOverlaySize(...args);
    },
    (...args) => {
      geometryRefreshes += 1;
      return refreshFunctions.applyOutputGeometryTarget(...args);
    },
    (key) => key,
  );

  assert.equal(bootstrap.config, returnedConfig);
  assert.deepEqual(
    {
      music: [musicWidth.value, musicHeight.value],
      geometry: [geometryWidth.value, geometryHeight.value],
      keepRatio: keepRatio.checked,
    },
    {
      music: ["800", "450"],
      geometry: ["800", "450"],
      keepRatio: true,
    },
  );
  assert.equal(geometryRefreshes, 1);
  assert.equal(directMusicRefreshes, 0);
});

test("music size command updates only the media widget", () => {
  const musicCommand = sourceBetween(
    commandsSource,
    "pub async fn set_music_widget_size",
    "#[tauri::command]\npub async fn save_credentials",
  );
  const geometryCommand = sourceBetween(
    commandsSource,
    "pub async fn set_output_geometry",
    "#[tauri::command]\npub async fn set_music_widget_size",
  );

  assert.match(musicCommand, /widget_keep_aspect_ratio/);
  assert.match(musicCommand, /widget::clamp_requested_size\(/);
  assert.match(musicCommand, /config\.widget_width = width/);
  assert.match(musicCommand, /config\.widget_height = height/);
  assert.match(musicCommand, /widget::apply_configured_size\(&app, width, height\)/);
  assert.doesNotMatch(musicCommand, /notification_widget::/);
  assert.doesNotMatch(musicCommand, /config\.(?:notification_widget|music_widget)_/);
  assert.doesNotMatch(geometryCommand, /config\.music_widget_(?:width|height)/);
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
  "en": "Windows Now Playing uses the 16:9 media floating widget. Message notifications use the separate compact notification widget.",
  "fr": "Windows Now Playing utilise le widget flottant multimédia 16:9. Les notifications de messages utilisent le widget compact séparé.",
  "es": "Windows Now Playing usa el widget flotante multimedia 16:9. Las notificaciones de mensajes usan el widget compacto independiente.",
  "de": "Windows Now Playing verwendet das schwebende 16:9-Medienwidget. Nachrichtenbenachrichtigungen verwenden das separate kompakte Widget.",
  "ru": "Windows Now Playing использует плавающий медиавиджет 16:9. Уведомления сообщений используют отдельный компактный виджет.",
  "zh": "Windows Now Playing 使用 16:9 媒体浮动小组件。消息通知使用独立的紧凑小组件。",
  "ko": "Windows Now Playing은 16:9 미디어 플로팅 위젯을 사용합니다. 메시지 알림은 별도의 소형 위젯을 사용합니다.",
  "ja": "Windows Now Playing は 16:9 のメディアフローティングウィジェットを使用します。メッセージ通知は別のコンパクトなウィジェットを使用します。",
  "id": "Windows Now Playing menggunakan widget mengambang media 16:9. Notifikasi pesan menggunakan widget ringkas yang terpisah."
};

  for (const [language, copy] of Object.entries(expectedCopy)) {
    assert.equal(translations[language].musicOverlayCopy, copy, language);
    if (language !== "en") {
      for (const key of ["musicWidgetWidth", "musicWidgetHeight", "saveMusicOverlay"]) {
        assert.notEqual(translations[language][key], translations.en[key], `${language}.${key}`);
      }
    }
  }

  assert.match(panelHtml, />Windows Now Playing uses the 16:9 media floating widget\. Message notifications use the separate compact notification widget\.</);
  assert.match(panelHtml, /data-i18n="musicWidgetWidth">Media widget width</);
  assert.match(panelHtml, /data-i18n="musicWidgetHeight">Media widget height</);
  assert.match(panelHtml, /data-i18n="saveMusicOverlay">Save media widget size</);
});
