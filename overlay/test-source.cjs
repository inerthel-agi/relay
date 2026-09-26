const fs = require("node:fs");
exports.styles = () => ["overlay.css", "audio-card.css"].map((file) => fs.readFileSync(__dirname + "/" + file, "utf8")).join("\n");
