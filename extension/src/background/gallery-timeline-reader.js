import { galleryPostGroups, galleryAlbumPayload } from '../shared/gallery-post-groups.js';
import { t } from '../shared/i18n.js';
import { recordDiagnostic } from '../shared/operation-diagnostics.js';
import { timelineRange } from '../shared/gallery-timeline-range.js';

function postsFor(rows, request) {
  const groups = galleryPostGroups(rows, request.url);
  if (!groups.length) return [];
  const album = galleryAlbumPayload(groups, request.siteName, request.url);
  return album.posts || [album];
}

/** A separate native port: never replaces the popup analysis or download task. */
export function createTimelineReader(prepareCookies) {
  return async function readTimeline(request, signal, onPage = () => {}) {
    signal?.throwIfAborted();
    const prepared = await prepareCookies({ action: 'galleryRead', backend: 'g', url: request.url,
      sourceTabId: request.sourceTabId, timelineRange: timelineRange(request.timelineRange), postStart: request.postStart || 1,
      ...(request.postCount != null ? { postCount: request.postCount } : {}) });
    signal?.throwIfAborted();
    return new Promise((resolve, reject) => {
      const startedAt = Date.now();
      recordDiagnostic('timeline.start', { traceId: prepared.traceId, nextStart: request.postStart || 1, postCount: request.postCount });
      const port = chrome.runtime.connectNative('kannagi0303.yt_dlp_gui');
      const rows = [];
      let settled = false, closing = false;
      let pending = Promise.resolve(), cursor = request.postStart || 1;
      const finish = (error, data) => {
        if (settled) return;
        settled = true; closing = true;
        recordDiagnostic('timeline.finish', { traceId: prepared.traceId, elapsedMs: Date.now() - startedAt, status: signal?.aborted ? 'cancelled' : error ? 'failed' : 'completed', nextStart: cursor, itemCount: rows.length });
        signal?.removeEventListener('abort', abort);
        port.disconnect();
        if (error) reject(error); else resolve(data);
      };
      const close = (error, data) => {
        if (closing) return;
        closing = true;
        pending.then(() => finish(error, data), error => finish(error));
      };
      const abort = () => finish(signal.reason);
      signal?.addEventListener('abort', abort, { once: true });
      port.onMessage.addListener(response => {
        if (closing) return;
        if (!response?.ok) { close(Error(response?.message || t('timelineReadFailed'))); return; }
        const data = response.data;
        try {
          if (data?.event === 'item') rows.push(data.item);
          if (data?.event === 'post') {
            if (!Array.isArray(data.items) || data.nextStart !== cursor + 1) throw Error('Non-contiguous timeline cursor.');
            cursor = data.nextStart; rows.push(...data.items);
            recordDiagnostic('timeline.post', { traceId: prepared.traceId, nextStart: cursor, itemCount: data.items.length });
            const page = { posts: postsFor(data.items, request), timelineRange: timelineRange(data.timelineRange), nextStart: cursor, hasMore: true };
            pending = pending.then(() => { if (!settled) return onPage(page); });
            pending.catch(error => finish(error));
          }
          if (data?.event === 'complete') {
            if (!Number.isInteger(data.nextStart) || data.nextStart < cursor || typeof data.hasMore !== 'boolean') throw Error('Incomplete timeline completion response.');
            close(null, { posts: postsFor(rows, request), timelineRange: timelineRange(data.timelineRange), nextStart: data.nextStart, hasMore: data.hasMore });
          }
          if (data?.event === 'stopped') close(Error(t('analysisStoppedStatus')));
        } catch (error) { close(error); }
      });
      port.onDisconnect.addListener(() => { if (!closing) close(Error(chrome.runtime.lastError?.message || t('nativeDisconnected'))); });
      port.postMessage(prepared);
      if (signal?.aborted) abort();
    });
  };
}
