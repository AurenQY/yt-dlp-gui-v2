/** Album session payload from the popup. Nothing in this module depends on UI. */
import { t } from '../shared/i18n.js';
export function isMediaUrl(value) {
  try {
    const url = new URL(value);
    return ['https:', 'http:'].includes(url.protocol) && !!url.hostname
      && !url.username && !url.password;
  } catch { return false; }
}

/** Validation shared by the background message handler and album page. */
export function isValidGalleryMedia(media) {
  return Array.isArray(media) && media.length > 0
    && media.every(item => isMediaUrl(item?.url));
}

export function isVideo(item) {
  return ['mp4', 'webm', 'mov', 'm3u8'].includes(String(item?.extension || '').toLowerCase());
}

export function safeFileName(item, index) {
  // UI labels and processing status must match the parser-owned download name.
  const formatted = item?.relativePath?.replaceAll('\\', '/').split('/').at(-1);
  if (formatted) return formatted;
  const extension = String(item?.extension || '').replace(/[^a-z0-9]/gi, '').slice(0, 10).toLowerCase();
  const stem = String(item?.filename || `media-${index + 1}`)
    .replace(/[\\/:*?"<>|\x00-\x1f]/g, '_').replace(/^\.+/, '').slice(0, 100)
    || `media-${index + 1}`;
  return extension && !stem.toLowerCase().endsWith(`.${extension}`)
    ? `${stem}.${extension}` : stem;
}

export function zipBase(gallery) {
  return String(gallery?.title || 'gallery')
    .replace(/[\\/:*?"<>|\x00-\x1f]/g, '_').slice(0, 64).trim() || 'gallery';
}

/** A multi-post album may supply `posts`; old one-post payloads use `media`. */
export function albumGalleryMedia(gallery) {
  if (Array.isArray(gallery?.posts) && gallery.posts.length) {
    return gallery.posts.flatMap(post => Array.isArray(post?.media) ? post.media : []);
  }
  return gallery?.media;
}

export async function loadAlbum(storage, pageUrl) {
  const id = new URL(pageUrl).searchParams.get('id');
  if (!id || !/^[0-9a-f-]{36}$/i.test(id)) throw Error(t('albumNotFound'));
  const key = `galleryAlbum:${id}`;
  const gallery = (await storage.get(key))[key];
  const galleryMedia = albumGalleryMedia(gallery);
  if ((!Array.isArray(galleryMedia) || !galleryMedia.length) && !gallery?.timeline) {
    throw Error(t('albumExpired'));
  }
  const media = (galleryMedia || []).filter(item => isMediaUrl(item?.url));
  if (!media.length && !gallery?.timeline) throw Error(t('albumEmpty'));
  return { gallery, media, key };
}
