(function notificationOverlayApp() {
const cardElement = document.querySelector("#notification");
const avatarElement = document.querySelector("#notification-avatar");
const authorElement = document.querySelector("#notification-author");
const guildTagElement = document.querySelector("#notification-guild-tag");
const guildTagBadgeElement = document.querySelector("#notification-guild-tag-badge");
const guildTagNameElement = document.querySelector("#notification-guild-tag-name");
const messageElement = document.querySelector("#notification-message");
const parameters = new URLSearchParams(window.location.search);
const relaySecret = RelayConnection.secret(parameters);
const target = parameters.get("target") === "widget" ? "widget" : "obs";
const isPreview = parameters.get("preview") === "1";
const outputClient = isPreview ? "preview" : target === "widget" ? "widget" : "obs";
let interfaceLanguage = parameters.get("lang") || "en";
const moveLabelElement = document.querySelector("#notification-move-label");
const moveLabels = { en: "Move notification", fr: "Déplacer la notification", es: "Mover notificación", de: "Benachrichtigung verschieben" };
const previewCopy = {
  en: { author: "Live preview", message: "Your notification will appear here." },
  fr: { author: "Aperçu en direct", message: "Votre notification apparaîtra ici." },
  es: { author: "Vista previa", message: "Tu notificación aparecerá aquí." },
  de: { author: "Live-Vorschau", message: "Deine Benachrichtigung erscheint hier." },
};
const fallbackAvatar = "/overlay-assets/relay-radar.png";
const anonymousNames = {
  en: "Anonymous", fr: "Anonyme", es: "Anónimo", de: "Anonym", ru: "Аноним",
  zh: "匿名", ko: "익명", ja: "匿名", id: "Anonim",
};
const queue = [];

let config = {
  showAuthor: true,
  notificationsObsEnabled: false,
  notificationQueueLimit: 50,
  notificationDurationMs: 8000,
  notificationSoundEnabled: false,
  notificationSoundObsEnabled: false,
  mediaVolume: 50,
  notificationObsGeometry: {},
  notificationWidgetGeometry: {},
};
let pingElement;
let currentNotification;
let socket;
let reconnectTimer;
let reconnectDelayMs = 1000;
let pendingPort;
let isUnloading = false;
let playbackGeneration = 0;
let visualTimer;
let pinnedNotification;
let reportedNotificationId;
// A reconnect waits for the server's pin snapshot before draining the local
// queue, so an already pinned message cannot be replaced by a queued item.
let messagePinStateReady = true;
/** True while overlay media or YouTube holds the shared stage. */
let mediaBusy = false;
/** True while server reports an active YouTube track (may be deferred locally on overlay). */
let musicActive = false;
/** True while this notification client (or a peer notification client) holds the stage. */
let notificationBusy = false;
let reportedNotificationStageBusy = false;
let notificationStageClaimPending = false;
let lastStageClockPayload = {};

function notificationSocketUrl(
  host,
  client = outputClient,
  protocol = window.location.protocol === "https:" ? "wss:" : "ws:",
) {
  return `${protocol}//${host}/ws?role=notification&source=notification&client=${encodeURIComponent(client)}&secret=${encodeURIComponent(relaySecret)}`;
}

document.documentElement.classList.toggle("notification-widget", target === "widget");

function isEnabled(notification) {
  return target === "widget"
    || Boolean(config.notificationsObsEnabled)
    || Boolean(notification?.relayTest);
}

function queueLimit() {
  return Math.min(50, Math.max(1, Number(config.notificationQueueLimit) || 50));
}

function displayDuration() {
  return Math.min(60000, Math.max(1000, Number(config.notificationDurationMs) || 8000));
}

function applyOutputGeometry() {
  const placement = target === "widget" ? config.notificationWidgetGeometry : config.notificationObsGeometry;
  const stage = document.querySelector(".notification-stage");
  if (stage) {
    const anchor = placement?.anchor || "legacy";
    stage.style.justifyContent = anchor === "legacy" ? "" : anchor.endsWith("Left") ? "flex-start" : anchor.endsWith("Right") ? "flex-end" : "center";
    stage.style.alignItems = anchor === "legacy" ? "" : anchor.startsWith("bottom") ? "flex-end" : anchor === "center" ? "center" : "flex-start";
    stage.style.padding = anchor === "legacy" ? "" : (placement.marginY || 0) + "px " + (placement.marginX || 0) + "px";
    cardElement.style.alignSelf = anchor === "legacy" ? "" : "auto";
  }
  const geometry = target === "widget"
    ? config.notificationWidgetGeometry
    : config.notificationObsGeometry;
  const crop = (value) => Math.min(40, Math.max(0, Number(value) || 0));
  const scale = Math.min(400, Math.max(50, Number(geometry?.contentScale) || 100));
  const rootStyle = document.documentElement.style;
  rootStyle.setProperty("--crop-top", `${crop(geometry?.cropTop)}%`);
  rootStyle.setProperty("--crop-right", `${crop(geometry?.cropRight)}%`);
  rootStyle.setProperty("--crop-bottom", `${crop(geometry?.cropBottom)}%`);
  rootStyle.setProperty("--crop-left", `${crop(geometry?.cropLeft)}%`);
  rootStyle.setProperty("--content-scale", String(scale / 100));
}

function setGuildTag(guildTag) {
  const name = typeof guildTag?.name === "string" ? guildTag.name.trim() : "";
  if (!name) {
    guildTagElement.hidden = true;
    guildTagNameElement.textContent = "";
    guildTagBadgeElement.hidden = true;
    guildTagBadgeElement.onerror = null;
    guildTagBadgeElement.removeAttribute("src");
    return;
  }

  guildTagNameElement.textContent = name;
  const badgeUrl = typeof guildTag.badgeUrl === "string" ? guildTag.badgeUrl : "";
  if (badgeUrl) {
    guildTagBadgeElement.onerror = () => {
      guildTagBadgeElement.onerror = null;
      guildTagBadgeElement.hidden = true;
    };
    guildTagBadgeElement.src = badgeUrl;
    guildTagBadgeElement.hidden = false;
  } else {
    guildTagBadgeElement.hidden = true;
    guildTagBadgeElement.onerror = null;
    guildTagBadgeElement.removeAttribute("src");
  }
  guildTagElement.hidden = false;
}

function setCardIdentity(notification) {
  authorElement.textContent = config.showAuthor
    ? notification.author?.username || "Discord"
    : anonymousNames[interfaceLanguage] || anonymousNames.en;
  setGuildTag(config.showAuthor ? notification.guildTag : undefined);
  avatarElement.onerror = () => {
    avatarElement.onerror = null;
    avatarElement.src = fallbackAvatar;
  };
  avatarElement.src = config.showAuthor
    ? notification.author?.displayAvatarUrl || fallbackAvatar
    : fallbackAvatar;
}

function setCardContent(notification) {
  const segments = Array.isArray(notification.segments) ? notification.segments : [];
  cardElement.classList.toggle("is-sticker-only", segments.some((segment) => segment.kind === "sticker" && segment.url)
    && segments.every((segment) => segment.kind === "sticker" || !String(segment.value || "").trim()));
  setCardIdentity(notification);
  messageElement.replaceChildren();
  if (Array.isArray(notification.segments)) {
    for (const segment of notification.segments) {
      if ((segment.kind === "emoji" || segment.kind === "sticker") && segment.url) {
        const image = document.createElement("img");
        image.className = segment.kind === "sticker"
          ? "notification-card__sticker"
          : "notification-card__emoji";
        image.src = segment.url;
        image.alt = segment.value || segment.kind;
        image.onerror = () => image.replaceWith(document.createTextNode(segment.value || ""));
        messageElement.append(image);
      } else {
        messageElement.append(document.createTextNode(segment.value || ""));
      }
    }
  } else {
    messageElement.textContent = notification.text || "";
  }
}

function showCard() {

  cardElement.classList.add("is-visible");
  cardElement.setAttribute("aria-hidden", "false");
}

function normalizeInterfaceLanguage(value) {
  const primary = String(value || "en").split(/[-_]/)[0].toLowerCase();
  return ["en", "fr", "es", "de", "ru", "zh", "ko", "ja", "id"].includes(primary) ? primary : "en";
}

function showPreview() {
  const copy = previewCopy[interfaceLanguage] || previewCopy.en;
  setCardContent(parameters.get("sample") === "sticker"
    ? { author: { username: copy.author }, segments: [{ kind: "sticker", url: "/overlay-assets/relay-radar.png", value: "Relay" }] }
    : { author: { username: copy.author }, text: copy.message });
  showCard();
}

function playNotificationPing() {
  if (isPreview) {
    return;
  }
  const enabled = target === "widget"
    ? Boolean(config.notificationSoundEnabled)
    : Boolean(config.notificationSoundObsEnabled);
  if (!enabled) {
    return;
  }
  if (!pingElement) {
    if (typeof Audio !== "function") {
      return;
    }
    pingElement = new Audio();
  }
  pingElement.volume = Math.min(1, Math.max(0, (Number(config.mediaVolume) || 50) / 100));
  pingElement.src = `/notification-sound?secret=${encodeURIComponent(relaySecret)}`;
  pingElement.play().catch(() => {});
}

function hideCard() {
  cardElement.classList.remove("is-visible");
  cardElement.setAttribute("aria-hidden", "true");
}

/** Ends the current card's timer; a newer generation ignores stale callbacks. */
function resetPlayback() {
  playbackGeneration += 1;
  window.clearTimeout(visualTimer);
}

function stageBlocked() {
  return mediaBusy || musicActive;
}

function syncNotificationStageBusy(busy, force = false) {
  if (isPreview || socket?.readyState !== 1) return;
  if (!force && busy === reportedNotificationStageBusy) return;
  reportedNotificationStageBusy = busy;
  if (!busy) notificationBusy = false;
  socket.send(JSON.stringify({
    type: "stageClock",
    payload: { lane: "notification", busy },
  }));
}

function sameNotification(left, right) {
  if (!left || !right) return false;
  if (left.id && right.id) return left.id === right.id;
  return left === right;
}

function syncNotificationVisibility(visible, notification = currentNotification) {
  if (isPreview || socket?.readyState !== 1) return;
  if (visible) {
    const id = typeof notification?.id === "string" ? notification.id : "";
    if (!id || reportedNotificationId === id) return;
    reportedNotificationId = id;
    socket.send(JSON.stringify({
      type: "notificationState",
      payload: { visible: true, notification },
    }));
    return;
  }
  if (!reportedNotificationId) return;
  const id = reportedNotificationId;
  reportedNotificationId = undefined;
  socket.send(JSON.stringify({
    type: "notificationState",
    payload: { visible: false, id },
  }));
}

function currentNotificationIsPinned() {
  return sameNotification(currentNotification, pinnedNotification);
}

function holdCurrentNotification() {
  if (!currentNotificationIsPinned()) return;
  // A pinned card stays visible and releases the shared stage for media and music.
  window.clearTimeout(visualTimer);
  syncNotificationStageBusy(false);
}

function applyMessagePin(payload = {}) {
  messagePinStateReady = true;
  const pinned = Boolean(payload.pinned) && payload.message;
  if (!pinned) {
    const wasPinned = Boolean(pinnedNotification);
    pinnedNotification = undefined;
    if (!wasPinned) {
      if (!currentNotification) playNext();
      return;
    }
    if (!currentNotification) {
      playNext();
      return;
    }
    finishCurrent(playbackGeneration);
    return;
  }

  pinnedNotification = payload.message;
  if (currentNotificationIsPinned()) {
    holdCurrentNotification();
    return;
  }

  if (currentNotification) {
    syncNotificationVisibility(false);
    resetPlayback();
    currentNotification = undefined;
    hideCard();
    syncNotificationStageBusy(false);
  }
  if (!isEnabled(pinnedNotification)) return;
  currentNotification = pinnedNotification;
  setCardContent(currentNotification);
  showCard();
  syncNotificationVisibility(true, currentNotification);
  syncNotificationStageBusy(false);
}

function applyStageClock(payload = {}) {
  lastStageClockPayload = payload && typeof payload === "object" ? payload : {};
  mediaBusy = Boolean(payload.mediaBusy);
  // Prefer server musicBusy when present so a missed musicIdle cannot strand
  // the OBS notification lane.
  if (Object.prototype.hasOwnProperty.call(payload, "musicBusy")) {
    musicActive = Boolean(payload.musicBusy);
  }
  notificationBusy = Boolean(payload.notificationBusy);
  resolveNotificationStageClaim();
  if (!stageBlocked() && !currentNotification && !notificationStageClaimPending) playNext();

}

function setMusicActive(active, { resume = false } = {}) {
  musicActive = Boolean(active);
  if (resume && !stageBlocked() && !notificationStageClaimPending) playNext();
}

function resolveNotificationStageClaim() {
  if (!notificationStageClaimPending) return;
  if (
    (lastStageClockPayload.granted === false && lastStageClockPayload.lane === "notification")
    || mediaBusy
    || musicActive
  ) {
    notificationStageClaimPending = false;
    reportedNotificationStageBusy = false;
    return;
  }
  if (!notificationBusy) return;
  notificationStageClaimPending = false;
  beginCurrentNotification();
}

function beginCurrentNotification() {
  if (currentNotification || queue.length === 0 || !isEnabled(queue[0])) {
    syncNotificationStageBusy(false);
    return;
  }
  currentNotification = queue.shift();
  const generation = playbackGeneration;
  setCardContent(currentNotification);
  // Exclusive stage: never leave a music card visible under a notification.

  showCard();
  playNotificationPing();
  syncNotificationVisibility(true, currentNotification);
  // The claim that caused this begin was sent before the server grant. Echo
  // the held state after the visibility report so clients observe both in a
  // deterministic order; the server treats an existing claim as idempotent.
  syncNotificationStageBusy(true, true);
  visualTimer = window.setTimeout(() => completeCurrentPlayback(generation), displayDuration());
}

function completeCurrentPlayback(expectedGeneration = playbackGeneration) {
  if (!currentNotification || expectedGeneration !== playbackGeneration) {
    return;
  }
  if (currentNotificationIsPinned()) {
    resetPlayback();
    holdCurrentNotification();
    return;
  }
  finishCurrent(expectedGeneration);
}

function finishCurrent(expectedGeneration = playbackGeneration) {
  if (!currentNotification || expectedGeneration !== playbackGeneration) {
    return;
  }
  syncNotificationVisibility(false, currentNotification);
  resetPlayback();
  currentNotification = undefined;
  hideCard();
  syncNotificationStageBusy(false);
  playNext();

}

function playNext() {
  if (
    currentNotification
    || queue.length === 0
    || !isEnabled(queue[0])
    || stageBlocked()
    || notificationStageClaimPending
    || !messagePinStateReady
  ) {
    return;
  }
  notificationStageClaimPending = true;
  syncNotificationStageBusy(true);
}

function enqueue(notification) {
  if (!isEnabled(notification)) {
    return;
  }
  if (currentNotificationIsPinned() && sameNotification(notification, pinnedNotification)) {
    return;
  }
  if (queue.length >= queueLimit()) {
    return;
  }
  queue.push(notification);
  playNext();
}

function clearNotifications() {
  queue.length = 0;
  if (currentNotification) {
    syncNotificationVisibility(false, currentNotification);
    resetPlayback();
    currentNotification = undefined;
  }
  pinnedNotification = undefined;
  notificationStageClaimPending = false;
  syncNotificationStageBusy(false);

  hideCard();

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
    applyOutputGeometry();

    const configuredPort = Number(message.payload?.port);
    if (
      Number.isInteger(configuredPort)
      && configuredPort > 0
      && configuredPort <= 65535
    ) {
      pendingPort = String(configuredPort) !== window.location.port
        ? configuredPort
        : undefined;
    }
    queue.length = Math.min(queue.length, queueLimit());
    if (!isEnabled()) {
      clearNotifications();
    }
    if (isPreview) showPreview();
    else if (currentNotification) setCardIdentity(currentNotification);
  } else if (message.type === "notification") {
    if (isPreview) return;
    if (message.payload) enqueue(message.payload);
  } else if (message.type === "messagePin") {
    if (isPreview) return;
    applyMessagePin(message.payload);
  } else if (message.type === "musicPlay") {
    if (isPreview) return;
    // YouTube renders in the media overlay. Notifications only track the
    // authoritative music occupancy so notifications wait on every output target.
    setMusicActive(true);
  } else if (message.type === "musicStop") {
    // Keep authoritative occupancy until musicIdle or stageClock releases it.
  } else if (message.type === "musicIdle") {
    if (isPreview) return;

    setMusicActive(false, { resume: true });

  } else if (message.type === "stageClock") {
    if (isPreview) return;
    applyStageClock(message.payload);
  } else if (message.type === "testOutput") {
    if (isPreview) return;
    const outputTest = message.payload;
    if (outputTest?.target === "notification" && outputTest.notification) {
      enqueue({ ...outputTest.notification, relayTest: true });
    }
  } else if (message.type === "skip") {
    if (isPreview || currentNotificationIsPinned()) return;
    finishCurrent();
  } else if (message.type === "clear") {
    if (isPreview) return;
    clearNotifications();
  } else if (message.type === "appearance") {
    applyAppearance(message.payload);
  }
}

function applyAppearance(preferences = {}) {
  interfaceLanguage = normalizeInterfaceLanguage(preferences.language || interfaceLanguage);
  document.documentElement.lang = interfaceLanguage;
  document.documentElement.dataset.theme = preferences.theme || "dark";
  const rgb = Array.isArray(preferences.accentRgb) ? preferences.accentRgb : [88, 185, 137];
  document.documentElement.style.setProperty("--accent", `rgb(${rgb.join(" ")})`);
  document.documentElement.style.setProperty("--font-scale", String((preferences.fontScale || 100) / 100));
  globalThis.relayOutputTheme?.applyOutputTheme(document.documentElement, preferences);
  if (moveLabelElement) moveLabelElement.textContent = moveLabels[interfaceLanguage] || moveLabels.en;
  if (isPreview) showPreview();
  else if (currentNotification) setCardIdentity(currentNotification);
}

applyAppearance({ language: interfaceLanguage, fontScale: 100, accentRgb: [88, 185, 137] });
applyOutputGeometry();

function scheduleReconnect() {
  if (isUnloading || reconnectTimer) {
    return;
  }
  reconnectTimer = window.setTimeout(() => {
    reconnectTimer = undefined;
    connect();
  }, reconnectDelayMs);
  reconnectDelayMs = Math.min(reconnectDelayMs * 2, 10000);
}

function moveToPendingPort() {
  RelayConnection.moveToPort(pendingPort, (host) => notificationSocketUrl(host, "probe", "ws:"), () => isUnloading);
}

function connect() {
  socket = new WebSocket(notificationSocketUrl(window.location.host));
  socket.addEventListener("open", () => {
    reconnectDelayMs = 1000;
    playNext();
  });
  socket.addEventListener("message", handleMessage);
  socket.addEventListener("close", () => {
    // Stop the current notification but keep the queue for after the reconnect.
    if (currentNotification) {
      resetPlayback();
      currentNotification = undefined;
    }
    reportedNotificationId = undefined;
    messagePinStateReady = false;
    reportedNotificationStageBusy = false;
    notificationStageClaimPending = false;
    notificationBusy = false;

    mediaBusy = false;
    // Retain music occupancy until the server supplies a fresh stage clock.
    if (!isPreview) {
      hideCard();
    }
    if (pendingPort) {
      moveToPendingPort();
      return;
    }
    scheduleReconnect();
  });
  socket.addEventListener("error", () => socket.close());
}

window.setWidgetLocked = (locked) => {
  document.documentElement.classList.toggle("widget-edit", !locked);
};

if (target === "widget") {
  const locked = parameters.get("locked") === "1";
  window.setWidgetLocked(locked);
  const moveLayer = document.querySelector("#notification-move-layer");
  moveLayer?.addEventListener("pointerdown", (event) => {
    if (event.button !== 0) {
      return;
    }
    const tauri = window.__TAURI__;
    const current =
      tauri?.webviewWindow?.getCurrentWebviewWindow?.() ||
      tauri?.window?.getCurrentWindow?.();
    current?.startDragging?.().catch(() => {});
  });
}

// Widget windows are not web pages: no browser menu on right click.
window.addEventListener("contextmenu", (event) => event.preventDefault());
window.addEventListener("beforeunload", () => {
  isUnloading = true;
  window.clearTimeout(reconnectTimer);

  socket?.close();
});

connect();
}());
