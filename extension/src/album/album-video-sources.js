import { isMediaUrl } from './album-data.js';
import { recordDiagnostic } from '../shared/operation-diagnostics.js';

/** Only extractor-provided URLs, in their original quality/fallback order. */
export function albumVideoSources(item) {
  const fallbacks = Array.isArray(item?.fallbackUrls) ? item.fallbackUrls.slice(0, 16) : [];
  return [...new Set([item?.url, ...fallbacks].filter(isMediaUrl))];
}

/** A failed candidate advances once; never retry the same URL or re-analyze.
 * Capture before Spotlight's error handler, which otherwise removes the video. */
export function installAlbumVideoFallback(video, item, {
  resumePlayback = true, onSourceChange = () => {},
} = {}) {
  const sources = albumVideoSources(item);
  let readySource = '';
  function record(status, phase, index) {
    // Fixed English/counts only: signed CDN URLs and headers stay out of logs.
    recordDiagnostic('album-video.source', { status, phase: `${phase}_${index + 1}`,
      itemCount: sources.length });
  }
  function failed(event) {
    const source = video.getAttribute('src');
    if (!source) return; // Ignore an unloaded/offscreen video's late error.
    const index = sources.indexOf(source);
    if (index < 0) return;
    if (index + 1 >= sources.length) { record('failed', 'exhausted', index); return; }
    event.stopImmediatePropagation();
    record('loading', 'fallback', index + 1);
    const resume = resumePlayback && !video.paused;
    video.src = sources[index + 1];
    video.load();
    onSourceChange(sources[index + 1]);
    if (resume) video.play().catch(() => {});
  }
  function loaded() {
    const source = video.getAttribute('src');
    const index = sources.indexOf(source);
    if (index < 0 || source === readySource) return;
    readySource = source;
    record('completed', index ? 'fallback' : 'primary', index);
  }
  video.addEventListener('error', failed, true);
  video.addEventListener('loadeddata', loaded);
  return () => {
    video.removeEventListener('error', failed, true);
    video.removeEventListener('loadeddata', loaded);
  };
}
