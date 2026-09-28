// Overview page: setup checklist, dashboard, sidebar badges, channel links and the trial mode.

// Shared with the startup code, which reopens the last page.
export function readStorage(key) {
  try { return localStorage.getItem(key); } catch { return null; }
}

export function initializeOverview({ $, $$, t, invoke, formatTranslation, setSaveState, setWidgetState, getBootstrap, getHistory }) {
  function markSetupTested() {
    try { localStorage.setItem("relay-setup-tested", "1"); } catch { /* Storage is optional. */ }
    updateOverview();
  }

  // Required steps first; the checklist and nav badges read only getBootstrap() state already polled.
  function setupStatus() {
    const config = getBootstrap()?.config || {};
    const visualClients = Number(getBootstrap()?.server?.outputs?.visual?.obsClients || 0);
    return {
      bot: Boolean(getBootstrap()?.credentials?.configured && getBootstrap()?.bot?.connected),
      invite: (getBootstrap()?.channels || []).length > 0,
      channel: Boolean(config.watchedChannelId),
      obs: visualClients > 0,
      test: readStorage("relay-setup-tested") === "1" || getHistory().length > 0,
      music: Boolean(getBootstrap()?.credentials?.youtubeConfigured && config.musicChannelId),
      reactions: null,
    };
  }

  const requiredSetupSteps = ["bot", "invite", "channel", "obs", "test"];
  let setupChecklistSettled = false;

  function selectedChannelName(select) {
    const option = select?.selectedOptions?.[0];
    return option && option.value ? option.textContent : t("channelNone");
  }

  function updateOverview() {
    if (!getBootstrap()) return;
    const status = setupStatus();
    const done = requiredSetupSteps.filter((step) => status[step]).length;
    for (const item of $$("[data-setup-step]")) {
      const value = status[item.dataset.setupStep];
      const optional = item.hasAttribute("data-setup-optional");
      item.classList.toggle("is-done", value === true);
      const state = item.querySelector("[data-setup-state]");
      state.textContent = value === true ? t("setupDone") : optional ? t("setupOptional") : t("setupTodo");
    }
    const complete = done === requiredSetupSteps.length;
    const progress = $("#setup-progress");
    if (progress) {
      progress.textContent = complete ? t("setupReady") : formatTranslation("setupProgress", { done, total: requiredSetupSteps.length });
      progress.classList.toggle("is-ready", complete);
    }
    const meter = $("#setup-meter-fill");
    if (meter) meter.style.width = `${Math.round((done / requiredSetupSteps.length) * 100)}%`;
    const checklist = $("#setup-checklist");
    // Collapse once on load when everything is ready; afterwards the user controls it.
    if (checklist && !setupChecklistSettled) {
      checklist.open = !complete;
      setupChecklistSettled = true;
    }

    const visual = getBootstrap().server?.outputs?.visual || {};
    const outputs = Number(getBootstrap().server?.overlayClients || 0);
    const pending = (getBootstrap().pendingMedia || []).length;
    const lastMedia = getHistory()[0];
    const text = (id, value) => { const element = document.getElementById(id); if (element) element.textContent = value; };
    text("dashboard-bot", getBootstrap().bot?.connected ? getBootstrap().bot.username || t("statusOnline") : t("botOffline"));
    text("dashboard-outputs", formatTranslation("dashboardOutputsValue", { count: outputs }));
    text("dashboard-pending", String(pending));
    text("dashboard-last-media", lastMedia?.filename || lastMedia?.title || t("dashboardNoMedia"));

    const badges = {
      discord: status.bot && status.channel ? "" : "!",
      overlay: getBootstrap().server?.connected && Number(visual.obsClients || 0) === 0 ? "!" : "",
      moderation: pending > 0 ? String(pending) : "",
    };
    for (const badge of $$("[data-nav-badge]")) {
      const value = badges[badge.dataset.navBadge] || "";
      badge.textContent = value;
      badge.hidden = !value;
    }

    // Trial mode: until Discord is connected, Overview offers local tests on the widget or OBS.
    const trial = $("#trial-mode");
    if (trial) trial.hidden = status.bot;
    const trialWidget = $("#trial-show-widget");
    if (trialWidget) {
      trialWidget.disabled = Boolean(getBootstrap().widget?.visible);
      trialWidget.textContent = t(getBootstrap().widget?.visible ? "trialWidgetShown" : "trialShowWidget");
    }

    for (const name of $$("[data-channel-name]")) {
      name.textContent = selectedChannelName(document.getElementById(name.dataset.channelName));
    }
  }

  $("#trial-show-widget").addEventListener("click", async () => {
    if (getBootstrap()?.widget?.visible) return;
    try {
      const state = await invoke("toggle_widget");
      setWidgetState(state);
      getBootstrap().widget = state;
      updateOverview();
    } catch (error) {
      setSaveState($("#trial-state"), "error", String(error));
    }
  });

  $("#trial-send").addEventListener("click", async (event) => {
    const button = event.currentTarget;
    const sample = $("#trial-sample").value;
    const target = ["portrait", "landscape", "gif", "video"].includes(sample) ? "visual" : sample;
    const output = getBootstrap()?.server?.outputs?.[target] || {};
    const state = $("#trial-state");
    if (Number(output.obsClients || 0) + Number(output.widgetClients || 0) === 0) {
      setSaveState(state, "error", t("trialNeedsOutput"));
      return;
    }
    button.disabled = true;
    try {
      await invoke("preview_output_sample", { sample });
      setSaveState(state, "saved", t("outputTestSent"));
      markSetupTested();
    } catch (error) {
      setSaveState(state, "error", String(error));
    } finally {
      button.disabled = false;
    }
  });

  return { update: updateOverview, setupStatus, markTested: markSetupTested };
}
