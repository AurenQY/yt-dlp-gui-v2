/** Justified photo rows: proportional image widths share the row width without cropping.
 * An incomplete final row stays at its target height instead of stretching excessively. */
export function imageAspectRatio(item, measuredRatio) {
  const width = Number(item?.width);
  const height = Number(item?.height);
  const ratio = Number.isFinite(measuredRatio) && measuredRatio > 0
    ? measuredRatio
    : width > 0 && height > 0 ? width / height : 1;
  return Math.max(0.2, Math.min(8, ratio));
}

export const ALBUM_ROW_GAP = 6;

export function justifiedAlbumRows(media, containerWidth, measuredRatios = new Map(), gap = ALBUM_ROW_GAP) {
  if (!Number.isFinite(containerWidth) || containerWidth <= 0 || !media.length) return [];
  const width = Math.max(1, containerWidth);
  const spacing = Math.max(0, gap);
  const targetHeight = Math.max(135, Math.min(235, width * 0.22));
  const rows = [];
  let pending = [];
  let ratioSum = 0;

  function finishRow(complete) {
    if (!pending.length) return;
    const available = Math.max(1, width - spacing * (pending.length - 1));
    const justifiedHeight = available / ratioSum;
    const full = complete || justifiedHeight <= targetHeight * 1.15;
    const height = full ? justifiedHeight : Math.min(targetHeight, justifiedHeight);
    rows.push({
      id: pending[0].index,
      height,
      justified: full,
      items: pending.map(({ index, ratio }) => ({ index, ratio, width: ratio * height })),
    });
    pending = [];
    ratioSum = 0;
  }

  media.forEach((item, index) => {
    const ratio = imageAspectRatio(item, measuredRatios.get(index));
    pending.push({ index, ratio });
    ratioSum += ratio;
    if (ratioSum * targetHeight + spacing * (pending.length - 1) >= width) finishRow(true);
  });
  finishRow(false);
  return rows;
}
