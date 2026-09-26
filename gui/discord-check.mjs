// Discord page → Bot check: shows the intent state and the permissions each channel is missing.

const featureLabels = {
  media: "checkFeatureMedia",
  notifications: "checkFeatureNotifications",
  music: "checkFeatureMusic",
  honeypot: "checkFeatureHoneypot",
};

// Discord permission names from Rust, shown with the wording of Discord's settings.
const permissionLabels = {
  VIEW_CHANNEL: "permViewChannel",
  READ_MESSAGE_HISTORY: "permReadMessageHistory",
  SEND_MESSAGES: "permSendMessages",
  EMBED_LINKS: "permEmbedLinks",
  MANAGE_MESSAGES: "permManageMessages",
  KICK_MEMBERS: "permKickMembers",
  BAN_MEMBERS: "permBanMembers",
};

/** Pure summary used by the page and by tests. */
export function describeDiscordCheck(report, t, formatTranslation) {
  if (!report?.botConnected) {
    return {
      state: report?.messageContentIntent === "missing" ? "error" : "idle",
      message: t(report?.messageContentIntent === "missing" ? "intentMissing" : "checkBotOffline"),
      intent: report?.messageContentIntent === "missing" ? t("intentMissing") : null,
      rows: [],
      needsInvite: false,
    };
  }
  const rows = (report.checks || []).map((check) => {
    const label = t(featureLabels[check.feature] || check.feature);
    const channel = check.channelName ? `#${check.channelName}` : "";
    if (check.status === "ok") return { label, channel, state: "saved", text: t("checkOk") };
    if (check.status === "notConfigured") return { label, channel, state: "idle", text: t("checkNotConfigured") };
    if (check.status === "channelNotFound") return { label, channel, state: "error", text: t("checkNotFound") };
    const permissions = check.missing.map((name) => t(permissionLabels[name] || name)).join(", ");
    return { label, channel, state: "error", text: formatTranslation("checkMissing", { permissions }) };
  });
  const intentOk = report.messageContentIntent === "enabled";
  const needsInvite = rows.some((row) => row.state === "error");
  const allGood = intentOk && !needsInvite;
  return {
    state: allGood ? "saved" : "error",
    message: allGood ? t("checkAllGood") : t("checkProblems"),
    intent: intentOk ? null : t(report.messageContentIntent === "missing" ? "intentMissing" : "intentUnknown"),
    rows,
    needsInvite,
  };
}

export function initializeDiscordCheck({ $, t, formatTranslation, invoke, setSaveState, getBootstrap }) {
  const runButton = $("#run-discord-check");
  const state = $("#discord-check-state");
  const intentBox = $("#discord-check-intent");
  const intentText = $("#discord-check-intent-text");
  const results = $("#discord-check-results");
  const reinvite = $("#discord-check-reinvite");
  let lastRun = 0;
  let lastReport = null;

  function render() {
    if (!lastReport) return;
    const summary = describeDiscordCheck(lastReport, t, formatTranslation);
    setSaveState(state, summary.state, summary.message);
    intentBox.hidden = !summary.intent;
    intentText.textContent = summary.intent || "";
    results.replaceChildren(...summary.rows.map((row) => {
      const item = document.createElement("li");
      item.className = "check-result";
      item.dataset.state = row.state;
      const name = document.createElement("strong");
      name.textContent = row.channel ? `${row.label} · ${row.channel}` : row.label;
      const detail = document.createElement("span");
      detail.textContent = row.text;
      item.append(name, detail);
      return item;
    }));
    reinvite.hidden = !summary.needsInvite || !getBootstrap()?.inviteUrl;
  }

  async function run() {
    runButton.disabled = true;
    setSaveState(state, "saving", t("checkRunning"));
    try {
      lastReport = await invoke("check_discord_setup");
      lastRun = Date.now();
      render();
    } catch (error) {
      setSaveState(state, "error", String(error));
    } finally {
      runButton.disabled = false;
    }
  }

  runButton.addEventListener("click", () => void run());
  reinvite.addEventListener("click", () => {
    const link = getBootstrap()?.inviteUrl;
    if (link) void invoke("open_help_link", { link });
  });

  return {
    run,
    render,
    // Opening the Discord page re-checks at most once a minute.
    runIfStale() {
      if (getBootstrap()?.bot?.connected && Date.now() - lastRun > 60_000) void run();
    },
  };
}
