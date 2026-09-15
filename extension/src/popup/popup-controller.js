import { get, writable } from 'svelte/store';
import { validateUrl } from '../shared/url-validation.js';
import {
  availableContentKinds,
  buildDownloadRequest,
  defaultFormatId,
  formatsForContent,
  outputFormatsForChoice,
  pageDownloadScope,
  recommendedDownloadChoice,
  resolvedOutputFormat,
} from '../shared/media-download-options.js';
import { collectPageSnapshot } from '../shared/page-snapshot.js';
import { downloadTimeRangeFromChoice } from '../shared/download-time-range.js';
import { selectedChapterIndicesFromChoice } from '../shared/download-chapter-range.js';
import { NATIVE_HOST_HELP_URL } from '../shared/native-host-errors.js';
import { t } from '../shared/i18n.js';

const ANALYSIS_CACHE_VERSION = 9;
const ANALYSIS_EVENT = 'YT_DLP_GUI_ANALYSIS_EVENT';

function initialState() {
  return {
    analysisView: 'idle',
    recognized: null,
    recognitionPending: true,
    analysis: null,
    busy: false,
    analysisRunning: false,
    downloadRunning: false,
    currentTask: { state: 'idle' },
    sourceUrl: '',
    sourceSiteName: t('recognizing'),
    sourceSiteIconUrl: '',
    sourcePageTitle: '',
    sourcePageSnapshot: null,
    sourceTabId: null,
    choices: {},
    selectedIndices: new Set(),
    knownSelectionItemCount: 0,
    playlistScope: 'current',
    playlistSelectable: false,
    playlistItemLimit: 32,
    outputDirectory: '',
    recognitionText: t('recognizing'),
    recognitionVisible: true,
    messageText: '',
    messageError: false,
    helpUrl: '',
    analysisPhase: 'preparing',
  };
}

function normalizedPlaylistItemLimit(value) {
  return Math.max(1, Math.min(100, Math.round(Number(value) || 32)));
}

function normalizedPostProcessMode(value, fallback = 'off') {
  return ['off', 'download', 'embed'].includes(value) ? value : fallback;
}

function savedChapterMode(saved, fallback = 'off') {
  if (['off', 'download', 'embed'].includes(saved.chapterMode)) return saved.chapterMode;
  if (saved.chapters === 'split') return 'download';
  if (saved.chapters === 'full') return 'off';
  return normalizedPostProcessMode(fallback);
}

function nativeRequest(request) {
  return chrome.runtime.sendMessage({ type: 'YT_DLP_GUI_NATIVE_REQUEST', request }).then(response => {
    if (!response?.ok) {
      const error = new Error(response?.message || t('hostRejected'));
      error.helpUrl = response?.helpUrl || '';
      throw error;
    }
    return response;
  });
}

