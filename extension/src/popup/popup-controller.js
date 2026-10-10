import { get, writable } from 'svelte/store';
import { validateUrl } from '../shared/url-validation.js';
import { builtinSiteIconPath } from '../shared/builtin-site-icons.js';
import { galleryPostGroups, galleryAlbumPayload } from '../shared/gallery-post-groups.js';
import { pageCacheUrl } from '../shared/page-cache-key.js';
import { ANALYSIS_CACHE_VERSION, preferFreshUnifiedAnalysisResponse } from '../shared/analysis-cache.js';
import {
  GALLERY_DOWNLOAD_STORAGE_KEY,
  normalizeGalleryDownloadMode,
  isBrowserGalleryDownload,
  galleryDownloadOrigins,
  galleryNativeRequests,
} from '../shared/gallery-download-mode.js';
import { normalizeRecognitionForAnalysis } from '../shared/analysis-backend-routing.js';
import { completeBackendFailure, pendingBackendAnalyses, shouldAcceptBackendUpdate } from '../shared/analysis-state.js';
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
import { capturePagePreview } from '../shared/capture-page-preview.js';
import { createPopupPageSource } from './popup-page-source.js';
import { downloadTimeRangeFromChoice } from '../shared/download-time-range.js';
import { selectedChapterIndicesFromChoice } from '../shared/download-chapter-range.js';
import { NATIVE_HOST_HELP_URL } from '../shared/native-host-errors.js';
import { t } from '../shared/i18n.js';
import { sitePermissionOrigins } from '../shared/site-domain.js';
import { ensureSitePermissions } from '../shared/site-permission-request.js';
import { popupResultItems, selectedPopupResults } from './popup-result-list.js';
import { ANALYSIS_TOOLS_KEY, ANALYSIS_DETECTION_KEY, normalizedAnalysisTools,
  enabledAnalysisTools, detectedAnalysisTools, initialToolsFromDetection } from '../shared/analysis-tool-preferences.js';
import { ALBUM_WHEEL_KEY, normalizedAlbumWheelMode } from '../shared/album-preferences.js';
import { rangeForTimeline, timelineRange } from '../shared/gallery-timeline-range.js';

const ANALYSIS_EVENT = 'YT_DLP_GUI_ANALYSIS_EVENT';

