/** Album input preferences are browser-local, independent of Native Host settings. */
export const ALBUM_WHEEL_KEY = 'albumWheelMode';
export function normalizedAlbumWheelMode(value) {
  return value === 'navigate' ? 'navigate' : 'zoom';
}

export function observeAlbumWheelMode(storage, changed, reportError = () => {}) {
  let disposed = false;
  let revision = 0;
  const listener = (changes, area) => {
    if (area !== 'local' || !changes[ALBUM_WHEEL_KEY]) return;
    revision++;
    changed(normalizedAlbumWheelMode(changes[ALBUM_WHEEL_KEY].newValue));
  };
  storage.onChanged.addListener(listener);
  const initialRevision = revision;
  storage.local.get(ALBUM_WHEEL_KEY).then(saved => {
    if (!disposed && revision === initialRevision) changed(normalizedAlbumWheelMode(saved[ALBUM_WHEEL_KEY]));
  }).catch(error => { if (!disposed) reportError(error); });
  return () => { disposed = true; storage.onChanged.removeListener(listener); };
}
