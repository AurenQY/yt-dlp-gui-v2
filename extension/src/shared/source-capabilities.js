/** Identity only: no live health inference and no tooltip. Gallery is optional. */
export function sourceCapabilities(recognized, detection) {
  const icons = [];
  const y = recognized?.y;
  if (y && !y.approximate && y.working !== false) {
    const name = { single: 'video', playlist: 'playlist', any: 'media' }[y.sourceScope];
    if (name) icons.push({ kind: 'video', name });
  }
  if (recognized?.g && detection?.g === true) icons.push({ kind: 'gallery', name: 'gallery' });
  // Music gets its own icon only when a future capability check proves it.
  return icons;
}
