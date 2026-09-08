const imageElement = document.querySelector("#image");
const videoElement = document.querySelector("#video");
const audioElement = document.querySelector("#audio");
const audioCardElement = document.querySelector("#audio-card");
const audioArtworkElement = document.querySelector("#audio-artwork");
const audioTitleElement = document.querySelector("#audio-title");
const audioArtistElement = document.querySelector("#audio-artist");
const audioMediaTextElement = document.querySelector("#audio-media-text");
const audioTimeElement = document.querySelector("#audio-time");
const audioProgressElement = document.querySelector("#audio-progress");
const audioProgressFillElement = document.querySelector("#audio-progress-fill");
let youtubePlayerElement = document.querySelector("#youtube-player");
const youtubeCreditElement = document.querySelector("#youtube-credit");
const youtubeCreditLabelElement = document.querySelector("#youtube-credit-label");
const youtubeCreditChannelElement = document.querySelector("#youtube-credit-channel");
const youtubeCreditSourceElement = document.querySelector("#youtube-credit-source");
const youtubeCreditAddedElement = document.querySelector("#youtube-credit-added");
const youtubeCreditTimeElement = document.querySelector("#youtube-credit-time");
const youtubeCreditProgressFillElement = document.querySelector("#youtube-credit-progress-fill");
const audioAuthorElement = document.querySelector("#audio-author");
const audioAuthorAvatarElement = document.querySelector("#audio-author-avatar");
const audioAuthorNameElement = document.querySelector("#audio-author-name");
const authorElement = document.querySelector("#author");
const authorAvatarElement = document.querySelector("#author-avatar");
const authorNameElement = document.querySelector("#author-name");
const mediaTextElement = document.querySelector("#media-text");
const widgetParameters = new URLSearchParams(window.location.search);
const relaySecret =
  widgetParameters.get("secret")
  || document.querySelector('meta[name="relay-secret"]')?.content
  || "";
const isWidgetWindow = widgetParameters.get("widget") === "1";
const isPreview = widgetParameters.get("preview") === "1";
document.documentElement.classList.toggle("overlay-preview", isPreview);
let interfaceLanguage = widgetParameters.get("lang") || "en";
const relayMode = document.querySelector('meta[name="relay-mode"]')?.content || "all";
const outputClient = isPreview ? "preview" : isWidgetWindow ? "widget" : "obs";
const coordinatesSplitOutputs = outputClient === "obs"
  && (relayMode === "visual" || relayMode === "audio");
const moveLabelElement = document.querySelector("#widget-move-label");
const moveLabels = {
  en: "Move overlay",
  fr: "Déplacer l’overlay",
  es: "Mover overlay",
  de: "Overlay verschieben",
  ru: "Переместить оверлей",
  zh: "移动叠加层",
  ko: "오버레이 이동",
  ja: "オーバーレイを移動",
  id: "Pindahkan overlay",
};
const previewLabels = {
  en: "Live preview",
  fr: "Aperçu en direct",
  es: "Vista previa",
  de: "Live-Vorschau",
  ru: "Предпросмотр",
  zh: "实时预览",
  ko: "실시간 미리보기",
  ja: "ライブプレビュー",
  id: "Pratinjau langsung",
};
const previewCaptionLabels = {
  en: "Discord message shown with the media",
  fr: "Message Discord affiché avec le média",
  es: "Mensaje de Discord mostrado con el medio",
  de: "Discord-Nachricht zum Medium",
  ru: "Сообщение Discord рядом с медиа",
  zh: "随媒体显示的 Discord 消息",
  ko: "미디어와 함께 표시되는 Discord 메시지",
  ja: "メディアと一緒に表示されるDiscordメッセージ",
  id: "Pesan Discord yang ditampilkan bersama media",
};
const youtubeAddedByLabels = {
  en: "Added by",
  fr: "Ajouté par",
  es: "Añadido por",
  de: "Hinzugefügt von",
  ru: "Добавил",
  zh: "添加者",
  ko: "추가한 사용자",
  ja: "追加者",
  id: "Ditambahkan oleh",
};

window.setWidgetLocked = (locked) => {
  document.documentElement.classList.toggle("widget-window", isWidgetWindow);
  document.documentElement.classList.toggle("widget-edit", isWidgetWindow && !locked);
};

window.setWidgetLocked(widgetParameters.get("locked") === "1");

const FADE_DURATION_MS = 320;
const FALLBACK_AVATAR = "/overlay-assets/relay-radar.png";
const VIDEO_COMPOSITOR_INSET_PX = 4;
const VISUAL_MEDIA_KINDS = new Set(["image", "gif", "video"]);
const queue = [];

let config = {
  displayDurationMs: 8000,
  gifDurationMs: 8000,
  mediaVolume: 50,
  showAuthor: true,
  showMediaTextObs: false,
  showMediaTextWidget: false,
  widgetSoundEnabled: false,
  mediaObsGeometry: {},
  mediaWidgetGeometry: {},
};
let currentMedia;
let activeVisual;
let activePlayback;
let clearLoadListeners = () => {};
let displayTimer;
let fadeTimer;
let mediaWatchdog;
let playbackGeneration = 0;
let reconnectTimer;
let reconnectDelayMs = 1000;
let socket;
let pendingPort;
let isUnloading = false;
let widgetPlaybackVisible = true;
let lastAudioPlaybackReport = "";
let stageClockReady = true;
let mediaClockReady = !coordinatesSplitOutputs;
let videoOutputBusy = false;
let audioOutputBusy = false;
let outputLeaseHeld = false;
let outputLeasePending = false;
let waitingForOutputLease = false;
let youtubePlayer;
let youtubePlayerReady = false;
let youtubeApiPromise;
let youtubePendingPlayback;
let youtubePlaybackId;
let youtubeGeneration = 0;
let youtubeActiveGeneration = 0;
/** YouTube payloads waiting because media or TTS is currently on screen. */
let deferredYoutubeQueue = [];
const DEFERRED_YOUTUBE_LIMIT = 20;
/** True while a notification/TTS client holds the shared stage. */
let ttsBusy = false;
/**
 * Peer Discord media occupancy from the server stage clock.
 * On split OBS outputs (/medias vs /youtube) this is the only way the YouTube
 * iframe learns that an image/GIF/video/audio is already on stage.
 */
let mediaBusy = false;
/**
 * Authoritative jukebox occupancy from the server (`stageClock.musicBusy`).
 * Prefer this over local musicPlay flags so a lagged socket cannot strand media.
 */
let musicBusy = false;
let reportedMediaStageBusy = false;
let mediaStageClaimPending = false;
let lastStageClockPayload = {};
let reconnectHydrationPending = false;
let youtubeCreditTimer;
let youtubeCreditRange;

function outputSocketUrl(
  host,
  client = outputClient,
  protocol = window.location.protocol === "https:" ? "wss:" : "ws:",
) {
  return `${protocol}//${host}/ws?role=overlay&source=${encodeURIComponent(relayMode)}&client=${encodeURIComponent(client)}&secret=${encodeURIComponent(relaySecret)}`;
}

function reportAudioPlayback(status, media = currentMedia) {
  if (isPreview || media?.kind !== "audio" || socket?.readyState !== 1) return;
  const reportKey = `${status}:${media.url}`;
  if (reportKey === lastAudioPlaybackReport) return;
  socket.send(JSON.stringify({
    type: "audioPlayback",
    payload: { status, target: isWidgetWindow ? "widget" : "obs", media },
  }));
  lastAudioPlaybackReport = reportKey;
}

