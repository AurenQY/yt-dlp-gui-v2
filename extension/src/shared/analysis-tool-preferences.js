/** Persistent user intent is separate from the last observed desktop state. */
export const ANALYSIS_TOOLS_KEY = 'analysisToolPreferences';
export const ANALYSIS_DETECTION_KEY = 'analysisToolDetection';

export function normalizedAnalysisTools(value) {
  return { y: value?.y !== false, g: value?.g !== false };
}

export function enabledAnalysisTools(value) {
  const tools = normalizedAnalysisTools(value);
  return ['y', 'g'].filter(tool => tools[tool]);
}

export function detectedAnalysisTools(data, checkedAt = Date.now()) {
  return {
    y: typeof data?.ytDlpAvailable === 'boolean' ? data.ytDlpAvailable : null,
    g: typeof data?.galleryDlAvailable === 'boolean' ? data.galleryDlAvailable : null,
    checkedAt,
  };
}

export function initialToolsFromDetection(detection) {
  return { y: detection.y !== false, g: detection.g !== false };
}
