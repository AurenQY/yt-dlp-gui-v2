/** Album browsing never inherits the popup's download range. Previously saved
 * range-limited albums restart at the newest post. Fresh probe seeds resume
 * after both analyzed posts, rather than fetching either seed again. */
export function albumBrowseSession(gallery) {
  if (!gallery?.timeline) return gallery;
  if (gallery.timeline.browsing) return { ...gallery,
    timeline: { ...gallery.timeline, range: { kind: 'all' } } };
  if (Number.isInteger(gallery.timeline.seedNextStart) && gallery.timeline.seedNextStart > 1) {
    return { ...gallery, timeline: { ...gallery.timeline, browsing: true, range: { kind: 'all' },
      postStart: gallery.timeline.seedNextStart, hasMore: gallery.timeline.seedHasMore !== false } };
  }
  const seed = gallery.posts?.[0];
  return { ...gallery, ...(seed ? { posts: [seed], media: undefined } : {}),
    timeline: { ...gallery.timeline, browsing: true, range: { kind: 'all' },
      pageSize: 1, postStart: 1, hasMore: true } };
}

/** Svelte action: the actual Framework7 scrolling pane owns visibility.
 * Re-observe after each page/layout update so a short page fills the viewport.
 * Error disables automatic retries; clicking the terminal card still retries. */
export function observeAlbumContinuation(node, options) {
  let current = options;
  let disposed = false;
  const observer = new IntersectionObserver(entries => {
    if (!disposed && current.enabled && !document.hidden && entries.some(entry => entry.isIntersecting)) current.load();
  }, { root: node.closest('.page-content'), threshold: 0 });
  function observe() {
    observer.disconnect();
    if (!disposed && current.enabled && !document.hidden) observer.observe(node);
  }
  document.addEventListener('visibilitychange', observe);
  observe();
  return {
    update(options) { current = options; observe(); },
    destroy() { disposed = true; observer.disconnect(); document.removeEventListener('visibilitychange', observe); },
  };
}
