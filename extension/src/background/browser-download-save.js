/** Service Workers cannot create Blob URLs. Encode only the current output. */
import { t } from '../shared/i18n.js';
export async function blobDataUrl(blob, signal) {
  const bytes = new Uint8Array(await blob.arrayBuffer());
  const chunks = [];
  // Multiple of three: concatenated Base64 chunks remain a single valid encoding.
  for (let offset = 0; offset < bytes.length; offset += 24576) {
    signal?.throwIfAborted();
    chunks.push(btoa(String.fromCharCode(...bytes.subarray(offset, offset + 24576))));
  }
  const mime = /^[\w.+-]+\/[\w.+-]+$/.test(blob.type) ? blob.type : 'application/octet-stream';
  return `data:${mime};base64,${chunks.join('')}`;
}

/** Persist Chrome's ID before waiting, so restart can reconcile instead of replaying. */
export async function saveBrowserDownload(blob, filename, signal, recordId) {
  signal?.throwIfAborted();
  const url = await blobDataUrl(blob, signal);
  signal?.throwIfAborted();
  const id = await chrome.downloads.download({ url, filename, conflictAction: 'uniquify', saveAs: false });
  if (!Number.isInteger(id)) throw Error('Browser returned no download ID.');
  try {
    await recordId(id);
    signal?.throwIfAborted();
    await new Promise((resolve, reject) => {
      const cleanup = () => {
        chrome.downloads.onChanged.removeListener(changed);
        signal?.removeEventListener('abort', abort);
      };
      const settle = state => {
        if (state === 'complete') { cleanup(); resolve(); }
        else if (state === 'interrupted') { cleanup(); reject(Error(t('browserDownloadInterrupted'))); }
      };
      const changed = delta => { if (delta.id === id) settle(delta.state?.current); };
      const abort = () => { cleanup(); reject(signal.reason); };
      chrome.downloads.onChanged.addListener(changed);
      signal?.addEventListener('abort', abort, { once: true });
      chrome.downloads.search({ id }).then(items => settle(items[0]?.state))
        .catch(error => { cleanup(); reject(error); });
      if (signal?.aborted) abort();
    });
  } catch (error) {
    if (signal?.aborted) await chrome.downloads.cancel(id).catch(() => {});
    throw error;
  }
}
