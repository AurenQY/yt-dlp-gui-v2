/** Bounded, fixed-English diagnostic journal. Never store requests or raw errors. */
const KEY = 'operationDiagnostics';
let writes = Promise.resolve();
export function operationTraceId() { return crypto.randomUUID(); }

export function recordDiagnostic(event, fields = {}) {
  if (typeof event !== 'string' || !/^[a-z.-]{1,80}$/.test(event)) return writes;
  const safe = {};
  for (const key of ['traceId', 'parentTraceId', 'backend', 'phase', 'status']) {
    const value = fields[key];
    if (typeof value === 'string' && /^[A-Za-z0-9_-]{1,80}$/.test(value)) safe[key] = value;
  }
  for (const key of ['elapsedMs', 'cookieCount', 'itemCount', 'postCount', 'nextStart', 'completedItems']) {
    if (Number.isFinite(fields[key]) && fields[key] >= 0) safe[key] = fields[key];
  }
  const row = { timestampMs: Date.now(), event, ...safe };
  writes = writes.catch(() => {}).then(async () => {
    const saved = await chrome.storage.local.get(KEY);
    const rows = Array.isArray(saved[KEY]) ? saved[KEY] : [];
    await chrome.storage.local.set({ [KEY]: [...rows.slice(-999), row] });
  }).catch(() => console.warn('[operation-diagnostics] journal write unavailable'));
  return writes;
}
export async function diagnosticJournal() {
  await writes;
  return (await chrome.storage.local.get(KEY))[KEY] || [];
}
