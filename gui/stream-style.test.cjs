const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");

const { describeStreamStyle, initializeStreamStyle, normalizeStreamBackground, normalizeStreamStyle, streamStyles } = require("./stream-style.mjs");
const { translations } = require("./translations.mjs");

const t = (key) => key;
const format = (key, values) => `${key}:${JSON.stringify(values)}`;
const panelHtml = fs.readFileSync(__dirname + "/panel.html", "utf8");

test("unknown stored values fall back to automatic", () => {
  assert.equal(normalizeStreamStyle("signal"), "signal");
  assert.equal(normalizeStreamStyle("psn"), "auto");
  assert.equal(normalizeStreamStyle(null), "auto");
  assert.equal(normalizeStreamBackground("light"), "light");
  assert.equal(normalizeStreamBackground("grey"), "auto");
});

test("the status line says whether notifications follow Relay", () => {
  const automatic = describeStreamStyle({ style: "auto", background: "auto", design: "paper", theme: "dark" }, t, format);
  assert.equal(automatic.isAutomatic, true);
  assert.equal(automatic.text, 'streamStyleFollows:{"style":"Paper","background":"dark"}');
  const custom = describeStreamStyle({ style: "subtitle", background: "light", design: "paper", theme: "dark" }, t, format);
  assert.equal(custom.isAutomatic, false);
  assert.equal(custom.text, 'streamStyleCustom:{"style":"streamStyleSubtitle","background":"light"}');
  const background = describeStreamStyle({ style: "auto", background: "light", design: "neo-brutalism", theme: "dark" }, t, format);
  assert.equal(background.text, 'streamStyleCustom:{"style":"Neo-Brutalism","background":"light"}');
});

function radio(name, value) {
  return { name, value, checked: false, listeners: {}, addEventListener(type, listener) { this.listeners[type] = listener; }, focus() { this.focused = true; } };
}

test("choosing a style saves it, updates the preview state and offers a way back", () => {
  const styleInputs = streamStyles.map((value) => radio("stream-style", value));
  const backgroundInputs = ["auto", "light", "dark"].map((value) => radio("stream-background", value));
  const elements = {
    "#stream-style-status": { textContent: "" },
    "#stream-style-reset": { hidden: true, listeners: {}, addEventListener(type, listener) { this.listeners[type] = listener; } },
    "#stream-style-preview": { src: "" },
  };
  const stored = {};
  global.localStorage = { setItem: (key, value) => { stored[key] = value; } };
  let changes = 0;
  try {
    const ui = initializeStreamStyle({
      $: (selector) => elements[selector],
      $$: (selector) => (selector.includes("stream-style") ? styleInputs : backgroundInputs),
      t, formatTranslation: format,
      readStorage: (key) => (key === "relay-output-style" ? "lumen" : null),
      getDesign: () => "graphite", getTheme: () => "light",
      onChange: () => { changes += 1; },
    });
    assert.deepEqual(ui.preferences(), { outputStyle: "lumen", outputBackground: "auto" });
    assert.equal(styleInputs.find((input) => input.value === "lumen").checked, true);
    assert.equal(elements["#stream-style-reset"].hidden, false);

    styleInputs.find((input) => input.value === "signal").listeners.change();
    backgroundInputs.find((input) => input.value === "dark").listeners.change();
    assert.deepEqual(ui.preferences(), { outputStyle: "signal", outputBackground: "dark" });
    assert.deepEqual(stored, { "relay-output-style": "signal", "relay-output-background": "dark" });
    assert.equal(changes, 2);

    elements["#stream-style-reset"].listeners.click();
    assert.deepEqual(ui.preferences(), { outputStyle: "auto", outputBackground: "auto" });
    assert.equal(elements["#stream-style-reset"].hidden, true);
    assert.equal(styleInputs[0].focused, true);
    assert.equal(changes, 3);

    ui.setPreviewPort(4901, "fr");
    assert.equal(elements["#stream-style-preview"].src, "http://127.0.0.1:4901/notifications?preview=1&lang=fr");
  } finally {
    delete global.localStorage;
  }
});

test("the panel offers every style as a keyboard-friendly radio group", () => {
  for (const value of streamStyles) assert.match(panelHtml, new RegExp(`name="stream-style" value="${value}"`));
  for (const value of ["auto", "light", "dark"]) assert.match(panelHtml, new RegExp(`name="stream-background" value="${value}"`));
  assert.match(panelHtml, /id="stream-style-preview"/);
  assert.match(panelHtml, /name="interface-design" value="signal"/);
});

test("the notification style is translated in every interface language", () => {
  const keys = ["signalDesignCopy", "streamStyleTitle", "streamStyleCopy", "streamStyleAuto", "streamStyleSubtitle", "streamBackgroundLabel", "streamStyleFollows", "streamStyleCustom", "streamStyleReset"];
  for (const [language, dictionary] of Object.entries(translations)) {
    for (const key of keys) assert.ok(dictionary[key], `${language}.${key}`);
    assert.match(dictionary.streamStyleFollows, /\{style\}[\s\S]*\{background\}/, language);
  }
});
