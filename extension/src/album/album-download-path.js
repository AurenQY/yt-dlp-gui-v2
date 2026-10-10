/** Consume gallery-dl's output path verbatim, never rebuild its naming rules. */
import { t } from '../shared/i18n.js';
export function albumDownloadPath(item) {
  const value = item?.relativePath;
  if (typeof value !== 'string' || !value) {
    throw Error(t('galleryPathMissing'));
  }
  const path = value.replaceAll('\\', '/');
  const segments = path.split('/');
  if (segments.some(segment => !segment || segment === '.' || segment === '..'
      || /[<>:"|?*\x00-\x1f\x7f]/.test(segment) || /[. ]$/.test(segment)
      || /^(?:con|prn|aux|nul|com[1-9¹²³]|lpt[1-9¹²³])(?:\.|$)/i.test(segment))) {
    throw Error(t('galleryPathInvalid'));
  }
  return path;
}

/** ZIP cannot rely on Chrome's uniquify. Preserve names unless they collide. */
export function uniqueAlbumArchivePaths(paths) {
  const used = new Set();
  return paths.map(path => {
    const slash = path.lastIndexOf('/');
    const folder = path.slice(0, slash + 1);
    const filename = path.slice(slash + 1);
    const dot = filename.lastIndexOf('.');
    const stem = dot > 0 ? filename.slice(0, dot) : filename;
    const extension = dot > 0 ? filename.slice(dot) : '';
    let candidate = path;
    let suffix = 2;
    while (used.has(candidate.normalize('NFC').toLowerCase())) {
      candidate = `${folder}${stem} (${suffix++})${extension}`;
    }
    used.add(candidate.normalize('NFC').toLowerCase());
    return candidate;
  });
}