function loadYoutubeApi() {
  if (!youtubePlayerElement) {
    return Promise.reject(new Error("YouTube player element is unavailable"));
  }
  if (window.YT?.Player) return Promise.resolve(window.YT);
  if (youtubeApiPromise) return youtubeApiPromise;

  youtubeApiPromise = new Promise((resolve, reject) => {
    const previousReady = window.onYouTubeIframeAPIReady;
    window.onYouTubeIframeAPIReady = () => {
      try {
        if (typeof previousReady === "function") previousReady();
      } finally {
        if (window.YT?.Player) resolve(window.YT);
        else reject(new Error("YouTube player API did not initialize"));
      }
    };
    const script = document.createElement("script");
    script.src = "https://www.youtube.com/iframe_api";
    script.async = true;
    script.addEventListener("error", () => reject(new Error("YouTube player API failed to load")), { once: true });
    document.head.appendChild(script);
  });
  return youtubeApiPromise;
}

function decodeBasicHtmlEntities(value) {
  return String(value ?? "")
    .replace(/&#x([0-9a-f]+);/gi, (match, hex) => {
      const code = Number.parseInt(hex, 16);
      try {
        return Number.isFinite(code) ? String.fromCodePoint(code) : match;
      } catch {
        return match;
      }
    })
    .replace(/&#(\d+);/g, (match, dec) => {
      const code = Number(dec);
      try {
        return Number.isFinite(code) ? String.fromCodePoint(code) : match;
      } catch {
        return match;
      }
    })
    .replace(/&quot;/g, "\"")
    .replace(/&apos;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&");
}

function showYoutubeCredit(payload = {}) {
  if ((relayMode !== "youtube" && !isWidgetWindow) || !youtubeCreditElement) return;
  window.clearTimeout(youtubeCreditTimer);
  youtubeCreditTimer = undefined;
  const title = decodeBasicHtmlEntities(payload.title || "").trim();
  const channel = decodeBasicHtmlEntities(payload.channelTitle || "").trim();
  const primary = title;
  const requester = decodeBasicHtmlEntities(payload.requestedBy || "").trim();
  if (!primary && !requester) {
    hideYoutubeCredit();
    return;
  }
  youtubeCreditElement.classList.toggle("youtube-credit--widget", isWidgetWindow);
  if (youtubeCreditChannelElement) {
    youtubeCreditChannelElement.textContent = primary;
    youtubeCreditChannelElement.hidden = !primary;
  }
  if (youtubeCreditAddedElement) {
    const label = youtubeAddedByLabels[interfaceLanguage] || youtubeAddedByLabels.en;
    youtubeCreditAddedElement.textContent = requester ? `${label} ${requester}` : "";
    youtubeCreditAddedElement.hidden = !requester;
  }
  if (isWidgetWindow) {
    if (youtubeCreditLabelElement) youtubeCreditLabelElement.textContent = "Now playing";
    if (youtubeCreditSourceElement) {
      youtubeCreditSourceElement.textContent = channel;
      youtubeCreditSourceElement.hidden = !channel;
    }
    const start = Math.max(0, Number(payload.startSeconds) || 0);
    const configuredEnd = Number(payload.endSeconds);
    const duration = Number(payload.durationSeconds);
    const end = Number.isFinite(configuredEnd) && configuredEnd > start
      ? configuredEnd
      : Math.max(start, Number.isFinite(duration) ? duration : 0);
    youtubeCreditRange = end > start ? { start, end } : undefined;
    updateYoutubeCreditProgress();
  }
  youtubeCreditElement.hidden = false;
  youtubeCreditElement.setAttribute("aria-hidden", "false");
  youtubeCreditElement.classList.add("is-visible");
}

function updateYoutubeCreditProgress() {
  window.clearTimeout(youtubeCreditTimer);
  youtubeCreditTimer = undefined;
  if (!isWidgetWindow || !youtubeCreditRange) {
    if (youtubeCreditTimeElement) youtubeCreditTimeElement.hidden = true;
    if (youtubeCreditProgressFillElement) youtubeCreditProgressFillElement.style.width = "0%";
    return;
  }
  const { start, end } = youtubeCreditRange;
  const playerTime = Number(youtubePlayer?.getCurrentTime?.());
  const current = Math.min(end, Math.max(start, Number.isFinite(playerTime) ? playerTime : start));
  const percent = ((current - start) / (end - start)) * 100;
  if (youtubeCreditTimeElement) {
    youtubeCreditTimeElement.textContent = `${formatAudioClock(current)} → ${formatAudioClock(end)}`;
    youtubeCreditTimeElement.hidden = false;
  }
  if (youtubeCreditProgressFillElement) {
    youtubeCreditProgressFillElement.style.width = `${percent.toFixed(2)}%`;
  }
  youtubeCreditTimer = window.setTimeout(updateYoutubeCreditProgress, 1000);
}

function hideYoutubeCredit() {
  if (!youtubeCreditElement) return;
  window.clearTimeout(youtubeCreditTimer);
  youtubeCreditTimer = undefined;
  youtubeCreditRange = undefined;
  youtubeCreditElement.classList.remove("is-visible");
  youtubeCreditElement.classList.remove("youtube-credit--widget");
  youtubeCreditElement.hidden = true;
  youtubeCreditElement.setAttribute("aria-hidden", "true");
  if (youtubeCreditChannelElement) {
    youtubeCreditChannelElement.textContent = "";
    youtubeCreditChannelElement.hidden = false;
  }
  if (youtubeCreditSourceElement) {
    youtubeCreditSourceElement.textContent = "";
    youtubeCreditSourceElement.hidden = true;
  }
  if (youtubeCreditAddedElement) {
    youtubeCreditAddedElement.textContent = "";
    youtubeCreditAddedElement.hidden = false;
  }
  if (youtubeCreditTimeElement) {
    youtubeCreditTimeElement.textContent = "";
    youtubeCreditTimeElement.hidden = true;
  }
  if (youtubeCreditProgressFillElement) youtubeCreditProgressFillElement.style.width = "0%";
}

function loadYoutubePendingPlayback() {
  if (
    !youtubePlayerReady
    || !youtubePlayer
    || !youtubePendingPlayback
    || youtubePendingPlayback.generation !== youtubeGeneration
    || typeof youtubePlayer.loadVideoById !== "function"
  ) return;
  const pending = youtubePendingPlayback;
  youtubePendingPlayback = undefined;
  youtubeActiveGeneration = pending.generation;
  const options = {
    videoId: pending.videoId,
    startSeconds: Math.max(0, Number(pending.startSeconds) || 0),
  };
  if (Number.isFinite(Number(pending.endSeconds)) && Number(pending.endSeconds) > options.startSeconds) {
    options.endSeconds = Number(pending.endSeconds);
  }
  showYoutubeCredit(pending);
  youtubePlayer.loadVideoById(options);
  if (typeof youtubePlayer.playVideo === "function") youtubePlayer.playVideo();
}

function applyYoutubeAudioSettings() {
  if (!youtubePlayer) return;
  const muted = isWidgetWindow && !config.widgetSoundEnabled;
  if (muted) youtubePlayer.mute?.();
  else youtubePlayer.unMute?.();
  youtubePlayer.setVolume?.(
    muted ? 0 : Math.min(100, Math.max(0, Number(config.mediaVolume) || 0)) * reactionMusicFactor,
  );
}

function allowYoutubeAutoplay() {
  const frame = youtubePlayer?.getIframe?.();
  if (!frame?.setAttribute) return;
  frame.setAttribute("allow", "autoplay; encrypted-media; picture-in-picture");
  frame.setAttribute("referrerpolicy", "strict-origin-when-cross-origin");
}

