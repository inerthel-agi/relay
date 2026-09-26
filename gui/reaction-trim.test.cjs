const assert = require("node:assert/strict");
const test = require("node:test");
const { selectReactionSound, validTrimRange, trimErrorText } = require("./reaction-trim.mjs");

class Element extends EventTarget {
  constructor(tag) { super(); this.tagName = tag; this.children = []; this.value = ""; this.disabled = false; }
  get valueAsNumber() { return this.value === "" ? NaN : Number(this.value); }
  append(...children) { this.children.push(...children); }
  setAttribute() {}
  showModal() { this.open = true; }
  close() { this.open = false; }
  remove() { this.removed = true; }
  focus() {}
}
const tick = () => new Promise(resolve => setImmediate(resolve));
test("trim failures retain the concrete backend cause", () => {
  assert.equal(trimErrorText("Audio trimming requires FFmpeg.", () => "Unable to prepare excerpt."), "Unable to prepare excerpt. Audio trimming requires FFmpeg.");
});
function setup(t, handler = async () => undefined) {
  const saved = { document: global.document, window: global.window, Audio: global.Audio };
  const body = new Element("body"); const calls = []; const audio = [];
  global.document = { body, activeElement: new Element("button"), createElement: tag => new Element(tag) };
  global.window = new EventTarget();
  global.Audio = class { constructor() { throw new Error("Blob media elements must not be used for trim previews."); } };
  global.window.AudioContext = class {
    constructor() { this.destination = {}; audio.push(this); }
    async resume() { this.resumed = true; }
    async decodeAudioData(buffer) { this.decoded = buffer; return { duration: 12.5 }; }
    createBufferSource() { const context = this; return { connect() {}, start() { context.played = true; }, stop() { context.stopped = true; } }; }
    async close() { this.closed = true; }
  };
  t.after(() => Object.assign(global, saved));
  const invoke = async (command, args) => { calls.push({ command, args }); return handler(command, args); };
  const result = selectReactionSound({ trimToken: "token", durationSeconds: 60, filename: "original.wav" }, { invoke, t: key => key });
  const dialog = body.children[0];
  const controls = dialog.children.find(element => element.className === "reaction-trim__controls");
  const buttons = dialog.children.find(element => element.className === "reaction-trim__actions").children;
  const button = key => buttons.find(element => element.textContent === key);
  const input = (index, value) => { const element = controls.children[index].children[1]; element.value = String(value); element.dispatchEvent(new Event("input")); };
  return { dialog, input, button, calls, result, audio };
}
test("trim range rejects invalid, nonfinite and overlong excerpts", () => {
  assert.equal(validTrimRange(20, 50, 60), true);
  assert.equal(validTrimRange(0, 26.749, 26.749), true);
  for (const range of [[0, 30.001, 60], [-1, 5, 60], [20, 20, 60], [55, 65, 60], [NaN, 30, 60], [0, Infinity, 60]]) assert.equal(validTrimRange(...range), false);
});
test("short audio imports keep the original direct import flow", async () => {
  assert.equal(await selectReactionSound({ soundId: "short-sound" }, {}), "short-sound");
  assert.equal(await selectReactionSound(null, {}), null);
});
test("the selected nonzero excerpt is sent to preview and final import", async t => {
  const h = setup(t, async command => command === "preview_reaction_trim" ? [1, 2, 3] : "saved-clip");
  h.input(0, 20); h.input(1, 32.5);
  h.button("trimPreview").dispatchEvent(new Event("click"));
  assert.equal(h.audio[0].resumed, true);
  assert.equal(h.calls.length, 0);
  await tick();
  assert.equal(h.audio[0].played, true);
  assert.deepEqual(h.calls[0], { command: "preview_reaction_trim", args: { token: "token", startSeconds: 20, endSeconds: 32.5 } });
  h.button("trimConfirm").dispatchEvent(new Event("click"));
  assert.equal(await h.result, "saved-clip");
  assert.equal(h.audio[0].closed, true);
  assert.equal(h.audio[0].stopped, true);
  assert.deepEqual(h.calls[1], { command: "finish_reaction_trim", args: h.calls[0].args });
  assert.equal(h.dialog.removed, true);
});
test("cancel during preview rendering cannot start late audio or save an excerpt", async t => {
  let finishPreview;
  const h = setup(t, command => command === "preview_reaction_trim" ? new Promise(resolve => finishPreview = resolve) : Promise.resolve());
  h.button("trimPreview").dispatchEvent(new Event("click"));
  await tick();
  h.button("trimCancel").dispatchEvent(new Event("click"));
  assert.equal(await h.result, null);
  finishPreview([1, 2, 3]); await tick();
  assert.equal(h.audio[0].played, undefined);
  assert.equal(h.audio[0].closed, true);
  assert.deepEqual(h.calls.map(call => call.command), ["preview_reaction_trim", "cancel_reaction_trim"]);
});
test("closing the page does not cancel a confirmed import already being finalized", async t => {
  let finish;
  const h = setup(t, command => command === "finish_reaction_trim" ? new Promise(resolve => finish = resolve) : Promise.resolve());
  h.button("trimConfirm").dispatchEvent(new Event("click"));
  global.window.dispatchEvent(new Event("beforeunload"));
  assert.deepEqual(h.calls.map(call => call.command), ["finish_reaction_trim"]);
  finish("saved-clip");
  assert.equal(await h.result, "saved-clip");
});
