const stickerElement = document.querySelector("#sticker");
const fallbackElement = document.querySelector("#sticker-fallback");
const parameters = new URLSearchParams(window.location.search);
const relaySecret = RelayConnection.secret(parameters);
const queue = [];

function stickerSocketUrl(
  host,
  client = "obs",
  protocol = window.location.protocol === "https:" ? "wss:" : "ws:",
) {
  return `${protocol}//${host}/ws?role=sticker&source=sticker&client=${encodeURIComponent(client)}&secret=${encodeURIComponent(relaySecret)}`;
}

let config = { stickerDurationMs: 8000 };
let currentSticker;
let displayTimer;
let loadWatchdog;
let reconnectTimer;
let reconnectDelayMs = 1000;
let socket;
let pendingPort;
let isUnloading = false;
let mediaBusy = false;
let musicActive = false;
let notificationBusy = false;
let reportedMediaStageBusy = false;
let mediaStageClaimPending = false;
let lastStageClockPayload = {};

function queueLimit() {
  return 50;
}

function stageBlocked() {
  return (mediaBusy && !mediaStageClaimPending && !currentSticker)
    || musicActive
    || notificationBusy;
}

function stickerSource(sticker) {
  if (sticker.cachedMediaId) {
    return `/media-cache/${encodeURIComponent(sticker.cachedMediaId)}?secret=${encodeURIComponent(relaySecret)}`;
  }
  return sticker.url || "";
}

function syncMediaStageBusy() {
  if (socket?.readyState !== 1) return;
  const busy = Boolean(currentSticker || mediaStageClaimPending);
  if (busy === reportedMediaStageBusy) return;
  reportedMediaStageBusy = busy;
  socket.send(JSON.stringify({
    type: "stageClock",
    payload: { lane: "media", busy },
  }));
}

function resetSticker() {
  window.clearTimeout(displayTimer);
  window.clearTimeout(loadWatchdog);
  stickerElement.onload = null;
  stickerElement.onerror = null;
  stickerElement.classList.remove("is-visible");
  fallbackElement.classList.remove("is-visible");
  stickerElement.removeAttribute("src");
  fallbackElement.hidden = true;
}

function finishCurrent() {
  if (!currentSticker) return;
  resetSticker();
  currentSticker = undefined;
  syncMediaStageBusy();
  playNext();
}

function reveal(element) {
  window.requestAnimationFrame(() => element.classList.add("is-visible"));
  const duration = Math.min(60000, Math.max(1000, Number(config.stickerDurationMs) || 8000));
  displayTimer = window.setTimeout(finishCurrent, duration);
}

function showFallback() {
  stickerElement.removeAttribute("src");
  fallbackElement.hidden = false;
  reveal(fallbackElement);
}

function beginCurrentSticker() {
  if (currentSticker || queue.length === 0) {
    syncMediaStageBusy();
    return;
  }
  currentSticker = queue.shift();
  resetSticker();
  syncMediaStageBusy();
  if (currentSticker.format === "lottie" || currentSticker.format === "unknown") {
    showFallback();
    return;
  }
  stickerElement.onload = () => {
    window.clearTimeout(loadWatchdog);
    reveal(stickerElement);
  };
  stickerElement.onerror = () => {
    window.clearTimeout(loadWatchdog);
    showFallback();
  };
  stickerElement.alt = currentSticker.name || "Discord sticker";
  stickerElement.src = stickerSource(currentSticker);
  loadWatchdog = window.setTimeout(showFallback, 12000);
}

function resolveMediaStageClaim() {
  if (!mediaStageClaimPending) return;
  if (
    (lastStageClockPayload.granted === false && lastStageClockPayload.lane === "media")
    || notificationBusy
    || musicActive
  ) {
    mediaStageClaimPending = false;
    reportedMediaStageBusy = false;
    return;
  }
  if (!mediaBusy) return;
  mediaStageClaimPending = false;
  beginCurrentSticker();
}