/** Soft CC / annotations off. Burned-in hardsubs in the video file cannot be removed. */
function disableYoutubeCaptions() {
  if (!youtubePlayer) return;
  try {
    youtubePlayer.unloadModule?.("captions");
  } catch {
    // Optional API; ignore when unavailable.
  }
  try {
    youtubePlayer.setOption?.("captions", "track", {});
  } catch {
    // Optional API; ignore when unavailable.
  }
}

function unloadYoutubePlayer() {
  hideYoutubeCredit();
  if (youtubePlayer) {
    try {
      if (typeof youtubePlayer.stopVideo === "function") youtubePlayer.stopVideo();
    } catch {
      // Ignore stop failures during teardown.
    }
    try {
      const frame = youtubePlayer.getIframe?.();
      if (frame) {
        // Blank the iframe before destroy so OBS/CEF drops the last frame / chrome.
        try {
          frame.src = "about:blank";
        } catch {
          // Ignore navigation failures on a tearing-down frame.
        }
        frame.removeAttribute("src");
        frame.style.display = "none";
        frame.setAttribute("aria-hidden", "true");
      }
    } catch {
      // Optional API; continue with destroy.
    }
    try {
      if (typeof youtubePlayer.destroy === "function") youtubePlayer.destroy();
    } catch {
      // Ignore destroy failures; still clear local state below.
    }
  }
  youtubePlayer = undefined;
  youtubePlayerReady = false;

  // Nuclear unload for OBS Browser Source: destroy alone can leave compositor
  // cache (play button corner chrome). Replace the host node with a fresh shell.
  const host = youtubePlayerElement || document.querySelector("#youtube-player");
  if (host?.parentNode) {
    const next = document.createElement("div");
    next.id = "youtube-player";
    next.className = "youtube-player";
    next.setAttribute("aria-hidden", "true");
    host.parentNode.replaceChild(next, host);
    youtubePlayerElement = next;
  } else if (host) {
    host.classList.remove("youtube-player--obs");
    host.setAttribute("aria-hidden", "true");
    host.style.display = "none";
    if (typeof host.replaceChildren === "function") host.replaceChildren();
    else host.innerHTML = "";
    youtubePlayerElement = host;
  }
}

function createYoutubePlayer() {
  if (!youtubePlayerElement || youtubePlayer || !window.YT?.Player) return;
  // Dedicated OBS /youtube and the combined Windows widget show the full video.
  const showVideo = relayMode === "youtube" || isWidgetWindow;
  const playbackId = youtubePendingPlayback?.playbackId || youtubePlaybackId;
  const generation = youtubePendingPlayback?.generation || youtubeGeneration;
  let createdPlayer;
  const isCurrentPlayer = () => (
    createdPlayer === youtubePlayer
    && playbackId === youtubePlaybackId
    && generation === youtubeGeneration
  );
  youtubePlayerReady = false;
  youtubePlayerElement.style.display = "";
  if (showVideo) {
    youtubePlayerElement.classList.add("youtube-player--obs");
    youtubePlayerElement.setAttribute("aria-hidden", "false");
  }
  createdPlayer = new window.YT.Player(youtubePlayerElement.id || "youtube-player", {
    width: showVideo ? "100%" : "1",
    height: showVideo ? "100%" : "1",
    playerVars: {
      autoplay: 1,
      cc_load_policy: 0,
      controls: 0,
      disablekb: 1,
      fs: 0,
      iv_load_policy: 3,
      modestbranding: 1,
      mute: 1,
      origin: window.location.origin,
      playsinline: 1,
      rel: 0,
    },
    events: {
      onReady: () => {
        if (!isCurrentPlayer()) return;
        youtubePlayerReady = true;
        allowYoutubeAutoplay();
        disableYoutubeCaptions();
        // Mute first so OBS/CEF autoplay is allowed, then apply intended volume.
        youtubePlayer.mute?.();
        loadYoutubePendingPlayback();
        applyYoutubeAudioSettings();
      },
      onStateChange: (event) => {
        if (!isCurrentPlayer()) return;
        if (event.data === (window.YT?.PlayerState?.PLAYING ?? 1)) {
          allowYoutubeAutoplay();
          disableYoutubeCaptions();
          applyYoutubeAudioSettings();
          return;
        }
        if (event.data === window.YT?.PlayerState?.ENDED) {
          finishYoutubePlayback(playbackId, generation, true);
        }
      },
      onError: () => {
        // A failed embed (e.g. OBS still on 127.0.0.1 getting YouTube error 150) must not
        // clear server-side playback while another output (Windows widget) still plays.
        if (finishYoutubePlayback(playbackId, generation, false)) {
          playDeferredYoutubeOrNextMedia();
        }
      },
    },
  });
  youtubePlayer = createdPlayer;
}

function finishYoutubePlayback(playbackId, generation, notifyServer = true) {
  if (!playbackId || playbackId !== youtubePlaybackId || generation !== youtubeGeneration) return false;
  if (
    notifyServer
    && !isPreview
    && (outputClient === "obs" || outputClient === "widget")
    && socket?.readyState === 1
  ) {
    socket.send(JSON.stringify({ type: "musicEnded", payload: { playbackId, completed: true } }));
  }
  youtubePendingPlayback = undefined;
  youtubePlaybackId = undefined;
  youtubeActiveGeneration = 0;
  unloadYoutubePlayer();
  syncMediaStageBusy();
  // Server may reply with musicPlay (next queued track) or musicIdle (resume media).
  // Do not advance media here — that would race a following MusicPlay.
  return true;
}

function removeDeferredYoutubePlayback(playbackId) {
  if (!playbackId) return false;
  const previousLength = deferredYoutubeQueue.length;
  deferredYoutubeQueue = deferredYoutubeQueue.filter(
    (playback) => playback.playbackId !== playbackId,
  );
  return deferredYoutubeQueue.length !== previousLength;
}

function stopYoutubePlayback(playbackId) {
  const removedDeferred = removeDeferredYoutubePlayback(playbackId);
  const activeId = youtubePlaybackId || youtubePendingPlayback?.playbackId;
  if (playbackId && activeId !== playbackId) {
    return removedDeferred;
  }
  if (!activeId && !youtubePlayer) {
    return removedDeferred;
  }
  youtubeGeneration += 1;
  youtubePendingPlayback = undefined;
  youtubePlaybackId = undefined;
  youtubeActiveGeneration = 0;
  unloadYoutubePlayer();
  syncMediaStageBusy();
  return true;
}

function mediaStageOccupied() {
  return Boolean(
    currentMedia
    || youtubePlaybackId
    || youtubePendingPlayback
    || mediaStageClaimPending
  );
}

function syncMediaStageBusy() {
  if (isPreview || socket?.readyState !== 1) return;
  const busy = mediaStageOccupied();
  if (busy === reportedMediaStageBusy) return;
  reportedMediaStageBusy = busy;
  // Clear optimistic local grant when we release; peer occupancy arrives via stageClock.
  if (!busy) mediaBusy = false;
  socket.send(JSON.stringify({
    type: "stageClock",
    payload: { lane: "media", busy },
  }));
}

function mediaStageBlocked() {
  // musicBusy: YouTube jukebox. ttsBusy: spoken notifications.
  // Peer mediaBusy: Discord file audio on /audios, or Windows Now Playing
  // reporting the media lane — ignore our own occupancy echo.
  return musicBusy || ttsBusy || (mediaBusy && !mediaStageOccupied());
}

function youtubeStageBlocked() {
  // Ignore our own media-lane report while we already hold YouTube on this page.
  return ttsBusy || (mediaBusy && !mediaStageOccupied());
}

