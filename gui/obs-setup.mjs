// OBS & widgets → add Relay's Browser Sources to OBS through its WebSocket server.

export function parseObsPort(value) {
  const port = Number(value);
  return Number.isInteger(port) && port > 0 && port <= 65535 ? port : null;
}

/** "Relay Visual (created), Relay Audio (updated)" in the interface language. */
export function describeInstalledSources(installed, t) {
  const actions = { created: "obsCreated", updated: "obsUpdated", added: "obsAdded" };
  return installed.map((source) => `${source.name} (${t(actions[source.action] || source.action)})`).join(", ");
}

export function initializeObsSetup({ $, t, formatTranslation, invoke, setSaveState, readStorage, onInstalled }) {
  const portInput = $("#obs-port");
  const passwordInput = $("#obs-password");
  const passwordState = $("#obs-password-state");
  const connectButton = $("#obs-connect");
  const forgetButton = $("#obs-forget");
  const installBox = $("#obs-install");
  const sceneSelect = $("#obs-scene");
  const reactionsInput = $("#obs-include-reactions");
  const installButton = $("#obs-install-button");
  const state = $("#obs-auto-state");
  let passwordSaved = false;

  portInput.value = parseObsPort(readStorage("relay-obs-port")) || 4455;

  function request() {
    const port = parseObsPort(portInput.value);
    if (!port) throw new Error(t("obsInvalidPort"));
    try { localStorage.setItem("relay-obs-port", String(port)); } catch { /* Storage is optional. */ }
    return { port, password: passwordInput.value || null };
  }

  function updateCredentials(status) {
    passwordSaved = Boolean(status?.obsPasswordConfigured);
    passwordState.textContent = passwordSaved ? t("obsPasswordSaved") : "";
    passwordInput.placeholder = passwordSaved ? "••••••••" : "";
    forgetButton.hidden = !passwordSaved;
  }

  async function connect() {
    connectButton.disabled = true;
    setSaveState(state, "saving", t("obsConnecting"));
    try {
      const scenes = await invoke("obs_list_scenes", request());
      if (passwordInput.value) updateCredentials({ obsPasswordConfigured: true });
      passwordInput.value = "";
      sceneSelect.replaceChildren(...scenes.scenes.map((name) => {
        const option = document.createElement("option");
        option.value = name;
        option.textContent = name;
        option.selected = name === scenes.current;
        return option;
      }));
      installBox.hidden = scenes.scenes.length === 0;
      setSaveState(state, "saved", formatTranslation("obsConnected", { count: scenes.scenes.length }));
    } catch (error) {
      installBox.hidden = true;
      setSaveState(state, "error", String(error.message || error));
    } finally {
      connectButton.disabled = false;
    }
  }

  async function install() {
    installButton.disabled = true;
    setSaveState(state, "saving", t("obsConnecting"));
    try {
      const installed = await invoke("obs_install_sources", {
        ...request(),
        scene: sceneSelect.value,
        includeReactions: reactionsInput.checked,
      });
      passwordInput.value = "";
      setSaveState(state, "saved", formatTranslation("obsInstalled", { sources: describeInstalledSources(installed, t) }));
      onInstalled?.();
    } catch (error) {
      setSaveState(state, "error", String(error.message || error));
    } finally {
      installButton.disabled = false;
    }
  }

  connectButton.addEventListener("click", () => void connect());
  installButton.addEventListener("click", () => void install());
  forgetButton.addEventListener("click", async () => {
    try {
      updateCredentials(await invoke("forget_obs_password"));
      setSaveState(state, "saved", t("obsPasswordForgotten"));
    } catch (error) {
      setSaveState(state, "error", String(error));
    }
  });

  return { updateCredentials, applyLanguage: () => updateCredentials({ obsPasswordConfigured: passwordSaved }) };
}
