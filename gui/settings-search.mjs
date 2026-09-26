// Settings search in the top bar, plus the reveal helper shared by internal links.

export function initializeSettingsSearch({ $, $$, t, pageMetadata, showPage, getLocale }) {
  const settingsSearchControl = $("#settings-search-control");
  const settingsSearchElement = $("#settings-search");
  const settingsSearchClearButton = $("#settings-search-clear");
  const settingsSearchResultsElement = $("#settings-search-results");
  let settingsSearchIndex = [];
  let settingsSearchHighlightTimer;

  function normalizeSettingsSearch(value) {
    return String(value)
      .normalize("NFKD")
      .replace(/\p{Mark}/gu, "")
      .toLocaleLowerCase(getLocale());
  }

  function buildSettingsSearchIndex() {
    const seen = new Set();
    settingsSearchIndex = $$('[data-page] [data-i18n]').flatMap((element) => {
      const page = element.closest("[data-page]")?.dataset.page;
      const key = element.dataset.i18n;
      const identity = `${page}:${key}`;
      if (!pageMetadata[page] || seen.has(identity)) return [];
      seen.add(identity);
      const label = t(key);
      const pageLabel = t(pageMetadata[page].title);
      const target = element.closest("label, .setting-row, details, fieldset, .panel-section, .help-step") || element;
      return [{
        label, page, pageLabel, target,
        searchable: normalizeSettingsSearch(`${label} ${pageLabel}`),
      }];
    });
  }

  function closeSettingsSearch() {
    settingsSearchResultsElement.hidden = true;
    settingsSearchElement.setAttribute("aria-expanded", "false");
  }

  function openSettingsSearchResult(entry) {
    settingsSearchElement.value = "";
    settingsSearchClearButton.hidden = true;
    closeSettingsSearch();
    showPage(entry.page, { moveFocus: true });
    revealElement(entry.target);
  }

  // Opens every enclosing disclosure, then scrolls to and briefly highlights the target.
  function revealElement(target) {
    if (!target) return;
    if (target.tagName === "DETAILS") target.open = true;
    for (let parent = target.parentElement?.closest("details"); parent; parent = parent.parentElement?.closest("details")) {
      parent.open = true;
    }
    window.requestAnimationFrame(() => {
      target.scrollIntoView({ behavior: "smooth", block: "center" });
      window.clearTimeout(settingsSearchHighlightTimer);
      target.classList.remove("settings-search-target");
      window.requestAnimationFrame(() => target.classList.add("settings-search-target"));
      settingsSearchHighlightTimer = window.setTimeout(
        () => target.classList.remove("settings-search-target"),
        1400,
      );
    });
  }

  function renderSettingsSearchResults() {
    const query = normalizeSettingsSearch(settingsSearchElement.value.trim());
    settingsSearchClearButton.hidden = !query;
    settingsSearchResultsElement.replaceChildren();
    if (!query) {
      closeSettingsSearch();
      return;
    }
    const terms = query.split(/\s+/).filter(Boolean);
    const results = settingsSearchIndex
      .filter((entry) => terms.every((term) => entry.searchable.includes(term)))
      .sort((left, right) => {
        const leftStarts = normalizeSettingsSearch(left.label).startsWith(query);
        const rightStarts = normalizeSettingsSearch(right.label).startsWith(query);
        return Number(rightStarts) - Number(leftStarts) || left.label.localeCompare(right.label, getLocale());
      })
      .slice(0, 8);
    settingsSearchResultsElement.hidden = false;
    settingsSearchElement.setAttribute("aria-expanded", "true");
    if (!results.length) {
      const empty = document.createElement("p");
      empty.className = "settings-search__empty";
      empty.textContent = t("searchNoResults");
      settingsSearchResultsElement.append(empty);
      return;
    }
    for (const entry of results) {
      const button = document.createElement("button");
      button.className = "settings-search__result";
      button.type = "button";
      button.setAttribute("role", "option");
      const label = document.createElement("strong");
      label.textContent = entry.label;
      const page = document.createElement("small");
      page.textContent = entry.pageLabel;
      button.append(label, page);
      button.addEventListener("click", () => openSettingsSearchResult(entry));
      settingsSearchResultsElement.append(button);
    }
  }

  settingsSearchElement.addEventListener("input", renderSettingsSearchResults);
  settingsSearchElement.addEventListener("focus", () => {
    buildSettingsSearchIndex();
    renderSettingsSearchResults();
  });
  settingsSearchElement.addEventListener("keydown", (event) => {
    const results = $$(".settings-search__result", settingsSearchResultsElement);
    if (event.key === "ArrowDown" && results.length) {
      event.preventDefault();
      results[0].focus();
    }
  });
  settingsSearchResultsElement.addEventListener("keydown", (event) => {
    const results = $$(".settings-search__result", settingsSearchResultsElement);
    const index = results.indexOf(document.activeElement);
    if (event.key === "ArrowDown" && index < results.length - 1) {
      event.preventDefault();
      results[index + 1].focus();
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      if (index > 0) results[index - 1].focus();
      else settingsSearchElement.focus();
    }
  });
  settingsSearchClearButton.addEventListener("click", () => {
    settingsSearchElement.value = "";
    renderSettingsSearchResults();
    settingsSearchElement.focus();
  });

  return {
    reveal: revealElement,
    applyLanguage() {
      settingsSearchElement.setAttribute("aria-label", t("searchLabel"));
      settingsSearchClearButton.title = t("clearSearch");
      settingsSearchClearButton.setAttribute("aria-label", t("clearSearch"));
      buildSettingsSearchIndex();
      renderSettingsSearchResults();
    },
    closeIfOutside(target) {
      if (!settingsSearchResultsElement.hidden && !settingsSearchControl.contains(target)) closeSettingsSearch();
    },
    closeAndFocus() {
      if (settingsSearchResultsElement.hidden) return;
      closeSettingsSearch();
      settingsSearchElement.focus();
    },
    focus() {
      settingsSearchElement.focus();
      settingsSearchElement.select();
    },
  };
}
