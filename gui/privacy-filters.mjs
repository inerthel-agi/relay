// Parsing helpers for the moderation and privacy filter fields.
// Pure functions: they only transform text and config values.

function filterWordKey(value) {
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

const ID_MENTIONS = { user: /^<@!?(\d+)>$/, role: /^<@&(\d+)>$/ };

/**
 * A list of Discord IDs typed by the user: raw IDs or mentions of the list's
 * kind ("user" or "role"), separated by spaces, commas, semicolons or lines.
 * A token is kept only when it is a whole ID; `invalid` lists the others and
 * `tooMany` says the list is over `max`. Duplicates are dropped.
 */
export function parseDiscordIdList(value, kind, max) {
  const ids = [];
  const invalid = [];
  for (const token of String(value || "").split(/[\s,;]+/).filter(Boolean)) {
    const id = token.match(ID_MENTIONS[kind])?.[1] ?? token;
    // Discord IDs are non-zero 64-bit numbers of 17 to 20 digits.
    const valid = /^\d{17,20}$/.test(id) && BigInt(id) > 0n && BigInt(id) < 2n ** 64n;
    if (!valid) invalid.push(token);
    else if (!ids.includes(id)) ids.push(id);
  }
  return { ids, invalid, tooMany: ids.length > max };
}

/** Role IDs of the filter exemption; an invalid list keeps the saved one. */
export function filterRoleIds(value, saved = []) {
  const { ids, invalid, tooMany } = parseDiscordIdList(value, "role", 100);
  return invalid.length || tooMany ? saved : ids;
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

/** Filter words Relay refuses: each needs a letter and 3 to 64 characters once normalized. */
export function refusedFilterWords(value) {
  return String(value || "")
    .split(/[\r\n,]+/)
    .map((word) => word.trim())
    .filter(Boolean)
    .filter((word) => {
      const normalized = filterWordKey(word);
      return !/\p{L}/u.test(word)
        || normalized.length < 3
        || normalized.length > 64;
    });
}

export function filterWordsAreSaveable(value) {
  return refusedFilterWords(value).length === 0;
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
