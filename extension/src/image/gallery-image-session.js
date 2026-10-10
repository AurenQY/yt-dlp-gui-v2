import { prepareGalleryImage } from './gallery-image-processor.js';

/** Per-album, bounded LRU of processed blobs. Limits are memory/concurrency
 * budgets, not a restriction on album size or number of downloadable files. */
export function createGalleryImageSession() {
  const entries = new Map();
  // Independent of the blob LRU and Svelte row remounts: reveal each item once.
  const displayed = new Set();
  const itemKey = item => JSON.stringify([item.url, item.archiveKey, item.canonicalMetadata]);
  const waiting = [];
  const running = new Set();
  let disposed = false;
  let retained = 0;
  const budget = 64 * 1024 * 1024;

  function evict() {
    for (const [key, entry] of entries) {
      if (retained <= budget) break;
      if (!entry.value) continue;
      retained -= entry.value.blob.size;
      entries.delete(key);
    }
  }
  function pump() {
    while (!disposed && running.size < 2 && waiting.length) {
      const job = waiting.shift();
      if (job.signal?.aborted) { entries.delete(job.key); job.reject(job.signal.reason); continue; }
      const aborter = new AbortController();
      const abort = () => aborter.abort(job.signal.reason);
      job.signal?.addEventListener('abort', abort, { once: true });
      running.add(aborter);
      prepareGalleryImage(job.item, { signal: aborter.signal })
        .then(result => {
          if (disposed) throw new DOMException('Album closed', 'AbortError');
          const value = { blob: new Blob([result.bytes], { type: result.mime }), xmp: result.xmp };
          const entry = entries.get(job.key);
          if (entry) { entry.value = value; retained += value.blob.size; evict(); }
          job.resolve(value);
        })
        .catch(error => { entries.delete(job.key); job.reject(error); })
        .finally(() => { job.signal?.removeEventListener('abort', abort); running.delete(aborter); pump(); });
    }
  }
  function prepare(item, { signal } = {}) {
    if (disposed) return Promise.reject(new DOMException('Album closed', 'AbortError'));
    if (signal?.aborted) return Promise.reject(signal.reason);
    const key = itemKey(item);
    let entry = entries.get(key);
    if (entry) { entries.delete(key); entries.set(key, entry); return abortable(entry.promise, signal); }
    entry = {};
    entry.promise = new Promise((resolve, reject) => waiting.push({ item, key, signal, resolve, reject }));
    entries.set(key, entry);
    pump();
    return abortable(entry.promise, signal);
  }
  function abortable(promise, signal) {
    if (!signal) return promise;
    return new Promise((resolve, reject) => {
      const abort = () => reject(signal.reason);
      signal.addEventListener('abort', abort, { once: true });
      promise.then(value => { signal.removeEventListener('abort', abort); resolve(value); },
        error => { signal.removeEventListener('abort', abort); reject(error); });
      if (signal.aborted) abort();
    });
  }
  function dispose() {
    disposed = true;
    for (const aborter of running) aborter.abort();
    for (const job of waiting.splice(0)) job.reject(new DOMException('Album closed', 'AbortError'));
    entries.clear();
    displayed.clear();
    retained = 0;
  }
  return { prepare, dispose,
    hasDisplayed: item => displayed.has(itemKey(item)),
    markDisplayed: item => { if (!disposed) displayed.add(itemKey(item)); } };
}