function resolveMediaStageClaim() {
  if (!mediaStageClaimPending) return;
  if (
    (lastStageClockPayload.granted === false && lastStageClockPayload.lane === "media")
    || ttsBusy
    || musicBusy
  ) {
    mediaStageClaimPending = false;
    reportedMediaStageBusy = false;
    return;
  }
  if (!mediaBusy) return;
  activateQueuedMedia();
  mediaStageClaimPending = false;
  syncMediaStageBusy();
}

function playDeferredYoutubeOrNextMedia() {
  if (
    !stageClockReady
    || currentMedia
    || youtubePlaybackId
    || ttsBusy
    || mediaStageClaimPending
  ) return;
  if (deferredYoutubeQueue.length > 0) {
    if (youtubeStageBlocked()) return;
    const next = deferredYoutubeQueue.shift();
    beginYoutubeMusic(next);
    return;
  }
  if (mediaStageBlocked()) return;
  showNextMedia();
}

function activateQueuedMedia() {
  if (
    !stageClockReady
    || (isWidgetWindow && !widgetPlaybackVisible)
    || currentMedia
    || youtubePlaybackId
    || musicBusy
    || ttsBusy
    || !mediaClockReady
  ) {
    syncMediaStageBusy();
    return;
  }
  if (deferredYoutubeQueue.length > 0) {
    playDeferredYoutubeOrNextMedia();
    return;
  }
  if (queue.length === 0) {
    syncMediaStageBusy();
    return;
  }
  currentMedia = queue.shift();
  const generation = ++playbackGeneration;
  setAuthor(currentMedia);
  setMediaText(currentMedia);
  syncMediaStageBusy();
  if (isCoordinatedMedia(currentMedia) && !outputLeaseHeld) {
    waitingForOutputLease = true;
    requestOutputLease();
    return;
  }
  startCurrentMedia(generation);
}

function beginYoutubeMusic(payload = {}) {
  const videoId = typeof payload.videoId === "string" ? payload.videoId : "";
  const playbackId = typeof payload.playbackId === "string" ? payload.playbackId : "";
  if (!/^[A-Za-z0-9_-]{11}$/.test(videoId) || !playbackId) return;

  // Only interrupt other YouTube / stray media when actually starting playback.
  // Media must not be interrupted when we are deferring (handled in startYoutubeMusic).
  if (youtubePlaybackId || youtubePendingPlayback) {
    stopYoutubePlayback();
  }

  const generation = ++youtubeGeneration;
  youtubePlaybackId = playbackId;
  youtubePendingPlayback = { ...payload, videoId, playbackId, generation };
  syncMediaStageBusy();
  loadYoutubeApi()
    .then(() => {
      if (generation !== youtubeGeneration || !youtubePendingPlayback) return;
      createYoutubePlayer();
      loadYoutubePendingPlayback();
    })
    .catch(() => {
      youtubeApiPromise = undefined;
      finishYoutubePlayback(playbackId, generation, false);
      playDeferredYoutubeOrNextMedia();
    });
}

function startYoutubeMusic(payload = {}) {
  // Discord file audio stays on /audios. YouTube uses /youtube in OBS and the
  // combined media widget on Windows.
  if (isPreview || !["youtube", "all"].includes(relayMode)) return;
  const videoId = typeof payload.videoId === "string" ? payload.videoId : "";
  const playbackId = typeof payload.playbackId === "string" ? payload.playbackId : "";
  if (!/^[A-Za-z0-9_-]{11}$/.test(videoId) || !playbackId) return;

  // While media or TTS holds the stage, queue YouTube instead of interrupting it.
  // `mediaBusy` covers peer OBS iframes (/medias) that share the stage clock.
  if (!stageClockReady || currentMedia || youtubeStageBlocked()) {
    if (deferredYoutubeQueue.length >= DEFERRED_YOUTUBE_LIMIT) return;
    deferredYoutubeQueue.push({ ...payload, videoId, playbackId });
    return;
  }

  beginYoutubeMusic(payload);
}

function sendOutputLeaseState(busy) {
  if (
    !coordinatesSplitOutputs
    || socket?.readyState !== 1
  ) return;
  socket.send(JSON.stringify({ type: "mediaClock", payload: { busy } }));
}

function isCoordinatedMedia(media) {
  return (relayMode === "visual" && VISUAL_MEDIA_KINDS.has(media?.kind))
    || (relayMode === "audio" && media?.kind === "audio");
}

function opposingOutputBusy() {
  return relayMode === "visual" ? audioOutputBusy : videoOutputBusy;
}

function requestOutputLease() {
  if (
    !waitingForOutputLease
    || outputLeaseHeld
    || outputLeasePending
    || !mediaClockReady
    || opposingOutputBusy()
    || socket?.readyState !== 1
  ) return;
  outputLeasePending = true;
  sendOutputLeaseState(true);
}

function releaseOutputLease() {
  const shouldNotifyServer = outputLeaseHeld || outputLeasePending;
  outputLeaseHeld = false;
  outputLeasePending = false;
  if (shouldNotifyServer) sendOutputLeaseState(false);
}

function mediaSources(media) {
  if (media.cachedMediaId) {
    return [`/media-cache/${encodeURIComponent(media.cachedMediaId)}?secret=${encodeURIComponent(relaySecret)}`];
  }
  if (media.kind === "audio" && media.audioId) {
    return [`/media-audio/${encodeURIComponent(media.audioId)}?secret=${encodeURIComponent(relaySecret)}`];
  }
  return [media.url, media.proxyUrl].filter(
    (source, index, sources) => source && sources.indexOf(source) === index,
  );
}

function setAudioArtwork(media) {
  audioArtworkElement.onerror = () => {
    audioArtworkElement.onerror = null;
    audioArtworkElement.src = FALLBACK_AVATAR;
  };
  audioArtworkElement.src = media.artworkId
    ? `/media-artwork/${encodeURIComponent(media.artworkId)}?secret=${encodeURIComponent(relaySecret)}`
    : FALLBACK_AVATAR;
}

function formatAudioClock(seconds) {
  const total = Math.max(0, Math.floor(Number(seconds) || 0));
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
}

function clearAudioTransport() {
  if (audioTimeElement) {
    audioTimeElement.textContent = "";
    audioTimeElement.hidden = true;
  }
  if (audioProgressFillElement) audioProgressFillElement.style.width = "0%";
  if (audioProgressElement) {
    audioProgressElement.hidden = true;
    audioProgressElement.setAttribute("aria-valuenow", "0");
  }
}

function updateAudioTransport(playbackElement = audioElement) {
  if (!audioTimeElement || !audioProgressElement || !audioProgressFillElement) return;
  const duration = Number(playbackElement?.duration);
  if (!Number.isFinite(duration) || duration <= 0) {
    clearAudioTransport();
    return;
  }
  const current = Math.min(duration, Math.max(0, Number(playbackElement.currentTime) || 0));
  const percent = Math.min(100, Math.max(0, (current / duration) * 100));
  audioTimeElement.textContent = `${formatAudioClock(current)} → ${formatAudioClock(duration)}`;
  audioTimeElement.hidden = false;
  audioProgressFillElement.style.width = `${percent}%`;
  audioProgressElement.hidden = false;
  audioProgressElement.setAttribute("aria-valuenow", String(Math.round(percent)));
}