export function createPopupController() {
  const state = writable(initialState());
  const matcherWorker = new Worker(chrome.runtime.getURL('build/matcher-worker.js'), { type: 'module' });
  let recognitionRevision = 0;
  let matcherFailure = '';

  const patch = values => state.update(current => ({ ...current, ...values }));

  matcherWorker.onerror = event => {
    matcherFailure = event.message || t('matcherIndexLoadFailed');
    patch({
      recognitionPending: false,
      recognitionText: matcherFailure,
      recognitionVisible: true,
    });
  };
  const message = (text = '', error = false, helpUrl = '') => patch({
    messageText: text,
    messageError: error,
    helpUrl,
  });
  const showAnalysisFailure = (rawMessage, helpUrl = '', current = get(state)) => {
    const detail = String(rawMessage || '').trim();
    const normalized = detail.toLowerCase();
    let hostname = '';
    try {
      hostname = new URL(current.sourceUrl).hostname.toLowerCase().replace(/^www\./, '');
    } catch {}
    const snapshot = current.sourcePageSnapshot;
    const snapshotHasMedia = Boolean(snapshot?.mediaKind || Number(snapshot?.mediaElementCount));
    const explicitNoMedia = normalized.includes('no media found');
    const redditGenericBlocked = (hostname === 'reddit.com' || hostname.endsWith('.reddit.com'))
      && Boolean(snapshot)
      && !snapshotHasMedia
      && normalized.includes('[generic]')
      && normalized.includes('http error 403')
      && normalized.includes('blocked');
    const noMedia = explicitNoMedia || redditGenericBlocked;
    message(
      noMedia ? t('noDownloadableMedia') : t('analysisFailed', { message: detail }),
      !noMedia,
      noMedia ? '' : helpUrl,
    );
  };
  const scopedStorageKey = (prefix, current = get(state)) =>
    `${prefix}:tab:${current.sourceTabId}:${current.playlistScope}:${current.sourceUrl}`;

  function subtitleIsAutomatic(item, language) {
    return Boolean(language && (item.subtitles || []).find(
      subtitle => subtitle.language === language,
    )?.automatic);
  }

  function storedDownloadChoice(item, saved = {}) {
    const contentKind = saved.contentKind === 'audio' && formatsForContent(item, 'audio').length
      ? 'audio'
      : 'video';
    const recommended = recommendedDownloadChoice(item, contentKind);
    const chapterIndices = selectedChapterIndicesFromChoice(item, saved);
    const timeRange = chapterIndices.length ? null : downloadTimeRangeFromChoice(saved);
    const thumbnailMode = normalizedPostProcessMode(
      saved.thumbnailMode ?? get(state).analysis?.thumbnailMode,
    );
    const subtitleLanguage = contentKind === 'video' ? saved.subtitleLanguage || '' : '';
    const subtitleMode = subtitleLanguage
      ? normalizedPostProcessMode(
          saved.subtitleMode,
          saved.subtitleLanguage ? 'download' : 'off',
        )
      : 'off';
    const chapterMode = !timeRange && chapterIndices.length === 0 && item.chapterCount > 0
      ? savedChapterMode(saved, get(state).analysis?.chapterMode)
      : 'off';
    const requestedOutputFormat = saved.outputFormat || recommended.outputFormat;
    return {
      ...recommended,
      fileName: typeof saved.fileName === 'string' ? saved.fileName : item.title || '',
      rangeStartSeconds: timeRange?.startSeconds ?? null,
      rangeEndSeconds: timeRange?.endSeconds ?? null,
      chapterIndices,
      videoFormatId: contentKind === 'video'
        ? saved.videoFormat || recommended.videoFormatId
        : '',
      audioFormatId: saved.audioFormat || recommended.audioFormatId,
      subtitleLanguage,
      subtitleAutomatic: contentKind === 'video'
        && subtitleIsAutomatic(item, saved.subtitleLanguage),
      subtitleMode,
      chapterMode,
      splitChapters: ['download', 'embed'].includes(chapterMode),
      useAria2: false,
      thumbnailMode,
      outputFormat: thumbnailMode === 'embed' && requestedOutputFormat === 'webm'
        ? 'mkv'
        : requestedOutputFormat,
    };
  }

  function normalizeChoice(index, requestedKind) {
    const current = get(state);
    const item = current.analysis?.items?.[index];
    if (!item) return null;
    const saved = current.choices[index] || {};
    const chapterIndices = selectedChapterIndicesFromChoice(item, saved);
    const timeRange = chapterIndices.length ? null : downloadTimeRangeFromChoice(saved);
    const kinds = availableContentKinds(item);
    const contentKind = requestedKind && kinds.includes(requestedKind)
      ? requestedKind
      : saved.contentKind === 'audio' && kinds.includes('audio') ? 'audio' : 'video';
    const videoFormats = formatsForContent(item, 'video');
    const audioFormats = contentKind === 'audio'
      ? formatsForContent(item, 'audio')
      : formatsForContent(item, 'audio').filter(format => format.kind === 'audio');
    const videoFormat = videoFormats.some(format => format.id === saved.videoFormat)
      ? saved.videoFormat
      : defaultFormatId(item, 'video');
    const audioFormat = audioFormats.some(format => format.id === saved.audioFormat)
      ? saved.audioFormat
      : defaultFormatId(item, 'audio');
    const requestChoice = {
      ...storedDownloadChoice(item, saved),
      contentKind,
      videoFormatId: contentKind === 'video' ? videoFormat : '',
      audioFormatId: audioFormat,
    };
    const outputs = outputFormatsForChoice(item, requestChoice);
    const derivedOutput = resolvedOutputFormat(item, requestChoice);
    let outputFormat = outputs.includes(saved.outputFormat)
      ? saved.outputFormat
      : outputs.includes(derivedOutput) ? derivedOutput : outputs[0] || '';
    const thumbnailMode = normalizedPostProcessMode(
      saved.thumbnailMode ?? current.analysis?.thumbnailMode,
    );
    const subtitleLanguage = contentKind === 'video' ? saved.subtitleLanguage || '' : '';
    const subtitleMode = subtitleLanguage
      ? normalizedPostProcessMode(
          saved.subtitleMode,
          saved.subtitleLanguage ? 'download' : 'off',
        )
      : 'off';
    const chapterMode = !timeRange && chapterIndices.length === 0 && item.chapterCount > 0
      ? savedChapterMode(saved, current.analysis?.chapterMode)
      : 'off';
    if (thumbnailMode === 'embed' && outputFormat === 'webm') outputFormat = 'mkv';
    return {
      ...saved,
      fileName: typeof saved.fileName === 'string' ? saved.fileName : item.title || '',
      rangeStartSeconds: timeRange?.startSeconds ?? null,
      rangeEndSeconds: timeRange?.endSeconds ?? null,
      chapterIndices,
      contentKind,
      videoFormat,
      audioFormat,
      subtitleLanguage,
      subtitleAutomatic: contentKind === 'video'
        && subtitleIsAutomatic(item, saved.subtitleLanguage),
      subtitleMode,
      chapterMode,
      chapters: ['download', 'embed'].includes(chapterMode) ? 'split' : 'full',
      useAria2: false,
      thumbnailMode,
      outputFormat,
    };
  }

  function persistChoices(choices) {
    patch({ choices });
    chrome.storage.session.set({ [scopedStorageKey('choices')]: choices });
  }

  function setChoice(index, values, normalize = true) {
    const current = get(state);
    const existing = current.choices[index] || {};
    const requested = { ...values };
    if (['off', 'download', 'embed'].includes(requested.chapterMode)) {
      requested.chapters = requested.chapterMode === 'off' ? 'full' : 'split';
      if (requested.chapterMode !== 'off') {
        requested.rangeStartSeconds = null;
        requested.rangeEndSeconds = null;
        requested.chapterIndices = [];
      }
    }
    if (requested.thumbnailMode === 'embed'
        && (requested.outputFormat ?? existing.outputFormat) === 'webm') {
      requested.outputFormat = 'mkv';
    } else if (requested.outputFormat === 'webm'
        && (requested.thumbnailMode ?? existing.thumbnailMode) === 'embed') {
      requested.thumbnailMode = 'download';
    }
    const choices = {
      ...current.choices,
      [index]: { ...existing, ...requested },
    };
    patch({ choices });
    if (normalize) {
      const normalized = normalizeChoice(index, choices[index].contentKind);
      if (normalized) choices[index] = normalized;
    }
    persistChoices({ ...choices });
  }

  function downloadChoiceAvailable(item, saved) {
    if (!item) return false;
    try {
      buildDownloadRequest(item, storedDownloadChoice(item, saved));
      return true;
    } catch {
      return false;
    }
  }

  function selectedDownloadIndices(current = get(state)) {
    const itemCount = current.analysis?.items?.length || 0;
    if (itemCount === 1) return [0];
    return [...current.selectedIndices]
      .filter(index => index >= 0 && index < itemCount)
      .sort((left, right) => left - right);
  }

  function applyPageSnapshotFallback(data, current = get(state)) {
    if (current.playlistScope !== 'current' || data.items?.length !== 1 || !current.sourcePageSnapshot) return;
    const item = data.items[0];
    if (!String(item.title || '').trim() && current.sourcePageSnapshot.title) {
      item.title = current.sourcePageSnapshot.title;
    }
    if (!String(item.thumbnail || '').trim() && current.sourcePageSnapshot.thumbnailUrl) {
      item.thumbnail = current.sourcePageSnapshot.thumbnailUrl;
    }
    if (!String(item.creator || '').trim() && current.sourcePageSnapshot.creator) {
      item.creator = current.sourcePageSnapshot.creator;
    }
    if (!String(item.duration || '').trim()
        && Number.isFinite(current.sourcePageSnapshot.mediaDurationSeconds)
        && current.sourcePageSnapshot.mediaDurationSeconds >= 0) {
      item.duration = String(current.sourcePageSnapshot.mediaDurationSeconds);
    }
  }

  function renderAnalysis(data) {
    const current = get(state);
    applyPageSnapshotFallback(data, current);
    const selectedIndices = new Set(current.selectedIndices);
    for (let index = current.knownSelectionItemCount; index < data.items.length; index += 1) {
      selectedIndices.add(index);
    }
    for (const index of [...selectedIndices]) {
      if (index >= data.items.length) selectedIndices.delete(index);
    }
    const analysisView = data.items.length
      ? 'results'
      : data.analyzing ? 'loading' : 'idle';
    patch({
      analysis: data,
      analysisRunning: Boolean(data.analyzing),
      analysisPhase: data.phase || current.analysisPhase,
      selectedIndices,
      knownSelectionItemCount: Math.max(current.knownSelectionItemCount, data.items.length),
      analysisView,
    });
  }

  async function restoreScopedPageState(expectedRevision = recognitionRevision) {
    const current = get(state);
    const analysisKey = scopedStorageKey('analysis', current);
    const choicesKey = scopedStorageKey('choices', current);
    const selectionKey = scopedStorageKey('selection', current);
    const saved = await chrome.storage.session.get([analysisKey, choicesKey, selectionKey]);
    if (expectedRevision !== recognitionRevision) return;
    const cached = saved[analysisKey];
    const itemCount = cached?.data?.items?.length || 0;
    patch({
      choices: saved[choicesKey] || {},
      selectedIndices: new Set(Array.isArray(saved[selectionKey])
        ? saved[selectionKey]
        : Array.from({ length: itemCount }, (_, index) => index)),
      knownSelectionItemCount: itemCount,
    });
    if (cached?.ok && cached.data?.schemaVersion === ANALYSIS_CACHE_VERSION) {
      renderAnalysis(cached.data);
    } else if (cached) {
      showAnalysisFailure(cached.message, cached.helpUrl);
    }
  }

  function renderTask(task = { state: 'idle' }) {
    const downloadRunning = ['running', 'stopping'].includes(task.state);
    patch({ currentTask: task, downloadRunning });
    if (task.state === 'running') {
      const percent = Math.max(0, Math.min(100, Number(task.percent) || 0));
      const batch = task.itemCount > 1 ? `${task.completedItems + 1} / ${task.itemCount}` : '';
      const detail = [
        batch,
        task.downloaded && task.total ? `${task.downloaded} / ${task.total}` : task.downloaded,
        task.speed,
        task.eta ? t('remaining', { eta: task.eta }) : '',
      ].filter(Boolean).join(' · ');
      message(`${task.title || t('downloading')} · ${percent.toFixed(0)}%${detail ? ` · ${detail}` : ''}`);
    } else if (task.state === 'stopping') {
      message(t('downloadStopping'));
    } else if (task.state === 'stopped') {
      message(t('downloadStopped'));
    } else if (task.state === 'completed') {
      message(task.itemCount > 1
        ? t('completedItems', { count: task.itemCount })
        : t('completedValue', { value: task.outputPath || task.title || t('downloadComplete') }));
    } else if (task.state === 'failed') {
      message(task.message || t('downloadFailed'), true, task.helpUrl);
    }
  }

  async function captureSnapshot(tabId) {
    if (!Number.isInteger(tabId) || !chrome.scripting?.executeScript) return null;
    try {
      const [execution] = await chrome.scripting.executeScript({
        target: { tabId },
        func: collectPageSnapshot,
      });
      const schemaVersion = Number(execution?.result?.schemaVersion);
      return schemaVersion >= 1 && schemaVersion <= 3 ? execution.result : null;
    } catch {
      return null;
    }
  }

  function scopeLabelKey(url, scope) {
    if (scope.value === 'playlist') return 'fullPlaylist';
    try {
      const host = new URL(url).hostname.toLowerCase().replace(/^www\./, '');
      if (host === 'youtube.com' || host.endsWith('.youtube.com') || host === 'youtu.be') {
        return 'currentVideo';
      }
    } catch {}
    return 'currentPage';
  }

  async function recognizeUrl(url) {
    const id = ++recognitionRevision;
    const sourceUrl = url.trim();
    patch({
      recognized: null,
      recognitionPending: true,
      analysis: null,
      choices: {},
      selectedIndices: new Set(),
      knownSelectionItemCount: 0,
      sourceUrl,
      sourceSiteName: t('recognizing'),
      recognitionText: t('recognizing'),
      recognitionVisible: true,
      analysisView: 'idle',
    });
    try {
      validateUrl(sourceUrl);
      const scope = pageDownloadScope(sourceUrl);
      const parsedHostname = new URL(sourceUrl).hostname.toLowerCase();
      const hostname = parsedHostname.startsWith('www.') ? parsedHostname.slice(4) : parsedHostname;
      const snapshotSiteName = String(get(state).sourcePageSnapshot?.siteName || '').trim();
      patch({
        recognized: {
          url: sourceUrl,
          kind: 'unknown',
          key: 'Generic',
          name: t('genericDownload'),
          operation: 'generic',
          approximate: true,
        },
        sourceSiteName: snapshotSiteName || hostname || t('genericDownload'),
        playlistScope: scope.value,
        playlistSelectable: scope.selectable,
        scopeLabel: t(scopeLabelKey(sourceUrl, scope)),
      });
    } catch (error) {
      patch({
        recognized: null,
        recognitionPending: false,
        sourceSiteName: t('unavailable'),
        recognitionText: error.message,
        recognitionVisible: true,
      });
      return;
    }
    if (matcherFailure) {
      patch({
        recognitionPending: false,
        recognitionText: matcherFailure,
        recognitionVisible: true,
      });
    } else {
      matcherWorker.postMessage({ id, url: sourceUrl });
    }
    await restoreScopedPageState(id);
  }

  matcherWorker.onmessage = ({ data }) => {
    if (data.id !== recognitionRevision) return;
    if (data.error) {
      patch({
        recognitionPending: false,
        recognitionText: data.error,
        recognitionVisible: true,
      });
      return;
    }
    const recognized = data.result;
    const matchedName = String(recognized.name || '').trim();
    const current = get(state);
    const snapshotSiteName = String(current.sourcePageSnapshot?.siteName || '').trim();
    let hostname = '';
    try {
      hostname = new URL(current.sourceUrl).hostname.toLowerCase().replace(/^www\./, '');
    } catch {}
    const sourceSiteName = recognized.kind === 'known'
      ? ({ youtube: 'YouTube', reddit: 'Reddit', twitter: 'X / Twitter' }[matchedName.toLowerCase()]
        || matchedName || t('supportedSite'))
      : snapshotSiteName || hostname || t('genericDownload');
    patch({
      recognized,
      recognitionPending: false,
      sourceSiteName,
      recognitionText: recognized.kind === 'known' ? '' : t('generic'),
      recognitionVisible: recognized.kind !== 'known',
    });
  };

  const runtimeListener = value => {
    if (value?.type === 'YT_DLP_GUI_TASK_EVENT') {
      renderTask(value.task);
    } else if (value?.type === ANALYSIS_EVENT && value.key === scopedStorageKey('analysis')) {
      if (value.response?.ok) renderAnalysis(value.response.data);
      else if (value.response) {
        patch({
          analysisRunning: false,
          analysisView: get(state).analysis?.items?.length ? 'results' : 'idle',
        });
        showAnalysisFailure(value.response.message, value.response.helpUrl);
      }
    }
  };

  const storageListener = (changes, area) => {
    if (area === 'local' && changes.v2DownloadDirectory) loadPreferences();
  };

  async function loadPreferences() {
    const saved = await chrome.storage.local.get(['v2DownloadDirectory', 'playlistItemLimit']);
    patch({
      playlistItemLimit: normalizedPlaylistItemLimit(saved.playlistItemLimit),
      outputDirectory: saved.v2DownloadDirectory || '',
    });
  }

  async function loadCurrentTab() {
    try {
      const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
      const sourceTabId = Number.isInteger(tab?.id) ? tab.id : null;
      const sourcePageSnapshot = await captureSnapshot(sourceTabId);
      patch({
        sourceTabId,
        sourcePageSnapshot,
        sourceSiteIconUrl: String(tab?.favIconUrl || sourcePageSnapshot?.iconUrl || '').trim(),
        sourcePageTitle: String(sourcePageSnapshot?.title || tab?.title || '').trim(),
      });
      await recognizeUrl(tab?.url || '');
    } catch (error) {
      message(error.message, true);
    }
  }

  async function restoreTask() {
    const response = await chrome.runtime.sendMessage({ type: 'YT_DLP_GUI_TASK_STATUS' });
    if (response?.ok) renderTask(response.data.task);
  }

  async function analyze() {
    const current = get(state);
    if (current.busy || current.analysisRunning || current.downloadRunning || !current.recognized) return;
    if (!await chrome.permissions.request({ origins: [new URL(current.sourceUrl).origin + '/*'] })) {
      message(t('permissionRequired'), true);
      return;
    }
    patch({ busy: true, analysisView: 'loading', analysisPhase: 'preparing' });
    message();
    try {
      const response = await nativeRequest({
        action: 'analyze',
        url: current.sourceUrl,
        playlistScope: current.playlistScope,
        itemLimit: current.playlistItemLimit,
        sourceTabId: current.sourceTabId,
      });
      if (current.sourceUrl === get(state).sourceUrl) renderAnalysis(response.data);
    } catch (error) {
      patch({ analysisView: 'idle' });
      showAnalysisFailure(error.message, error.helpUrl);
    } finally {
      patch({ busy: false });
    }
  }

  async function stopAnalysis() {
    const current = get(state);
    if (current.busy || !current.analysisRunning) return;
    patch({ busy: true });
    try {
      const response = await nativeRequest({
        action: 'cancelAnalysis',
        url: current.sourceUrl,
        playlistScope: current.playlistScope,
        sourceTabId: current.sourceTabId,
      });
      renderAnalysis(response.data);
      message(t('analysisStopped', { count: response.data.items.length }));
    } catch (error) {
      message(t('stopFailed', { message: error.message }), true);
    } finally {
      patch({ busy: false });
    }
  }

  async function downloadSelected() {
    const current = get(state);
    if (current.busy || current.downloadRunning) return;
    const indices = selectedDownloadIndices(current);
    if (!indices.length) return;
    const requests = indices.map(index => ({
      ...buildDownloadRequest(
        current.analysis.items[index],
        storedDownloadChoice(current.analysis.items[index], current.choices[index]),
      ),
      sourcePageUrl: current.sourceUrl,
      sourceTabId: current.sourceTabId,
      sourceItemIndex: index,
    }));
    const origins = [...new Set(requests.map(request => new URL(request.url).origin + '/*'))];
    if (!await chrome.permissions.request({ origins })) {
      message(t('permissionRequired'), true);
      return;
    }
    patch({ busy: true });
    message(requests.length > 1
      ? t('preparingDownloads', { count: requests.length })
      : t('preparingDownload', { title: requests[0].title || requests[0].url }));
    try {
      const response = await nativeRequest({ action: 'downloadBatch', requests });
      renderTask(response.data.task);
    } catch (error) {
      message(error.message, true, error.helpUrl);
    } finally {
      patch({ busy: false });
    }
  }

  async function stopDownload() {
    const current = get(state);
    if (current.busy || !current.downloadRunning || current.currentTask?.state === 'stopping') return;
    patch({ busy: true });
    try {
      const response = await nativeRequest({ action: 'cancelDownload' });
      renderTask(response.data.task);
    } catch (error) {
      message(t('downloadStopFailed', { message: error.message }), true);
    } finally {
      patch({ busy: false });
    }
  }

  async function setPlaylistScope(playlistScope) {
    const current = get(state);
    if (current.busy || current.downloadRunning || current.playlistScope === playlistScope) return;
    patch({
      playlistScope,
      analysis: null,
      choices: {},
      selectedIndices: new Set(),
      knownSelectionItemCount: 0,
      analysisView: 'idle',
    });
    message();
    await restoreScopedPageState();
  }

  async function setPlaylistItemLimit(value) {
    const playlistItemLimit = normalizedPlaylistItemLimit(value);
    patch({ playlistItemLimit });
    await chrome.storage.local.set({ playlistItemLimit });
  }

  function toggleSelection(index, selected) {
    const current = get(state);
    const selectedIndices = new Set(current.selectedIndices);
    if (selected) selectedIndices.add(index);
    else selectedIndices.delete(index);
    patch({ selectedIndices });
    chrome.storage.session.set({
      [scopedStorageKey('selection')]: [...selectedIndices].sort((a, b) => a - b),
    });
  }

  function prepareDetail(index) {
    const choice = normalizeChoice(index);
    if (!choice) return;
    persistChoices({ ...get(state).choices, [index]: choice });
  }

  async function chooseDirectory() {
    const current = get(state);
    if (current.busy || current.analysisRunning || current.downloadRunning) return;
    patch({ busy: true });
    try {
      await nativeRequest({ action: 'chooseDirectory' });
      await loadPreferences();
    } catch (error) {
      message(error.message, true);
    } finally {
      patch({ busy: false });
    }
  }

  async function openDirectory() {
    const current = get(state);
    if (
      current.busy
      || current.analysisRunning
      || current.downloadRunning
      || !current.outputDirectory
    ) return;
    patch({ busy: true });
    try {
      await nativeRequest({ action: 'openDirectory' });
    } catch (error) {
      message(t('openFolderFailed', { message: error.message }), true);
    } finally {
      patch({ busy: false });
    }
  }

  function destroy() {
    matcherWorker.terminate();
    chrome.runtime.onMessage.removeListener(runtimeListener);
    chrome.storage.onChanged.removeListener(storageListener);
  }

  async function init() {
    document.documentElement.lang = chrome.i18n.getUILanguage?.() || 'en';
    chrome.runtime.onMessage.addListener(runtimeListener);
    chrome.storage.onChanged.addListener(storageListener);
    await Promise.all([loadPreferences(), loadCurrentTab(), restoreTask()]);
  }

  return {
    subscribe: state.subscribe,
    init,
    destroy,
    analyze,
    stopAnalysis,
    downloadSelected,
    stopDownload,
    setPlaylistScope,
    setPlaylistItemLimit,
    toggleSelection,
    prepareDetail,
    setChoice,
    chooseDirectory,
    openDirectory,
    selectedDownloadIndices,
    storedDownloadChoice,
    downloadChoiceAvailable,
  };
}
