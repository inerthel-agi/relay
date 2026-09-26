const invoke = window.__TAURI__.core.invoke;

const trayTranslations = {
  en: { relayStatus: "Relay status", connectionStatus: "Connection status", discord: "Discord", localRelay: "Local relay", system: "System", openPanel: "Open control panel", displayWidgets: "Display widgets", mediaOverlay: "Media overlay", ttsNotifications: "Message notifications", runningLocally: "Running locally", startWithWindows: "Start with Windows", quitRelay: "Quit Relay", checking: "Checking", connecting: "Connecting…", zeroOutputs: "0 outputs", online: "Online", offline: "Offline", waitingDiscord: "Waiting for Discord", output: "output", outputs: "outputs", hidden: "Hidden", visibleLocked: "Visible · Locked", visibleMovable: "Visible · Movable", hide: "Hide", show: "Show", lock: "Lock", unlock: "Unlock", mediaWidget: "media widget", notificationWidget: "notification widget", unavailable: "Unavailable", notResponding: "Relay not responding" },
  fr: { relayStatus: "Statut de Relay", connectionStatus: "État de la connexion", discord: "Discord", localRelay: "Relais local", system: "Système", openPanel: "Ouvrir le panneau de contrôle", displayWidgets: "Widgets d’affichage", mediaOverlay: "Overlay média", ttsNotifications: "Notifications de messages", runningLocally: "Exécution locale", startWithWindows: "Démarrer avec Windows", quitRelay: "Quitter Relay", checking: "Vérification", connecting: "Connexion…", zeroOutputs: "0 sortie", online: "En ligne", offline: "Hors ligne", waitingDiscord: "En attente de Discord", output: "sortie", outputs: "sorties", hidden: "Masqué", visibleLocked: "Visible · Verrouillé", visibleMovable: "Visible · Déplaçable", hide: "Masquer", show: "Afficher", lock: "Verrouiller", unlock: "Déverrouiller", mediaWidget: "le widget média", notificationWidget: "le widget de notifications", unavailable: "Indisponible", notResponding: "Relay ne répond pas" },
  es: { relayStatus: "Estado de Relay", connectionStatus: "Estado de la conexión", discord: "Discord", localRelay: "Relay local", system: "Sistema", openPanel: "Abrir el panel de control", displayWidgets: "Widgets de pantalla", mediaOverlay: "Overlay multimedia", ttsNotifications: "Notificaciones de mensajes", runningLocally: "Ejecución local", startWithWindows: "Iniciar con Windows", quitRelay: "Salir de Relay", checking: "Comprobando", connecting: "Conectando…", zeroOutputs: "0 salidas", online: "En línea", offline: "Sin conexión", waitingDiscord: "Esperando a Discord", output: "salida", outputs: "salidas", hidden: "Oculto", visibleLocked: "Visible · Bloqueado", visibleMovable: "Visible · Desplazable", hide: "Ocultar", show: "Mostrar", lock: "Bloquear", unlock: "Desbloquear", mediaWidget: "widget multimedia", notificationWidget: "widget de notificaciones", unavailable: "No disponible", notResponding: "Relay no responde" },
  de: { relayStatus: "Relay-Status", connectionStatus: "Verbindungsstatus", discord: "Discord", localRelay: "Lokales Relay", system: "System", openPanel: "Bedienfeld öffnen", displayWidgets: "Anzeige-Widgets", mediaOverlay: "Medien-Overlay", ttsNotifications: "Nachrichtenbenachrichtigungen", runningLocally: "Läuft lokal", startWithWindows: "Mit Windows starten", quitRelay: "Relay beenden", checking: "Wird geprüft", connecting: "Verbindung…", zeroOutputs: "0 Ausgaben", online: "Online", offline: "Offline", waitingDiscord: "Warten auf Discord", output: "Ausgabe", outputs: "Ausgaben", hidden: "Ausgeblendet", visibleLocked: "Sichtbar · Gesperrt", visibleMovable: "Sichtbar · Verschiebbar", hide: "Ausblenden", show: "Anzeigen", lock: "Sperren", unlock: "Entsperren", mediaWidget: "Medien-Widget", notificationWidget: "Benachrichtigungs-Widget", unavailable: "Nicht verfügbar", notResponding: "Relay antwortet nicht" },
};

