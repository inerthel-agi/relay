const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");
const source = fs.readFileSync(__dirname + "/../overlay/overlay.js", "utf8");
const ducking = source.slice(source.indexOf("let reactionMusicFactor ="), source.indexOf("function handleMessage(event)"));
const youtube = source.slice(source.indexOf("function applyYoutubeAudioSettings()"), source.indexOf("function allowYoutubeAutoplay()"));
test("audio and YouTube restore the latest chosen volume after reaction completion or timeout", () => {
  let youtubeVolume, timeout;
  const context = vm.createContext({ config: { mediaVolume: 80, widgetSoundEnabled: true }, audioElement: { muted: false, volume: .8 }, youtubePlayer: { mute() {}, unMute() {}, setVolume(value) { youtubeVolume = value; } }, isWidgetWindow: false, setTimeout(fn) { timeout = fn; return 1; }, clearTimeout() {} });
  context.window = context;
  vm.runInContext(youtube + ducking, context);
  vm.runInContext("applyReactionDucking({ musicPercent: 25, endsAt: Date.now() + 30000 })", context);
  assert.equal(context.audioElement.volume, .2); assert.equal(youtubeVolume, 20);
  context.config.mediaVolume = 40;
  vm.runInContext("applyReactionDucking(null)", context);
  assert.equal(context.audioElement.volume, .4); assert.equal(youtubeVolume, 40);
  vm.runInContext("applyReactionDucking({ musicPercent: 25, endsAt: Date.now() + 30000 })", context);
  timeout(); assert.equal(context.audioElement.volume, .4); assert.equal(youtubeVolume, 40);
});
test("reaction ducking preserves the Windows widget mute setting", () => {
  let youtubeVolume;
  const context = vm.createContext({ config: { mediaVolume: 80, widgetSoundEnabled: false }, audioElement: { muted: true, volume: 0 }, youtubePlayer: { mute() {}, setVolume(value) { youtubeVolume = value; } }, isWidgetWindow: true, setTimeout() {}, clearTimeout() {} });
  context.window = context;
  vm.runInContext(youtube + ducking, context); vm.runInContext("applyReactionDucking(null)", context);
  assert.equal(context.audioElement.volume, 0); assert.equal(youtubeVolume, 0);
});
