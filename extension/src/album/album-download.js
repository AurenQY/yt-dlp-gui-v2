import { storedZip } from './zip-store.js';
import { t } from '../shared/i18n.js';
import { safeFileName, zipBase } from './album-data.js';
import { albumDownloadPath, uniqueAlbumArchivePaths } from './album-download-path.js';
import { ensureSitePermissions } from '../shared/site-permission-request.js';

async function requirePermissions(media, permissionsAlreadyGranted) {
  const origins = [...new Set(media.map(item => new URL(item.url).origin + '/*'))];
  if (await chrome.permissions.contains({ origins })) return;
  if (permissionsAlreadyGranted || !await ensureSitePermissions(origins)) {
    throw Error(t('mediaPermissionRequired'));
  }
}

/** A processed Blob remains alive until Chrome confirms the download is terminal. */
function saveBlob(blob, filename, signal) {
  signal?.throwIfAborted();
  const url = URL.createObjectURL(blob);
  return new Promise((resolve, reject) => {
    let id;
    let cleaned = false;
    const cleanup = () => {
      if (cleaned) return;
      cleaned = true;
      chrome.downloads.onChanged.removeListener(changed);
      signal?.removeEventListener('abort', abort);
      URL.revokeObjectURL(url);
    };
    const settle = state => {
      if (state === 'complete') { cleanup(); resolve(); }
      else if (state === 'interrupted') { cleanup(); reject(Error(t('browserDownloadInterrupted'))); }
    };
    const changed = delta => { if (delta.id === id) settle(delta.state?.current); };
    const abort = () => {
      if (id != null) chrome.downloads.cancel(id).catch(() => {});
      cleanup();
      reject(signal.reason);
    };
    chrome.downloads.onChanged.addListener(changed);
    signal?.addEventListener('abort', abort, { once: true });
    chrome.downloads.download({ url, filename, conflictAction: 'uniquify', saveAs: false })
      .then(async value => {
        id = value;
        if (!Number.isInteger(id)) throw Error('Browser returned no download ID.');
        if (signal?.aborted) { await chrome.downloads.cancel(id); return; }
        // Covers a tiny file completing before the ID promise resolved.
        const [item] = await chrome.downloads.search({ id });
        settle(item?.state);
      })
      .catch(error => { cleanup(); reject(error); });
  });
}

/** Single files and ZIP consume the exact same XMP-prepared bytes as the viewer. */
export async function downloadAlbumSelection({ gallery, media, selected, asZip, progress,
  session, signal, permissionsAlreadyGranted = false, save = saveBlob }) {
  const indexes = [...selected].sort((a,b) => a-b);
  if (!indexes.length) return;
  // Preflight every path before permission requests, processing or downloads.
  const paths = indexes.map(index => albumDownloadPath(media[index]));
  const archivePaths = asZip ? uniqueAlbumArchivePaths(paths) : paths;
  await requirePermissions(indexes.map(i => media[i]), permissionsAlreadyGranted);
  const files = [];
  let unsupported = 0;
  for (const [position, index] of indexes.entries()) {
    signal?.throwIfAborted();
    await progress(t('processingMedia', { current: position + 1, total: indexes.length, name: safeFileName(media[index], index) }));
    const result = await session.prepare(media[index], { signal });
    signal?.throwIfAborted();
    if (!result.xmp) unsupported += 1;
    const name = archivePaths[position];
    if (asZip) {
      files.push({ name, bytes: new Uint8Array(await result.blob.arrayBuffer()) });
    } else {
      await progress(t('savingMedia', { current: position + 1, total: indexes.length, name }));
      await save(result.blob, name, signal);
    }
  }
  if (asZip) {
    signal?.throwIfAborted();
    await progress(t('creatingZip'));
    await save(storedZip(files), `${zipBase(gallery)}.zip`, signal);
  }
  await progress(unsupported ? t('downloadCompletedOriginal', { count: indexes.length, unsupported }) : t('downloadCompletedXmp', { count: indexes.length }));
}
