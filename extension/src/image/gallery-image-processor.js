/** Browser-owned image bytes. No extractor-to-XMP mapping is duplicated here. */
import { t } from '../shared/i18n.js';
let worker;
let serial = 0;
const pending = new Map();

function imageWorker() {
  if (worker) return worker;
  worker = new Worker(chrome.runtime.getURL('build/xmp-worker.js'), { type: 'module' });
  worker.onmessage = ({ data }) => {
    const call = pending.get(data.id);
    if (!call) return;
    pending.delete(data.id);
    if (data.error) call.reject(Error(data.error));
    else call.resolve(new Uint8Array(data.bytes));
  };
  worker.onerror = event => {
    for (const call of pending.values()) call.reject(Error(event.message || 'XMP worker failed'));
    pending.clear();
    worker?.terminate();
    worker = null;
  };
  return worker;
}

function format(bytes) {
  if (bytes[0] === 255 && bytes[1] === 216) return ['image/jpeg', true];
  if (bytes.length >= 8 && [137,80,78,71,13,10,26,10].every((n,i) => bytes[i] === n)) return ['image/png', true];
  if (bytes.length >= 6 && new TextDecoder().decode(bytes.subarray(0,6)).match(/^GIF8[79]a$/)) return ['image/gif', true];
  return ['', false];
}

export async function prepareGalleryImage(item, { signal, writeXmp = writeInWorker } = {}) {
  const response = await fetch(item.url, { credentials: 'include', signal });
  if (!response.ok) throw Error(`HTTP ${response.status}`);
  const original = new Uint8Array(await response.arrayBuffer());
  signal?.throwIfAborted();
  const [mime, supported] = format(original);
  if (!supported) return { bytes: original, mime: response.headers.get('content-type') || 'application/octet-stream', xmp: false };
  if (!item.archiveKey || !item.canonicalMetadata) throw Error(t('missingXmpMetadata'));
  const metadata = { archive_key: item.archiveKey, fields: item.canonicalMetadata };
  const bytes = await writeXmp(original, metadata);
  signal?.throwIfAborted();
  return { bytes, mime, xmp: true };
}

function writeInWorker(original, metadata) {
  return new Promise((resolve, reject) => {
    const id = ++serial;
    const target = imageWorker();
    pending.set(id, { resolve, reject });
    try { target.postMessage({ id, bytes: original.buffer, metadata }, [original.buffer]); }
    catch (error) { pending.delete(id); reject(error); }
  });
}
