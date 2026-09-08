const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const read = file => fs.readFileSync(path.join(__dirname, "..", file), "utf8");
test("every module action is registered and permitted only in the bundled control panel", () => {
  const commands = new Set([...(read("gui/modules.mjs") + read("gui/reaction-trim.mjs")).matchAll(/invoke\("([a-z_]+)"/g)].map(match => match[1]));
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
  const csp = JSON.parse(read("src-tauri/tauri.conf.json")).app.security.csp;
  assert.match(csp, /img-src[^;]+blob:/);
  assert.match(csp, /media-src[^;]+blob:/);
  assert.match(csp, /script-src 'self';/);
  assert.doesNotMatch(csp, /unsafe-eval|script-src[^;]*blob:/);
});
