const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const read = file => fs.readFileSync(path.join(__dirname, "..", file), "utf8");
test("every module action is registered and permitted only in the bundled control panel", () => {
  const sources = ["modules.mjs", "reaction-trim.mjs", "panic.mjs", "discord-check.mjs", "obs-setup.mjs", "moderation-ui.mjs", "stream-style.mjs", "autosave.mjs", "overview.mjs", "panel.js", "tray.js"]
    .map((file) => read(`gui/${file}`)).join("\n");
  const commands = new Set([...sources.matchAll(/invoke\("([a-z_]+)"/g)].map(match => match[1]));
  const handlers = read("src-tauri/src/lib.rs");
  const manifest = read("src-tauri/build.rs");
  const permissions = read("src-tauri/permissions/relay.toml");
  for (const command of commands) {
    assert.match(handlers, new RegExp(`\\b${command}\\s*,`), command);
    assert.ok(manifest.includes(`"${command}"`), command + " is declared to Tauri");
    assert.ok(permissions.includes(`"allow-${command.replaceAll("_", "-")}"`), command + " is allowed in panel");
  }
  const local = JSON.parse(read("src-tauri/capabilities/default.json"));
  const outputs = JSON.parse(read("src-tauri/capabilities/overlay.json"));
  assert.ok(local.permissions.includes("allow-app-ipc"));
  assert.ok(outputs.windows.includes("reaction-widget"));
  assert.ok(!outputs.permissions.includes("allow-app-ipc"));
  assert.ok(outputs.permissions.every(permission => permission.startsWith("core:window:")));
});
test("module previews have local blob support without widening script or network origins", () => {
  // The panel page adds its own meta CSP: the stricter of both applies, so check both.
  const panelCsp = read("gui/panel.html").match(/http-equiv="Content-Security-Policy"\s+content="([^"]+)"/)[1];
  for (const csp of [JSON.parse(read("src-tauri/tauri.conf.json")).app.security.csp, panelCsp]) {
    assert.match(csp, /img-src[^;]+blob:/);
    assert.match(csp, /media-src[^;]+blob:/);
    assert.match(csp, /img-src[^;]+https:\/\/media\.tenor\.com/);
    assert.match(csp, /script-src 'self';/);
    assert.doesNotMatch(csp, /unsafe-eval|script-src[^;]*blob:|asset:/);
  }
});
