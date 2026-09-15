const FALLBACK_MESSAGES = {
  selectAudioFormat: 'Select an audio format.',
  selectVideoFormat: 'Select a video format.',
  selectMergeAudioFormat: 'Select an audio format to merge with the video.',
  invalidUrlType: 'Enter a URL.',
  invalidUrlLength: 'The URL contains whitespace or is too long.',
  invalidUrlComplete: 'Enter a complete HTTP or HTTPS URL.',
  invalidUrlProtocol: 'Only HTTP and HTTPS URLs are accepted.',
  invalidUrlCredentials: 'URLs containing a username or password are not accepted.',
  invalidUrlIndex: 'The URL index format is invalid.',
  genericDownload: 'Generic download',
  language: 'Language',
  fileName: 'File name',
  fileNameBaseHint: 'The extension is determined by the output format.',
  thumbnailMode: 'Thumbnail',
  thumbnailOff: 'Off',
  thumbnailDownload: 'Download thumbnail',
  thumbnailEmbed: 'Embed thumbnail',
  subtitleMode: 'Subtitle handling',
  subtitleOff: 'Off',
  subtitleDownload: 'Download subtitle file',
  subtitleEmbed: 'Embed subtitles',
  chapterMode: 'Chapter handling',
  chapterOff: 'Full file',
  chapterDownload: 'Split chapters',
  chapterEmbed: 'Split and embed chapters',
  downloadRange: 'Download range',
  chapterRange: 'Chapter range',
  chapterRangeHint: 'Select the chapters to download. Adjacent chapters are combined into one output.',
  selectedChapters: '{count} chapters selected',
  selectAll: 'Select all',
  clearSelection: 'Clear selection',
  chapterNumber: 'Chapter {number}',
  fullVideo: 'Full video',
  rangeStart: 'Start',
  rangeEnd: 'End',
  timeRangeHint: 'Use HH:MM:SS or MM:SS. A custom range turns off chapter splitting.',
  timeRangeInvalid: 'Enter a valid start and end time.',
  timeRangeOrderInvalid: 'The end time must be later than the start time.',
  timeRangeBeyondDuration: 'The end time exceeds the media duration.',
  confirm: 'Confirm',
  openDownloadFolder: 'Open download folder',
  openFolderFailed: 'Could not open the download folder: {message}',
  ready: 'Ready',
  stopping: 'Stopping',
  downloadStopping: 'Stopping download…',
  downloadStopped: 'Download stopped.',
  downloadNotRunning: 'No download is running.',
  downloadStopFailed: 'Could not stop download: {message}',
  matcherIndexLoadFailed: 'Could not load the URL index.',
  noDownloadableMedia: 'This page has no downloadable video or audio.',
};

export function t(key, values = {}) {
  const localized = globalThis.chrome?.i18n?.getMessage(key);
  const message = localized || FALLBACK_MESSAGES[key] || key;
  return Object.entries(values).reduce(
    (result, [name, value]) => result.replaceAll(`{${name}}`, String(value)),
    message,
  );
}

export function localizeDocument(root = document) {
  root.documentElement.lang = globalThis.chrome?.i18n?.getUILanguage?.() || 'en';
  root.querySelectorAll('[data-i18n]').forEach(element => {
    element.textContent = t(element.dataset.i18n);
  });
  for (const [attribute, dataAttribute] of [
    ['aria-label', 'i18nAriaLabel'],
    ['placeholder', 'i18nPlaceholder'],
    ['title', 'i18nTitle'],
  ]) {
    root.querySelectorAll(`[data-${dataAttribute.replace(/[A-Z]/g, letter => `-${letter.toLowerCase()}`)}]`)
      .forEach(element => element.setAttribute(attribute, t(element.dataset[dataAttribute])));
  }
}
