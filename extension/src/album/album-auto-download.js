/** Auto-download is single-use. Only a popup-requested album tab may consume it. */
export function albumAutoDownloadMode(pageUrl) {
  const mode = new URL(pageUrl).searchParams.get('download');
  return mode === 'files' || mode === 'zip' ? mode : null;
}

export async function consumeAlbumAutoDownload(storage, pageUrl) {
  const url = new URL(pageUrl);
  const mode = albumAutoDownloadMode(pageUrl);
  const id = url.searchParams.get('id');
  if (!mode || !id || !/^[0-9a-f-]{36}$/i.test(id)) return null;
  const key = `galleryAutoDownload:${id}`;
  const saved = (await storage.get(key))[key];
  if (saved !== mode) return null;
  // Consume before network I/O; refreshing a tab never downloads the same post again.
  await storage.remove(key);
  return mode;
}
