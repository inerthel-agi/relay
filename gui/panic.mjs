// Panic button: the top bar button clears every output and pauses Relay; the banner resumes it.
export function initializePanic({ $, t, invoke, notify, getShortcutLabel }) {
  const button = $("#panic-button");
  const banner = $("#paused-banner");
  const resumeButton = $("#resume-button");
  let paused = false;

  function setPaused(next) {
    paused = Boolean(next);
    banner.hidden = !paused;
    button.classList.toggle("is-active", paused);
    button.setAttribute("aria-pressed", String(paused));
    document.documentElement.classList.toggle("relay-paused", paused);
  }

  function applyLanguage() {
    button.title = `${t("panicTitle")} (${getShortcutLabel()})`;
  }

  button.addEventListener("click", async () => {
    button.disabled = true;
    try {
      await invoke("panic_stop");
      setPaused(true);
      notify("saved", t("panicDone"));
    } catch (error) {
      notify("error", String(error));
    } finally {
      button.disabled = false;
    }
  });

  resumeButton.addEventListener("click", async () => {
    resumeButton.disabled = true;
    try {
      await invoke("resume_outputs");
      setPaused(false);
      notify("saved", t("resumed"));
    } catch (error) {
      notify("error", String(error));
    } finally {
      resumeButton.disabled = false;
    }
  });

  return { setPaused, isPaused: () => paused, applyLanguage };
}
