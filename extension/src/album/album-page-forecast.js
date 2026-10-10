//! Browse batch estimation, not UI positioning. Reuse the production justified
//! layout with recent complete-post shapes; never alter rendered row geometry.
import { justifiedAlbumRows, imageAspectRatio, ALBUM_ROW_GAP } from './album-justified-layout.js';

const INITIAL_BROWSE_VIEWPORT_SPAN = 1.5;

export function albumContentHeight(tiles, width, measuredRatios = new Map()) {
  tiles = tiles.filter(tile => tile.kind !== 'continue');
  const rows = justifiedAlbumRows(tiles, width, measuredRatios);
  return rows.reduce((sum, row) => sum + row.height, 0) + Math.max(0, rows.length - 1) * ALBUM_ROW_GAP;
}

/** Return a bounded whole-post count that is predicted to cover the target.
 * Future post shapes are unknown; use up to six recently displayed examples. */
export function forecastAlbumPostCount({ tiles = [], width, targetHeight, measuredRatios = new Map(), limit = 10 }) {
  const cap = Math.max(1, Math.min(10, Math.floor(limit) || 1));
  if (!(width > 0) || !(targetHeight > 0)) return 1;
  const current = [];
  const samples = new Map();
  tiles.forEach((tile, index) => {
    if (tile.kind === 'continue') return;
    current.push({ ...tile });
    const sample = samples.get(tile.postIndex) || [];
    sample.push({ ...tile, width: imageAspectRatio(tile, measuredRatios.get(index)), height: 1 });
    samples.set(tile.postIndex, sample);
  });
  const recent = [...samples.values()].slice(-6).sort((a, b) => albumContentHeight(a, width) - albumContentHeight(b, width));
  const conservative = recent.slice(0, Math.max(1, Math.ceil(recent.length / 2)));
  if (!recent.length) return 1;
  for (let count = 1; count <= cap; count++) {
    current.push(...conservative[(count - 1) % conservative.length]);
    if (albumContentHeight(current, width, measuredRatios) >= targetHeight) return count;
  }
  return cap;
}

/** Freeze an absolute content-height goal at the start of a read cycle.
 * Keep the initial 1.5-viewport fill, but every continuation adds at least one
 * viewport beyond the already-loaded content, not just the visible shortfall.
 * Browse capacity only; never changes rendered geometry or scroll position. */
export function albumReadTarget(grid, contentHeight = 0) {
  const pane = grid?.closest('.page-content');
  if (!pane) return 0;
  const gridTop = grid.getBoundingClientRect().top - pane.getBoundingClientRect().top + pane.scrollTop;
  const initialFillTarget = pane.scrollTop + pane.clientHeight * INITIAL_BROWSE_VIEWPORT_SPAN - gridTop;
  const loadedHeight = Number.isFinite(contentHeight) ? Math.max(0, contentHeight) : 0;
  return Math.max(initialFillTarget, loadedHeight + pane.clientHeight);
}
