const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const customCommands = require("../gui/custom-commands.mjs");

const panelSource = require("./test-source.cjs").panelSource();
const panelHtml = fs.readFileSync(__dirname + "/../gui/panel.html", "utf8");
const helperSource = fs.readFileSync(__dirname + "/../gui/custom-commands.mjs", "utf8");

function helpers() {
  return customCommands;
}

test("the editor exposes every closed custom action", () => {
  const { defaultCustomAction, customActionPermissionKey } = helpers();
  const expected = [
    "ban", "unban", "kick", "timeout", "removeTimeout", "clearMessages", "addRole", "removeRole", "reply",
  ];
  for (const type of expected) {
    assert.equal(defaultCustomAction(type).type, type);
  }
  assert.equal(customActionPermissionKey("ban"), "permissionBanMembers");
  assert.equal(customActionPermissionKey("clearMessages"), "permissionManageMessages");
  assert.equal(customActionPermissionKey("reply"), undefined);
  assert.doesNotMatch(helperSource, /shell|webhook|script action/i);
});

test("Discord IDs and supported mentions are normalized before persistence", () => {
  const { normalizeDiscordId, discordIdListFromInput } = helpers();
  assert.equal(normalizeDiscordId("123456789012345678"), "123456789012345678");
  assert.equal(normalizeDiscordId("<@!223456789012345678>"), "223456789012345678");
  assert.equal(normalizeDiscordId("<@&323456789012345678>"), "323456789012345678");
  assert.equal(normalizeDiscordId("<#423456789012345678>"), "423456789012345678");
  assert.equal(normalizeDiscordId("user 123456789012345678"), null);
  assert.deepEqual(
    JSON.parse(JSON.stringify(discordIdListFromInput("123456789012345678\n<@!123456789012345678>"))),
    ["123456789012345678"],
  );
});

test("default and custom commands are separate and custom saves synchronize through Rust", () => {
  assert.match(panelHtml, /data-i18n="defaultCommands"/);
  assert.match(panelHtml, /data-i18n="customCommands"/);
  assert.match(panelHtml, /id="custom-command-form"/);
  assert.match(panelHtml, /name="custom-permission"/);
  assert.match(panelSource, /invoke\("save_custom_commands", \{ commands:/);
  assert.match(panelSource, /customValidating/);
  assert.match(panelSource, /customSyncing/);
  assert.match(panelSource, /customActive/);
});
