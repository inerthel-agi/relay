const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

const panelSource = require("./test-source.cjs").panelSource();
const filterSource = panelSource.slice(
  panelSource.indexOf("function filterWordKey"),
  panelSource.indexOf("function applyConfig"),
);

function filters() {
  const context = vm.createContext({ Map, Set, String });
  vm.runInContext(
    `${filterSource}\nglobalThis.filters = { filterWordsToConcepts, filterWordsAreSaveable, privacyListFromInput };`,
    context,
  );
  return context.filters;
}

test("automatic filter words accept completed comma-separated values", () => {
  const { filterWordsAreSaveable, filterWordsToConcepts } = filters();
  assert.equal(filterWordsAreSaveable("fdp, hitler"), true);
  assert.deepEqual(
    JSON.parse(JSON.stringify(filterWordsToConcepts("fdp, hitler", []))),
    [
      { canonical: "fdp", aliases: [], regexes: [] },
      { canonical: "hitler", aliases: [], regexes: [] },
    ],
  );
});

test("private lists preserve values without logging or comma splitting", () => {
  const { privacyListFromInput } = filters();
  assert.deepEqual(
    JSON.parse(JSON.stringify(privacyListFromInput("Old Alias\n12 rue Example, Paris\nold alias"))),
    ["Old Alias", "12 rue Example, Paris"],
  );
  assert.match(panelSource, /privacyAllowlist: privacyListFromInput/);
  assert.match(panelSource, /privacyCustomPatterns: privacyListFromInput/);
});

test("automatic filter words wait for incomplete values", () => {
  const { filterWordsAreSaveable } = filters();
  assert.equal(filterWordsAreSaveable("f"), false);
  assert.equal(filterWordsAreSaveable("123"), false);
  assert.equal(filterWordsAreSaveable(""), true);
  assert.match(panelSource, /privacyConceptsElement\.addEventListener\("input", schedulePrivacyFilterSave\)/);
});
