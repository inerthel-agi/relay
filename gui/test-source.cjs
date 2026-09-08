const fs = require("node:fs");
exports.panelSource = () => ["translations.mjs", "panel.js"].map((file) =>
  fs.readFileSync(__dirname + "/" + file, "utf8").replace(/^const \{ invoke \} =.*$/gm, "").replace(/^import .*;\r?$/gm, "").replace(/^export \{.*\};\r?$/gm, "")
    .replace("for (const [locale, dictionary] of Object.entries(translations))", () => fs.readFileSync(__dirname + "/module-translations.mjs", "utf8").replace("export const", "const") + "\nfor (const [locale, dictionary] of Object.entries(translations))")
).join("\n");

exports.panelStyles = () => ["panel-fonts.css", "panel.css"].map((file) => fs.readFileSync(__dirname + "/" + file, "utf8")).join("\n");