Object.assign(trayTranslations, {
  ru: {
    relayStatus: "Статус Relay", connectionStatus: "Состояние подключения", discord: "Discord", localRelay: "Локальный Relay", system: "Система",
    openPanel: "Открыть панель управления", displayWidgets: "Виджеты", mediaOverlay: "Медиа-оверлей", ttsNotifications: "Уведомления о сообщениях",
    runningLocally: "Работает локально", startWithWindows: "Запускать с Windows", quitRelay: "Выйти из Relay", checking: "Проверка", connecting: "Подключение…",
    zeroOutputs: "0 выходов", online: "В сети", offline: "Не в сети", waitingDiscord: "Ожидание Discord", output: "выход", outputs: "выходов",
    hidden: "Скрыт", visibleLocked: "Виден · заблокирован", visibleMovable: "Виден · можно перемещать", hide: "Скрыть", show: "Показать", lock: "Заблокировать", unlock: "Разблокировать",
    mediaWidget: "медиа-виджет", notificationWidget: "виджет уведомлений", unavailable: "Недоступно", notResponding: "Relay не отвечает",
  },
  zh: {
    relayStatus: "Relay 状态", connectionStatus: "连接状态", discord: "Discord", localRelay: "本地 Relay", system: "系统",
    openPanel: "打开控制面板", displayWidgets: "显示小组件", mediaOverlay: "媒体叠加层", ttsNotifications: "消息通知",
    runningLocally: "正在本地运行", startWithWindows: "随 Windows 启动", quitRelay: "退出 Relay", checking: "正在检查", connecting: "正在连接…",
    zeroOutputs: "0 个输出", online: "在线", offline: "离线", waitingDiscord: "正在等待 Discord", output: "个输出", outputs: "个输出",
    hidden: "已隐藏", visibleLocked: "可见 · 已锁定", visibleMovable: "可见 · 可移动", hide: "隐藏", show: "显示", lock: "锁定", unlock: "解锁",
    mediaWidget: "媒体小组件", notificationWidget: "通知小组件", unavailable: "不可用", notResponding: "Relay 没有响应",
  },
  ko: {
    relayStatus: "Relay 상태", connectionStatus: "연결 상태", discord: "Discord", localRelay: "로컬 Relay", system: "시스템",
    openPanel: "제어 패널 열기", displayWidgets: "화면 위젯", mediaOverlay: "미디어 오버레이", ttsNotifications: "메시지 알림",
    runningLocally: "로컬 실행 중", startWithWindows: "Windows와 함께 시작", quitRelay: "Relay 종료", checking: "확인 중", connecting: "연결 중…",
    zeroOutputs: "출력 0개", online: "온라인", offline: "오프라인", waitingDiscord: "Discord 대기 중", output: "출력", outputs: "출력",
    hidden: "숨겨짐", visibleLocked: "표시 중 · 잠김", visibleMovable: "표시 중 · 이동 가능", hide: "숨기기", show: "표시", lock: "잠그기", unlock: "잠금 해제",
    mediaWidget: "미디어 위젯", notificationWidget: "알림 위젯", unavailable: "사용할 수 없음", notResponding: "Relay가 응답하지 않습니다",
  },
  ja: {
    relayStatus: "Relay の状態", connectionStatus: "接続状態", discord: "Discord", localRelay: "ローカル Relay", system: "システム",
    openPanel: "コントロール パネルを開く", displayWidgets: "表示ウィジェット", mediaOverlay: "メディアオーバーレイ", ttsNotifications: "メッセージ通知",
    runningLocally: "ローカルで実行中", startWithWindows: "Windows と同時に起動", quitRelay: "Relay を終了", checking: "確認中", connecting: "接続中…",
    zeroOutputs: "出力 0", online: "オンライン", offline: "オフライン", waitingDiscord: "Discord を待機中", output: "出力", outputs: "出力",
    hidden: "非表示", visibleLocked: "表示中 · ロック済み", visibleMovable: "表示中 · 移動可能", hide: "隠す", show: "表示", lock: "ロック", unlock: "ロック解除",
    mediaWidget: "メディアウィジェット", notificationWidget: "通知ウィジェット", unavailable: "利用不可", notResponding: "Relay が応答していません",
  },
  id: {
    relayStatus: "Status Relay", connectionStatus: "Status koneksi", discord: "Discord", localRelay: "Relay lokal", system: "Sistem",
    openPanel: "Buka panel kontrol", displayWidgets: "Widget tampilan", mediaOverlay: "Overlay media", ttsNotifications: "Notifikasi pesan",
    runningLocally: "Berjalan secara lokal", startWithWindows: "Mulai dengan Windows", quitRelay: "Keluar dari Relay", checking: "Memeriksa", connecting: "Menghubungkan…",
    zeroOutputs: "0 keluaran", online: "Online", offline: "Offline", waitingDiscord: "Menunggu Discord", output: "keluaran", outputs: "keluaran",
    hidden: "Tersembunyi", visibleLocked: "Terlihat · Terkunci", visibleMovable: "Terlihat · Dapat dipindahkan", hide: "Sembunyikan", show: "Tampilkan", lock: "Kunci", unlock: "Buka kunci",
    mediaWidget: "widget media", notificationWidget: "widget notifikasi", unavailable: "Tidak tersedia", notResponding: "Relay tidak merespons",
  },
});

