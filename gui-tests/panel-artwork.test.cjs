const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");
const test = require("node:test");
const source = require("./test-source.cjs").panelSource();
const helpers = source.slice(source.indexOf("function releaseNowPlayingArtwork"), source.indexOf("function renderNowPlaying"));
function harness(invoke) {
  class Reader {
    readAsDataURL(blob) {
      blob.arrayBuffer().then((bytes) => {
        this.result = "data:application/octet-stream;base64," + Buffer.from(bytes).toString("base64");
        this.onload();
      });
    }
  }
  const context = vm.createContext({ invoke, FileReader: Reader, Blob, Uint8Array,
    nowPlayingArtworkRequest: 0, artworkCache: new Map(),
    nowPlayingArtworkElement: {}, currentAudioPlayback: { media: { artworkId: "1" } },
  });
  vm.runInContext(helpers, context);
  return context;
}
test("artwork bytes become CSP-compatible data URLs and share one native request", async () => {
  let calls = 0;
  const context = harness(async () => { calls++; return [137, 80, 78, 71]; });
  const [first, second] = await Promise.all([context.loadArtwork("1"), context.loadArtwork("1")]);
  assert.equal(first, second);
  assert.equal(calls, 1);
  assert.deepEqual([...Buffer.from(first.split(",")[1], "base64")], [137, 80, 78, 71]);
  const csp = JSON.parse(fs.readFileSync(__dirname + "/../src-tauri/tauri.conf.json", "utf8")).app.security.csp;
  assert.match(csp, /img-src[^;]*data:/);
});
test("a late artwork response cannot replace a newer track", async () => {
  let finish;
  const context = harness(() => new Promise((resolve) => { finish = resolve; }));
  const pending = context.loadNowPlayingArtwork({ artworkId: "1" });
  context.releaseNowPlayingArtwork();
  context.nowPlayingArtworkElement.src = "new-track";
  finish(new Uint8Array([1, 2]).buffer);
  await pending;
  assert.equal(context.nowPlayingArtworkElement.src, "new-track");
});
test("unavailable artwork can be retried instead of caching its failure", async () => {
  let calls = 0;
  const context = harness(async () => { if (++calls === 1) throw Error("expired"); return [1]; });
  await assert.rejects(context.loadArtwork("1"));
  assert.match(await context.loadArtwork("1"), /^data:/);
});
test("audio history thumbnails load the embedded artwork", async () => {
  const context = harness(async () => [1, 2]);
  context.bootstrap = undefined;
  context.isVideoThumbnail = () => false;
  const thumbnail = {};
  vm.runInContext(source.slice(source.indexOf("function setMediaThumbnail"), source.indexOf("function replaceHistory")), context);
  context.setMediaThumbnail({ querySelector: () => thumbnail }, { artworkId: "1", filename: "track.mp3" }, "audio");
  await context.loadArtwork("1");
  await new Promise(setImmediate);
  assert.match(thumbnail.src, /^data:/);
  thumbnail.onerror();
  assert.equal(thumbnail.src, "./assets/relay-radar.png");
});