function setAuthor(media) {
  const audio = media?.kind === "audio";
  if (audioAuthorElement) {
    audioAuthorElement.hidden = !audio || !config.showAuthor || !media?.author;
    if (!audioAuthorElement.hidden) {
      audioAuthorNameElement.textContent = media.author.username;
      audioAuthorAvatarElement.onerror = () => {
        audioAuthorAvatarElement.onerror = null;
        audioAuthorAvatarElement.src = FALLBACK_AVATAR;
      };
      audioAuthorAvatarElement.src = media.author.displayAvatarUrl || FALLBACK_AVATAR;
    }
  }
  if (!audio && config.showAuthor && media?.author) {
    authorAvatarElement.onerror = () => {
      authorAvatarElement.onerror = null;
      authorAvatarElement.src = FALLBACK_AVATAR;
    };
    authorAvatarElement.src = media.author.displayAvatarUrl || FALLBACK_AVATAR;
    authorNameElement.textContent = media.author.username;
    authorElement.hidden = false;
  } else {
    authorElement.hidden = true;
  }
}

function setMediaText(media) {
  const enabled = isWidgetWindow ? config.showMediaTextWidget : config.showMediaTextObs;
  const text = enabled && typeof media?.text === "string" ? media.text.trim() : "";
  const isAudio = media?.kind === "audio";
  mediaTextElement.textContent = isAudio ? "" : text;
  mediaTextElement.hidden = isAudio || !text;
  mediaTextElement.classList.toggle(
    "is-visible",
    !isAudio && Boolean(text) && (isPreview || activeVisual?.classList.contains("is-visible")),
  );
  audioMediaTextElement.textContent = isAudio ? text : "";
  audioMediaTextElement.hidden = !isAudio || !text;
  audioMediaTextElement.classList.toggle(
    "is-visible",
    isAudio && Boolean(text) && (isPreview || activeVisual?.classList.contains("is-visible")),
  );
}

function clearMediaTextPlacement() {
  mediaTextElement.style.left = "";
  mediaTextElement.style.bottom = "";
  mediaTextElement.style.maxWidth = "";
}

function positionAudioCard() {
  if (!audioCardElement.style) return;
  const geometry = isWidgetWindow ? config.mediaWidgetGeometry : config.mediaObsGeometry;
  const bounds = audioCardElement.getBoundingClientRect();
  const placement = RelayLayout.position(bounds.width, bounds.height, window.innerWidth, window.innerHeight, geometry);
  audioCardElement.style.left = placement ? placement.left + "px" : "";
  audioCardElement.style.right = placement ? "auto" : "";
  audioCardElement.style.top = placement ? placement.top + "px" : "";
  audioCardElement.style.transformOrigin = placement ? "left top" : "";
}

function positionMediaText() {
  positionAudioCard();
  if (activeVisual === imageElement) {
    fitVisualToViewport(imageElement, imageElement.naturalWidth, imageElement.naturalHeight);
  } else if (activeVisual === videoElement) {
    fitVisualToViewport(videoElement, videoElement.videoWidth, videoElement.videoHeight, VIDEO_COMPOSITOR_INSET_PX);
  }
}

function showPreview() {
  const sample = widgetParameters.get("sample");
  if (sample === "audio") {
    audioCardElement.hidden = false;
    audioTitleElement.textContent = "Relay audio test";
    audioArtistElement.textContent = "Relay"; audioArtistElement.hidden = false;
    audioArtworkElement.src = "/output-samples/landscape";
    audioCardElement.classList.add("is-visible");
    activeVisual = audioCardElement;
    setAuthor({ kind: "audio", author: { username: "Relay", displayAvatarUrl: FALLBACK_AVATAR } });
    positionMediaText();
    return;
  }
  if (sample === "video") {
    activeVisual = videoElement;
    videoElement.muted = true; videoElement.loop = true;
    videoElement.onloadeddata = () => { positionMediaText(); videoElement.classList.add("is-visible"); };
    if (!videoElement.src.endsWith("/output-samples/video")) {
      videoElement.src = "/output-samples/video";
      videoElement.play().catch(() => {});
    }
    setAuthor({ author: { username: "Relay", displayAvatarUrl: FALLBACK_AVATAR } });
    authorElement.classList.add("is-visible");
    positionMediaText();
    return;
  }
  activeVisual = imageElement;
  imageElement.onload = positionMediaText;
  imageElement.src = ["portrait", "landscape", "gif"].includes(sample) ? "/output-samples/" + sample : FALLBACK_AVATAR;
  imageElement.alt = "Relay preview";
  imageElement.style.width = "auto";
  imageElement.style.height = "100%";
  imageElement.classList.add("is-visible");
  setAuthor({
    author: {
      username: previewLabels[interfaceLanguage] || previewLabels.en,
      displayAvatarUrl: FALLBACK_AVATAR,
    },
  });
  setMediaText({
    text: previewCaptionLabels[interfaceLanguage] || previewCaptionLabels.en,
  });
  if (!authorElement.hidden) authorElement.classList.add("is-visible");
  positionMediaText();
}

function mediaLayout(width, height, viewportWidth, viewportHeight, scale, footerHeight, insetPx = 0, geometry = {}) {
  const margin = Math.max(12, insetPx);
  const custom = geometry.anchor && geometry.anchor !== "legacy";
  const marginX = custom ? Math.max(insetPx, Number(geometry.marginX) || 0) : margin;
  const marginY = custom ? Math.max(insetPx, Number(geometry.marginY) || 0) : margin;
  const availableWidth = Math.max(1, viewportWidth - marginX * 2);
  const availableHeight = Math.max(1, viewportHeight - marginY * 2 - footerHeight);
  const ratio = Math.min(availableWidth / width, availableHeight / height);
  const displayedWidth = width * ratio;
  const displayedHeight = height * ratio;
  return {
    width: displayedWidth / scale, height: displayedHeight / scale,
    centerY: marginY + availableHeight / 2,
    left: viewportWidth - displayedWidth - marginX,
    bottom: marginY + (availableHeight + displayedHeight) / 2,
    displayedWidth,
  };
}

function fitVisualToViewport(element, width, height, insetPx = 0) {
  if (!width || !height || !window.innerWidth || !window.innerHeight) return;
  const scale = Math.max(0.5, Number.parseFloat(
    getComputedStyle(document.documentElement).getPropertyValue("--content-scale"),
  ) || 1);
  const authorHeight = authorElement.hidden ? 0 : 44 * scale;
  const captionHeight = mediaTextElement.hidden ? 0 : 80 * scale;
  const geometry = isWidgetWindow ? config.mediaWidgetGeometry : config.mediaObsGeometry;
  const layout = mediaLayout(width, height, window.innerWidth, window.innerHeight,
    scale, authorHeight + captionHeight, insetPx, geometry);
  const placement = RelayLayout.position(layout.displayedWidth, layout.height * scale + authorHeight + captionHeight,
    window.innerWidth, window.innerHeight, geometry);
  if (placement) {
    layout.left = placement.left;
    layout.centerY = placement.top + layout.height * scale / 2;
    layout.bottom = placement.top + layout.height * scale;
  }
  element.style.width = layout.width + "px";
  element.style.height = layout.height + "px";
  element.style.top = layout.centerY + "px";
  element.style.left = (layout.left + layout.displayedWidth / 2) + "px";
  for (const [footer, offset] of [[authorElement, 0], [mediaTextElement, authorHeight]]) {
    footer.style.left = layout.left + "px";
    footer.style.top = (layout.bottom + 8 + offset) + "px";
    footer.style.bottom = "auto";
    footer.style.maxWidth = (layout.displayedWidth / scale) + "px";
  }
}