// Attention row: the tray opens the panel on the page that needs action.
Object.assign(trayTranslations.en, { pendingReview: "{count} media awaiting moderation", reviewNow: "Review now", discordNotReady: "Discord is not connected", openDiscordSettings: "Open Discord settings" });
Object.assign(trayTranslations.fr, { pendingReview: "{count} média(s) en attente de modération", reviewNow: "Vérifier maintenant", discordNotReady: "Discord n’est pas connecté", openDiscordSettings: "Ouvrir les réglages Discord" });
Object.assign(trayTranslations.es, { pendingReview: "{count} medios pendientes de moderación", reviewNow: "Revisar ahora", discordNotReady: "Discord no está conectado", openDiscordSettings: "Abrir ajustes de Discord" });
Object.assign(trayTranslations.de, { pendingReview: "{count} Medien warten auf Moderation", reviewNow: "Jetzt prüfen", discordNotReady: "Discord ist nicht verbunden", openDiscordSettings: "Discord-Einstellungen öffnen" });
Object.assign(trayTranslations.ru, { pendingReview: "Ожидают модерации: {count}", reviewNow: "Проверить", discordNotReady: "Discord не подключён", openDiscordSettings: "Открыть настройки Discord" });
Object.assign(trayTranslations.zh, { pendingReview: "{count} 个媒体待审核", reviewNow: "立即审核", discordNotReady: "Discord 未连接", openDiscordSettings: "打开 Discord 设置" });
Object.assign(trayTranslations.ko, { pendingReview: "검토 대기 미디어 {count}개", reviewNow: "지금 검토", discordNotReady: "Discord가 연결되지 않음", openDiscordSettings: "Discord 설정 열기" });
Object.assign(trayTranslations.ja, { pendingReview: "モデレーション待ちのメディア {count} 件", reviewNow: "今すぐ確認", discordNotReady: "Discord に接続されていません", openDiscordSettings: "Discord 設定を開く" });
Object.assign(trayTranslations.id, { pendingReview: "{count} media menunggu moderasi", reviewNow: "Tinjau sekarang", discordNotReady: "Discord tidak terhubung", openDiscordSettings: "Buka pengaturan Discord" });

