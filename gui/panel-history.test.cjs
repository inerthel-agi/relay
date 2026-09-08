const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

const panelSource = require("./test-source.cjs").panelSource();
const thumbnailSource = panelSource.slice(
  panelSource.indexOf("function isVideoThumbnail"),
  panelSource.indexOf("function replaceHistory"),
);
const historyRenderSource = panelSource.slice(
  panelSource.indexOf("function renderHistory"),
  panelSource.indexOf("function replaceHistory"),
);

function createHarness() {
  const videos = [];
  const historyListElement = {
    querySelectorAll(selector) {
      assert.equal(selector, "video[data-thumbnail-source]");
      return videos;
    },
  };
  const context = vm.createContext({
    URL,
    Number,
    bootstrap: {
      config: { port: 4590 },
      wsUrl: "ws://127.0.0.1:4590/ws?role=panel&token=private",
    },
    encodeURIComponent,
    historyListElement,
    document: {
      createElement(tagName) {
        assert.equal(tagName, "video");
        const listeners = new Map();
        const video = {
          className: "",
          dataset: {},
          duration: 4,
          src: "",
          currentTime: 0,
          pauseCalls: 0,
          loadCalls: 0,
          addEventListener(type, listener) {
            listeners.set(type, listener);
          },
          emit(type) {
            listeners.get(type)?.();
          },
          load() { this.loadCalls += 1; },
          pause() { this.pauseCalls += 1; },
          removeAttribute(name) {
            if (name === "src") this.src = "";
          },
        };
        videos.push(video);
        return video;
      },
    },
  });
  vm.runInContext(`${thumbnailSource}\nglobalThis.thumbnailFunctions = { isVideoThumbnail, loadHistoryVideoThumbnails, setMediaThumbnail };`, context);
  return { videos, functions: context.thumbnailFunctions };
}

function createItem() {
  const thumbnail = {
    className: "history-item__thumb",
    src: "",
    alt: "",
    replaceWith(replacement) { this.replacement = replacement; },
  };
  return { item: { querySelector: () => thumbnail }, thumbnail };
}

test("history defers and snapshots direct video thumbnails", () => {
  const { functions, videos } = createHarness();
  const { item, thumbnail } = createItem();
  functions.setMediaThumbnail(item, {
    kind: "video", url: "https://cdn.discordapp.com/video.mp4", contentType: "video/mp4",
  }, "video");

  const video = thumbnail.replacement;
  assert.equal(video.poster, "./assets/relay-radar.png");
  assert.equal(video.preload, "none");
  assert.equal(video.dataset.thumbnailSource, "https://cdn.discordapp.com/video.mp4");
  assert.equal(video.src, "");
  assert.equal(video.muted, true);
  assert.equal(video.playsInline, true);

  functions.loadHistoryVideoThumbnails();
  assert.equal(videos.length, 1);
  assert.equal(video.src, "https://cdn.discordapp.com/video.mp4");
  assert.equal(video.loadCalls, 1);
  video.emit("loadeddata");
  assert.equal(video.currentTime, 0.1);
  video.emit("seeked");
  assert.equal(video.pauseCalls, 1);
});

test("history starts loading video thumbnails after rendering", () => {
  const video = {
    dataset: { thumbnailSource: "https://cdn.discordapp.com/video.mp4" },
    src: "",
    loadCalls: 0,
    load() { this.loadCalls += 1; },
  };
  const context = vm.createContext({
    history: [],
    historyEmptyElement: {},
    document: { querySelector: () => null },
    t: (key) => key,
    historyListElement: {
      replaceChildren() {},
      querySelectorAll() { return [video]; },
    },
  });
  vm.runInContext(`${historyRenderSource}\nglobalThis.renderHistoryForTest = renderHistory;`, context);

  context.renderHistoryForTest();

  assert.equal(video.src, "https://cdn.discordapp.com/video.mp4");
  assert.equal(video.loadCalls, 1);
});

test("history uses the authenticated cache for video GIF thumbnails", () => {
  const { functions } = createHarness();
  const { item, thumbnail } = createItem();
  functions.setMediaThumbnail(item, {
    kind: "gif", contentType: "video/mp4", cachedMediaId: "123456789012345678-embed-0",
  }, "gif");

  assert.equal(
    thumbnail.replacement.dataset.thumbnailSource,
    "http://127.0.0.1:4590/media-cache/123456789012345678-embed-0?token=private",
  );
  assert.equal(functions.isVideoThumbnail("gif", "image/gif"), false);
});

const rememberMediaSource = panelSource.slice(
  panelSource.indexOf("function sameHistoryMedia"),
  panelSource.indexOf("function renderModeration"),
);

function createRememberMediaHarness() {
  let renderCalls = 0;
  const history = [];
  const context = vm.createContext({
    history,
    renderHistory() {
      renderCalls += 1;
    },
  });
  vm.runInContext(
    `${rememberMediaSource}\nglobalThis.rememberMediaForTest = rememberMedia;`,
    context,
  );
  return {
    history,
    rememberMedia: context.rememberMediaForTest,
    renderCalls: () => renderCalls,
  };
}

test("rememberMedia skips replayed media that is already in history", () => {
  const { history, rememberMedia, renderCalls } = createRememberMediaHarness();
  const video = {
    kind: "video",
    messageId: "msg-1",
    url: "https://cdn.discordapp.com/attachments/1/TTT.mov",
    filename: "TTT.mov",
  };
  const image = {
    kind: "image",
    messageId: "msg-2",
    url: "https://cdn.discordapp.com/attachments/2/photo.png",
    filename: "photo.png",
  };
  const gif = {
    kind: "gif",
    messageId: "msg-3",
    url: "https://cdn.discordapp.com/attachments/3/clip.gif",
    filename: "clip.gif",
  };
  const audio = {
    kind: "audio",
    messageId: "msg-4",
    url: "https://cdn.discordapp.com/attachments/4/track.mp3",
    filename: "track.mp3",
  };

  for (const media of [video, image, gif, audio]) {
    rememberMedia(media);
  }
  assert.equal(history.length, 4);
  assert.equal(renderCalls(), 4);

  for (const media of [video, image, gif, audio]) {
    rememberMedia({ ...media });
  }
  assert.equal(history.length, 4);
  assert.equal(renderCalls(), 4);
  assert.deepEqual(
    history.map(({ messageId, url }) => ({ messageId, url })),
    [audio, gif, image, video].map(({ messageId, url }) => ({ messageId, url })),
  );
});

test("rememberMedia still records distinct attachments from the same message", () => {
  const { history, rememberMedia } = createRememberMediaHarness();
  rememberMedia({
    kind: "image",
    messageId: "msg-multi",
    url: "https://cdn.discordapp.com/attachments/1/a.png",
  });
  rememberMedia({
    kind: "image",
    messageId: "msg-multi",
    url: "https://cdn.discordapp.com/attachments/1/b.png",
  });
  assert.equal(history.length, 2);
});
