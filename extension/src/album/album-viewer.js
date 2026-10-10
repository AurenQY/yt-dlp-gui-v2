import Spotlight from 'spotlight.js/src/js/spotlight.js';
import { t } from '../shared/i18n.js';

function escapeSvgText(value) {
  return value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
}
import { isVideo } from './album-data.js';
import { albumSpotlightSlides } from './album-spotlight-slides.js';
import { installAlbumViewerWheel } from './album-viewer-wheel.js';
import { installAlbumVideoFallback } from './album-video-sources.js';

/** Use Spotlight's own gallery navigation and fit/original-size toggle.
 * Do not create a second viewer: Spotlight pans images larger than its viewport. */
export function openAlbumViewer(media, index, gallery = {}, postByMediaIndex = [], { session, report = () => {}, wheelMode = () => 'zoom' } = {}) {
  if (isVideo(media[index]) && String(media[index].extension).toLowerCase() === 'm3u8') {
    // Chromium does not natively decode HLS in a regular video element.
    chrome.tabs.create({ url: media[index].url });
    return;
  }
  const slides = albumSpotlightSlides(media, gallery, postByMediaIndex);
  // Spotlight has no asynchronous source provider. Its native image element
  // starts with a local placeholder, then receives only XMP-processed bytes.
  // All navigation/fit/zoom/panning remain owned by Spotlight, not a second viewer.
  const placeholders = slides.map((slide, i) => {
    if (isVideo(media[i])) return '';
    const url = `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="640" height="360"><text x="20" y="40" fill="#8c8c8c">${escapeSvgText(t('albumProcessingNumber', { number: i + 1 }))}</text></svg>`)}`;
    if (session) slide.src = url;
    return url;
  });
  let closed = false;
  let current = -1;
  let generation = 0;
  let activeUrl = '';
  let releaseVideo = () => {};
  async function prepareSlide(oneBasedIndex) {
    const next = oneBasedIndex - 1;
    const requestGeneration = ++generation;
    current = next;
    releaseVideo();
    releaseVideo = () => {};
    if (isVideo(media[next])) {
      const video = document.querySelectorAll('#spotlight .spl-pane')[next]?.querySelector('video');
      if (video) releaseVideo = installAlbumVideoFallback(video, media[next]);
      return;
    }
    if (!session) return;
    try {
      const result = await session.prepare(media[next]);
      if (closed || current !== next || generation !== requestGeneration) return;
      // Scope to Spotlight's returned slide order. A recently revisited pane
      // may still contain its previous loaded image rather than the placeholder.
      const pane = document.querySelectorAll('#spotlight .spl-pane')[next];
      const image = pane?.querySelector('img');
      if (!image) throw Error('Viewer provided no replaceable image element.');
      if (activeUrl) URL.revokeObjectURL(activeUrl);
      activeUrl = URL.createObjectURL(result.blob);
      image.src = activeUrl;
      report(result.xmp ? '' : t('albumOriginalNoXmp'));
    } catch (error) { if (!closed && current === next) report(String(error?.message || error)); }
  }
  // Must precede show(): Spotlight registers its own capturing wheel listener there.
  const releaseWheel = installAlbumViewerWheel(Spotlight, wheelMode);
  try { Spotlight.show(slides, {
    index: index + 1, // Spotlight uses a 1-based gallery index.
    preload: false,
    autohide: false,
    // Keep native previous/next navigation enabled in the control whitelist.
    // autofit toggles fitted images to natural-pixel size inside the same viewer.
    control: ['page', 'prev', 'next', 'autofit', 'zoom', 'fullscreen', 'close'],
    theme: 'dark',
    onchange: prepareSlide,
    onclose: () => { closed = true; releaseVideo(); releaseWheel(); if (activeUrl) URL.revokeObjectURL(activeUrl); },
  }); } catch (error) { releaseVideo(); releaseWheel(); throw error; }
}

export function closeAlbumViewer() { Spotlight.close(); }