// 1.4.0: panic button and the header state pill.
Object.assign(trayTranslations.en, {"panic": "Panic", "resume": "Resume", "live": "Live", "paused": "Paused", "problem": "Problem"});
Object.assign(trayTranslations.fr, {"panic": "Panique", "resume": "Reprendre", "live": "En direct", "paused": "En pause", "problem": "Problème"});
Object.assign(trayTranslations.es, {"panic": "Pánico", "resume": "Reanudar", "live": "En directo", "paused": "En pausa", "problem": "Problema"});
Object.assign(trayTranslations.de, {"panic": "Panik", "resume": "Fortsetzen", "live": "Live", "paused": "Pausiert", "problem": "Problem"});
Object.assign(trayTranslations.ru, {"panic": "Паника", "resume": "Возобновить", "live": "В эфире", "paused": "Пауза", "problem": "Ошибка"});
Object.assign(trayTranslations.zh, {"panic": "紧急", "resume": "恢复", "live": "运行中", "paused": "已暂停", "problem": "异常"});
Object.assign(trayTranslations.ko, {"panic": "긴급", "resume": "다시 시작", "live": "작동 중", "paused": "일시 중지", "problem": "문제"});
Object.assign(trayTranslations.ja, {"panic": "緊急停止", "resume": "再開", "live": "稼働中", "paused": "一時停止", "problem": "問題あり"});
Object.assign(trayTranslations.id, {"panic": "Panik", "resume": "Lanjutkan", "live": "Aktif", "paused": "Dijeda", "problem": "Masalah"});

const trayRegionalTranslations = {
  "en-US": {
    displayWidgets: "Display widgets",
    visibleMovable: "Visible · Movable",
    startWithWindows: "Launch with Windows",
  },
  "en-GB": {
    displayWidgets: "Show widgets",
    visibleMovable: "Visible · Can be moved",
    startWithWindows: "Start with Windows",
  },
  "en-IN": {
    displayWidgets: "Show widgets",
    visibleMovable: "Visible · Can be moved",
    localRelay: "Local Relay service",
  },
};

let language = "en";
let locale = "en-US";

function translate(key) {
  return trayRegionalTranslations[locale]?.[key] || trayTranslations[language]?.[key] || trayTranslations.en[key] || key;
}

function applyTrayAccent() {
  let accent = [88, 185, 137];
  try {
    const storedAccent = JSON.parse(localStorage.getItem("relay-accent-rgb") || "null");
    if (Array.isArray(storedAccent) && storedAccent.length === 3 && storedAccent.every((value) => Number.isInteger(value) && value >= 0 && value <= 255)) {
      accent = storedAccent;
    }
  } catch {}
  const [red, green, blue] = accent;
  const lightAccent = (red * 299 + green * 587 + blue * 114) / 1000 > 156;
  document.documentElement.style.setProperty("--tray-accent", `rgb(${accent.join(" ")})`);
  document.documentElement.style.setProperty("--tray-accent-ink", lightAccent ? "#08130e" : "#ffffff");
}

