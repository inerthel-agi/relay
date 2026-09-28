const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");

const panelHtml = fs.readFileSync(__dirname + "/../gui/panel.html", "utf8");
const panelSource = require("./test-source.cjs").panelSource();
const panelCss = require("./test-source.cjs").panelStyles();
const traySource = fs.readFileSync(__dirname + "/../gui/tray.js", "utf8");
const trayCss = fs.readFileSync(__dirname + "/../gui/tray.css", "utf8");
const tauriConfig = JSON.parse(fs.readFileSync(__dirname + "/../src-tauri/tauri.conf.json", "utf8"));

const fonts = {
  bricolage: "BricolageGrotesque.woff2",
  "dm-sans": "DMSans.woff2",
  figtree: "Figtree.woff2",
  inter: "Inter.woff2",
  "jetbrains-mono": "JetBrainsMono.woff2",
  manrope: "Manrope.woff2",
  poppins: "Poppins.woff2",
  "space-grotesk": "SpaceGrotesk.woff2",
};

test("personalization exposes every bundled interface font", () => {
  assert.match(panelHtml, /id="interface-font"/);
  assert.match(panelHtml, /value="design" data-i18n="fontDesignDefault"/);
  for (const name of Object.keys(fonts)) {
    assert.match(panelHtml, new RegExp(`option value="${name}"`), name);
    assert.match(panelSource, new RegExp(`"${name}"`), name);
    assert.match(panelCss, new RegExp(`data-interface-font="${name}"`), name);
    assert.match(trayCss, new RegExp(`data-interface-font="${name}"`), name);
  }
});

test("every interface font is a local WOFF2 file with licensing metadata", () => {
  for (const file of Object.values(fonts)) {
    const path = `${__dirname}/../gui/assets/fonts/${file}`;
    const bytes = fs.readFileSync(path);
    assert.equal(bytes.subarray(0, 4).toString("ascii"), "wOF2", file);
    assert.match(panelCss, new RegExp(`assets/fonts/${file.replace(".", "\\.")}`), file);
  }
  assert.ok(fs.existsSync(`${__dirname}/../gui/assets/fonts/OFL-1.1.txt`));
  assert.ok(fs.existsSync(`${__dirname}/../gui/assets/fonts/SOURCES.md`));
  assert.match(panelHtml, /(?:^|;)\s*font-src 'self'(?:;|$)/);
  assert.match(tauriConfig.app.security.csp, /(?:^|;)\s*font-src 'self'(?:;|$)/);
});

test("the selected font is persisted for the panel and tray only", () => {
  assert.match(panelSource, /localStorage\.setItem\("relay-interface-font", interfaceFont\)/);
  assert.match(panelSource, /document\.documentElement\.dataset\.interfaceFont = interfaceFont/);
  assert.match(traySource, /localStorage\.getItem\("relay-interface-font"\)/);
  assert.doesNotMatch(panelSource, /set_interface_preferences[\s\S]{0,100}interfaceFont/);
});