function applyOutputGeometry() {
  const geometry = isWidgetWindow ? config.mediaWidgetGeometry : config.mediaObsGeometry;
  const crop = (value) => Math.min(40, Math.max(0, Number(value) || 0));
  // Widget: never zoom past 100% — that was cropping images at the edges.
  let scale = Math.min(200, Math.max(50, Number(geometry?.contentScale) || 100));
  if (isWidgetWindow) {
    scale = Math.min(scale, 100);
  }
  const rootStyle = document.documentElement.style;
  // Windows media widget always shows the full frame (no user crop).
  if (isWidgetWindow) {
    rootStyle.setProperty("--crop-top", "0%");
    rootStyle.setProperty("--crop-right", "0%");
    rootStyle.setProperty("--crop-bottom", "0%");
    rootStyle.setProperty("--crop-left", "0%");
  } else {
    rootStyle.setProperty("--crop-top", `${crop(geometry?.cropTop)}%`);
    rootStyle.setProperty("--crop-right", `${crop(geometry?.cropRight)}%`);
    rootStyle.setProperty("--crop-bottom", `${crop(geometry?.cropBottom)}%`);
    rootStyle.setProperty("--crop-left", `${crop(geometry?.cropLeft)}%`);
  }
  rootStyle.setProperty("--content-scale", String(scale / 100));
}

function resetElements() {
  clearLoadListeners();
  clearLoadListeners = () => {};
  window.clearTimeout(displayTimer);
  window.clearTimeout(fadeTimer);
  window.clearTimeout(mediaWatchdog);

  for (const element of [imageElement, videoElement]) {
    element.classList.remove("is-visible", "is-hiding");
    element.removeAttribute("src");
    element.style.width = "";
    element.style.height = "";
  }
  for (const element of [videoElement, audioElement]) {
    element.pause();
    element.removeAttribute("src");
    element.load();
  }
  videoElement.loop = false;
  audioCardElement.classList.remove("is-visible", "is-hiding");
  audioCardElement.hidden = true;
  audioTitleElement.textContent = "";
  audioArtistElement.textContent = "";
  audioArtistElement.hidden = true;
  audioMediaTextElement.classList.remove("is-visible");
  audioMediaTextElement.hidden = true;
  audioMediaTextElement.textContent = "";
  clearAudioTransport();
  setAudioArtwork({});
  authorElement.classList.remove("is-visible");
  authorElement.hidden = true;
  authorAvatarElement.removeAttribute("src");
  mediaTextElement.classList.remove("is-visible");
  mediaTextElement.hidden = true;
  mediaTextElement.textContent = "";
  clearMediaTextPlacement();
  activeVisual = undefined;
  activePlayback = undefined;
}

function revealMedia({ timed = false } = {}) {
  if (!currentMedia || !activeVisual) {
    return;
  }
  activeVisual.classList.add("is-visible");
  positionMediaText();
  if (!authorElement.hidden) {
    authorElement.classList.add("is-visible");
  }
  for (const caption of [mediaTextElement, audioMediaTextElement]) {
    if (!caption.hidden) caption.classList.add("is-visible");
  }
  if (timed) {
    const durationMs = currentMedia.kind === "gif"
      ? config.gifDurationMs
      : config.displayDurationMs;
    displayTimer = window.setTimeout(hideCurrentMedia, durationMs);
  }
}

function finishCurrentMedia(expectedGeneration = playbackGeneration) {
  if (!currentMedia || expectedGeneration !== playbackGeneration) return;
  const finishedMedia = currentMedia;
  resetElements();
  currentMedia = undefined;
  waitingForOutputLease = false;
  syncMediaStageBusy();
  if (!stageClockReady) return;
  if (deferredYoutubeQueue.length > 0) {
    if (isCoordinatedMedia(finishedMedia)) releaseOutputLease();
    if (finishedMedia.kind === "audio") reportAudioPlayback("idle", finishedMedia);
    if (youtubeStageBlocked()) return;
    const next = deferredYoutubeQueue.shift();
    beginYoutubeMusic(next);
    return;
  }
  showNextMedia();
  if (isCoordinatedMedia(finishedMedia) && !isCoordinatedMedia(currentMedia)) {
    releaseOutputLease();
  }
  if (finishedMedia.kind === "audio" && currentMedia?.kind !== "audio") {
    reportAudioPlayback("idle", finishedMedia);
  }
}

function hideCurrentMedia(expectedGeneration = playbackGeneration) {
  if (!currentMedia || expectedGeneration !== playbackGeneration) {
    return;
  }
  window.clearTimeout(displayTimer);
  activePlayback?.pause();
  activeVisual?.classList.remove("is-visible");
  activeVisual?.classList.add("is-hiding");
  authorElement.classList.remove("is-visible");
  mediaTextElement.classList.remove("is-visible");
  fadeTimer = window.setTimeout(() => finishCurrentMedia(expectedGeneration), FADE_DURATION_MS);
}

function skipCurrentMedia() {
  if (stopYoutubePlayback()) {
    // Server skip/promote may follow with musicPlay; musicIdle resumes media if not.
    return;
  }
  // IDs may already be cleared while the iframe still shows paused chrome.
  if (youtubePlayer) unloadYoutubePlayer();
  if (currentMedia) {
    finishCurrentMedia();
  } else {
    playDeferredYoutubeOrNextMedia();
  }
}

function loadImage(media, generation) {
  activeVisual = imageElement;
  const sources = mediaSources(media);
  let sourceIndex = 0;
  let revealed = false;

  const clear = () => {
    imageElement.removeEventListener("load", onLoad);
    imageElement.removeEventListener("error", onError);
  };
  const onLoad = () => {
    if (revealed || generation !== playbackGeneration) {
      return;
    }
    revealed = true;
    window.clearTimeout(mediaWatchdog);
    fitVisualToViewport(imageElement, imageElement.naturalWidth, imageElement.naturalHeight);
    clear();
    revealMedia({ timed: true });
  };
  const onError = () => {
    if (generation !== playbackGeneration) return;
    clear();
    sourceIndex += 1;
    if (sourceIndex < sources.length) {
      trySource();
    } else {
      hideCurrentMedia(generation);
    }
  };
  const trySource = () => {
    imageElement.addEventListener("load", onLoad, { once: true });
    imageElement.addEventListener("error", onError, { once: true });
    imageElement.src = sources[sourceIndex];
  };

  clearLoadListeners = clear;
  mediaWatchdog = window.setTimeout(() => hideCurrentMedia(generation), 15000);
  if (sources.length === 0) {
    hideCurrentMedia();
    return;
  }
  trySource();
  if (imageElement.complete && imageElement.naturalWidth > 0) {
    onLoad();
  }
}

