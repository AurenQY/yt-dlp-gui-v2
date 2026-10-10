/** Upstream URL-only source decisions. A failed/unsupported branch stays unknown. */
export function compileSourceBranches(index, compileRegex) {
  const rules = new Map(), regexes = [];
  function regex(pattern) {
    if (typeof pattern !== 'string') throw Error('Invalid source guard regex');
    const result = compileRegex(pattern.replaceAll('\\ufeff', '\\x{FEFF}')); regexes.push(result); return result;
  }
  function guard(value) {
    if (typeof value === 'string') return { regex: regex(value) };
    if (!Array.isArray(value) || !['q', 'g'].includes(value[0])) throw Error('Invalid source value guard');
    const offset = value[0] === 'q' ? 2 : 3;
    const patterns = offset === 3 ? (Array.isArray(value[1]) ? value[1] : [value[1]]) : [];
    if (![offset, offset + 1].includes(value.length)
        || (offset === 2 ? typeof value[1] !== 'string' : !patterns.length || patterns.some(pattern => typeof pattern !== 'string'))
        || (offset === 3 && typeof value[2] !== 'string')
        || (value.length > offset && value[offset] !== null && typeof value[offset] !== 'string')) throw Error('Invalid source value guard');
    return { query: offset === 2 ? value[1] : undefined, captures: patterns.map(regex),
      group: offset === 3 ? value[2] : undefined, equality: value.length > offset, expected: value[offset] };
  }
  function compile(value, depth = 0) {
    if (depth > 32) throw Error('Source branch depth exceeded');
    if (Number.isInteger(value) && value >= 0 && value <= 2) return value;
    if (!Array.isArray(value) || value.length !== 4 || value[0] !== 3) throw Error('Invalid source branch');
    return [guard(value[1]), compile(value[2], depth + 1), compile(value[3], depth + 1)];
  }
  try {
    for (const row of index.yc || []) {
      if (!Array.isArray(row) || row.length !== 2 || !Number.isInteger(row[0]) || row[0] < 0 || row[0] >= index.y.length || rules.has(row[0])) throw Error('Invalid source branch index');
      rules.set(row[0], compile(row[1]));
    }
  } catch (error) { regexes.forEach(regex => regex.destroy()); throw error; }
  return { rules, destroy() { regexes.forEach(regex => regex.destroy()); } };
}

export function evaluateSourceBranch(tree, url) {
  if (Number.isInteger(tree)) return ['unknown', 'single', 'playlist'][tree];
  const guard = tree[0]; let matched;
  if (guard.query !== undefined || guard.group !== undefined) {
    let value;
    if (guard.query !== undefined) value = new URL(url).searchParams.getAll(guard.query).find(value => value !== '') ?? null;
    else {
      let match = null;
      for (const regex of guard.captures) {
        match = regex.match(url, { matchLimit: 10000, depthLimit: 500 });
        if (match) break;
      }
      if (match && !Object.hasOwn(match.namedGroups || {}, guard.group)) throw Error('Missing source capture name');
      value = match?.namedGroups?.[guard.group] ?? null;
    }
    matched = guard.equality ? value === guard.expected : Boolean(value);
  } else matched = guard.regex.test(url, { matchLimit: 10000, depthLimit: 500 });
  return evaluateSourceBranch(matched ? tree[1] : tree[2], url);
}
