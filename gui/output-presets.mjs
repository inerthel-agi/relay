const anchors = new Set(["legacy", "topLeft", "topCenter", "topRight", "center", "bottomLeft", "bottomCenter", "bottomRight"]);
export function normalizePreset(value) {
  if (!value || typeof value.name !== "string" || !value.name.trim() || value.name.length > 64) return null;
  const source = value.geometry || {};
  const range = (key, fallback, min, max) => Number.isFinite(Number(source[key])) ? Math.round(Math.min(max, Math.max(min, Number(source[key])))) : fallback;
  return {
    name: value.name.trim(),
    geometry: {
      contentScale: range("contentScale", 100, 50, 200),
      cropTop: range("cropTop", 0, 0, 40), cropRight: range("cropRight", 0, 0, 40),
      cropBottom: range("cropBottom", 0, 0, 40), cropLeft: range("cropLeft", 0, 0, 40),
      anchor: anchors.has(source.anchor) ? source.anchor : "legacy",
      marginX: range("marginX", 12, 0, 200), marginY: range("marginY", 16, 0, 200),
    },
    width: Math.round(Math.min(16384, Math.max(160, Number(value.width) || 640))),
    height: Math.round(Math.min(16384, Math.max(90, Number(value.height) || 360))),
    keepAspectRatio: value.keepAspectRatio !== false,
  };
}
export function readPresets(storage, key) {
  try {
    const list = JSON.parse(storage.getItem(key) || "[]");
    return Array.isArray(list) ? list.slice(0, 12).map(normalizePreset).filter(Boolean) : [];
  } catch { return []; }
}
export function initializePresetControls(card, target, { payload, persist, state, storage }) {
  const key = "relay-output-presets-" + target;
  const select = card.querySelector("[data-preset]");
  const name = card.querySelector("[data-preset-name]");
  const refresh = () => {
    select.replaceChildren(...readPresets(storage, key).map((preset) => {
      const option = document.createElement("option"); option.value = preset.name; option.textContent = preset.name; return option;
    }));
  };
  card.querySelector("[data-save-preset]").addEventListener("click", () => {
    try {
      const preset = normalizePreset({ ...payload(target), name: name.value });
      if (!preset) throw new Error("Enter a preset name (1–64 characters).");
      const list = readPresets(storage, key).filter((item) => item.name !== preset.name);
      if (list.length >= 12) throw new Error("A maximum of 12 presets can be saved per output.");
      storage.setItem(key, JSON.stringify([...list, preset])); refresh(); select.value = preset.name; state("saved");
    } catch (error) { state("error", String(error)); }
  });
  card.querySelector("[data-apply-preset]").addEventListener("click", async () => {
    const preset = readPresets(storage, key).find((item) => item.name === select.value);
    if (!preset) return;
    for (const input of card.querySelectorAll("[data-geometry-field]")) input.value = String(preset.geometry[input.dataset.geometryField]);
    card.querySelector("[data-output-anchor]").value = preset.geometry.anchor;
    for (const input of card.querySelectorAll("[data-size-field]")) input.value = String(preset[input.dataset.sizeField]);
    const ratio = card.querySelector("[data-keep-aspect-ratio]"); if (ratio) ratio.checked = preset.keepAspectRatio;
    await persist(target);
  });
  card.querySelector("[data-delete-preset]").addEventListener("click", () => {
    try { storage.setItem(key, JSON.stringify(readPresets(storage, key).filter((item) => item.name !== select.value))); refresh(); }
    catch (error) { state("error", String(error)); }
  });
  refresh();
}
