import { selectReactionSound } from "./reaction-trim.mjs";
import { renderAccessMenu, roleChoicesForChannels } from "./reaction-access.mjs";

export function initializeModules({ invoke, t, getBootstrap }) {
  const $ = (id) => document.getElementById(id);
  let library = [], reactionSettings, messageStatus, refreshInFlight = false, previewAudio, previewSource, previewImageUrl, previewGeneration = 0, musicSignature;
  let accessCatalog = { channels: [], roles: [] }, accessError = "", channelMenu, roleMenu, accessState, accessLoading = false, accessBotConnected = false;
  const node = (tag, text) => { const element = document.createElement(tag); if (text !== undefined) element.textContent = text; return element; };
  const label = (tag, key) => { const element = node(tag, t(key)); element.dataset.i18n = key; return element; };
  const button = (key, action) => { const element = label("button", key); element.type = "button"; element.className = "button button--quiet"; element.addEventListener("click", () => run(element, action)); return element; };
  // Module settings save themselves: toggles and menus at once, typed values when the field is left.
  function autosave(root, save) {
    let timer;
    const schedule = (delay) => { clearTimeout(timer); timer = setTimeout(() => void save(), delay); };
    root.addEventListener("change", (event) => { if (event.target.matches("input, select, textarea")) schedule(0); });
    root.addEventListener("input", (event) => { if (event.target.matches('input[type="checkbox"], input[type="range"], select')) schedule(300); });
    return () => schedule(0);
  }
  const status = (root) => { const element = node("p"); element.setAttribute("role", "status"); root.append(element); return element; };
  function errorText(error) {
    const value = String(error);
    const key = /No reaction output/i.test(value) ? "modReactionNoOutput" : /channel or your roles/i.test(value) ? "modErrorAccess" : /queue.*full|full.*queue|requests are temporarily full/i.test(value) ? "modErrorQueueFull" : /already playing/i.test(value) ? "modErrorBusy" : /wait before/i.test(value) ? "modErrorCooldown" : /Reactions are disabled/i.test(value) ? "modErrorDisabled" : /unavailable|no longer exists/i.test(value) ? "modErrorUnavailable" : /invalid|must be|must stay|does not match/i.test(value) ? "modErrorInvalid" : null;
    return key ? t(key) : value;
  }
  function reactionTriggerText(result) {
    const position = Number(result?.position);
    return Number.isInteger(position) && position > 0
      ? t("modReactionQueued").replace("{position}", String(position))
      : t("modReactionStarted");
  }
  async function run(element, action) { element.disabled = true; try { await action(); } catch (error) { (element.closest("section")?.querySelector('[role="status"]') || reactionState).textContent = errorText(error); } finally { element.disabled = element === pin ? !messageStatus?.current || Boolean(messageStatus?.pinned) : element === unpin ? !messageStatus?.pinned : false; } }
  function field(root, key, type, value, change, min, max) { const wrapper = node("label"); wrapper.className = "module-field"; wrapper.append(label("span", key)); const input = node("input"); input.type = type; if (type === "checkbox") input.checked = value; else input.value = value; if (min !== undefined) input.min = min; if (max !== undefined) input.max = max; input.addEventListener("input", () => change(type === "checkbox" ? input.checked : type === "number" || type === "range" ? Number(input.value) : input.value)); wrapper.append(input); root.append(wrapper); return input; }

  const messages = $("message-module"), messageState = status(messages), messageText = node("p");
  messages.append(messageText);
  const pin = button("modPin", async () => { await invoke("pin_message", { messageId: messageStatus?.current?.id }); await refresh(); });
  const unpin = button("modUnpin", async () => { await invoke("unpin_message"); await refresh(); });
  messages.append(pin, unpin);

  const libraryRoot = $("library-module"); libraryRoot.append(label("h3", "modLibrary"), label("p", "modLibraryCopy"));
  const libraryState = status(libraryRoot), search = node("input"); search.type = "search"; search.setAttribute("aria-label", t("modSearch")); search.placeholder = t("modSearch");
  libraryRoot.append(search, button("modImport", async () => { await invoke("import_library_media"); await loadLibrary(); }));
  const libraryList = node("ul"); libraryList.className = "module-list"; libraryRoot.append(libraryList); search.addEventListener("input", renderLibrary);
  async function loadLibrary() { const result = await invoke("get_media_library"); library = result.items; libraryState.textContent = `${t("modStorage")}: ${(result.totalBytes / 1048576).toFixed(1)} MiB`; renderLibrary(); if (reactionSettings) renderReactions(); }
  function renderLibrary() {
    libraryList.replaceChildren();
    for (const item of library.filter(item => item.name.toLocaleLowerCase().includes(search.value.toLocaleLowerCase()))) {
      const row = node("li"); const name = node("input"); name.value = item.name; name.maxLength = 120; name.setAttribute("aria-label", t("modName"));
      row.append(name, node("span", `${item.kind} · ${(item.sizeBytes / 1048576).toFixed(1)} MiB`),
        button("modRename", async () => { await invoke("rename_library_media", { id: item.id, name: name.value }); await loadLibrary(); }),
        button("modPlay", () => invoke("play_library_media", { id: item.id })),
        button("modDelete", async () => { if (await confirmDelete(row)) { await invoke("delete_library_media", { id: item.id }); await loadLibrary(); } }));
      libraryList.append(row);
    }
    if (!libraryList.children.length) libraryList.append(node("li", t("modEmptyLibrary")));
  }
  function confirmDelete(row) {
    return new Promise(resolve => {
      const confirmation = node("div"); confirmation.setAttribute("role", "group"); confirmation.append(label("span", "modDeleteConfirm"));
      const yes = button("modDelete", () => { confirmation.remove(); resolve(true); });
      const no = button("modCancel", () => { confirmation.remove(); resolve(false); }); confirmation.append(yes, no); row.append(confirmation); no.focus();
    });
  }

  const musicRoot = $("music-queue-module"); musicRoot.append(label("h3", "modMusicRequests")); const musicState = status(musicRoot);
  const musicConfig = { maxPendingPerUser: getBootstrap()?.config.musicMaxPendingPerUser ?? 3, rejectDuplicatePending: getBootstrap()?.config.musicRejectDuplicatePending ?? true };
  field(musicRoot, "modRequestLimit", "number", musicConfig.maxPendingPerUser, value => musicConfig.maxPendingPerUser = value, 0, 10);
  field(musicRoot, "modRejectDuplicates", "checkbox", musicConfig.rejectDuplicatePending, value => musicConfig.rejectDuplicatePending = value);
  autosave(musicRoot, async () => {
    const snapshot = { ...musicConfig };
    musicState.textContent = t("saving");
    try { await invoke("save_music_queue_settings", snapshot); musicState.textContent = t(JSON.stringify(snapshot) === JSON.stringify(musicConfig) ? "modSaved" : "unsaved"); }
    catch (error) { musicState.textContent = errorText(error); }
  });
  const musicList = node("ol"); musicList.className = "module-list"; musicRoot.append(musicList);
  function renderMusic(items) {
    const signature = JSON.stringify(items) + t("modQueueEmpty");
    if (signature === musicSignature) return;
    musicSignature = signature;
    musicList.replaceChildren();
    for (const [index, item] of items.entries()) {
      const row = node("li"); row.append(node("span", `${index + 1}. ${item.title} — ${item.author}`));
      for (const [key, direction] of [["modUp", "up"], ["modDown", "down"]]) {
        const move = button(key, async () => { await invoke("move_music_queue", { id: item.id, direction }); await refresh(); });
        move.disabled = direction === "up" ? index === 0 : index === items.length - 1; row.append(move);
      }
      row.append(button("modRemove", async () => { await invoke("remove_music_queue", { id: item.id }); await refresh(); })); musicList.append(row);
    }
    if (!items.length) musicList.append(node("li", t("modQueueEmpty")));
  }

  const reactions = $("reaction-module"), reactionState = status(reactions), reactionEditor = node("div"); reactionEditor.className = "reaction-editor"; reactions.append(reactionEditor);
  const reactionPreview = node("div"); reactionPreview.className = "reaction-local-preview"; reactionPreview.hidden = true; reactions.append(reactionPreview);
  function stopPreview() {
    previewGeneration++;
    if (previewSource) { previewSource.onended = null; try { previewSource.stop(); } catch { /* Already stopped. */ } previewSource = undefined; }
    void previewAudio?.close().catch(() => {}); previewAudio = undefined;
    if (previewImageUrl) URL.revokeObjectURL(previewImageUrl);
    previewImageUrl = undefined; reactionPreview.hidden = true; reactionPreview.replaceChildren();
  }
  async function testReaction(item) {
    stopPreview(); const generation = previewGeneration;
    reactionState.textContent = "";
    try {
      // Unlock audio during the click, before IPC and decoding consume user activation.
      const AudioContext = window.AudioContext || window.webkitAudioContext;
      if (!AudioContext) throw new Error("Audio preview is unavailable in this webview.");
      const context = new AudioContext(); previewAudio = context;
      await context.resume();
      if (generation !== previewGeneration) return;
      const bytes = await invoke("preview_reaction_sound", { id: item.soundId });
      if (generation !== previewGeneration) return;
      const buffer = await context.decodeAudioData(new Uint8Array(bytes).buffer);
      if (generation !== previewGeneration) return;
      const asset = item.visualId ? await invoke("preview_library_asset", { id: item.visualId }) : null;
      if (generation !== previewGeneration) return;
      const gain = context.createGain(); gain.gain.value = Math.min(1, Math.max(0, item.volume / 100)); gain.connect(context.destination);
      const source = context.createBufferSource(); source.buffer = buffer; source.connect(gain); previewSource = source;
      reactionPreview.hidden = false; reactionPreview.append(node("strong", item.name));
      if (asset) { previewImageUrl = URL.createObjectURL(new Blob([new Uint8Array(asset.bytes)], { type: asset.contentType })); const image = node("img"); image.src = previewImageUrl; image.alt = ""; reactionPreview.append(image); }
      source.onended = () => { if (generation === previewGeneration) stopPreview(); };
      source.start();
    } catch (error) { if (generation === previewGeneration) { stopPreview(); throw error; } }
  }
  function renderAccess() {
    if (!channelMenu || !roleMenu) return;
    renderAccessMenu(channelMenu, { choices: accessCatalog.channels, selectedIds: reactionSettings.allowedChannelIds, kind: "channel", t,
      onChange: ids => { reactionSettings.allowedChannelIds = ids; renderRoleAccess(); } });
    renderRoleAccess();
    accessState.textContent = accessError ? t("modAccessUnavailable") : "";
  }
  function renderRoleAccess() {
    renderAccessMenu(roleMenu, { choices: roleChoicesForChannels(accessCatalog, reactionSettings.allowedChannelIds), selectedIds: reactionSettings.allowedRoleIds, kind: "role", t,
      onChange: ids => { reactionSettings.allowedRoleIds = ids; } });
  }
  async function loadAccess() {
    if (accessLoading) return;
    accessLoading = true;
    try {
      const catalog = await invoke("get_reaction_access_options");
      if (!Array.isArray(catalog?.channels) || !Array.isArray(catalog?.roles)) throw new Error("Unavailable catalog");
      accessCatalog = catalog; accessError = "";
    } catch { accessError = t("modAccessUnavailable"); }
    finally { accessLoading = false; }
    renderAccess();
  }
  async function saveReactions() {
    if (!reactionSettings) return;
    const snapshot = structuredClone(reactionSettings);
    reactionState.textContent = t("saving");
    try {
      const result = await invoke("save_reactions", { settings: snapshot });
      reactionState.textContent = t(JSON.stringify(snapshot) !== JSON.stringify(reactionSettings) ? "unsaved" : result?.discordPending ? "modSavedOffline" : "modSaved");
    } catch (error) { reactionState.textContent = errorText(error); }
  }
  const saveReactionsSoon = autosave(reactionEditor, saveReactions);
  const reactionSections = new Map();
  function reactionSection(key) {
    const details = node("details"); details.className = "panel-disclosure reaction-disclosure";
    details.open = reactionSections.get(key) || false;
    details.addEventListener("toggle", () => reactionSections.set(key, details.open));
    const content = node("div"); content.className = "reaction-disclosure-content";
    details.append(label("summary", key), content); reactionEditor.append(details);
    return content;
  }
  function renderReactions() {
    reactionEditor.replaceChildren(); const settings = reactionSettings;
    field(reactionEditor, "modEnabled", "checkbox", settings.enabled, value => settings.enabled = value);
    const general = reactionSection("modReactionGeneral");
    field(general, "modGlobalCooldown", "number", settings.globalCooldownSeconds, value => settings.globalCooldownSeconds = value, 0, 3600);
    field(general, "modMemberCooldown", "number", settings.memberCooldownSeconds, value => settings.memberCooldownSeconds = value, 0, 3600);
    field(general, "modMusicPercent", "number", settings.musicPercent, value => settings.musicPercent = value, 0, 100);
    const access = reactionSection("modReactionAccess");
    const accessMenus = node("div"); accessMenus.className = "reaction-access";
    channelMenu = node("div"); roleMenu = node("div");
    accessMenus.append(channelMenu, roleMenu); access.append(accessMenus);
    accessState = node("p"); accessState.setAttribute("role", "status");
    access.append(accessState, button("modAccessRefresh", loadAccess));
    renderAccess();
    access.append(label("p", "modAccessCopy"));
    const protectedMessage = field(access, "modProtectedMessage", "text", settings.protectedMessageId || "", value => settings.protectedMessageId = value.trim());
    protectedMessage.parentElement.className += " module-field--wide";
    protectedMessage.placeholder = t("musicWelcomePlaceholder");
    protectedMessage.dataset.i18nPlaceholder = "musicWelcomePlaceholder";
    access.append(label("p", "modProtectedHelp"));
    const display = reactionSection("modReactionDisplay");
    const anchorLabel = label("label", "modAnchor"), anchor = node("select"); anchorLabel.className = "module-field module-field--wide";
    for (const value of ["legacy", "topLeft", "topCenter", "topRight", "center", "bottomLeft", "bottomCenter", "bottomRight"]) { const option = node("option", t(`anchor${value[0].toUpperCase()}${value.slice(1)}`)); option.value = value; anchor.append(option); }
    anchor.value = settings.geometry.anchor; anchor.addEventListener("change", () => settings.geometry.anchor = anchor.value); anchorLabel.append(anchor); display.append(anchorLabel);
    field(display, "modMarginX", "number", settings.geometry.marginX, value => settings.geometry.marginX = value, 0, 200);
    field(display, "modMarginY", "number", settings.geometry.marginY, value => settings.geometry.marginY = value, 0, 200);
    field(display, "modScale", "number", settings.geometry.contentScale, value => settings.geometry.contentScale = value, 50, 200);
    reactionEditor.append(button("modAddReaction", async () => {
      stopPreview();
      const imported = await invoke("import_reaction_sound");
      const soundId = await selectReactionSound(imported, { invoke, t }); if (!soundId) return;
      settings.definitions.push({ id: crypto.randomUUID().replaceAll("-", ""), name: t("modNewReaction"), soundId, visualId: null, volume: 70, enabled: true }); renderReactions(); saveReactionsSoon();
    }));
    const list = node("ul"); list.className = "module-list reaction-list";
    for (const item of settings.definitions) {
      const row = node("li");
      field(row, "modName", "text", item.name, value => item.name = value);
      field(row, "modEnabled", "checkbox", item.enabled, value => item.enabled = value);
      field(row, "modVolume", "number", item.volume, value => item.volume = value, 0, 100);
      const visualLabel = label("label", "modVisual"), select = node("select"); visualLabel.className = "module-field"; const empty = node("option", t("modNoVisual")); empty.value = ""; select.append(empty);
      for (const media of library.filter(media => (media.kind === "image" || media.kind === "gif") && media.contentType.startsWith("image/"))) { const option = node("option", media.name); option.value = media.id; select.append(option); }
      select.value = item.visualId || ""; select.addEventListener("change", () => item.visualId = select.value || null); visualLabel.append(select); row.append(visualLabel);
      row.append(button("modTestLocal", () => testReaction(item)), button("modPlay", async () => { const result = await invoke("trigger_reaction", { id: item.id }); reactionState.textContent = reactionTriggerText(result); }), button("modDelete", async () => { if (await confirmDelete(row)) { settings.definitions = settings.definitions.filter(value => value.id !== item.id); renderReactions(); saveReactionsSoon(); } })); list.append(row);
    }
    reactionEditor.append(list, button("modStop", async () => { stopPreview(); await invoke("stop_reaction"); }));
    // The OBS link lives with the other Browser Sources on the Overlay page.
    reactionEditor.append(label("p", "reactionsSourceMoved"));
  }
  async function refresh() {
    if (refreshInFlight) return; refreshInFlight = true;
    const connected = Boolean(getBootstrap()?.bot?.connected);
    if (connected && !accessBotConnected) void loadAccess();
    accessBotConnected = connected;
    try {
      const results = await Promise.allSettled([invoke("get_message_status"), invoke("get_music_queue")]);
      if (results[0].status === "fulfilled") { messageStatus = results[0].value; messageText.textContent = messageStatus.current ? `${messageStatus.current.author.username}: ${messageStatus.current.text}` : t("modNoMessage"); pin.disabled = !messageStatus.current || messageStatus.pinned; unpin.disabled = !messageStatus.pinned; messageState.textContent = messageStatus.pinned ? t("modPinned") : ""; }
      else messageState.textContent = String(results[0].reason);
      if (results[1].status === "fulfilled") renderMusic(results[1].value); else musicState.textContent = String(results[1].reason);
    } finally { refreshInFlight = false; }
  }
  async function initialize() {
    const results = await Promise.allSettled([loadLibrary(), invoke("get_reactions")]);
    if (results[0].status === "rejected") libraryState.textContent = errorText(results[0].reason);
    if (results[1].status === "fulfilled") { reactionSettings = results[1].value; renderReactions(); } else reactionState.textContent = errorText(results[1].reason);
    await Promise.all([refresh(), loadAccess()]);
  }
  void initialize().catch(error => reactionState.textContent = String(error));
  window.addEventListener("beforeunload", stopPreview);
  window.addEventListener("relay-language-change", () => { for (const root of [messages, libraryRoot, musicRoot, reactions]) for (const element of root.querySelectorAll("[data-i18n]")) { if (element.tagName === "LABEL") { if (element.firstChild?.nodeType === Node.TEXT_NODE) element.firstChild.textContent = t(element.dataset.i18n); } else element.textContent = t(element.dataset.i18n); } renderLibrary(); renderAccess(); });
  return { refresh, loadLibrary };
}
