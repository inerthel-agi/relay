const MAX_REACTION_SECONDS = 30;

export function validTrimRange(start, end, duration) {
  return [start, end, duration].every(Number.isFinite)
    && start >= 0 && end > start && end <= duration && end - start <= MAX_REACTION_SECONDS + 1e-9;
}

export function trimErrorText(error, t) {
  const detail = String(error?.message || error || "").replace(/[\r\n\t]+/g, " ").slice(0, 300);
  return detail ? `${t("trimError")} ${detail}` : t("trimError");
}

export async function selectReactionSound(imported, { invoke, t }) {
  if (!imported) return null;
  if (imported.soundId) return imported.soundId;
  const token = imported.trimToken;
  const duration = Number(imported.durationSeconds);
  if (!token || !Number.isFinite(duration) || duration <= 0) throw new Error(t("trimError"));

  return new Promise((resolve) => {
    const previousFocus = document.activeElement;
    const dialog = document.createElement("dialog");
    dialog.className = "reaction-trim";
    dialog.setAttribute("aria-labelledby", "reaction-trim-title");
    const make = (tag, text) => {
      const element = document.createElement(tag);
      if (text !== undefined) element.textContent = text;
      return element;
    };
    const heading = make("h2", t("trimTitle"));
    heading.id = "reaction-trim-title";
    const description = make("p", t("trimCopy"));
    const source = make("p", `${t("trimSource")}: ${imported.filename}`);
    source.className = "reaction-trim__source";
    const sourceDuration = make("p", `${t("trimSourceDuration")}: ${duration.toFixed(3)} s`);
    const controls = make("div");
    controls.className = "reaction-trim__controls";
    function timeControl(key, value) {
      const row = make("label");
      row.className = "reaction-trim__time";
      row.append(make("span", t(key)));
      const number = make("input");
      number.type = "number"; number.min = "0"; number.max = String(duration); number.step = "0.001"; number.value = String(value);
      const range = make("input");
      range.type = "range"; range.min = "0"; range.max = String(duration); range.step = "0.001"; range.value = String(value);
      // Separate names keep the numeric fields and sliders unambiguous for assistive tools.
      range.setAttribute("aria-label", t(key));
      row.append(number, range); controls.append(row);
      return { number, range };
    }
    const start = timeControl("trimStart", 0);
    const end = timeControl("trimEnd", Math.min(MAX_REACTION_SECONDS, duration));
    const selection = make("p");
    const state = make("p"); state.setAttribute("role", "status"); state.className = "reaction-trim__state";
    const actions = make("div"); actions.className = "reaction-trim__actions";
    function action(key) {
      const element = make("button", t(key)); element.type = "button"; element.className = "button button--quiet";
      actions.append(element); return element;
    }
    const preview = action("trimPreview"), stop = action("trimStop"), cancel = action("trimCancel"), confirm = action("trimConfirm");
    confirm.className = "button button--primary";
    dialog.append(heading, description, source, sourceDuration, controls, selection, state, actions);
    let generation = 0, audio, playback, closed = false, committing = false, previewing = false;
    function stopAudio() {
      generation++;
      if (playback) {
        playback.onended = null;
        try { playback.stop(); } catch { /* The source may already have ended. */ }
        playback = undefined;
      }
      void audio?.close().catch(() => {}); audio = undefined;
      previewing = false; stop.disabled = true;
    }
    function values() { return { token, startSeconds: start.number.valueAsNumber, endSeconds: end.number.valueAsNumber }; }
    function update() {
      const { startSeconds, endSeconds } = values();
      const valid = validTrimRange(startSeconds, endSeconds, duration);
      selection.textContent = `${t("trimSelected")}: ${Number.isFinite(endSeconds - startSeconds) ? (endSeconds - startSeconds).toFixed(3) : "—"} / ${MAX_REACTION_SECONDS} s`;
      confirm.disabled = committing || !valid;
      preview.disabled = committing || previewing || !valid;
      if (!valid) state.textContent = t("trimInvalid");
      else if (!previewing && !committing) state.textContent = "";
    }
    for (const [control, isStart] of [[start, true], [end, false]]) {
      for (const input of [control.number, control.range]) input.addEventListener("input", () => {
        stopAudio();
        const value = input.valueAsNumber;
        if (input === control.range) control.number.value = input.value;
        else if (Number.isFinite(value)) control.range.value = input.value;
        if (isStart && Number.isFinite(value) && value >= 0 && value < duration) {
          end.number.value = String(Math.min(duration, value + MAX_REACTION_SECONDS)); end.range.value = end.number.value;
        }
        update();
      });
    }
    async function close(id = null) {
      if (closed || committing) return;
      closed = true; stopAudio();
      window.removeEventListener("beforeunload", unload);
      dialog.close(); dialog.remove(); previousFocus?.focus();
      if (!id) { try { await invoke("cancel_reaction_trim", { token }); } catch { /* A replaced or expired selection needs no further cleanup. */ } }
      resolve(id);
    }
    const unload = () => {
      stopAudio();
      if (!committing) void invoke("cancel_reaction_trim", { token }).catch(() => {});
    };
    preview.addEventListener("click", async () => {
      stopAudio(); previewing = true; stop.disabled = false; update(); state.textContent = t("trimWorking");
      const current = generation;
      try {
        // Resume during the click itself: rendering may outlast the browser's
        // user-activation window. Decode PCM directly instead of a blob URL,
        // which can be rejected by the desktop WebView media element.
        const AudioContext = window.AudioContext || window.webkitAudioContext;
        if (!AudioContext) throw new Error("Audio preview is unavailable in this webview.");
        const context = new AudioContext();
        audio = context;
        await context.resume();
        if (closed || current !== generation) return;
        const bytes = await invoke("preview_reaction_trim", values());
        if (closed || current !== generation) return;
        const buffer = await context.decodeAudioData(new Uint8Array(bytes).buffer);
        if (closed || current !== generation) return;
        const source = context.createBufferSource();
        source.buffer = buffer;
        source.connect(context.destination);
        playback = source;
        const finish = () => { if (current === generation) { stopAudio(); update(); } };
        source.onended = finish;
        source.start();
        if (current === generation) state.textContent = "";
      } catch (error) { if (current === generation && !closed) { stopAudio(); update(); state.textContent = trimErrorText(error, t); } }
    });
    stop.addEventListener("click", () => { stopAudio(); update(); });
    cancel.addEventListener("click", () => void close());
    dialog.addEventListener("cancel", event => { event.preventDefault(); void close(); });
    confirm.addEventListener("click", async () => {
      const range = values();
      if (!validTrimRange(range.startSeconds, range.endSeconds, duration)) return;
      stopAudio(); committing = true; cancel.disabled = true;
      for (const input of [start.number, start.range, end.number, end.range]) input.disabled = true;
      update(); state.textContent = t("trimWorking");
      try {
        const id = await invoke("finish_reaction_trim", range);
        committing = false; await close(id);
      } catch (error) {
        committing = false; cancel.disabled = false;
        for (const input of [start.number, start.range, end.number, end.range]) input.disabled = false;
        update(); state.textContent = trimErrorText(error, t);
      }
    });
    window.addEventListener("beforeunload", unload);
    document.body.append(dialog); stop.disabled = true; update(); dialog.showModal(); start.number.focus();
  });
}