function initialState() {
  return {
    analysisView: 'idle',
    recognized: null,
    recognitionPending: true,
    analysis: null,
    unifiedAnalysis: null,
    analysisStorageScope: null,
    analysisByBackend: { y: null, g: null },
    analysisErrors: { y: '', g: '' },
    selectionsByBackend: { y: new Set(), g: new Set() },
    knownCountsByBackend: { y: 0, g: 0 },
    busy: false,
    analysisRunning: false,
    downloadRunning: false,
    currentTask: { state: 'idle' },
    browserDownloadTask: { state: 'idle' },
    sourceUrl: '',
    sourceSiteName: t('recognizing'),
    sourceSiteIconUrl: '',
    sourcePageTitle: '',
    sourcePageSnapshot: null,
    pageResetRevision: 0,
    sourceTabId: null,
    choices: {},
    playlistScope: 'current',
    playlistSelectable: false,
    playlistItemLimit: 32,
    outputDirectory: '',
    galleryDownloadMode: 'browser-files',
    galleryModes: {},
    galleryRanges: {},
    recognitionText: t('recognizing'),
    recognitionVisible: true,
    messageText: '',
    messageError: false,
    helpUrl: '',
    analysisPhase: 'preparing',
    cookieSaveRunning: false,
    cookieSaveMessage: '',
    cookieSaveError: false,
    preferencesLoaded: false,
    albumWheelMode: 'zoom',
    albumPreferenceSaving: false,
    albumSettingsMessage: '',
    analysisTools: normalizedAnalysisTools(),
    toolDetection: null,
    toolDetectionRunning: false,
    toolPreferenceSaving: false,
    toolSettingsMessage: '',
    toolSettingsError: false,
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
  const matcherIndexUrl = chrome.runtime.getURL('resources/extractor-index.json');
  let recognitionRevision = 0;
  let restoreRevision = 0;
  let matcherFailure = '';
  let destroyed = false;

  const patch = values => state.update(current => ({ ...current, ...values }));
  const pageSource = createPopupPageSource({
    captureSnapshot: capturePagePreview,
    onPage(tab, changed, documentReset) {
      patch({ sourceTabId: tab.id, sourceUrl: tab.url || '' });
      if (!changed) return;
      const current = get(state);
      patch({
        pageResetRevision: current.pageResetRevision + (documentReset ? 1 : 0),
        sourcePageSnapshot: null,
        sourcePageTitle: '',
        sourceSiteIconUrl: chrome.runtime.getURL(builtinSiteIconPath(tab.url || '') || 'resources/site-icons/Web.svg'),
        analysisRunning: false,
        busy: current.analysisRunning ? false : current.busy,
        messageText: '', messageError: false, helpUrl: '',
      });
      void recognizeUrl(tab.url || '').catch(error => {
        if (!destroyed && get(state).sourceUrl === (tab.url || '')) message(error.message, true);
      });
    },
    onSnapshot(tab, snapshot) {
      patch({ sourcePageSnapshot: snapshot,
        sourcePageTitle: String(snapshot?.title || tab.title || '').trim() });
    },
    onError(error) { message(error.message, true); },
  });

  matcherWorker.onerror = event => {
    matcherFailure = event.message || t('matcherIndexLoadFailed');
    if (destroyed) return;
    patch({
      recognized: null,
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
    `${prefix}:tab:${current.sourceTabId}:${current.analysisStorageScope ?? current.playlistScope}:${pageCacheUrl(current.sourceUrl)}`;

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

  function toggleGalleryGroup(index, selected) {
    const current = get(state);
    const group = galleryPostGroups(current.analysisByBackend.g?.items, current.sourceUrl)[index];
    if (!group) return;
    const selectedIndices = new Set(current.selectionsByBackend.g);
    for (const sourceIndex of group.sourceItemIndices) {
      if (selected) selectedIndices.add(sourceIndex);
      else selectedIndices.delete(sourceIndex);
    }
    patch({ selectionsByBackend: {
      ...current.selectionsByBackend, g: new Set(selectedIndices),
    } });
    chrome.storage.session.set({
      [`${scopedStorageKey('selection', current)}:g`]: [...selectedIndices].sort((a, b) => a - b),
    });
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

  // One Native Host analysis document; backend views below are projections for
  // existing video-format and gallery-post UI, never separate analysis jobs.
  function renderUnifiedAnalysisResponse(response) {
    if (!response?.ok || !response.data?.request) {
      if (response?.message) showAnalysisFailure(response.message, response.helpUrl);
      return;
    }
    const document = response.data;
    const current = get(state);
    const live = current.unifiedAnalysis ? { ok: true, data: current.unifiedAnalysis } : null;
    if (preferFreshUnifiedAnalysisResponse(live, response)?.data !== document) return;
    patch({ unifiedAnalysis: document,
      analysisStorageScope: document.request.playlistScope ?? current.analysisStorageScope ?? current.playlistScope });
    for (const backend of ['y', 'g']) {
      const group = document[backend === 'y' ? 'yt-dlp' : 'gallery-dl'];
      const status = group?.status || 'pending';
      const info = group?.info || {};
      const pending = ['pending', 'analyzing'].includes(status);
      const mapped = {
        ...info, items: group?.items || [], backend,
        status, analyzing: pending,
        phase: info.phase || (pending ? 'preparing' : 'complete'),
        host: document.request.host,
        schemaVersion: ANALYSIS_CACHE_VERSION,
        startedAt: document.request.startedAt,
        updatedAt: document.request.updatedAt,
      };
      renderBackendAnalysis(backend, mapped);
      patch({ analysisErrors: { ...get(state).analysisErrors,
        [backend]: ['failed', 'interrupted'].includes(status)
          ? info.message || document.request.message || t('nativeAnalysisFailed') : '' } });
    }
    const available = ['y', 'g'].filter(backend =>
      !['notMatched', 'disabled', 'pending'].includes(document[backend === 'y' ? 'yt-dlp' : 'gallery-dl']?.status));
    const running = document.request.status === 'analyzing';
    patch({ analysisRunning: running });
    if (!running && document['yt-dlp']?.info?.playlistRequired) {
      patch({ playlistScope: 'playlist', playlistSelectable: false, scopeLabel: t('fullPlaylist') });
    }
    if (!running && !available.length) {
      patch({ analysis: null, analysisView: 'idle', analysisPhase: 'complete' });
      message(t('noMatchingExtractor'), true);
    } else if (!running && available.length && !available.some(backend =>
      get(state).analysisByBackend[backend]?.items?.length)) {
      const detail = available.map(backend => {
        const group = document[backend === 'y' ? 'yt-dlp' : 'gallery-dl'];
        return group?.info?.message || '';
      }).filter(Boolean).join(' / ');
      if (detail && !Object.values(get(state).analysisErrors).some(Boolean)) showAnalysisFailure(detail, response.helpUrl);
    }
  }

  function renderBackendAnalysis(backend, data) {
    if (!['y', 'g'].includes(backend)) return;
    const current = get(state);
    if (!shouldAcceptBackendUpdate(current, backend, data)) return;
    if (backend === 'y') applyPageSnapshotFallback(data, current);
    const analysisByBackend = { ...current.analysisByBackend, [backend]: data };
    const selectedIndices = new Set(current.selectionsByBackend[backend]);
    const knownCount = current.knownCountsByBackend[backend] || 0;
    for (let index = knownCount; index < data.items.length; index += 1) {
      selectedIndices.add(index);
    }
    for (const index of selectedIndices) {
      if (index >= data.items.length) selectedIndices.delete(index);
    }
    const running = Object.values(analysisByBackend).some(result => Boolean(result?.analyzing));
    patch({
      analysisByBackend,
      // Existing yt-dlp settings pages keep an explicit yt-dlp-only projection.
      analysis: analysisByBackend.y,
      selectionsByBackend: { ...current.selectionsByBackend, [backend]: selectedIndices },
      knownCountsByBackend: { ...current.knownCountsByBackend, [backend]: Math.max(knownCount, data.items.length) },
      analysisPhase: Object.values(analysisByBackend).find(result => result?.analyzing)?.phase || 'complete',
      analysisView: Object.values(analysisByBackend).some(result => result?.items?.length)
        ? 'results' : running ? 'loading' : 'idle',
      analysisRunning: running,
    });
    if (!data.analyzing && !data.stopped) {
      const key = `${scopedStorageKey('selection')}${backend === 'g' ? ':g' : ''}`;
      chrome.storage.session.set({ [key]: [...selectedIndices].sort((a, b) => a - b) });
    }
  }

  async function restoreScopedPageState(expectedRevision = recognitionRevision) {
    const expectedRestore = ++restoreRevision;
    const current = get(state);
    if (!Number.isInteger(current.sourceTabId) || !current.sourceUrl) return;
    const choicesKey = scopedStorageKey('choices', current);
    const selectionKey = scopedStorageKey('selection', current);
    const galleryModesKey = scopedStorageKey('galleryModes', current);
    // The background is the single authority for analysis state. There is no
    // popup-owned cache and no second copy of a completed result in local.
    const [saved, snapshot] = await Promise.all([
      chrome.storage.session.get([
        choicesKey, selectionKey, `${selectionKey}:g`, galleryModesKey, scopedStorageKey('galleryRanges', current),
      ]),
      chrome.runtime.sendMessage({
        type: 'YT_DLP_GUI_ANALYSIS_SNAPSHOT',
        request: {
          sourceTabId: current.sourceTabId,
          playlistScope: current.analysisStorageScope ?? current.playlistScope,
          url: current.sourceUrl,
        },
      }).catch(() => null),
    ]);
    if (destroyed || expectedRevision !== recognitionRevision || expectedRestore !== restoreRevision) return;
    const previous = get(state);
    if (previous.unifiedAnalysis !== current.unifiedAnalysis) return; // A live frame already owns newer state.
    if (previous.sourceTabId !== current.sourceTabId
        || previous.playlistScope !== current.playlistScope
        || pageCacheUrl(previous.sourceUrl) !== pageCacheUrl(current.sourceUrl)) return;

    const restoredSelected = {
      y: new Set(saved[selectionKey] || []),
      g: new Set(saved[`${selectionKey}:g`] || []),
    };
    patch({
      choices: saved[choicesKey] || {},
      galleryModes: saved[galleryModesKey] || {},
      galleryRanges: saved[scopedStorageKey('galleryRanges', current)] || {},
      selectionsByBackend: restoredSelected,
      knownCountsByBackend: {
        y: Array.isArray(saved[selectionKey]) ? snapshot?.data?.data?.['yt-dlp']?.items?.length || 0 : 0,
        g: Array.isArray(saved[`${selectionKey}:g`]) ? snapshot?.data?.data?.['gallery-dl']?.items?.length || 0 : 0,
      },
      analysisStorageScope: current.analysisStorageScope ?? current.playlistScope,
      analysisByBackend: { y: null, g: null },
      unifiedAnalysis: null,
      analysisErrors: { y: '', g: '' },
    });
    if (snapshot?.ok && snapshot.data) renderUnifiedAnalysisResponse(snapshot.data);
  }

  function renderTask(task = { state: 'idle' }) {
    const downloadRunning = ['running', 'stopping'].includes(task.state)
      || ['running', 'stopping'].includes(get(state).browserDownloadTask.state);
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
      unifiedAnalysis: null,
      analysisStorageScope: null,
      analysisByBackend: { y: null, g: null },
      analysisErrors: { y: '', g: '' },
      selectionsByBackend: { y: new Set(), g: new Set() },
      knownCountsByBackend: { y: 0, g: 0 },
      choices: {},
      galleryModes: {},
      galleryRanges: {},
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
      matcherWorker.postMessage({ id, url: sourceUrl, indexUrl: matcherIndexUrl });
    }
    await restoreScopedPageState(id);
  }

  matcherWorker.onmessage = async ({ data }) => {
    if (destroyed || data.id !== recognitionRevision) return;
    if (data.error) {
      patch({
        recognized: null,
        recognitionPending: false,
        recognitionText: data.error,
        recognitionVisible: true,
      });
      return;
    }
    const recognized = normalizeRecognitionForAnalysis(data.result, get(state).sourceUrl);
    const matchedName = String(recognized.name || '').trim();
    const current = get(state);
    const snapshotSiteName = String(current.sourcePageSnapshot?.siteName || '').trim();
    const scope = current.unifiedAnalysis?.['yt-dlp']?.info?.playlistRequired
      ? { value: 'playlist', selectable: false }
      : pageDownloadScope(current.sourceUrl, recognized.y?.sourceScope);
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
      playlistScope: scope.value,
      playlistSelectable: scope.selectable,
      scopeLabel: t(scopeLabelKey(current.sourceUrl, scope)),
      recognitionPending: false,
      sourceSiteName,
      recognitionText: recognized.kind === 'known' ? '' : t('generic'),
      recognitionVisible: recognized.kind !== 'known',
    });
    if (scope.value !== current.playlistScope && !current.unifiedAnalysis) {
      patch({ analysisStorageScope: scope.value });
      await restoreScopedPageState(data.id);
    }
  };

  function renderBrowserDownloadTask(task = { state: 'idle' }) {
    const current = get(state);
    if ((task.revision || 0) < (current.browserDownloadTask.revision || 0)) return;
    patch({ browserDownloadTask: task, downloadRunning: ['running', 'stopping'].includes(task.state)
      || ['running', 'stopping'].includes(current.currentTask.state) });
    if (task.state === 'failed') message(task.message || t('downloadFailed'), true);
  }

  const runtimeListener = value => {
    if (value?.type === 'YT_DLP_GUI_TASK_EVENT') {
      renderTask(value.task);
    } else if (value?.type === 'YT_DLP_GUI_BROWSER_DOWNLOAD_EVENT') {
      renderBrowserDownloadTask(value.task);
    } else if (value?.type === ANALYSIS_EVENT) {
      if (value.key === scopedStorageKey('analysis')) {
        renderUnifiedAnalysisResponse(value.response);
      }
    }
  };

  const storageListener = (changes, area) => {
    if (area === 'local' && (changes.v2DownloadDirectory || changes[GALLERY_DOWNLOAD_STORAGE_KEY]
        || changes[ANALYSIS_TOOLS_KEY] || changes[ANALYSIS_DETECTION_KEY] || changes[ALBUM_WHEEL_KEY])) {
      loadPreferences();
    }
  };

  async function loadPreferences() {
    const saved = await chrome.storage.local.get([
      'v2DownloadDirectory', 'playlistItemLimit', GALLERY_DOWNLOAD_STORAGE_KEY,
      ANALYSIS_TOOLS_KEY, ANALYSIS_DETECTION_KEY, ALBUM_WHEEL_KEY,
    ]);
    patch({
      playlistItemLimit: normalizedPlaylistItemLimit(saved.playlistItemLimit),
      outputDirectory: saved.v2DownloadDirectory || '',
      galleryDownloadMode: normalizeGalleryDownloadMode(saved[GALLERY_DOWNLOAD_STORAGE_KEY]),
      analysisTools: normalizedAnalysisTools(saved[ANALYSIS_TOOLS_KEY]),
      albumWheelMode: normalizedAlbumWheelMode(saved[ALBUM_WHEEL_KEY]),
      toolDetection: saved[ANALYSIS_DETECTION_KEY] || null,
      preferencesLoaded: true,
    });
  }

  async function setAlbumWheelMode(value) {
    const current = get(state);
    if (!current.preferencesLoaded || current.albumPreferenceSaving) return;
    const albumWheelMode = normalizedAlbumWheelMode(value);
    patch({ albumWheelMode, albumPreferenceSaving: true, albumSettingsMessage: '' });
    try { await chrome.storage.local.set({ [ALBUM_WHEEL_KEY]: albumWheelMode }); }
    catch (error) { patch({ albumWheelMode: current.albumWheelMode,
      albumSettingsMessage: t('albumSettingsFailed', { message: error.message }) }); }
    finally { patch({ albumPreferenceSaving: false }); }
  }

  async function setAnalysisTool(tool, enabled) {
    const current = get(state);
    if (!['y', 'g'].includes(tool) || !current.preferencesLoaded || current.busy
        || current.analysisRunning || current.downloadRunning || current.toolDetectionRunning || current.toolPreferenceSaving) return;
    const analysisTools = { ...current.analysisTools, [tool]: Boolean(enabled) };
    patch({ analysisTools, toolPreferenceSaving: true });
    try {
      await chrome.storage.local.set({ [ANALYSIS_TOOLS_KEY]: analysisTools });
      patch({ analysisTools, toolSettingsMessage: '', toolSettingsError: false });
    } catch (error) {
      patch({ analysisTools: current.analysisTools, toolSettingsMessage: t('toolSettingsFailed', { message: error.message }), toolSettingsError: true });
    } finally {
      patch({ toolPreferenceSaving: false });
    }
  }

  async function detectAnalysisTools() {
    const current = get(state);
    if (!current.preferencesLoaded || current.busy || current.analysisRunning
        || current.downloadRunning || current.toolDetectionRunning || current.toolPreferenceSaving) return;
    patch({ toolDetectionRunning: true, toolSettingsMessage: '', toolSettingsError: false });
    try {
      const response = await nativeRequest({ action: 'status' });
      const detection = detectedAnalysisTools(response.data);
      const saved = await chrome.storage.local.get(ANALYSIS_TOOLS_KEY);
      await chrome.storage.local.set({
        [ANALYSIS_DETECTION_KEY]: detection,
        ...(saved[ANALYSIS_TOOLS_KEY] == null
          ? { [ANALYSIS_TOOLS_KEY]: initialToolsFromDetection(detection) } : {}),
      });
      await loadPreferences();
      patch({ toolSettingsMessage: detection.g == null ? t('toolDetectionLegacy') : t('toolDetectionDone') });
    } catch (error) {
      patch({ toolSettingsMessage: t('toolSettingsFailed', { message: error.message }), toolSettingsError: true });
    } finally {
      patch({ toolDetectionRunning: false });
    }
  }

  function galleryModeForGroup(group, current = get(state)) {
    return normalizeGalleryDownloadMode(current.galleryModes[group.key] ?? current.galleryDownloadMode);
  }

  async function setGalleryRange(key, value) {
    const current = get(state);
    if (current.busy || current.downloadRunning || current.analysisRunning) return;
    const galleryRanges = { ...current.galleryRanges, [key]: timelineRange(value) };
    patch({ galleryRanges });
    try { await chrome.storage.session.set({ [scopedStorageKey('galleryRanges', current)]: galleryRanges }); }
    catch (error) { patch({ galleryRanges: current.galleryRanges }); message(error.message, true); }
  }

  function albumForGroup(group, current) {
    const gallery = galleryAlbumPayload([group], current.sourceSiteName, current.sourceUrl);
    if (gallery.timeline) gallery.timeline = { ...gallery.timeline,
      range: rangeForTimeline(group, current), sourceTabId: current.sourceTabId };
    return gallery;
  }

  async function setGalleryDownloadMode(mode, postKey) {
    const current = get(state);
    const group = galleryPostGroups(current.analysisByBackend.g?.items, current.sourceUrl)
      .find(item => item.key === postKey);
    if (!group || current.downloadRunning || current.analysisRunning || current.busy) return;
    const value = normalizeGalleryDownloadMode(mode);
    // Keep the displayed methods of other current posts unchanged when the
    // extension-wide default is updated for the next analyzed post.
    const galleryModes = { ...current.galleryModes };
    for (const item of galleryPostGroups(current.analysisByBackend.g?.items, current.sourceUrl)) {
      galleryModes[item.key] = galleryModeForGroup(item, current);
    }
    galleryModes[postKey] = value;
    patch({ galleryDownloadMode: value, galleryModes });
    try {
      await Promise.all([
        chrome.storage.local.set({ [GALLERY_DOWNLOAD_STORAGE_KEY]: value }),
        chrome.storage.session.set({ [scopedStorageKey('galleryModes', current)]: galleryModes }),
      ]);
    } catch (error) {
      message(t('galleryDownloadChoiceSaveFailed', { message: error.message }), true);
    }
  }

  async function restoreTask() {
    await Promise.all([
      chrome.runtime.sendMessage({ type: 'YT_DLP_GUI_TASK_STATUS' })
        .then(response => { if (response?.ok) renderTask(response.data.task); }),
      chrome.runtime.sendMessage({ type: 'YT_DLP_GUI_BROWSER_DOWNLOAD_STATUS' })
        .then(response => { if (response?.ok) renderBrowserDownloadTask(response.task); }),
    ]);
  }

  async function analyze(retryBackend = null) {
    const current = get(state);
    const expectedRevision = recognitionRevision;
    if (!current.preferencesLoaded || current.toolDetectionRunning || current.toolPreferenceSaving || current.busy
        || current.analysisRunning || current.downloadRunning || current.recognitionPending || !current.recognized) return;
    if (retryBackend != null && (!['y', 'g'].includes(retryBackend) || !current.analysisErrors[retryBackend])) return;
    const analysisTools = retryBackend ? [retryBackend] : enabledAnalysisTools(current.analysisTools);
    if (!analysisTools.length) { message(t('selectAnalysisTool'), true); return; }
    if (!await ensureSitePermissions([new URL(current.sourceUrl).origin + '/*'])) {
      message(t('permissionRequired'), true);
      return;
    }
    if (destroyed || expectedRevision !== recognitionRevision) return;
    const latest = get(state);
    if (latest.busy || latest.analysisRunning || latest.downloadRunning
        || latest.recognitionPending || latest.recognized !== current.recognized
        || latest.playlistScope !== current.playlistScope
        || (retryBackend && latest.unifiedAnalysis !== current.unifiedAnalysis)) return;
    const requestScope = retryBackend
      ? current.analysisStorageScope ?? current.playlistScope : current.playlistScope;
    restoreRevision += 1; // A new command supersedes any in-flight initial snapshot.
    const pending = pendingBackendAnalyses(analysisTools, current.playlistItemLimit);
    const analysisByBackend = retryBackend
      ? { ...current.analysisByBackend, [retryBackend]: pending[retryBackend] } : pending;
    patch({
      busy: true,
      analysisStorageScope: requestScope,
      unifiedAnalysis: retryBackend ? current.unifiedAnalysis : null,
      analysis: analysisByBackend.y,
      analysisByBackend,
      analysisRunning: true,
      analysisErrors: retryBackend ? { ...current.analysisErrors, [retryBackend]: '' } : { y: '', g: '' },
      selectionsByBackend: retryBackend
        ? { ...current.selectionsByBackend, [retryBackend]: new Set() } : { y: new Set(), g: new Set() },
      knownCountsByBackend: retryBackend
        ? { ...current.knownCountsByBackend, [retryBackend]: 0 } : { y: 0, g: 0 },
      analysisView: 'loading',
      analysisPhase: 'preparing',
    });
    message();
    try {
      const response = await nativeRequest({
        action: 'analyzeIndexedSources',
        analysisTools,
        ...(retryBackend ? { retryBackend, retryStartedAt: current.unifiedAnalysis?.request?.startedAt } : {}),
        url: current.sourceUrl,
        playlistScope: requestScope,
        itemLimit: current.playlistItemLimit,
        sourceTabId: current.sourceTabId,
      });
      if (!destroyed && expectedRevision === recognitionRevision && current.sourceUrl === get(state).sourceUrl) {
        renderUnifiedAnalysisResponse(response.data);
      }
    } catch (error) {
      if (!destroyed && expectedRevision === recognitionRevision && current.sourceUrl === get(state).sourceUrl) {
        if (retryBackend) {
          const { switchTo: _switchTo, ...failed } = completeBackendFailure(get(state), retryBackend, error.message);
          patch(failed);
        } else showAnalysisFailure(error.message, error.helpUrl);
      }
    } finally {
      if (!destroyed && expectedRevision === recognitionRevision && current.sourceUrl === get(state).sourceUrl) {
        patch({ busy: false });
      }
    }
  }

  async function stopAnalysis() {
    const current = get(state);
    if (current.busy || !current.analysisRunning) return;
    patch({ busy: true });
    try {
      try {
        const response = await nativeRequest({
          action: 'cancelAnalysis',
          url: current.sourceUrl,
          playlistScope: current.analysisStorageScope ?? current.playlistScope,
          sourceTabId: current.sourceTabId,
        });
        renderUnifiedAnalysisResponse(response.data);
      } catch (error) {
        showAnalysisFailure(error.message, error.helpUrl);
      }
    } finally {
      patch({ busy: false });
    }
  }

  async function downloadSelected() {
    const current = get(state);
    if (current.busy || current.downloadRunning || current.analysisRunning) return;
    const selected = selectedPopupResults(current);
    if (!selected.length) return;
    try {
      const byMode = selected.filter(result => result.backend === 'g')
        .map(result => ({ group: result.item, mode: galleryModeForGroup(result.item, current), key: result.key, index: result.index }));
      const nativeGroups = byMode.filter(({ mode }) => !isBrowserGalleryDownload(mode));
      const browserDownloads = byMode.filter(({ mode }) => isBrowserGalleryDownload(mode));
      const capabilities = current.analysisByBackend.g?.host?.capabilities || [];
      if (byMode.some(({ group }) => group.sourceScope === 'timeline') && !capabilities.includes('gallery.timeline.v1')) {
        message(t('galleryTimelineUpgradeRequired'), true);
        return;
      }
      if (nativeGroups.length && !capabilities.includes('download.gallery-post.v1')) {
        message(t('nativeGalleryDownloadUpgradeRequired'), true);
        return;
      }
      const galleryRequests = nativeGroups.flatMap(({ group, key, index }) =>
        galleryNativeRequests([{ ...group, index }], current)
          .map(request => ({ ...request, sourceItemKey: key,
            sourceAnalysisStartedAt: current.unifiedAnalysis?.request?.startedAt })));
      if (galleryRequests.length !== nativeGroups.length) {
        message(t('galleryDownloadUrlUnavailable'), true);
        return;
      }
      const videoRequests = selected.filter(result => result.backend === 'y').map(result => ({
        ...buildDownloadRequest(result.item, storedDownloadChoice(result.item, current.choices[result.index])),
        sourcePageUrl: current.sourceUrl,
        sourceTabId: current.sourceTabId,
        sourceItemIndex: result.index,
        sourceItemKey: result.key,
        sourceAnalysisStartedAt: current.unifiedAnalysis?.request?.startedAt,
      }));
      const requests = [...videoRequests, ...galleryRequests];
      if (requests.length > 100) {
        message(t('downloadListInvalid'), true);
        return;
      }
      const browserGroups = browserDownloads.map(({ group }) => group);
      const origins = [...new Set([
        ...galleryDownloadOrigins(browserGroups),
        ...videoRequests.map(request => `${new URL(request.url).origin}/*`),
      ])];
      if (!await ensureSitePermissions(origins)) {
        message(t('permissionRequired'), true);
        return;
      }
      const asAlbum = group => albumForGroup(group, current);
      patch({ busy: true });
      try {
        if (browserDownloads.length) {
          const response = await chrome.runtime.sendMessage({
            type: 'YT_DLP_GUI_BROWSER_DOWNLOAD_START',
            jobs: browserDownloads.map(({ group, mode, key }) => ({ gallery: asAlbum(group), mode, key })),
            sourceTabId: current.sourceTabId, sourcePageUrl: current.sourceUrl,
            sourceAnalysisStartedAt: current.unifiedAnalysis?.request?.startedAt,
          });
          if (!response?.ok) throw new Error(response?.message || t('browserDownloadUnavailable'));
          renderBrowserDownloadTask(response.task);
        }
        if (requests.length) {
          const response = await nativeRequest({ action: 'downloadBatch', requests });
          renderTask(response.data.task);
        }
      } catch (error) {
        message(error.message || t('browserDownloadUnavailable'), true, error.helpUrl);
      } finally { patch({ busy: false }); }
    } catch (error) {
      message(error.message, true, error.helpUrl);
    }
  }

  async function openGalleryAlbum(index) {
    const current = get(state);
    const item = galleryPostGroups(current.analysisByBackend.g?.items, current.sourceUrl)[index];
    if (!item) return;
    try {
      if (item.sourceScope === 'timeline' && !current.analysisByBackend.g?.host?.capabilities?.includes('gallery.timeline.v1')) throw Error(t('galleryTimelineUpgradeRequired'));
      if (!await ensureSitePermissions(galleryDownloadOrigins([item]))) {
        message(t('permissionRequired'), true);
        return;
      }
      const response = await chrome.runtime.sendMessage({
        type: 'YT_DLP_GUI_OPEN_GALLERY',
        gallery: albumForGroup(item, current),
      });
      if (!response?.ok) throw new Error(response?.message || t('galleryAlbumOpenFailed'));
    } catch (error) {
      message(error.message || t('galleryAlbumOpenFailed'), true);
    }
  }

  async function openGalleryAlbumAll() {
    const current = get(state);
    const groups = galleryPostGroups(current.analysisByBackend.g?.items, current.sourceUrl);
    if (groups.length < 2 || current.busy || current.analysisRunning) return;
    try {
      if (!await ensureSitePermissions(galleryDownloadOrigins(groups))) {
        message(t('permissionRequired'), true);
        return;
      }
      const response = await chrome.runtime.sendMessage({
        type: 'YT_DLP_GUI_OPEN_GALLERY',
        gallery: galleryAlbumPayload(groups, current.sourceSiteName, current.sourceUrl),
      });
      if (!response?.ok) throw new Error(response?.message || t('galleryAlbumOpenFailed'));
    } catch (error) {
      message(error.message || t('galleryAlbumOpenFailed'), true);
    }
  }

  async function stopDownload() {
    const current = get(state);
    if (current.busy || !current.downloadRunning || current.currentTask?.state === 'stopping') return;
    patch({ busy: true });
    try {
      if (['running', 'stopping'].includes(current.browserDownloadTask.state)) {
        const response = await chrome.runtime.sendMessage({ type: 'YT_DLP_GUI_BROWSER_DOWNLOAD_STOP' });
        if (!response?.ok) throw Error(response?.message || t('downloadStopFailed'));
        renderBrowserDownloadTask(response.task);
      }
      if (['running', 'stopping'].includes(current.currentTask.state)) {
        const response = await nativeRequest({ action: 'cancelDownload' });
        renderTask(response.data.task);
      }
    } catch (error) {
      message(t('downloadStopFailed', { message: error.message }), true);
    } finally {
      patch({ busy: false });
    }
  }

  async function setPlaylistScope(playlistScope) {
    const current = get(state);
    if (current.busy || current.analysisRunning || current.downloadRunning || current.playlistScope === playlistScope) return;
    patch({
      playlistScope,
      analysisStorageScope: playlistScope,
      analysis: null,
      unifiedAnalysis: null,
      analysisByBackend: { y: null, g: null },
      analysisErrors: { y: '', g: '' },
      choices: {},
      galleryModes: {},
      selectionsByBackend: { y: new Set(), g: new Set() },
      galleryRanges: {},
      knownCountsByBackend: { y: 0, g: 0 },
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
    const selectedIndices = new Set(current.selectionsByBackend.y);
    if (selected) selectedIndices.add(index);
    else selectedIndices.delete(index);
    patch({ selectionsByBackend: {
      ...current.selectionsByBackend,
      y: new Set(selectedIndices),
    } });
    chrome.storage.session.set({
      [scopedStorageKey('selection')]:
        [...selectedIndices].sort((a, b) => a - b),
    });
  }

  function prepareDetail(index) {
    const choice = normalizeChoice(index);
    if (!choice) return;
    persistChoices({ ...get(state).choices, [index]: choice });
  }


  async function saveSiteCookies() {
    const current = get(state);
    if (current.busy || current.analysisRunning || current.downloadRunning || current.cookieSaveRunning) return;
    if (!current.sourceUrl || !Number.isInteger(current.sourceTabId)) {
      patch({ cookieSaveMessage: t('cookieSaveNoPage'), cookieSaveError: true });
      return;
    }
    patch({ cookieSaveRunning: true, cookieSaveMessage: '', cookieSaveError: false });
    try {
      const origins = await sitePermissionOrigins(current.sourceUrl);
      if (!await ensureSitePermissions(origins)) {
        patch({ cookieSaveMessage: t('permissionRequired'), cookieSaveError: true });
        return;
      }
      const response = await nativeRequest({
        action: 'saveBrowserCookies',
        url: current.sourceUrl,
        sourceTabId: current.sourceTabId,
      });
      patch({
        cookieSaveMessage: t(response.data?.autoUpdateEnabled === true ? 'cookieSaveSuccess' : 'cookieSaveAutoUpdateUnavailable'),
        cookieSaveError: response.data?.autoUpdateEnabled !== true,
      });
    } catch (error) {
      patch({ cookieSaveMessage: error.message || t('cookieSaveFailed'), cookieSaveError: true });
    } finally {
      patch({ cookieSaveRunning: false });
    }
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
    destroyed = true;
    recognitionRevision += 1;
    pageSource.destroy();
    matcherWorker.terminate();
    chrome.runtime.onMessage.removeListener(runtimeListener);
    chrome.storage.onChanged.removeListener(storageListener);
  }

  async function init() {
    document.documentElement.lang = chrome.i18n.getUILanguage?.() || 'en';
    chrome.runtime.onMessage.addListener(runtimeListener);
    chrome.storage.onChanged.addListener(storageListener);
    await Promise.all([loadPreferences(), pageSource.start(), restoreTask()]);
  }

  return {
    subscribe: state.subscribe,
    init,
    destroy,
    analyze,
    retryAnalysis: backend => analyze(backend),
    stopAnalysis,
    setGalleryDownloadMode,
    setGalleryRange,
    galleryModeForGroup,
    toggleGalleryGroup,
    downloadSelected,
    openGalleryAlbum,
    openGalleryAlbumAll,
    stopDownload,
    setPlaylistScope,
    setPlaylistItemLimit,
    toggleSelection,
    prepareDetail,
    setChoice,
    chooseDirectory,
    openDirectory,
    saveSiteCookies,
    setAnalysisTool,
    setAlbumWheelMode,
    detectAnalysisTools,
    storedDownloadChoice,
    downloadChoiceAvailable,
  };
}