function playNext() {
  if (currentSticker || queue.length === 0 || stageBlocked() || mediaStageClaimPending) return;
  mediaStageClaimPending = true;
  syncMediaStageBusy();
}

function enqueue(sticker) {
  if (!sticker || queue.length >= queueLimit()) return;
  queue.push(sticker);
  playNext();
}

function clearStickers() {
  queue.length = 0;
  resetSticker();
  currentSticker = undefined;
  mediaStageClaimPending = false;
  syncMediaStageBusy();
}

function interruptStickerPlayback() {
  resetSticker();
  currentSticker = undefined;
  mediaStageClaimPending = false;
}

function applyStageClock(payload = {}) {
  lastStageClockPayload = payload && typeof payload === "object" ? payload : {};
  mediaBusy = Boolean(payload.mediaBusy);
  if (Object.prototype.hasOwnProperty.call(payload, "musicBusy")) {
    musicActive = Boolean(payload.musicBusy);
  }
  notificationBusy = Boolean(payload.notificationBusy);
  resolveMediaStageClaim();
  if (!stageBlocked() && !currentSticker && !mediaStageClaimPending) playNext();
}

// Reloads a page kept open across a Relay restart (often an update).
const reloadAfterRelayRestart = RelayConnection.reloadOnRestart();

function handleMessage(event) {
  let message;
  try {
    message = JSON.parse(event.data);
  } catch {
    return;
  }
  if (message.type === "config") {
    if (reloadAfterRelayRestart(message.payload?.relaySession)) return;
    config = { ...config, ...message.payload };
    const configuredPort = Number(message.payload?.port);
    if (Number.isInteger(configuredPort) && configuredPort > 0 && configuredPort <= 65535
      && String(configuredPort) !== window.location.port) {
      pendingPort = configuredPort;
    }
  } else if (message.type === "sticker") {
    enqueue(message.payload);
  } else if (message.type === "testOutput") {
    const outputTest = message.payload;
    if (outputTest?.target === "sticker" && outputTest.sticker) {
      enqueue(outputTest.sticker);
    }
  } else if (message.type === "stageClock") {
    applyStageClock(message.payload);
  } else if (message.type === "musicPlay") {
    musicActive = true;
  } else if (message.type === "musicIdle") {
    musicActive = false;
    if (!stageBlocked()) playNext();
  } else if (message.type === "skip") {
    // The skip shortcut also ends a sticker, which releases the media stage.
    finishCurrent();
  } else if (message.type === "clear") {
    clearStickers();
  }
}

function scheduleReconnect() {
  if (isUnloading || reconnectTimer) return;
  reconnectTimer = window.setTimeout(() => {
    reconnectTimer = undefined;
    connect();
  }, reconnectDelayMs);
  reconnectDelayMs = Math.min(reconnectDelayMs * 2, 10000);
}

function moveToPendingPort() {
  RelayConnection.moveToPort(pendingPort, (host) => stickerSocketUrl(host, "probe", "ws:"), () => isUnloading);
}

function connect() {
  socket = new WebSocket(stickerSocketUrl(window.location.host));
  socket.addEventListener("open", () => {
    reconnectDelayMs = 1000;
    playNext();
  });
  socket.addEventListener("message", handleMessage);
  socket.addEventListener("close", () => {
    interruptStickerPlayback();
    mediaBusy = false;
    musicActive = false;
    notificationBusy = false;
    mediaStageClaimPending = false;
    reportedMediaStageBusy = false;
    if (pendingPort) {
      moveToPendingPort();
      return;
    }
    scheduleReconnect();
  });
  socket.addEventListener("error", () => socket.close());
}

// Widget windows are not web pages: no browser menu on right click.
window.addEventListener("contextmenu", (event) => event.preventDefault());
window.addEventListener("beforeunload", () => {
  isUnloading = true;
  clearStickers();
  window.clearTimeout(reconnectTimer);
  socket?.close();
});

connect();
