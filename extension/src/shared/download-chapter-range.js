export function selectedChapterIndicesFromChoice(item, choice = {}) {
  const chapterCount = Array.isArray(item?.chapters) ? item.chapters.length : 0;
  if (!Array.isArray(choice.chapterIndices) || chapterCount === 0) return [];
  return [...new Set(choice.chapterIndices)]
    .filter(index => Number.isSafeInteger(index) && index >= 0 && index < chapterCount)
    .sort((left, right) => left - right);
}

export function chapterDownloadRangesFromChoice(item, choice = {}) {
  const chapters = Array.isArray(item?.chapters) ? item.chapters : [];
  const indices = selectedChapterIndicesFromChoice(item, choice);
  const groups = [];
  for (const index of indices) {
    const current = groups.at(-1);
    if (current && index === current.endIndex + 1) {
      current.endIndex = index;
    } else {
      groups.push({ startIndex: index, endIndex: index });
    }
  }
  return groups
    .map(group => ({
      startMillis: Number(chapters[group.startIndex]?.startMillis),
      endMillis: Number(chapters[group.endIndex]?.endMillis),
    }))
    .filter(range =>
      Number.isSafeInteger(range.startMillis)
      && Number.isSafeInteger(range.endMillis)
      && range.startMillis >= 0
      && range.endMillis > range.startMillis);
}
