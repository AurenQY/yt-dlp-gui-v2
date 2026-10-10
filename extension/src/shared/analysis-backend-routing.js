/** Route each extractor from shared JSON; V2 retains authoritative matching. */
export function normalizeRecognitionForAnalysis(recognized) {
  if (!recognized) return recognized;
  const y = recognized.y || null;
  const g = recognized.g || null;
  return {
    ...recognized, y, g,
    tools: { y: Boolean(y), g: Boolean(g) },
    approximate: Boolean(recognized.approximate || g?.fallback),
  };
}

export function analysisBackendsForRecognition(recognized) {
  const backends = ['y', 'g'].filter(backend => Boolean(recognized?.[backend]));
  return backends.length ? backends : ['y'];
}
