const test = require("node:test");
const assert = require("node:assert/strict");
const { renderAccessMenu, roleChoicesForChannels } = require("./reaction-access.mjs");

class Element extends EventTarget {
  constructor(tag) { super(); this.tag = tag; this.children = []; this.value = ""; }
  append(...children) { for (const child of children) { child.parent = this; this.children.push(child); } }
  replaceChildren() { this.children = []; }
  setAttribute() {}
  remove() { this.parent.children = this.parent.children.filter(child => child !== this); }
  focus() {}
}
const all = root => [root, ...root.children.flatMap(all)];
function setup(t, choices, selectedIds) {
  const previous = global.document;
  global.document = { createElement: tag => new Element(tag) };
  t.after(() => global.document = previous);
  const root = new Element("div"); const changes = [];
  renderAccessMenu(root, { choices, selectedIds, kind: "channel", t: key => key, onChange: ids => changes.push(ids) });
  return { root, changes };
}
test("saved IDs are shown as named choices without changing the configuration", t => {
  const { root, changes } = setup(t, [{ id: "123", name: "test-reactions", guildId: "g1", guildName: "Community" }], ["123"]);
  assert.equal(all(root).find(element => element.tag === "select").value, "123");
  assert.ok(all(root).some(element => element.textContent === "# test-reactions — Community"));
  assert.deepEqual(changes, []);
});
test("unavailable choices remain selected until explicitly removed", t => {
  const { root, changes } = setup(t, [], ["123"]);
  assert.equal(all(root).find(element => element.tag === "select").value, "123");
  assert.ok(all(root).some(element => element.textContent === "modUnknownChoice — 123"));
  all(root).find(element => element.tag === "button" && element.textContent === "modRemove").dispatchEvent(new Event("click"));
  assert.deepEqual(changes, [[]]);
});
test("dropdown changes persist identifiers, not display labels", t => {
  const { root, changes } = setup(t, [{ id: "456", name: "music", guildId: "g1", guildName: "Community" }], []);
  const select = all(root).find(element => element.tag === "select"); select.value = "456";
  select.dispatchEvent(new Event("change"));
  assert.deepEqual(changes, [["456"]]);
});
test("roles are restricted to the servers of the selected channels", () => {
  const member = { id: "r1", guildId: "g1", name: "Member" }, foreign = { id: "r2", guildId: "g2", name: "Member" };
  const catalog = { channels: [{ id: "c1", guildId: "g1" }], roles: [member, foreign] };
  assert.deepEqual(roleChoicesForChannels(catalog, ["c1"]), [member]);
});
