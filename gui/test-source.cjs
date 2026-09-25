const fs = require("node:fs");
// Modules split out of panel.js. Tests slice functions by name, so every panel
// module is concatenated ahead of panel.js to keep those slices resolvable.
const panelModules = ["privacy-filters.mjs", "changelog-markdown.mjs", "diagnostics.mjs", "custom-commands.mjs", "overview.mjs", "settings-search.mjs", "autosave.mjs"];
exports.panelSource = () => ["translations.mjs", ...panelModules, "panel.js"].map((file) =>
  fs.readFileSync(__dirname + "/" + file, "utf8").replace(/^const \{ invoke \} =.*$/gm, "").replace(/^import .*;\r?$/gm, "").replace(/^export \{.*\};\r?$/gm, "").replace(/^export (?=(?:async )?function |const |let |class )/gm, "")
    .replace("for (const [locale, dictionary] of Object.entries(translations))", () => fs.readFileSync(__dirname + "/module-translations.mjs", "utf8").replace("export const", "const") + "\nfor (const [locale, dictionary] of Object.entries(translations))")
).join("\n");

exports.panelStyles = () => ["panel-fonts.css", "panel.css"].map((file) => fs.readFileSync(__dirname + "/" + file, "utf8")).join("\n");
