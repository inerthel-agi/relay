const assert = require("node:assert/strict");
const test = require("node:test");
const vm = require("node:vm");
const fs = require("node:fs");
const source = fs.readFileSync(__dirname + "/reactions.js", "utf8").replace(/connect\(\);\s*$/, "");
test("the idle reaction widget has no visible move arrow", () => {
  const html = fs.readFileSync(__dirname + "/index.html", "utf8");
  assert.doesNotMatch(html, /reaction-move|↔/);
  assert.match(source, /card\.addEventListener\("pointerdown"/);
});
function harness(search = "") {
  const timers = new Map(); const sent = []; let plays = 0;
  const elements = Object.fromEntries(["#reaction", "#visual", "#sound", "#name"].map(id => [id, { hidden: true, style: {}, offsetWidth: 200, offsetHeight: 100, addEventListener() {}, removeAttribute(name) { delete this[name]; }, pause() {}, load() {}, play() { plays++; return Promise.resolve(); } }]));
  const context = vm.createContext({ document: { querySelector: selector => elements[selector] || { content: "test" } }, location: { origin: "http://localhost:4590", search }, URL, URLSearchParams, window: { addEventListener() {} }, innerWidth: 800, innerHeight: 600, RelayLayout: { position: () => null }, WebSocket: { OPEN: 1 }, setTimeout(fn) { const id = timers.size + 1; timers.set(id, fn); return id; }, clearTimeout(id) { timers.delete(id); } });
  vm.runInContext(source + "\nsocket = { readyState: 1, send: value => report(value) }; globalThis.api = { play, finish };", context);
  context.report = value => sent.push(JSON.parse(value));
  return { api: context.api, elements, sent, timers, plays: () => plays };
}
const playback = id => ({ playbackId: id, endsAt: Date.now() + 15000, reaction: { name: "<script>safe text</script>", soundId: "a".repeat(32), visualId: null, volume: 70 } });
test("repeated reaction frames do not replay a sound and natural end acknowledges its ID", () => {
  const h = harness(); const value = playback("first"); h.api.play(value); h.api.play(value);
  assert.equal(h.plays(), 1);
  assert.equal(h.elements["#reaction"].hidden, true);
  assert.doesNotMatch(fs.readFileSync(__dirname + "/index.html", "utf8"), /id="name"/);
  h.elements["#sound"].onended(); assert.equal(h.elements["#reaction"].hidden, true); assert.deepEqual(h.sent, [{ type: "reactionEnded", payload: "first" }]);
});
test("reconnect seek uses the server duration and keeps legacy payloads usable", () => {
  const h = harness(); h.elements["#sound"].duration = 30;
  h.api.play({ ...playback("duration"), endsAt: Date.now() + 20_000, durationMs: 30_000 });
  h.elements["#sound"].onloadedmetadata();
  assert.ok(h.elements["#sound"].currentTime >= 8 && h.elements["#sound"].currentTime <= 12);

  h.api.play(null); h.elements["#sound"].duration = 20;
  h.api.play({ ...playback("legacy"), endsAt: Date.now() + 15_000 });
  h.elements["#sound"].onloadedmetadata();
  assert.ok(h.elements["#sound"].currentTime >= 3 && h.elements["#sound"].currentTime <= 7);
});
test("late playback callbacks cannot stop a newer reaction", () => {
  const h = harness(); h.api.play(playback("first")); const oldError = h.elements["#sound"].onerror;
  const second = playback("second"); second.reaction.visualId = "b".repeat(32);
  h.api.play(second); oldError(); assert.equal(h.elements["#reaction"].hidden, false); assert.equal(h.sent.length, 0);
  h.api.play(null); assert.equal(h.elements["#reaction"].hidden, true);
});
test("expired reaction snapshots remain silent", () => {
  const h = harness(); h.api.play({ ...playback("expired"), endsAt: Date.now() - 1 }); assert.equal(h.plays(), 0);
});


test("an output decoding failure does not stop playback on the other outputs", () => {
  const h = harness(); h.api.play(playback("failed-output"));
  h.elements["#sound"].onerror();
  assert.deepEqual(h.sent, []);
  assert.equal(h.elements["#reaction"].hidden, true);
});


test("a rejected play promise cannot cancel the other reaction outputs", async () => {
  const h = harness();
  h.elements["#sound"].play = () => Promise.reject(new Error("Playback blocked"));
  h.api.play(playback("blocked-output"));
  await Promise.resolve();
  assert.deepEqual(h.sent, []);
});


test("the invisible Windows receiver plays audio without the visual widget sound setting", () => {
  const h = harness("?client=widget&audioOnly=1");
  const value = playback("windows-audio"); value.reaction.visualId = "b".repeat(32);
  h.api.play(value);
  assert.equal(h.plays(), 1);
  assert.equal(h.elements["#sound"].muted, false);
  assert.equal(h.elements["#sound"].volume, 0.7);
  assert.equal(h.elements["#reaction"].hidden, true);
  assert.equal(h.elements["#visual"].src, undefined);
});

test("the OBS receiver remains audible alongside the Windows receiver", () => {
  const h = harness(); h.api.play(playback("obs-audio"));
  assert.equal(h.elements["#sound"].muted, false);
  assert.equal(h.plays(), 1);
});
