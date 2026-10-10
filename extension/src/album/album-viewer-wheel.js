/** Intercept before Spotlight's window-capture zoom handler; retain its navigation API. */
export function installAlbumViewerWheel(spotlight, mode, target = window) {
  let lastEvent = -Infinity;
  let lastStep = -Infinity;
  let accumulated = 0;
  const listener = event => {
    if (mode() !== 'navigate' || !event.target?.closest?.('#spotlight')) return;
    // Text scrolling and video-native fullscreen are not album navigation surfaces.
    if (event.target.closest('.spl-footer, .spl-header')
        || document.fullscreenElement?.tagName === 'VIDEO') {
      event.stopImmediatePropagation();
      return;
    }
    // Preserve browser pinch/zoom gestures, without Spotlight also zooming the slide.
    if (event.ctrlKey || event.metaKey) { event.stopImmediatePropagation(); return; }
    if (!event.deltaY || Math.abs(event.deltaX) > Math.abs(event.deltaY)) {
      event.stopImmediatePropagation();
      return;
    }
    event.preventDefault();
    event.stopImmediatePropagation();
    const now = event.timeStamp;
    if (now - lastEvent > 180 || Math.sign(accumulated) !== Math.sign(event.deltaY)) accumulated = 0;
    lastEvent = now;
    // Normalize line/page wheels and avoid touchpad inertia skipping many slides.
    accumulated += event.deltaY * (event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? 800 : 1);
    if (Math.abs(accumulated) < 40 || now - lastStep < 500) return;
    lastStep = now;
    const direction = Math.sign(accumulated);
    accumulated = 0;
    direction > 0 ? spotlight.next() : spotlight.prev();
  };
  target.addEventListener('wheel', listener, { capture: true, passive: false });
  return () => target.removeEventListener('wheel', listener, { capture: true });
}