function applyTrayLanguage() {
  const storedLanguage = localStorage.getItem("relay-language");
  const storedLocale = localStorage.getItem("relay-locale");
  const legacyDesignNames = { openai: "graphite", anthropic: "paper" };
  const rawDesign = localStorage.getItem("relay-design");
  const storedDesign = legacyDesignNames[rawDesign] || rawDesign;
  const storedTheme = localStorage.getItem("relay-theme");
  const storedInterfaceFont = localStorage.getItem("relay-interface-font");
  language = Object.hasOwn(trayTranslations, storedLanguage) ? storedLanguage : "en";
  locale = /^(en-(US|GB|IN)|fr-FR|de-DE|es-(ES|419)|ru-RU|zh-CN|ko-KR|ja-JP|id-ID)$/.test(storedLocale || "")
    ? storedLocale
    : language === "en" ? "en-US" : language;
  document.documentElement.lang = locale;
  document.documentElement.dataset.design = ["paper", "neo-brutalism", "gridline", "lumen", "signal"].includes(storedDesign)
    ? storedDesign
    : "graphite";
  document.documentElement.dataset.theme = storedTheme === "light" ? "light" : "dark";
  document.documentElement.dataset.interfaceFont = [
    "bricolage", "dm-sans", "figtree", "inter", "jetbrains-mono",
    "manrope", "poppins", "space-grotesk",
  ].includes(storedInterfaceFont) ? storedInterfaceFont : "design";
  applyTrayAccent();
  for (const element of document.querySelectorAll("[data-i18n]")) {
    element.textContent = translate(element.dataset.i18n);
  }
  for (const element of document.querySelectorAll("[data-i18n-aria]")) {
    element.setAttribute("aria-label", translate(element.dataset.i18nAria));
  }
}

const elements = {
  indicator: document.querySelector("#relay-indicator"),
  stateLabel: document.querySelector("#relay-state-label"),
  discordDot: document.querySelector("#discord-dot"),
  serverDot: document.querySelector("#server-dot"),
  panic: document.querySelector("#panic-toggle"),
  panicLabel: document.querySelector("#panic-label"),
  panicIcon: document.querySelector("#panic-icon"),
  resumeIcon: document.querySelector("#resume-icon"),
  discordStatus: document.querySelector("#discord-status"),
  discordDetail: document.querySelector("#discord-detail"),
  serverStatus: document.querySelector("#server-status"),
  serverDetail: document.querySelector("#server-detail"),
  mediaState: document.querySelector("#media-widget-state"),
  notificationState: document.querySelector("#notification-widget-state"),
  toggleMedia: document.querySelector("#toggle-media-widget"),
  lockMedia: document.querySelector("#lock-media-widget"),
  toggleNotification: document.querySelector("#toggle-notification-widget"),
  lockNotification: document.querySelector("#lock-notification-widget"),
  openPanel: document.querySelector("#open-panel"),
  startup: document.querySelector("#toggle-startup"),
  quit: document.querySelector("#quit"),
};

function renderWidget(state, stateElement, toggleButton, lockButton, name) {
  stateElement.textContent = state.visible
    ? translate(state.locked ? "visibleLocked" : "visibleMovable")
    : translate("hidden");
  toggleButton.setAttribute("aria-checked", String(state.visible));
  toggleButton.setAttribute("aria-label", `${translate(state.visible ? "hide" : "show")} ${translate(name)}`);
  lockButton.classList.toggle("is-unlocked", !state.locked);
  lockButton.setAttribute("aria-label", `${translate(state.locked ? "unlock" : "lock")} ${translate(name)}`);
  lockButton.setAttribute("aria-pressed", String(state.locked));
}

let outputsPaused = false;

function render(status) {
  const relayOnline = status.bot.connected && status.server.connected;
  outputsPaused = Boolean(status.outputsPaused);
  const state = outputsPaused ? "paused" : relayOnline ? "live" : status.server.connected ? "offline" : "problem";
  elements.indicator.dataset.state = state;
  elements.indicator.classList.toggle("is-online", relayOnline);
  elements.indicator.classList.toggle("is-error", !status.server.connected || Boolean(status.bot.error));
  elements.stateLabel.textContent = translate(state);
  elements.discordDot.dataset.state = status.bot.connected ? "on" : "off";
  elements.serverDot.dataset.state = status.server.connected ? "on" : "off";
  elements.panic.dataset.state = outputsPaused ? "paused" : "ready";
  elements.panicLabel.textContent = translate(outputsPaused ? "resume" : "panic");
  elements.panicIcon.toggleAttribute("hidden", outputsPaused);
  elements.resumeIcon.toggleAttribute("hidden", !outputsPaused);

  elements.discordStatus.textContent = translate(status.bot.connected ? "online" : "offline");
  elements.discordDetail.textContent = status.bot.username || status.bot.error || translate("waitingDiscord");
  elements.serverStatus.textContent = translate(status.server.connected ? "online" : "offline");
  elements.serverDetail.textContent = `${status.server.overlayClients} ${translate(status.server.overlayClients === 1 ? "output" : "outputs")}`;

  renderAttention(status);
  renderWidget(status.widget, elements.mediaState, elements.toggleMedia, elements.lockMedia, "mediaWidget");
  renderWidget(
    status.notificationWidget,
    elements.notificationState,
    elements.toggleNotification,
    elements.lockNotification,
    "notificationWidget",
  );
}

