// Parsing helpers for the moderation and privacy filter fields.
// Pure functions: they only transform text and config values.

export function filterWordKey(value) {
  return String(value || "")
    .normalize("NFKC")
    .toLocaleLowerCase()
    .trim()
    .replace(/[\s._-]+/g, "");
}

export function filterConceptsToLines(concepts) {
  return (Array.isArray(concepts) ? concepts : [])
    .map((concept) => typeof concept?.canonical === "string" ? concept.canonical.trim() : "")
    .filter(Boolean)
    .join(", ");
}

export function filterRoleIdsToInput(roleIds) {
  return (Array.isArray(roleIds) ? roleIds : [])
    .filter((roleId) => typeof roleId === "string" && /^\d{17,20}$/.test(roleId))
    .join(", ");
}

export function filterRoleIds(value) {
  const seen = new Set();
  return String(value || "")
    .split(/[\r\n,]+/)
    .map((entry) => entry.trim())
    .map((entry) => entry.match(/^<@&(\d{17,20})>$/)?.[1] || entry)
    .reduce((roleIds, entry) => {
      if (entry && !seen.has(entry)) {
        seen.add(entry);
        roleIds.push(entry);
      }
      return roleIds;
    }, []);
}

export function filterWordsToConcepts(value, existingConcepts) {
  const existing = new Map(
    (Array.isArray(existingConcepts) ? existingConcepts : [])
      .filter((concept) => concept && typeof concept.canonical === "string")
      .map((concept) => [filterWordKey(concept.canonical), concept]),
  );
  const seen = new Set();
  const words = String(value || "")
    .split(/[\r\n,]+/)
    .map((word) => word.trim())
    .filter(Boolean);
  return words.reduce((concepts, word) => {
    const key = filterWordKey(word);
    if (!key || seen.has(key)) {
      return concepts;
    }
    seen.add(key);
    const previous = existing.get(key);
    concepts.push({
      canonical: word,
      aliases: Array.isArray(previous?.aliases)
        ? previous.aliases.filter((alias) => typeof alias === "string")
        : [],
      regexes: Array.isArray(previous?.regexes)
        ? previous.regexes.filter((pattern) => typeof pattern === "string")
        : [],
    });
    return concepts;
  }, []);
}

export function filterWordsAreSaveable(value) {
  return String(value || "")
    .split(/[\r\n,]+/)
    .map((word) => word.trim())
    .filter(Boolean)
    .every((word) => {
      const normalized = filterWordKey(word);
      return /\p{L}/u.test(word)
        && normalized.length >= 3
        && normalized.length <= 64;
    });
}

export function privacyListToInput(values) {
  return (Array.isArray(values) ? values : [])
    .filter((value) => typeof value === "string" && value.trim())
    .join("\n");
}

export function privacyListFromInput(value) {
  const seen = new Set();
  return String(value || "")
    .split(/[\r\n]+/)
    .map((entry) => entry.trim())
    .filter((entry) => {
      const key = entry.normalize("NFKC").toLocaleLowerCase();
      if (!entry || seen.has(key)) {
        return false;
      }
      seen.add(key);
      return true;
    });
}
