import { galleryPostGroups } from '../shared/gallery-post-groups.js';
import { isBrowserGalleryDownload, normalizeGalleryDownloadMode } from '../shared/gallery-download-mode.js';

/** One visible list, with backend-local indexes retained for settings and selection. */
export function popupResultItems(state) {
  const videos = (state.analysisByBackend.y?.items || []).map((item, index) => ({
    backend: 'y', index, key: `y:${index}`, item,
  }));
  const galleries = galleryPostGroups(state.analysisByBackend.g?.items || [], state.sourceUrl)
    .map((item, index) => ({ backend: 'g', index, key: `g:${item.key}`, item }));
  return [...videos, ...galleries];
}

export function popupResultSelected(state, result, count = popupResultItems(state).length) {
  if (count === 1) return true;
  const selected = state.selectionsByBackend[result.backend];
  return result.backend === 'y' ? selected.has(result.index)
    : result.item.sourceItemIndices.every(index => selected.has(index));
}

export function selectedPopupResults(state) {
  const results = popupResultItems(state);
  return results.filter(result => popupResultSelected(state, result, results.length));
}

export function galleryResultMode(state, item) {
  return normalizeGalleryDownloadMode(state.galleryModes[item.key] ?? state.galleryDownloadMode);
}

export function popupDownloadPresentation(state) {
  const selected = selectedPopupResults(state);
  return {
    selected,
    browserOnly: selected.length > 0 && selected.every(result => result.backend === 'g'
      && isBrowserGalleryDownload(galleryResultMode(state, result.item))),
    disabled: !selected.length || state.busy || state.analysisRunning || state.downloadRunning
      || selected.some(result => result.backend === 'g'
        && !isBrowserGalleryDownload(galleryResultMode(state, result.item)) && !result.item.downloadUrl),
  };
}
