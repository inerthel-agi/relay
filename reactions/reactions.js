"use strict";
const secret = document.querySelector('meta[name="relay-secret"]')?.content || "";
const widget = new URLSearchParams(location.search).get("client") === "widget";
const audioOnly = widget && new URLSearchParams(location.search).get("audioOnly") === "1";
const card = document.querySelector("#reaction");
const visual = document.querySelector("#visual");
const sound = document.querySelector("#sound");
if (widget && !audioOnly) {
  card.style.cursor = "move";
  card.addEventListener("pointerdown", event => {
    if (event.button !== 0) return;
    const tauri = window.__TAURI__;
    const current = tauri?.webviewWindow?.getCurrentWebviewWindow?.() || tauri?.window?.getCurrentWindow?.();
    current?.startDragging?.().catch(() => {});
  });
}
let socket, active, deadline, pendingPort, config = {};
const MAX_REACTION_SECONDS = 30;
function asset(path) { const url = new URL(path, location.origin); url.searchParams.set("secret", secret); return url.href; }
function position() {
  const geometry = config.reactionGeometry || {};
  const scale = Math.min(2, Math.max(.5, (geometry.contentScale || 100) / 100));
  card.style.transform = `scale(${scale})`;
  card.style.transformOrigin = "top left";
  const width = card.offsetWidth * scale, height = card.offsetHeight * scale;
  const pos = RelayLayout.position(width, height, innerWidth, innerHeight, geometry) || { left: Math.max(0, innerWidth - width - 12), top: Math.max(0, innerHeight - height - 12) };
  card.style.right = card.style.bottom = "auto";
  card.style.left = `${pos.left}px`; card.style.top = `${pos.top}px`;
}
function finish(report = false) {
  const id = active?.playbackId;
  active = undefined; clearTimeout(deadline); sound.onended = sound.onerror = sound.onloadedmetadata = null; sound.pause(); sound.removeAttribute("src"); sound.load(); card.hidden = true;
  if (report && id && socket?.readyState === WebSocket.OPEN) socket.send(JSON.stringify({ type: "reactionEnded", payload: id }));
}
function play(payload) {
  if (!payload || payload.endsAt <= Date.now()) { finish(); return; }
  if (active?.playbackId === payload.playbackId) return;
  finish(); active = payload;
  const item = payload.reaction;
  visual.hidden = audioOnly || !item.visualId;
  if (item.visualId && !audioOnly) visual.src = asset(`/library-asset/${encodeURIComponent(item.visualId)}`);
  else visual.removeAttribute("src");
  card.hidden = audioOnly || !item.visualId; position();
  sound.volume = Math.min(1, Math.max(0, item.volume / 100)); sound.muted = widget && !audioOnly && !config.widgetSoundEnabled;
  sound.src = asset(`/reaction-sound/${encodeURIComponent(item.soundId)}`);
  const id = payload.playbackId;
  sound.onended = () => { if (active?.playbackId === id) finish(true); };
  sound.onerror = () => { if (active?.playbackId === id) finish(); };
  sound.onloadedmetadata = () => {
    if (active?.playbackId !== id) return;
    const configuredDuration = Number(payload.durationMs) / 1000;
    const soundDuration = Number.isFinite(sound.duration) && sound.duration > 0 ? sound.duration : 0;
    const playbackDuration = Number.isFinite(configuredDuration) && configuredDuration > 0
      ? configuredDuration
      : soundDuration || MAX_REACTION_SECONDS;
    const elapsed = playbackDuration - (payload.endsAt - Date.now()) / 1000;
    sound.currentTime = Math.min(Math.max(0, elapsed), soundDuration || playbackDuration);
  };
  sound.play().catch(() => { if (active?.playbackId === id) finish(); });
  deadline = setTimeout(() => finish(true), Math.max(0, payload.endsAt - Date.now()));
}
visual.addEventListener("load", position); window.addEventListener("resize", position);
function connect() {
  const url = new URL("/ws", location.origin); url.protocol = "ws:";
  url.searchParams.set("role", "reaction"); url.searchParams.set("secret", secret); url.searchParams.set("client", widget ? "widget" : "obs");
  socket = new WebSocket(url);
  socket.addEventListener("message", event => {
    let message; try { message = JSON.parse(event.data); } catch { return; }
    if (message.type === "reaction") play(message.payload);
    if (message.type === "config") { config = message.payload; if (Number.isInteger(config.port) && config.port > 0 && config.port <= 65535 && String(config.port) !== location.port) pendingPort = config.port; sound.muted = widget && !audioOnly && !config.widgetSoundEnabled; position(); }
    if (message.type === "serverMove" && Number.isInteger(message.payload?.port) && message.payload.port > 0 && message.payload.port <= 65535) pendingPort = message.payload.port;
    if (message.type === "clear") finish(true);
  });
  socket.addEventListener("close", () => { finish(); if (pendingPort) { const next = new URL(location.href); next.port = String(pendingPort); location.replace(next.href); } else setTimeout(connect, 1500); });
}
connect();