function loadPlayback(media, playbackElement, visualElement, generation) {
  activePlayback = playbackElement;
  activeVisual = visualElement;
  const isTimedGif = media.kind === "gif";
  const mustMute = (isWidgetWindow && !config.widgetSoundEnabled) || isTimedGif;
  playbackElement.loop = isTimedGif;
  playbackElement.muted = mustMute;
  playbackElement.volume = mustMute
    ? 0
    : Math.min(1, Math.max(0, config.mediaVolume / 100)) * (media.kind === "audio" ? reactionMusicFactor : 1);
  const sources = mediaSources(media);
  let sourceIndex = 0;

  const clearSourceListeners = () => {
    playbackElement.removeEventListener("loadeddata", onLoaded);
    playbackElement.removeEventListener("error", onError);
  };
  const clear = () => {
    clearSourceListeners();
    playbackElement.removeEventListener("ended", onEnded);
    for (const eventName of ["playing", "timeupdate", "waiting", "stalled", "abort", "emptied"]) {
      playbackElement.removeEventListener(eventName, playbackStateChanged);
    }
  };
  const armWatchdog = (delay = 20000) => {
    window.clearTimeout(mediaWatchdog);
    mediaWatchdog = window.setTimeout(() => hideCurrentMedia(generation), delay);
  };
  const playbackStateChanged = (event) => {
    if (generation !== playbackGeneration) return;
    if (event.type === "playing" || event.type === "timeupdate") {
      armWatchdog(20000);
      reportAudioPlayback("playing");
      if (visualElement === audioCardElement) updateAudioTransport(playbackElement);
    }
    else armWatchdog(15000);
  };
  const onEnded = () => {
    if (!isTimedGif) hideCurrentMedia(generation);
  };
  const onLoaded = () => {
    if (generation !== playbackGeneration) return;
    if (visualElement === videoElement) {
      fitVisualToViewport(
        videoElement,
        videoElement.videoWidth,
        videoElement.videoHeight,
        VIDEO_COMPOSITOR_INSET_PX,
      );
    }
    clearSourceListeners();
    revealMedia({ timed: isTimedGif });
    armWatchdog();
    playbackElement.play().catch((error) => {
      if (
        error?.name !== "NotAllowedError"
        || generation !== playbackGeneration
        || !isWidgetWindow
        || visualElement !== videoElement
        || playbackElement.muted
      ) {
        hideCurrentMedia(generation);
        return;
      }
      playbackElement.muted = true;
      playbackElement.volume = 0;
      playbackElement.play().catch(() => hideCurrentMedia(generation));
    });
  };
  const onError = () => {
    clearSourceListeners();
    sourceIndex += 1;
    if (sourceIndex < sources.length) {
      trySource();
    } else {
      hideCurrentMedia(generation);
    }
  };
  const trySource = () => {
    playbackElement.addEventListener("loadeddata", onLoaded, { once: true });
    playbackElement.addEventListener("error", onError, { once: true });
    playbackElement.src = sources[sourceIndex];
    playbackElement.load();
  };

  playbackElement.addEventListener("ended", onEnded, { once: true });
  for (const eventName of ["playing", "timeupdate", "waiting", "stalled", "abort", "emptied"]) {
    playbackElement.addEventListener(eventName, playbackStateChanged);
  }
  clearLoadListeners = clear;
  armWatchdog(15000);
  if (sources.length === 0) {
    hideCurrentMedia();
    return;
  }
  trySource();
}

function startCurrentMedia(generation = playbackGeneration) {
  if (!currentMedia || generation !== playbackGeneration) return;
  if (
    currentMedia.kind === "video"
    || (currentMedia.kind === "gif" && currentMedia.contentType?.startsWith("video/"))
  ) {
    loadPlayback(currentMedia, videoElement, videoElement, generation);
  } else if (currentMedia.kind === "audio") {
    audioTitleElement.textContent = currentMedia.title || currentMedia.filename || "Discord audio";
    audioArtistElement.textContent = currentMedia.artist || "";
    audioArtistElement.hidden = !currentMedia.artist;
    setAudioArtwork(currentMedia);
    clearAudioTransport();
    audioCardElement.hidden = false;
    loadPlayback(currentMedia, audioElement, audioCardElement, generation);
  } else {
    loadImage(currentMedia, generation);
  }
}

function showNextMedia() {
  if (
    !stageClockReady
    || (isWidgetWindow && !widgetPlaybackVisible)
    || currentMedia
    || youtubePlaybackId
    || mediaStageBlocked()
    || !mediaClockReady
    || mediaStageClaimPending
  ) return;
  if (deferredYoutubeQueue.length > 0) {
    playDeferredYoutubeOrNextMedia();
    return;
  }
  if (queue.length === 0) return;
  mediaStageClaimPending = true;
  syncMediaStageBusy();
}

function clearOverlay() {
  queue.length = 0;
  deferredYoutubeQueue.length = 0;
  stopYoutubePlayback();
  finishCurrentMedia();
  waitingForOutputLease = false;
  releaseOutputLease();
  syncMediaStageBusy();
}

const MEDIA_QUEUE_LIMIT = 50;

function enqueueMedia(mediaEvent) {
  if (
    (relayMode === "visual" && mediaEvent.kind === "audio")
    || (relayMode === "audio" && mediaEvent.kind !== "audio")
    || queue.length >= MEDIA_QUEUE_LIMIT
  ) {
    return;
  }
  queue.push(mediaEvent);
  showNextMedia();
}

function interruptPlayback() {
  stopYoutubePlayback();
  if (!currentMedia) {
    waitingForOutputLease = false;
    releaseOutputLease();
    syncMediaStageBusy();
    return;
  }
  const interruptedMedia = currentMedia;
  reportAudioPlayback("idle", interruptedMedia);
  waitingForOutputLease = false;
  if (isCoordinatedMedia(interruptedMedia)) releaseOutputLease();
  playbackGeneration += 1;
  resetElements();
  currentMedia = undefined;
  syncMediaStageBusy();
}

function applyStageClock(payload = {}) {
  stageClockReady = true;
  lastStageClockPayload = payload && typeof payload === "object" ? payload : {};
  mediaBusy = Boolean(payload.mediaBusy);
  const isReconnectHydration = reconnectHydrationPending;
  reconnectHydrationPending = false;
  // Prefer server musicBusy when present so a missed musicIdle cannot strand media.
  if (Object.prototype.hasOwnProperty.call(payload, "musicBusy")) {
    musicBusy = Boolean(payload.musicBusy);
  }
  ttsBusy = Boolean(payload.ttsBusy);
  if (isReconnectHydration && payload.musicBusy === false) {
    deferredYoutubeQueue.length = 0;
  }
  resolveMediaStageClaim();
  if (isWidgetWindow && widgetPlaybackVisible && currentMedia && !activeVisual) {
    setAuthor(currentMedia);
    setMediaText(currentMedia);
    startCurrentMedia();
    return;
  }
  playDeferredYoutubeOrNextMedia();
}

function controlAudio(control = {}) {
  if (control.action === "previous" && control.media?.kind === "audio") {
    queue.unshift(control.media);
    if (currentMedia?.kind === "audio") finishCurrentMedia();
    else showNextMedia();
    return;
  }
  if (currentMedia?.kind !== "audio") return;
  if (control.action === "pause") {
    window.clearTimeout(mediaWatchdog);
    activePlayback?.pause();
    reportAudioPlayback("paused");
  } else if (control.action === "resume") {
    activePlayback?.play().then(() => reportAudioPlayback("playing")).catch(() => hideCurrentMedia());
  } else if (control.action === "skip") {
    finishCurrentMedia();
  }
}

function applyMediaClock(payload = {}) {
  mediaClockReady = true;
  videoOutputBusy = Boolean(payload.videoBusy);
  audioOutputBusy = Boolean(payload.audioBusy);
}

let reactionMusicFactor = 1;
let reactionRestoreTimer;
function applyReactionDucking(playback) {
  window.clearTimeout(reactionRestoreTimer);
  reactionMusicFactor = playback && playback.endsAt > Date.now() ? Math.min(1, Math.max(0, playback.musicPercent / 100)) : 1;
  audioElement.volume = audioElement.muted ? 0 : Math.min(1, Math.max(0, config.mediaVolume / 100)) * reactionMusicFactor;
  applyYoutubeAudioSettings();
  if (playback && playback.endsAt > Date.now()) reactionRestoreTimer = window.setTimeout(() => applyReactionDucking(null), playback.endsAt - Date.now());
}

