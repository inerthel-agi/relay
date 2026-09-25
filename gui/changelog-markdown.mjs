// Splits CHANGELOG.md into releases and renders a release body as DOM nodes.

export function parseChangelogReleases(markdown) {
  const releases = [];
  let current;
  const flush = () => {
    if (!current) return;
    const body = current.lines.join("\n").trim();
    if (body) {
      releases.push({ version: current.version, date: current.date, body });
    }
    current = undefined;
  };
  for (const line of String(markdown).split(/\r?\n/)) {
    if (line.startsWith("## [")) {
      flush();
      const match = line.match(/^## \[([^\]]+)\](?:\s*-\s*(.+))?$/);
      if (!match || match[1].trim().toLowerCase() === "unreleased") continue;
      current = {
        version: match[1].trim(),
        date: match[2]?.trim() || null,
        lines: [],
      };
      continue;
    }
    if (line.startsWith("[") && line.includes("]: http")) continue;
    current?.lines.push(line);
  }
  flush();
  return releases;
}

export function changelogBodyForLanguage(body, languageCode) {
  const buckets = { default: [] };
  let current = "default";
  for (const line of String(body).split(/\r?\n/)) {
    const heading = line.match(/^###\s+(.+)$/);
    if (heading) {
      current = heading[1].trim().toLowerCase();
      buckets[current] ??= [];
      continue;
    }
    buckets[current].push(line);
  }
  const aliases = {
    en: ["english"],
    fr: ["français", "francais"],
    es: ["español", "espanol", "spanish"],
    de: ["deutsch", "german"],
    ru: ["русский", "russian"],
    zh: ["简体中文", "chinese"],
    ko: ["한국어", "korean"],
    ja: ["日本語", "japanese"],
    id: ["bahasa indonesia", "indonesian"],
  };
  const preferred = [...(aliases[languageCode] || aliases.en)];
  if (languageCode !== "en") preferred.push("english");
  for (const key of preferred) {
    const text = (buckets[key] || []).join("\n").trim();
    if (text) return text;
  }
  return String(body).trim();
}

function appendInlineChangelogText(parent, text) {
  const parts = String(text).split(/(\*\*[^*]+\*\*|`[^`]+`)/g);
  for (const part of parts) {
    if (part.startsWith("**") && part.endsWith("**") && part.length > 4) {
      const strong = document.createElement("strong");
      strong.textContent = part.slice(2, -2);
      parent.append(strong);
      continue;
    }
    if (part.startsWith("`") && part.endsWith("`") && part.length > 2) {
      const code = document.createElement("code");
      code.textContent = part.slice(1, -1);
      parent.append(code);
      continue;
    }
    parent.append(document.createTextNode(part));
  }
}

export function appendChangelogMarkdown(parent, markdown) {
  let list;
  const closeList = () => {
    list = undefined;
  };
  for (const line of String(markdown).split(/\r?\n/)) {
    if (line.startsWith("#### ")) {
      closeList();
      const heading = document.createElement("h4");
      heading.textContent = line.slice(5).trim();
      parent.append(heading);
      continue;
    }
    if (line.startsWith("### ")) {
      closeList();
      const heading = document.createElement("h3");
      heading.textContent = line.slice(4).trim();
      parent.append(heading);
      continue;
    }
    if (line.startsWith("- ")) {
      if (!list) {
        list = document.createElement("ul");
        parent.append(list);
      }
      const item = document.createElement("li");
      appendInlineChangelogText(item, line.slice(2));
      list.append(item);
      continue;
    }
    if (!line.trim()) {
      closeList();
      continue;
    }
    closeList();
    const paragraph = document.createElement("p");
    appendInlineChangelogText(paragraph, line);
    parent.append(paragraph);
  }
}
