import { rangeForTimeline } from './gallery-timeline-range.js';
import { t } from './i18n.js';
/** One extension-wide choice for gallery-dl posts; never alters yt-dlp formats. */
export const GALLERY_DOWNLOAD_STORAGE_KEY = 'galleryDownloadMode';
export const GALLERY_DOWNLOAD_MODES = Object.freeze([
  'browser-files', 'v2-direct',
]);

export function normalizeGalleryDownloadMode(value) {
  // Preserve the chosen destination for obsolete ZIP preferences, without ZIP.
  if (value === 'v2-zip') return 'v2-direct';
  return GALLERY_DOWNLOAD_MODES.includes(value) ? value : 'browser-files';
}

export function galleryDownloadModeLabelKey(value) {
  return normalizeGalleryDownloadMode(value) === 'v2-direct'
    ? 'galleryModeV2Direct' : 'galleryModeBrowserFiles';
}

export function isBrowserGalleryDownload(mode) {
  return normalizeGalleryDownloadMode(mode) === 'browser-files';
}

export function galleryDownloadOrigins(groups) {
  return [...new Set(groups.flatMap(group => group.media || []).map(media => {
    const url = new URL(media.url);
    if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) {
      throw new Error('Invalid media download URL.');
    }
    return `${url.origin}/*`;
  }))];
}

export function galleryNativeRequests(groups, current) {
  return groups.filter(group => group.downloadUrl).map(group => ({
    action: 'download', backend: 'g', url: group.downloadUrl,
    archiveMode: 'files',
    title: group.title, sourcePageUrl: current.sourceUrl,
    ...(group.sourceScope === 'timeline' ? { timelineRange: rangeForTimeline(group, current) } : {}),
    sourceTabId: current.sourceTabId,
    sourceItemIndex: group.index,
  }));
}
