/** Publish each complete post immediately and preserve ordered UI persistence.
 * Abort/disposal closes only this album's port and native read. */
import { t } from '../shared/i18n.js';
export function readAlbumTimeline(request, signal, onPage) {
  signal?.throwIfAborted();
  return new Promise((resolve, reject) => {
    const port = chrome.runtime.connect({ name: 'yt-dlp-gui-timeline-browse' });
    let settled = false, closing = false, pending = Promise.resolve();
    const finish = (error, page) => {
      if (settled) return;
      settled = true; closing = true;
      signal?.removeEventListener('abort', abort);
      port.disconnect();
      if (error) reject(error); else resolve(page);
    };
    const close = (error, page) => {
      if (closing) return;
      closing = true;
      pending.then(() => finish(error, page), error => finish(error));
    };
    const abort = () => finish(signal.reason);
    signal?.addEventListener('abort', abort, { once: true });
    port.onMessage.addListener(message => {
      if (closing) return;
      if (message?.event === 'post') {
        pending = pending.then(() => { if (!settled) return onPage(message.page); });
        pending.catch(error => finish(error));
      } else if (message?.event === 'complete') close(null, message.page);
      else if (message?.event === 'error') close(Error(message.message || t('timelineReadFailed')));
    });
    port.onDisconnect.addListener(() => { if (!closing) close(Error(chrome.runtime.lastError?.message || t('nativeDisconnected'))); });
    port.postMessage({ request });
    if (signal?.aborted) abort();
  });
}
