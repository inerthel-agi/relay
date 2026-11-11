const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const panelSource = require("./test-source.cjs").panelSource();
let loaded;
async function filters() {
  loaded ??= await import("../gui/privacy-filters.mjs");
  return loaded;
}

test("automatic filter words accept completed comma-separated values", async () => {
  const { filterWordsAreSaveable, filterWordsToConcepts } = await filters();
  assert.equal(filterWordsAreSaveable("fdp, hitler"), true);
  assert.deepEqual(
    JSON.parse(JSON.stringify(filterWordsToConcepts("fdp, hitler", []))),
    [
      { canonical: "fdp", aliases: [], regexes: [] },
      { canonical: "hitler", aliases: [], regexes: [] },
    ],
  );
});

test("private lists preserve values without logging or comma splitting", async () => {
  const { privacyListFromInput } = await filters();
  assert.deepEqual(
    JSON.parse(JSON.stringify(privacyListFromInput("Old Alias\n12 rue Example, Paris\nold alias"))),
    ["Old Alias", "12 rue Example, Paris"],
  );
  assert.match(panelSource, /privacyAllowlist: privacyListFromInput/);
  assert.match(panelSource, /privacyCustomPatterns: privacyListFromInput/);
});

test("automatic filter words wait for incomplete values", async () => {
  const { filterWordsAreSaveable } = await filters();
  assert.equal(filterWordsAreSaveable("f"), false);
  assert.equal(filterWordsAreSaveable("123"), false);
  assert.equal(filterWordsAreSaveable(""), true);
  assert.match(panelSource, /privacyConceptsElement\.addEventListener\("input", schedulePrivacyFilterSave\)/);
});

test("a refused filter word is named under the field and nothing is saved until it is fixed", async () => {
  const { refusedFilterWords } = await filters();
  assert.deepEqual(refusedFilterWords("ab, hitler, 123,"), ["ab", "123"]);
  const vm = require("node:vm");
  const panel = fs.readFileSync(__dirname + "/../gui/panel.js", "utf8");
  const slice = panel.slice(panel.indexOf("function renderRefusedFilterWords"), panel.indexOf("let statusRefreshInFlight"));
  const saves = [];
  const hint = { hidden: true, textContent: "" };
  const context = vm.createContext({
    refusedFilterWords, privacyConceptsRefusedElement: hint, privacyFilterSaveGeneration: 1,
    bootstrap: { config: {} }, privacyFilterDraft: "ab, hitler", moderationSaveStateElement: {}, moderationForm: {},
    formatTranslation: (key, values) => `${key}:${values.words}`,
    saveConfig: async (...args) => { saves.push(args); return true; },
  });
  vm.runInContext(slice, context);
  await context.savePrivacyFiltersAutomatically(1);
  assert.equal(hint.hidden, false);
  assert.equal(hint.textContent, "filterWordsRefused:ab");
  assert.equal(saves.length, 0);
  context.privacyFilterDraft = "abc, hitler";
  await context.savePrivacyFiltersAutomatically(1);
  assert.equal(hint.hidden, true);
  assert.equal(saves.length, 1);
});