let attentionPage = null;
function renderAttention(status) {
  const pending = (status.pendingMedia || []).length;
  const button = document.querySelector("#tray-attention");
  if (pending > 0) {
    attentionPage = "moderation";
    document.querySelector("#tray-attention-label").textContent = translate("pendingReview").replace("{count}", pending);
    document.querySelector("#tray-attention-action").textContent = translate("reviewNow");
  } else if (!status.bot.connected) {
    attentionPage = "discord";
    document.querySelector("#tray-attention-label").textContent = translate("discordNotReady");
    document.querySelector("#tray-attention-action").textContent = translate("openDiscordSettings");
  } else {
    attentionPage = null;
  }
  button.hidden = !attentionPage;
}

async function refreshTray() {
  applyTrayLanguage();
  try {
    const [status, startupEnabled] = await Promise.all([
      invoke("get_runtime_status"),
      invoke("get_start_with_windows").catch(() => false),
    ]);
    render(status);
    elements.startup.setAttribute("aria-pressed", String(startupEnabled));
    elements.startup.setAttribute("aria-checked", String(startupEnabled));
  } catch {
    elements.indicator.classList.remove("is-online");
    elements.indicator.classList.add("is-error");
    elements.serverStatus.textContent = translate("unavailable");
    elements.serverDetail.textContent = translate("notResponding");
  }
}

async function runAction(button, command, payload) {
  button.disabled = true;
  try {
    await invoke(command, payload);
    await refreshTray();
  } catch {
    await refreshTray();
  } finally {
    button.disabled = false;
  }
}

elements.openPanel.addEventListener("click", () => invoke("tray_open_control_panel"));
document.querySelector("#tray-attention").addEventListener("click", () => invoke("tray_open_control_panel", { page: attentionPage }));
elements.toggleMedia.addEventListener("click", () => runAction(elements.toggleMedia, "tray_toggle_media_widget"));
elements.lockMedia.addEventListener("click", () => runAction(elements.lockMedia, "tray_toggle_media_widget_lock"));
elements.toggleNotification.addEventListener("click", () => runAction(elements.toggleNotification, "tray_toggle_notification_widget"));
elements.lockNotification.addEventListener("click", () => runAction(elements.lockNotification, "tray_toggle_notification_widget_lock"));
elements.startup.addEventListener("click", () => {
  const enabled = elements.startup.getAttribute("aria-pressed") !== "true";
  return runAction(elements.startup, "set_start_with_windows", { enabled });
});
elements.panic.addEventListener("click", () => runAction(elements.panic, outputsPaused ? "resume_outputs" : "panic_stop"));
elements.quit.addEventListener("click", () => invoke("tray_quit"));

window.refreshTray = refreshTray;
// No browser menu (Back, Refresh, Print…) on right click; text fields keep Copy and Paste.
window.addEventListener("contextmenu", (event) => {
  if (!event.target?.closest?.("input, textarea, [contenteditable='true']")) event.preventDefault();
});
window.addEventListener("focus", refreshTray);
document.addEventListener("visibilitychange", () => {
  if (!document.hidden) refreshTray();
});

refreshTray();
