import { pageCacheUrl } from './page-cache-key.js';

export const ANALYSIS_CACHE_VERSION = 22;

/** One source tab + one document + one analysis scope; yt-dlp and gallery-dl
 * have independent results. Popup lifetime never participates in the key. */
export function analysisKeys({ sourceTabId, playlistScope, url }) {
  if (!Number.isInteger(sourceTabId) || !url) return null;
  const scope = playlistScope === 'playlist' ? 'playlist' : 'current';
  const base = `analysis:tab:${sourceTabId}:${scope}:${pageCacheUrl(url)}`;
  return { unified: base, y: `${base}:y`, g: `${base}:g` };
}

/** Source-document frames have a different schema from backend projections.
 * Compare before touching selections/settings or replacing a completed result. */
export function preferFreshUnifiedAnalysisResponse(live, incoming) {
  const valid = value => value?.ok && value.data?.request?.schemaVersion === ANALYSIS_CACHE_VERSION;
  if (!valid(live)) return incoming || live || null;
  if (!valid(incoming)) return live;
  const a = live.data.request, b = incoming.data.request;
  if (a.startedAt !== b.startedAt) return Number(a.startedAt || 0) > Number(b.startedAt || 0) ? live : incoming;
  if (a.updatedAt !== b.updatedAt) return Number(a.updatedAt || 0) > Number(b.updatedAt || 0) ? live : incoming;
  if (a.status !== 'analyzing' && b.status === 'analyzing') return live;
  return incoming;
}

export function isCurrentAnalysisResponse(value) {
  return Boolean(value?.ok && value.data?.schemaVersion === ANALYSIS_CACHE_VERSION
    && Array.isArray(value.data.items));
}

/** A popup may receive events after requesting its initial background snapshot.
 * Never let an older progress frame replace a completed frame, nor let the
 * snapshot of an earlier run replace the currently displayed run. */
export function preferFreshAnalysisResponse(live, restored) {
  if (!isCurrentAnalysisResponse(live)) return restored || live || null;
  if (!isCurrentAnalysisResponse(restored)) return live;
  const liveStarted = Number(live.data.startedAt || 0);
  const restoredStarted = Number(restored.data.startedAt || 0);
  if (liveStarted !== restoredStarted) return liveStarted > restoredStarted ? live : restored;
  if (live.data.analyzing !== restored.data.analyzing) {
    return live.data.analyzing ? restored : live;
  }
  const liveUpdated = Number(live.data.updatedAt || 0);
  const restoredUpdated = Number(restored.data.updatedAt || 0);
  if (liveUpdated !== restoredUpdated) return liveUpdated > restoredUpdated ? live : restored;
  return live.data.items.length >= restored.data.items.length ? live : restored;
}
