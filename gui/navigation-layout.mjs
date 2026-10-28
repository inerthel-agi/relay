// Sidebar layout: foldable groups and the Simple / Advanced interface mode.
// Simple only hides what is marked data-advanced; settings keep their values.

export const INTERFACE_MODES = ["simple", "advanced"];
const MODE_KEY = "relay-interface-mode";
const FOLDED_KEY = "relay-folded-nav-groups";

/** Stored mode, or "simple" for a first launch or an unknown value. */
export function readInterfaceMode(raw) {
  return INTERFACE_MODES.includes(raw) ? raw : "simple";
}

/** Folded group names from storage; anything malformed means nothing folded. */
export function readFoldedGroups(raw, knownGroups) {
  try {
    const value = JSON.parse(raw || "[]");
    return Array.isArray(value) ? value.filter((name) => knownGroups.includes(name)) : [];
  } catch {
    return [];
  }
}

/** The page to show after a mode change: Simple never leaves an advanced page open. */
export function pageForMode(mode, currentPage, advancedPages) {
  return mode === "simple" && advancedPages.includes(currentPage) ? "overview" : currentPage;
}

export function initializeNavigationLayout({ t, showPage, getCurrentPage, doc = document, storage = localStorage }) {
  const root = doc.documentElement;
  const nav = doc.querySelector(".navigation");
  const groups = [...nav.querySelectorAll("[data-nav-group]")];
  // The root carries data-ui-mode, so this selector cannot match it.
  const modeButtons = [...doc.querySelectorAll(".interface-mode__button[data-interface-mode]")];
  const hint = doc.querySelector("#interface-mode-hint");
  const read = (key) => { try { return storage.getItem(key); } catch { return null; } };
  const write = (key, value) => { try { storage.setItem(key, value); } catch { /* Storage is optional. */ } };

  let folded = readFoldedGroups(read(FOLDED_KEY), groups.map((group) => group.dataset.navGroup));
  function renderGroups() {
    for (const group of groups) {
      const isFolded = folded.includes(group.dataset.navGroup);
      group.setAttribute("aria-expanded", String(!isFolded));
      for (let item = group.nextElementSibling; item && !item.hasAttribute("data-nav-group"); item = item.nextElementSibling) {
        item.toggleAttribute("data-folded", isFolded);
      }
    }
  }
  for (const group of groups) {
    group.addEventListener("click", () => {
      const name = group.dataset.navGroup;
      folded = folded.includes(name) ? folded.filter((other) => other !== name) : [...folded, name];
      write(FOLDED_KEY, JSON.stringify(folded));
      renderGroups();
    });
  }

  const advancedPages = [...nav.querySelectorAll("[data-page-target][data-advanced]")].map((item) => item.dataset.pageTarget);
  let mode = readInterfaceMode(read(MODE_KEY));
  function renderMode() {
    root.dataset.uiMode = mode;
    for (const button of modeButtons) button.setAttribute("aria-pressed", String(button.dataset.interfaceMode === mode));
    // Compact sidebars show one letter of the translated name.
    for (const button of modeButtons) button.dataset.short = [...t(button.querySelector("[data-i18n]").dataset.i18n)][0]?.toUpperCase() || "";
    hint.dataset.i18n = mode === "simple" ? "interfaceModeSimpleHint" : "interfaceModeAdvancedHint";
    hint.textContent = t(hint.dataset.i18n);
  }
  function setMode(next) {
    mode = readInterfaceMode(next);
    write(MODE_KEY, mode);
    renderMode();
    const page = pageForMode(mode, getCurrentPage(), advancedPages);
    if (page !== getCurrentPage()) showPage(page);
  }
  for (const button of modeButtons) button.addEventListener("click", () => setMode(button.dataset.interfaceMode));

  renderGroups();
  renderMode();
  return { setMode, getMode: () => mode, advancedPages, applyLanguage: renderMode };
}
