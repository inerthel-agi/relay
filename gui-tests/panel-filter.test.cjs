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
