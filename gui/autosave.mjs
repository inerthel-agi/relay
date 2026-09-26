// Settings save themselves: toggles and menus at once, typed values when the field is left.
// Credentials and the YouTube key keep an explicit button because they are secrets.
export function initializeAutosave({ forms, dirtyForms, formSaveState, setSaveState, t, skipTarget, doc = document, clock = window }) {
  const timers = new WeakMap();
  function schedule(form, delay) {
    clock.clearTimeout(timers.get(form));
    timers.set(form, clock.setTimeout(() => {
      timers.delete(form);
      if (!dirtyForms.has(form)) return;
      if (!form.checkValidity()) {
        setSaveState(formSaveState(form), "error", t("autosaveInvalid"));
        return;
      }
      form.requestSubmit();
    }, delay));
  }
  for (const form of forms) {
    form.addEventListener("change", (event) => {
      // Filter words have their own debounced save.
      if (event.target === skipTarget) return;
      schedule(form, 0);
    });
    form.addEventListener("input", (event) => {
      if (event.target.matches('input[type="checkbox"], input[type="radio"], input[type="range"], select')) {
        schedule(form, 300);
      }
    });
  }
  doc.addEventListener("visibilitychange", () => {
    if (doc.visibilityState !== "hidden") return;
    for (const form of forms) {
      if (dirtyForms.has(form)) schedule(form, 0);
    }
  });
  return { schedule };
}

// Personalization → System: the same Windows startup entry the tray toggles.
export function initializeStartWithWindows({ $, invoke, t, setSaveState }) {
  const checkbox = $("#start-with-windows");
  const state = $("#start-with-windows-state");
  async function refresh() {
    try {
      checkbox.checked = Boolean(await invoke("get_start_with_windows"));
      checkbox.disabled = false;
    } catch {
      checkbox.disabled = true;
    }
  }
  checkbox.addEventListener("change", async () => {
    const enabled = checkbox.checked;
    checkbox.disabled = true;
    setSaveState(state, "saving");
    try {
      await invoke("set_start_with_windows", { enabled });
      setSaveState(state, "saved");
    } catch (error) {
      checkbox.checked = !enabled;
      setSaveState(state, "error", `${t("startWithWindowsFailed")} ${String(error)}`);
    } finally {
      checkbox.disabled = false;
    }
  });
  // The tray can change this too, so re-read it whenever the panel comes back.
  window.addEventListener("focus", () => void refresh());
  void refresh();
  return { refresh };
}
