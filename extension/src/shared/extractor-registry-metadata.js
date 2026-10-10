/** Stable upstream metadata belongs to registry classes, not UI display names. */
export function registryMetadata(index, tool) {
  const records = index.meta?.[tool] || [];
  if (!Array.isArray(records)) throw Error('Invalid extractor registry metadata');
  const rules = new Map(), keys = new Set();
  for (const record of records) {
    if (!record || typeof record.key !== 'string' || !record.key || keys.has(record.key)
        || typeof record.class !== 'string' || typeof record.module !== 'string'
        || !Number.isInteger(record.order) || record.order < 0
        || !Array.isArray(record.rules) || !Number.isInteger(record.scope) || record.scope < 0 || record.scope > 3
        || !Number.isInteger(record.declaredScope) || record.declaredScope < 0 || record.declaredScope > 3) throw Error('Invalid extractor registry metadata');
    keys.add(record.key);
    if (record.working != null && typeof record.working !== 'boolean') throw Error('Invalid extractor working declaration');
    for (const rule of record.rules) {
      if (!Number.isInteger(rule) || rule < 0 || rule >= index[tool].length || rules.has(rule)) throw Error('Invalid extractor metadata rule');
      rules.set(rule, record);
    }
  }
  if (records.length && rules.size !== index[tool].length) throw Error('Incomplete extractor registry metadata');
  return rules;
}
