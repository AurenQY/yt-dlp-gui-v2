import { isMediaUrl } from './album-data.js';
import { installAlbumVideoFallback } from './album-video-sources.js';

/** A real separate poster is optional; the direct video URL is not a poster. */
export function albumVideoPoster(item) {
  return isMediaUrl(item?.thumbnail) && item.thumbnail !== item.url ? item.thumbnail : '';
}

/** Visible clips without a poster prepare a paused first frame; hover/focus starts muted playback.
 * Leave preserves the decoded frame. Offscreen/hidden clips release their source. */
export function installAlbumVideoPreview(frame, video, item, report = () => {}) {
  let hovered = false;
  let focused = false;
  let visible = false;
  let disposed = false;
  let source = '';
  let generation = 0;
  let playPending = false;
  let ready = false;
  const playable = ['mp4', 'webm', 'mov'].includes(String(item.extension).toLowerCase());
  const hasPoster = Boolean(albumVideoPoster(item));
  video.muted = true;
  video.loop = true;
  video.playsInline = true;
  function stop() {
    generation++;
    playPending = false;
    ready = false;
    video.pause();
    if (source) { video.removeAttribute('src'); video.load(); source = ''; }
    report('idle');
  }
  function update() {
    if (disposed || !playable || !visible || document.hidden) {
      stop(); return;
    }
    if (!source) {
      if (hasPoster && !(hovered || focused)) { report('ready'); return; }
      generation++;
      source = item.url;
      video.preload = 'metadata';
      video.src = source;
      report('loading');
      video.load();
    }
    if (!(hovered || focused)) {
      playPending = false;
      video.pause();
      report(ready ? 'ready' : 'loading');
      return;
    }
    if (playPending || !video.paused) return;
    playPending = true;
    video.preload = 'auto';
    const current = generation;
    video.play().then(() => {
      if (!disposed && generation === current) {
        playPending = false;
        if (hovered || focused) report('playing');
        else { video.pause(); report(ready ? 'ready' : 'loading'); }
      }
    }).catch(() => {
      if (!disposed && generation === current) {
        playPending = false;
        report(ready ? 'ready' : 'error');
      }
    });
  }
  const metadata = () => {
    if (!source || disposed || hasPoster || hovered || focused) return;
    // Decode the actual first frame, not a sampled later frame or poster image.
    video.currentTime = 0;
  };
  const frameReady = () => {
    if (!source || disposed || video.seeking || video.readyState < 2) return;
    ready = true;
    if (!(hovered || focused)) report('ready');
  };
  const enter = () => { hovered = true; update(); };
  const leave = () => { hovered = false; update(); };
  const focus = () => { focused = true; update(); };
  const blur = () => { focused = false; update(); };
  const error = () => { if (source && !disposed) report('error'); };
  const releaseFallback = installAlbumVideoFallback(video, item, {
    resumePlayback: false,
    onSourceChange: url => {
      generation++;
      source = url;
      ready = false;
      playPending = false;
      report('loading');
      update();
    },
  });
  frame.addEventListener('pointerenter', enter);
  frame.addEventListener('pointerleave', leave);
  frame.addEventListener('focusin', focus);
  frame.addEventListener('focusout', blur);
  video.addEventListener('error', error);
  video.addEventListener('loadedmetadata', metadata);
  video.addEventListener('loadeddata', frameReady);
  video.addEventListener('seeked', frameReady);
  document.addEventListener('visibilitychange', update);
  const observer = new IntersectionObserver(entries => {
    visible = entries[0].isIntersecting;
    update();
  });
  observer.observe(frame);
  return () => {
    disposed = true;
    releaseFallback();
    observer.disconnect();
    frame.removeEventListener('pointerenter', enter);
    frame.removeEventListener('pointerleave', leave);
    frame.removeEventListener('focusin', focus);
    frame.removeEventListener('focusout', blur);
    video.removeEventListener('error', error);
    video.removeEventListener('loadedmetadata', metadata);
    video.removeEventListener('loadeddata', frameReady);
    video.removeEventListener('seeked', frameReady);
    document.removeEventListener('visibilitychange', update);
    stop();
  };
}