function handleMessage(event) {
  let message;
  try {
    message = JSON.parse(event.data);
  } catch {
    return;
  }

  if (message.type === "reaction") {
    applyReactionDucking(message.payload);
  } else if (message.type === "config") {
    config = { ...config, ...message.payload };
    applyOutputGeometry();
    positionMediaText();
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
    const volume = Math.min(1, Math.max(0, config.mediaVolume / 100));
    const widgetMuted = isWidgetWindow && !config.widgetSoundEnabled;
    videoElement.muted = widgetMuted || currentMedia?.kind === "gif";
    audioElement.muted = widgetMuted;
    videoElement.volume = videoElement.muted ? 0 : volume;
    audioElement.volume = audioElement.muted ? 0 : volume * reactionMusicFactor;
    applyYoutubeAudioSettings();
    if (!config.showAuthor) {
      authorElement.classList.remove("is-visible");
      authorElement.hidden = true;
    }
    if (isPreview) showPreview();
    else {
      setAuthor(currentMedia);
      setMediaText(currentMedia);
      positionMediaText();
    }
  } else if (message.type === "media") {
    if (isPreview) return;
    if (message.payload) enqueueMedia(message.payload);
  } else if (message.type === "testOutput") {
    if (isPreview) return;
    const outputTest = message.payload;
    if (
      (outputTest?.target === "visual" || outputTest?.target === "audio")
      && outputTest.media
    ) {
      enqueueMedia(outputTest.media);
    }
  } else if (message.type === "image") {
    if (isPreview) return;
    if (message.payload) enqueueMedia({ kind: "image", ...message.payload });
  } else if (message.type === "skip") {
    if (isPreview) return;
    skipCurrentMedia();
  } else if (message.type === "audioControl") {
    if (isPreview) return;
    controlAudio(message.payload);
  } else if (message.type === "musicPlay") {
    if (isPreview) return;
    // Hint before stageClock arrives so /medias can queue immediately on OBS splits.
    musicBusy = true;
    startYoutubeMusic(message.payload);
  } else if (message.type === "musicStop") {
    if (isPreview) return;
    // Only stop local YouTube. Do not advance media here — musicPlay (next track)
    // or musicIdle (resume media) follows from the server. Leave musicBusy set;
    // MusicStop is often followed by the next MusicPlay while the jukebox stays busy.
    stopYoutubePlayback(message.payload?.playbackId);
  } else if (message.type === "musicIdle") {
    if (isPreview) return;
    musicBusy = false;
    deferredYoutubeQueue.length = 0;
    stopYoutubePlayback();
    playDeferredYoutubeOrNextMedia();
  } else if (message.type === "clear") {
    if (isPreview) return;
    clearOverlay();
  } else if (message.type === "stageClock") {
    if (isPreview) return;
    applyStageClock(message.payload);
  } else if (message.type === "mediaClock") {
    applyMediaClock(message.payload);
    showNextMedia();
    requestOutputLease();
  } else if (message.type === "mediaGrant") {
    outputLeasePending = false;
    if (message.payload?.clock) applyMediaClock(message.payload.clock);
    if (message.payload?.granted && waitingForOutputLease && isCoordinatedMedia(currentMedia)) {
      outputLeaseHeld = true;
      waitingForOutputLease = false;
      startCurrentMedia();
    } else if (message.payload?.granted) {
      outputLeaseHeld = true;
      releaseOutputLease();
    } else {
      requestOutputLease();
    }
  } else if (message.type === "serverMove") {
    const movedPort = Number(message.payload?.port);
    if (Number.isInteger(movedPort) && movedPort > 0 && movedPort <= 65535) {
      pendingPort = movedPort;
    }
  } else if (message.type === "appearance") {
    applyAppearance(message.payload);
  }
}

function applyAppearance(preferences = {}) {
  interfaceLanguage = Object.hasOwn(moveLabels, preferences.language)
    ? preferences.language
    : interfaceLanguage;
  document.documentElement.lang = interfaceLanguage;
  document.documentElement.dataset.theme = preferences.theme || "dark";
  const rgb = Array.isArray(preferences.accentRgb) ? preferences.accentRgb : [88, 185, 137];
  document.documentElement.style.setProperty("--accent", `rgb(${rgb.join(" ")})`);
  document.documentElement.style.setProperty("--font-scale", String((preferences.fontScale || 100) / 100));
  if (moveLabelElement) moveLabelElement.textContent = moveLabels[interfaceLanguage] || moveLabels.en;
  if (isPreview) showPreview();
}

applyAppearance({ language: interfaceLanguage, fontScale: 100, accentRgb: [88, 185, 137] });
applyOutputGeometry();

function scheduleReconnect() {
  if (isUnloading || reconnectTimer || (isWidgetWindow && !widgetPlaybackVisible)) {
    return;
  }
  reconnectTimer = window.setTimeout(() => {
    reconnectTimer = undefined;
    connect();
  }, reconnectDelayMs);
  reconnectDelayMs = Math.min(reconnectDelayMs * 2, 10000);
}

function moveToPendingPort() {
  const nextUrl = new URL(window.location.href);
  nextUrl.port = String(pendingPort);
  // Probe the moved server before navigating: OBS browser sources never
  // retry a failed page load, so a blind navigation can leave them dead.
  const probe = new WebSocket(
    outputSocketUrl(`${window.location.hostname}:${pendingPort}`, "probe", "ws:"),
  );
  let ready = false;
  const probeWatchdog = window.setTimeout(() => {
    if (!ready) probe.close();
  }, 5000);
  probe.addEventListener("open", () => {
    ready = true;
    window.clearTimeout(probeWatchdog);
    probe.close();
    window.location.replace(nextUrl);
  });
  probe.addEventListener("close", () => {
    window.clearTimeout(probeWatchdog);
    if (!ready && !isUnloading) {
      window.setTimeout(moveToPendingPort, 1000);
    }
  });
}

function connect() {
  const nextSocket = new WebSocket(outputSocketUrl(window.location.host));
  socket = nextSocket;
  nextSocket.addEventListener("open", () => {
    reconnectDelayMs = 1000;
    showNextMedia();
  });
  nextSocket.addEventListener("message", handleMessage);
  nextSocket.addEventListener("close", () => {
    if (socket !== nextSocket) return;
    applyReactionDucking(null);
    if (isWidgetWindow && !widgetPlaybackVisible) return;
    // Stop the current playback but keep the queue: the server does not
    // rebroadcast queued media after a transient reconnect.
    interruptPlayback();
    outputLeaseHeld = false;
    outputLeasePending = false;
    waitingForOutputLease = false;
    reportedMediaStageBusy = false;
    mediaStageClaimPending = false;
    lastStageClockPayload = {};
    reconnectHydrationPending = true;
    stageClockReady = isPreview;
    ttsBusy = false;
    mediaBusy = false;
    musicBusy = false;
    mediaClockReady = !coordinatesSplitOutputs;
    if (pendingPort) {
      moveToPendingPort();
      return;
    }
    scheduleReconnect();
  });
  nextSocket.addEventListener("error", () => nextSocket.close());
}

window.setWidgetVisible = (visible) => {
  if (!isWidgetWindow || widgetPlaybackVisible === visible) return;
  widgetPlaybackVisible = visible;
  if (!visible) {
    reportAudioPlayback("idle");
    deferredYoutubeQueue.length = 0;
    reconnectHydrationPending = true;
    stopYoutubePlayback();
    playbackGeneration += 1;
    resetElements();
    stageClockReady = isPreview;
    window.clearTimeout(reconnectTimer);
    reconnectTimer = undefined;
    socket?.close();
    return;
  }
  if (!socket || socket.readyState >= 2) connect();
  if (stageClockReady && currentMedia) {
    setAuthor(currentMedia);
    setMediaText(currentMedia);
    startCurrentMedia();
  } else {
    showNextMedia();
  }
};

window.addEventListener("resize", positionMediaText);

window.addEventListener("beforeunload", () => {
  isUnloading = true;
  stopYoutubePlayback();
  clearOverlay();
  window.clearTimeout(reconnectTimer);
  socket?.close();
});

connect();
