/** Portable upstream suitable() IR; unsupported expressions remain unknown. */
export function suitabilityRules(index) {
  const rules = new Map();
  function validate(p, depth = 0) {
    if (!Array.isArray(p) || depth > 64) throw Error('Invalid suitability predicate');
    const op = p[0];
    if (op === 0 && p.length === 1) return;
    if (op === 1 && p.length === 2 && Number.isInteger(p[1]) && p[1] >= 0 && p[1] < index.y.length) return;
    if (op === 5 && p.length === 2 && typeof p[1] === 'string') return;
    if (op === 6 && p.length === 2 && typeof p[1] === 'boolean') return;
    if ((op === 4 && p.length === 2) || ([2, 3].includes(op) && p.length >= 3)) {
      p.slice(1).forEach(child => validate(child, depth + 1)); return;
    }
    throw Error('Invalid suitability predicate');
  }
  for (const row of index.yp || []) {
    if (!Array.isArray(row) || row.length !== 2 || !Number.isInteger(row[0]) || row[0] < 0 || row[0] >= index.y.length || rules.has(row[0])) throw Error('Invalid suitability rule');
    validate(row[1]); rules.set(row[0], row[1]);
  }
  return rules;
}

export function evaluateSuitability(p, match, url) {
  switch (p[0]) {
    case 0: return null;
    case 1: return match(p[1]);
    case 5: return new URL(url).searchParams.getAll(p[1]).some(value => value !== '');
    case 6: return p[1];
    case 4: { const value = evaluateSuitability(p[1], match, url); return value == null ? null : !value; }
    case 2: { const values = p.slice(1).map(v => evaluateSuitability(v, match, url)); return values.includes(false) ? false : values.includes(null) ? null : true; }
    case 3: { const values = p.slice(1).map(v => evaluateSuitability(v, match, url)); return values.includes(true) ? true : values.includes(null) ? null : false; }
    default: return null;
  }
}

export function conditionalScopes(index) {
  const rules = new Map();
  const seen = new Set();
  for (const row of index.yu || []) {
    if (!Array.isArray(row) || row.length !== 3 || !Number.isInteger(row[0]) || row[0] < 0 || row[0] >= index.y.length || typeof row[1] !== 'string' || row[2] !== 2 || seen.has(JSON.stringify(row.slice(0, 2)))) throw Error('Invalid conditional scope');
    seen.add(JSON.stringify(row.slice(0, 2)));
    rules.set(row[0], [...(rules.get(row[0]) || []), row[1]]);
  }
  return rules;
}
