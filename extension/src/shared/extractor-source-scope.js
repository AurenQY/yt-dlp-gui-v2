/** Sparse source evidence uses original rule positions, never display names. */
export function ytSourceScopes(index, tool = 'y') {
  const names = ['unknown', 'single', 'playlist', 'any'];
  const valid = value => Number.isInteger(value) && value >= 0 && value < names.length;
  const fallback = index[`${tool}d`] ?? 0, exceptions = index[`${tool}s`];
  if (!valid(fallback) || (exceptions != null && !Array.isArray(exceptions))) throw Error('Invalid source scope index');
  const scopes = Array(index[tool].length).fill(names[fallback]);
  const seen = new Set();
  for (const row of exceptions || []) {
    if (!Array.isArray(row) || row.length !== 2 || !Number.isInteger(row[0])
        || row[0] < 0 || row[0] >= scopes.length || seen.has(row[0]) || !valid(row[1])) throw Error('Invalid source scope index');
    seen.add(row[0]);
    scopes[row[0]] = names[row[1]];
  }
  return scopes;
}
