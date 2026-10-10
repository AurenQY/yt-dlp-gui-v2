import { t } from '../shared/i18n.js';
import { operationTraceId, recordDiagnostic, diagnosticJournal } from '../shared/operation-diagnostics.js';
import { galleryPostGroups } from '../shared/gallery-post-groups.js';
import { pageCacheUrl } from '../shared/page-cache-key.js';
import { ANALYSIS_CACHE_VERSION, analysisKeys, preferFreshAnalysisResponse } from '../shared/analysis-cache.js';
import { installTimelineBrowsePort } from './gallery-timeline-port.js';
import { albumGalleryMedia, isValidGalleryMedia } from '../album/album-data.js';
import { createMatcher } from '../matcher/url-matcher.js';
import { NATIVE_HOST_HELP_URL, isNativeHostMissingError } from '../shared/native-host-errors.js';
import { hostProtocolFrom, withClientProtocol } from '../shared/native-protocol.js';
import { siteDomainFromUrl } from '../shared/site-domain.js';
import { createBrowserDownloadJobs } from './browser-download-jobs.js';
import { createTimelineReader } from './gallery-timeline-reader.js';
import { analysisFrameToolNames, retryAnalysisDocument, mergedAnalysisStatus } from './analysis-retry.js';
import { createTabDocumentResetState } from './tab-document-reset.js';
const readTimeline = createTimelineReader(withBrowserCookies);
installTimelineBrowsePort(readTimeline);
const browserDownloads = createBrowserDownloadJobs({ readTimeline });

const HOST = 'kannagi0303.yt_dlp_gui';
const TASK_EVENT = 'YT_DLP_GUI_TASK_EVENT';
const ANALYSIS_EVENT = 'YT_DLP_GUI_ANALYSIS_EVENT';
const ENABLED_ICONS = { 16: '/icons/app-enabled-16.png', 32: '/icons/app-enabled-32.png' };
const GRAY_ICONS = { 16: '/icons/app-disabled-16.png', 32: '/icons/app-disabled-32.png' };
const ACTION_STATES = {
  analyzing: { badge: '', color: '#1f5fbf', label: t('analyzingFormats') },
  analyzed: { badge: '', color: '#1f5fbf', label: t('formatsAnalyzed') },
  downloading: { badge: '', color: '#287a47', label: t('downloading') },
  completed: { badge: '', color: '#287a47', label: t('downloadCompleted') },
  failed: { badge: '!', color: '#a83a3a', label: t('operationFailed') },
};

function badgeCount(value) {
  const count = Math.max(0, Math.floor(Number(value) || 0));
  return count > 99 ? '99+' : count ? String(count) : '';
}
const matcherReady = fetch(chrome.runtime.getURL('resources/extractor-index.json'))
  .then(response => {
    if (!response.ok) throw new Error(t('matcherIndexLoadFailed'));
    return response.json();
  })
  .then(createMatcher);
matcherReady.catch(() => {});
function nativeConnectionFailure(message, fallbackKey) {
  const detail = String(message || '');
  if (isNativeHostMissingError(detail)) {
    return {
      code: 'nativeHostUnavailable',
      message: t('nativeHostUnavailable'),
      helpUrl: NATIVE_HOST_HELP_URL,
    };
  }
  return { message: detail || t(fallbackKey) };
}

function reportActionStateError(error) {
  if (String(error?.message || error).includes('No tab with id')) return;
  console.error('[toolbar-state]', error);
}

async function updateActionForTab(tabId, url) {
  let eligible = false;
  let known = false;
  try {
    const parsed = new URL(url);
    if (['http:', 'https:'].includes(parsed.protocol) && parsed.hostname) {
      eligible = true;
      const matcher = await matcherReady;
      known = matcher.classify(url).kind === 'known';
    }
  } catch {}
  const key = `actionState:tab:${tabId}`;
  const stored = eligible ? await chrome.storage.session.get(key) : {};
  const savedState = stored[key];
  const operation = typeof savedState === 'string' ? savedState : savedState?.operation;
  const state = ACTION_STATES[operation];
  const badge = operation === 'analyzed' || operation === 'completed'
    ? badgeCount(savedState?.count)
    : state?.badge || '';
  await chrome.action.setIcon({ tabId, path: known ? ENABLED_ICONS : GRAY_ICONS });
  await chrome.action.setBadgeText({ tabId, text: eligible ? badge : '' });
  if (state) await chrome.action.setBadgeBackgroundColor({ tabId, color: state.color });
  await chrome.action.setTitle({
    tabId,
    title: eligible
      ? state?.label || (known ? t('supportedTitle') : t('genericTitle'))
      : t('unsupportedTitle'),
  });
  await (eligible ? chrome.action.enable(tabId) : chrome.action.disable(tabId));
}

async function setActionState(tabId, state, count = 0) {
  if (!Number.isInteger(tabId)) return;
  const key = `actionState:tab:${tabId}`;
  if (state) await chrome.storage.session.set({ [key]: { operation: state, count } });
  else await chrome.storage.session.remove(key);
  try {
    const tab = await chrome.tabs.get(tabId);
    await updateActionForTab(tabId, tab.url);
  } catch (error) {
    reportActionStateError(error);
  }
}

function tabScopedStoragePrefixes(tabId) {
  return ['analysis', 'choices', 'selection', 'galleryModes', 'galleryRanges', 'popupRoute'].map(prefix => `${prefix}:tab:${tabId}:`);
}

async function clearLegacyPageState() {
  const legacyKey = key =>
    key === 'analysisCacheIndex' || key.startsWith('analysisResult:v1:')
    || /^(analysis|choices|selection|galleryModes):(current|playlist):/.test(key);
  for (const storage of [chrome.storage.session, chrome.storage.local]) {
    const saved = await storage.get(null);
    const keys = Object.keys(saved).filter(legacyKey);
    if (keys.length) await storage.remove(keys);
  }
}

function isTabAnalysisKey(key, tabId) {
  return key.startsWith(`analysis:tab:${tabId}:`);
}

function belongsToPage(key, pageUrl) {
  return key.endsWith(`:${pageUrl}`) || key.endsWith(`:${pageUrl}:y`)
    || key.endsWith(`:${pageUrl}:g`);
}

// Retire native operations before waiting on their queued session writes.
// Otherwise a late progress frame can recreate an evicted document's cache.
function retireTabAnalyses(tabId, keepPageUrl = null) {
  const ports = new Set();
  for (const key of new Set([...activeAnalyses.keys(), ...analysisWrites.keys()])) {
    if (!isTabAnalysisKey(key, tabId)
        || (keepPageUrl !== null && belongsToPage(key, keepPageUrl))) continue;
    retiredAnalyses.add(key);
    activeAnalyses.delete(key);
  }
  for (const [key, port] of analysisPorts) {
    if (!isTabAnalysisKey(key, tabId)
        || (keepPageUrl !== null && belongsToPage(key, keepPageUrl))) continue;
    retiredAnalyses.add(key);
    activeAnalyses.delete(key);
    analysisPorts.delete(key);
    ports.add(port);
  }
  // A dual-backend analysis shares one host: cancel/disconnect that port once.
  for (const port of ports) {
    try { port.postMessage({ action: 'cancelAnalysis' }); port.disconnect(); } catch {}
  }
}

async function waitTabAnalysisWrites(tabId, keepPageUrl = null) {
  await Promise.all([...analysisWrites].filter(([key]) =>
    isTabAnalysisKey(key, tabId)
      && (keepPageUrl === null || !belongsToPage(key, keepPageUrl)))
    .map(([, pending]) => pending.catch(() => {})));
}

async function clearTabDocumentState(tabId) {
  retireTabAnalyses(tabId);
  await waitTabAnalysisWrites(tabId);
  const prefixes = tabScopedStoragePrefixes(tabId);
  const saved = await chrome.storage.session.get(null);
  const keys = Object.keys(saved).filter(key =>
    key === `actionState:tab:${tabId}`
    || prefixes.some(prefix => key.startsWith(prefix)));
  if (keys.length) await chrome.storage.session.remove(keys);
}

// SPA tracking-query changes keep the same source; a full reload clears all
// document state separately, even when its URL is unchanged.
async function clearOtherTabPageState(tabId, keepPageUrl) {
  retireTabAnalyses(tabId, keepPageUrl);
  await waitTabAnalysisWrites(tabId, keepPageUrl);
  const prefixes = tabScopedStoragePrefixes(tabId);
  const saved = await chrome.storage.session.get(null);
  const keys = Object.keys(saved).filter(key =>
    prefixes.some(prefix => key.startsWith(prefix))
    && !belongsToPage(key, keepPageUrl));
  if (keys.length) await chrome.storage.session.remove([...keys, `actionState:tab:${tabId}`]);
}

async function updateActiveAction() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (tab?.id != null) await updateActionForTab(tab.id, tab.url);
}

const tabDocumentResets = createTabDocumentResetState();
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  const documentChanged = Boolean(changeInfo.url);
  const reloading = changeInfo.status === 'loading';
  let reset = null;
  if (reloading || documentChanged) {
    const keepPageUrl = reloading ? null : pageCacheUrl(changeInfo.url);
    // Retire synchronously, before a queued native frame can publish again.
    retireTabAnalyses(tabId, keepPageUrl);
    reset = tabDocumentResets.reset(tabId, () => reloading
      ? clearTabDocumentState(tabId) : clearOtherTabPageState(tabId, keepPageUrl));
    if (reloading) recordDiagnostic('analysis.page-reset', { phase: 'reload' });
  }
  const update = async () => {
    if (reset) await reset;
    if (reloading || changeInfo.url || changeInfo.status === 'complete') {
      await updateActionForTab(tabId, changeInfo.url || tab.url);
    }
  };
  update().catch(reportActionStateError);
});
chrome.tabs.onRemoved.addListener(tabId => {
  retireTabAnalyses(tabId);
  tabDocumentResets.reset(tabId, () => clearTabDocumentState(tabId)).catch(reportActionStateError);
});
chrome.tabs.onActivated.addListener(({ tabId }) => {
  chrome.tabs.get(tabId).then(tab => updateActionForTab(tabId, tab.url)).catch(reportActionStateError);
});
chrome.windows?.onFocusChanged?.addListener(() => updateActiveAction().catch(reportActionStateError));
chrome.runtime.onInstalled.addListener(() => {
  clearLegacyPageState().catch(reportActionStateError);
  updateActiveAction().catch(reportActionStateError);
});
chrome.runtime.onStartup.addListener(() => updateActiveAction().catch(reportActionStateError));
updateActiveAction().catch(reportActionStateError);

async function withBrowserCookies(request) {
  const traceId = request.traceId || operationTraceId();
  recordDiagnostic('cookies.start', { traceId });
  try {
    const prepared = await prepareBrowserCookies({ ...request, traceId });
    recordDiagnostic('cookies.resolved', { traceId, cookieCount: prepared.browserCookies?.length || 0 });
    return prepared;
  } catch (error) {
    recordDiagnostic('cookies.failed', { traceId });
    throw error;
  }
}

async function prepareBrowserCookies(request) {
  request = withClientProtocol(request);
  if (!['analyze', 'analyzeSelected', 'analyzeSources', 'analyzeIndexedSources', 'galleryRead', 'download', 'saveBrowserCookies'].includes(request?.action)) return request;
  const sourceTabId = Number.isInteger(request.sourceTabId) ? request.sourceTabId : null;
  const [activeTab] = sourceTabId == null
    ? await chrome.tabs.query({ active: true, currentWindow: true }) : [null];
  const tabId = sourceTabId ?? activeTab?.id;
  if (!Number.isInteger(tabId)) throw new Error(t('sourceTabMissing'));
  const sourceTab = activeTab || await chrome.tabs.get(tabId);
  const snapshotUnixMs = Date.now();
  const stores = await chrome.cookies.getAllCookieStores();
  const store = stores.find(value => value.tabIds.includes(tabId));
  if (!store) throw new Error(t('cookieReadFailed'));
  // Save, analysis, album reads and downloads collect the same site snapshot.
  // Do not use URL-filtered subsets or a separate tool-specific Cookie scope.
  const domain = await siteDomainFromUrl(request.url);
  const batches = [await chrome.cookies.getAll({ domain, storeId: store.id })];
  const cookies = [...new Map(batches.flat().map(cookie => [
    `${cookie.domain}\n${cookie.path}\n${cookie.name}`,
    cookie,
  ])).values()];
  return { ...request,
    browserCookieContext: {
      incognito: typeof sourceTab.incognito === 'boolean' ? sourceTab.incognito : null,
      snapshotUnixMs,
    },
    browserCookies: cookies.map(({ domain, hostOnly, path, secure, expirationDate, name, value }) =>
    ({ domain, hostOnly, path, secure, expirationDate, name, value })) };
}
let nativePort = null;
let activeTask = { state: 'idle' };
let downloadQueue = [];
let completedOutputPaths = [];
const activeAnalyses = new Map();
const analysisPorts = new Map();

// One document, one analysis job, one cached JSON. Tool groups are properties
// of the result, not independent native requests or independent cache entries.
function analysisStorageKey(request) {
  const keys = analysisKeys(request);
  if (!keys) throw new Error(t('sourceTabMissing'));
  return keys.unified;
}

const analysisWrites = new Map();
const retiredAnalyses = new Set();
async function publishAnalysis(key, response) {
  const snapshot = structuredClone(response);
  const previous = analysisWrites.get(key) || Promise.resolve();
  const write = previous.catch(() => {}).then(async () => {
    if (retiredAnalyses.has(key)) return;
    await chrome.storage.session.set({ [key]: snapshot });
    if (!retiredAnalyses.has(key)) {
      chrome.runtime.sendMessage({ type: ANALYSIS_EVENT, key, response: snapshot }).catch(() => {});
    }
  });
  analysisWrites.set(key, write);
  try { await write; } finally {
    if (analysisWrites.get(key) === write) analysisWrites.delete(key);
  }
}

function emptyTool(status = 'pending') {
  return { status, info: {}, items: [] };
}
function analysisDocument(request, startedAt) {
  return {
    request: { url: request.url, status: 'analyzing', startedAt, updatedAt: startedAt,
      playlistScope: request.playlistScope === 'playlist' ? 'playlist' : 'current',
      analysisTools: ['analyzeSelected', 'analyzeSources', 'analyzeIndexedSources'].includes(request.action) ? request.analysisTools : ['y', 'g'],
      schemaVersion: ANALYSIS_CACHE_VERSION },
    'yt-dlp': emptyTool(['analyzeSelected', 'analyzeSources', 'analyzeIndexedSources'].includes(request.action) && !request.analysisTools?.includes('y') ? 'disabled' : 'pending'),
    'gallery-dl': emptyTool(['analyzeSelected', 'analyzeSources', 'analyzeIndexedSources'].includes(request.action) && !request.analysisTools?.includes('g') ? 'disabled' : 'pending'),
  };
}
function toolName(backend) {
  return backend === 'y' ? 'yt-dlp' : backend === 'g' ? 'gallery-dl' : null;
}
function analysisItemCount(document) {
  return (document['yt-dlp']?.items?.length || 0)
    + galleryPostGroups(document['gallery-dl']?.items, document.request.url).length;
}
function analysisState(document) {
  if (document.request.status === 'analyzing') return 'analyzing';
  if (analysisItemCount(document)) return 'analyzed';
  return ['failed', 'partial'].includes(document.request.status) ? 'failed' : null;
}

async function analysisSnapshot(request) {
  await tabDocumentResets.wait(request.sourceTabId);
  const pageRevision = tabDocumentResets.revision(request.sourceTabId);
  const key = analysisStorageKey(request);
  const saved = (await chrome.storage.session.get(key))[key];
  if (!tabDocumentResets.isCurrent(request.sourceTabId, pageRevision)) return { ok: true, data: null };
  const running = activeAnalyses.get(key);
  const response = running || saved || null;
  recordDiagnostic('analysis.cache', { traceId: response?.data?.request?.traceId, status: running ? 'running' : saved ? 'hit' : 'miss' });
  if (!running && response?.data?.request?.schemaVersion !== ANALYSIS_CACHE_VERSION) {
    return { ok: true, data: null };
  }
  if (!running && response?.ok && response.data?.request?.status === 'analyzing') {
    const interrupted = structuredClone(response);
    interrupted.data.request.status = 'interrupted';
    interrupted.data.request.updatedAt = Math.max(Date.now(), Number(interrupted.data.request.updatedAt || 0) + 1);
    for (const tool of ['yt-dlp', 'gallery-dl']) {
      if (['pending', 'analyzing'].includes(interrupted.data[tool]?.status)) {
        interrupted.data[tool].status = 'interrupted';
      }
    }
    await publishAnalysis(key, interrupted);
    if (!tabDocumentResets.isCurrent(request.sourceTabId, pageRevision)) return { ok: true, data: null };
    return { ok: true, data: interrupted };
  }
  return { ok: true, data: response };
}

function completeGalleryProbe(group) {
  if (!['found', 'notFound'].includes(group.status)) return;
  for (const item of group.items) {
    if (item?.analysisProbeComplete === false) item.analysisProbeComplete = true;
  }
}

function acceptNativeAnalysisFrame(document, response) {
  const event = response?.data || {};
  // A pre-unified EXE interprets the missing backend as yt-dlp-only. Reject
  // it instead of displaying an incomplete result or waiting indefinitely.
  if (response?.ok && !document.request.routed && event.event !== 'plan') {
    document.request.status = 'failed';
    document.request.message = t('nativeAnalysisUpgradeRequired');
    for (const name of analysisFrameToolNames(document)) {
      if (document[name].status !== 'disabled') {
        document[name].status = 'failed';
        document[name].info.message = document.request.message;
      }
    }
    document.request.status = mergedAnalysisStatus(document, document.request.status);
    return true;
  }
  if (!response?.ok) {
    document.request.status = 'failed';
    document.request.message = !document.request.routed
      && /action is not supported/i.test(response?.message || '')
      ? t('nativeAnalysisUpgradeRequired') : response?.message || t('nativeAnalysisFailed');
    for (const name of analysisFrameToolNames(document)) {
      if (['pending', 'analyzing'].includes(document[name].status)) {
        document[name].status = 'failed';
        document[name].info.message = document.request.message;
      }
    }
    document.request.status = mergedAnalysisStatus(document, document.request.status);
    return true;
  }
  if (event.event === 'plan' && event.result) {
    document.request = { ...document.request, ...event.result.request, routed: true };
    for (const name of analysisFrameToolNames(document)) {
      document[name].status = event.result[name]?.status || 'notMatched';
      document[name].info = { ...(event.result[name]?.info || {}) };
    }
    if (event.protocol) document.request.host = hostProtocolFrom({ protocol: event.protocol });
    return false;
  }
  const name = toolName(event.backend);
  if (name && document[name] && analysisFrameToolNames(document).includes(name)) {
    const group = document[name];
    if (event.event === 'item' && event.item) {
      group.items[event.index ?? group.items.length] = event.item;
      group.status = 'analyzing';
    } else if (event.event === 'backendResult' && event.result) {
      group.status = event.result.status || 'failed';
      group.info = { ...group.info, ...(event.result.info || {}) };
      if (name === 'gallery-dl') completeGalleryProbe(group);
    } else if (event.event === 'phase' || event.event === 'started') {
      group.status = 'analyzing';
      group.info = { ...group.info, ...event };
      if (event.protocol) document.request.host = hostProtocolFrom(event);
    }
    if (event.outputDirectory) {
      chrome.storage.local.set({ v2DownloadDirectory: event.outputDirectory }).catch(reportActionStateError);
    }
  }
  if (event.event === 'result' && event.result) {
    document.request = { ...document.request, ...event.result.request };
    for (const name of analysisFrameToolNames(document)) {
      const resultGroup = event.result[name] || {};
      document[name] = { ...document[name], status: resultGroup.status || 'failed',
        info: { ...document[name].info, ...(resultGroup.info || {}) },
        // Keep streamed media. A full Instagram profile can exceed a single
        // Native Messaging response's 1 MiB frame limit.
        items: document[name].items };
      if (name === 'gallery-dl') completeGalleryProbe(document[name]);
      if (document[name].info.outputDirectory) {
        chrome.storage.local.set({ v2DownloadDirectory: document[name].info.outputDirectory })
          .catch(reportActionStateError);
      }
    }
    document.request.status = mergedAnalysisStatus(document, document.request.status);
    return true;
  }
  return false;
}

async function startAnalysis(request) {
  await tabDocumentResets.wait(request.sourceTabId);
  const pageRevision = tabDocumentResets.revision(request.sourceTabId);
  const requireCurrentPage = () => {
    if (!tabDocumentResets.isCurrent(request.sourceTabId, pageRevision)) throw new Error(t('analysisStoppedStatus'));
  };
  const key = analysisStorageKey(request);
  const existing = activeAnalyses.get(key);
  if (existing) return { ok: true, message: t('analysisAlreadyRunning'), data: structuredClone(existing) };
  // User intent can restrict analyzers, never force a URL match. V2 retains
  // matching/Force ownership; legacy routing fields are still stripped.
  const old = (await chrome.storage.session.get(key))[key];
  requireCurrentPage();
  if (request.retryBackend && old?.ok && old.data?.request?.schemaVersion === ANALYSIS_CACHE_VERSION) {
    const name = toolName(request.retryBackend);
    if (name && ((!['failed', 'interrupted'].includes(old.data[name]?.status))
        || (request.retryStartedAt != null && request.retryStartedAt !== old.data.request.startedAt))) {
      return { ok: true, data: structuredClone(old) }; // Refresh stale UI; never relabel a successful tool as failed.
    }
  }
  const startedAt = Math.max(Date.now(), Number(old?.data?.request?.startedAt || 0) + 1);
  const response = { ok: true, message: t('analysisStarted'),
    data: retryAnalysisDocument(request, old, startedAt) || analysisDocument(request, startedAt) };
  const { backend: _legacyBackend, backends: _legacyBackends, retryBackend: _retryBackend,
    retryStartedAt: _retryStartedAt, ...withoutRouting } = request;
  const prepared = await withBrowserCookies(withoutRouting);
  requireCurrentPage();
  const concurrent = activeAnalyses.get(key);
  if (concurrent) return { ok: true, message: t('analysisAlreadyRunning'), data: structuredClone(concurrent) };
  recordDiagnostic('analysis.start', { traceId: prepared.traceId, backend: request.retryBackend });
  response.data.request.traceId = prepared.traceId;
  retiredAnalyses.delete(key);
  activeAnalyses.set(key, response);
  await publishAnalysis(key, response);
  requireCurrentPage();
  await setActionState(request.sourceTabId, 'analyzing');
  requireCurrentPage();
  const port = chrome.runtime.connectNative(HOST);
  analysisPorts.set(key, port);
  let pendingNativeEvents = Promise.resolve();
  port.onMessage.addListener(nativeResponse => {
    pendingNativeEvents = pendingNativeEvents.then(async () => {
      if (activeAnalyses.get(key) !== response) return;
      const data = nativeResponse?.data;
      if (['phase', 'started', 'backendResult', 'result', 'complete', 'error'].includes(data?.event) || !nativeResponse?.ok) {
        recordDiagnostic('analysis.frame', { traceId: prepared.traceId, backend: data?.backend, phase: data?.phase, status: nativeResponse?.ok ? data?.event : 'failed', itemCount: data?.itemCount });
      }
      const complete = acceptNativeAnalysisFrame(response.data, nativeResponse);
      response.data.request.updatedAt = Math.max(Date.now(), response.data.request.updatedAt + 1);
      if (complete) {
        recordDiagnostic('analysis.finish', { traceId: prepared.traceId, elapsedMs: Date.now() - startedAt, status: analysisState(response.data), itemCount: analysisItemCount(response.data) });
        response.data.request.cachedAt = Date.now();
        activeAnalyses.delete(key);
        analysisPorts.delete(key);
      }
      await publishAnalysis(key, response);
      await setActionState(request.sourceTabId, analysisState(response.data),
        analysisItemCount(response.data));
      if (complete && !nativeResponse?.ok) port.disconnect();
    }).catch(error => console.error('[analysis-frame]', error));
    return pendingNativeEvents;
  });
  port.onDisconnect.addListener(async () => {
    await pendingNativeEvents;
    if (activeAnalyses.get(key) !== response) return;
    const failure = nativeConnectionFailure(chrome.runtime.lastError?.message, 'analysisDisconnected');
    recordDiagnostic('analysis.disconnect', { traceId: prepared.traceId, elapsedMs: Math.max(0, Date.now() - startedAt) });
    response.data.request.status = 'interrupted';
    response.data.request.updatedAt = Math.max(Date.now(), response.data.request.updatedAt + 1);
    response.data.request.message = failure.message;
    response.helpUrl = failure.helpUrl;
    for (const name of ['yt-dlp', 'gallery-dl']) {
      if (['pending', 'analyzing'].includes(response.data[name].status)) {
        response.data[name].status = 'interrupted';
        response.data[name].info.message = failure.message;
      }
    }
    activeAnalyses.delete(key);
    analysisPorts.delete(key);
    await publishAnalysis(key, response);
    await setActionState(request.sourceTabId, analysisState(response.data),
      analysisItemCount(response.data));
  });
  port.postMessage(prepared);
  return { ok: true, message: t('analysisStarted'), data: structuredClone(response) };
}

async function cancelAnalysis(request) {
  const key = analysisStorageKey(request);
  const port = analysisPorts.get(key);
  const current = activeAnalyses.get(key);
  if (!port || !current) return { ok: false, message: t('analysisNotRunning') };
  current.data.request.status = 'stopped';
  current.data.request.updatedAt = Math.max(Date.now(), current.data.request.updatedAt + 1);
  for (const name of ['yt-dlp', 'gallery-dl']) {
    if (['pending', 'analyzing'].includes(current.data[name].status)) {
      current.data[name].status = 'stopped';
    }
  }
  activeAnalyses.delete(key);
  analysisPorts.delete(key);
  await publishAnalysis(key, current);
  await setActionState(request.sourceTabId, analysisState(current.data),
    analysisItemCount(current.data));
  port.postMessage({ action: 'cancelAnalysis' });
  return { ok: true, message: t('analysisStoppedStatus'), data: structuredClone(current) };
}

async function publishTask(next) {
  if (next.state && next.state !== activeTask.state) recordDiagnostic('download.state', { traceId: activeTask.traceId, status: next.state });
  activeTask = { ...activeTask, ...next };
  await chrome.storage.session.set({ activeTask });
  chrome.runtime.sendMessage({ type: TASK_EVENT, task: activeTask }).catch(() => {});
}

async function failTask(failure) {
  const detail = typeof failure === 'string' ? { message: failure } : failure;
  await publishTask({ state: 'failed', ...detail, finishedAt: Date.now() });
  await setActionState(activeTask.sourceTabId, 'failed');
  nativePort = null;
  downloadQueue = [];
  completedOutputPaths = [];
}

async function finishStoppedTask(taskId) {
  if (activeTask.id !== taskId || !['running', 'stopping'].includes(activeTask.state)) return;
  nativePort = null;
  downloadQueue = [];
  const outputPaths = [...completedOutputPaths];
  completedOutputPaths = [];
  await publishTask({
    state: 'stopped',
    outputPath: outputPaths.at(-1) || '',
    outputPaths,
    message: t('downloadStopped'),
    finishedAt: Date.now(),
  });
  await setActionState(activeTask.sourceTabId, 'analyzed', activeTask.itemCount);
}

async function cancelDownload() {
  if (!['running', 'stopping'].includes(activeTask.state)) {
    return { ok: false, message: t('downloadNotRunning') };
  }
  if (activeTask.state === 'running') {
    downloadQueue = [];
    await publishTask({ state: 'stopping', message: t('downloadStopping') });
    if (nativePort) {
      const port = nativePort;
      try {
        port.postMessage({ action: 'cancelDownload' });
      } catch {
        try { port.disconnect(); } catch {}
        await finishStoppedTask(activeTask.id);
      }
    } else {
      await finishStoppedTask(activeTask.id);
    }
  }
  return { ok: true, message: t('downloadStopping'), data: { task: activeTask } };
}

async function runNextDownload(taskId) {
  if (activeTask.id !== taskId || activeTask.state !== 'running') return;
  const request = downloadQueue.shift();
  if (!request) return;

  const itemIndex = activeTask.itemCount - downloadQueue.length;
  await publishTask({
    title: request.title || request.url,
    traceId: request.traceId,
    url: request.url,
    itemIndex,
    currentSourceItemIndex: request.sourceItemIndex,
    currentSourceItemKey: request.sourceItemKey,
    downloaded: '',
    total: '',
    speed: '',
    eta: '',
    itemPercent: 0,
    message: t('downloadPreparing'),
  });

  const port = chrome.runtime.connectNative(HOST);
  nativePort = port;
  port.onMessage.addListener(async response => {
    if (activeTask.id !== taskId || nativePort !== port) return;
    if (!response?.ok) {
      if (activeTask.state === 'stopping') await finishStoppedTask(taskId);
      else await failTask(response?.message || t('nativeDownloadFailed'));
      port.disconnect();
      return;
    }
    const data = response.data || {};
    const host = hostProtocolFrom(data);
    if (host.advertised) await publishTask({ host });
    if (data.event === 'stopped') {
      await finishStoppedTask(taskId);
      return;
    }
    if (activeTask.state === 'stopping') {
      if (data.event === 'complete' && data.outputPath) completedOutputPaths.push(data.outputPath);
      await finishStoppedTask(taskId);
      return;
    }
    if (data.event === 'progress') {
      const itemPercent = Number.isFinite(data.percent) ? data.percent : 0;
      const percent = ((activeTask.completedItems + itemPercent / 100) / activeTask.itemCount) * 100;
      await publishTask({
        state: 'running',
        percent,
        itemPercent,
        downloaded: data.downloaded || '',
        total: data.total || '',
        speed: data.speed || '',
        eta: data.eta || '',
        message: t('downloading'),
      });
    } else if (data.event === 'complete') {
      if (data.outputPath) completedOutputPaths.push(data.outputPath);
      nativePort = null;
      const completedItems = activeTask.completedItems + 1;
      const completedSourceItemKeys = request.sourceItemKey
        ? [...activeTask.completedSourceItemKeys, request.sourceItemKey]
        : activeTask.completedSourceItemKeys;
      if (downloadQueue.length) {
        await publishTask({
          completedItems,
          percent: (completedItems / activeTask.itemCount) * 100,
          outputPaths: [...completedOutputPaths],
          completedSourceItemKeys,
        });
        await runNextDownload(taskId);
      } else {
        await publishTask({
          state: 'completed',
          completedItems,
          percent: 100,
          outputPath: completedOutputPaths.at(-1) || '',
          outputPaths: [...completedOutputPaths],
          completedSourceItemKeys,
          itemPercent: 100,
          message: t('downloadCompleted'),
          finishedAt: Date.now(),
        });
        await setActionState(activeTask.sourceTabId, 'completed', completedItems);
        completedOutputPaths = [];
      }
    }
  });
  port.onDisconnect.addListener(async () => {
    const error = chrome.runtime.lastError?.message;
    if (activeTask.id !== taskId || nativePort !== port) return;
    if (activeTask.state === 'stopping') await finishStoppedTask(taskId);
    else if (activeTask.state === 'running') {
      await failTask(nativeConnectionFailure(error, 'nativeDisconnected'));
    }
  });
  port.postMessage(request);
}

async function startDownloadBatch(requests) {
  if (['running', 'stopping'].includes(activeTask.state)) {
    return { ok: false, message: t('downloadAlreadyRunning') };
  }
  if (!Array.isArray(requests) || !requests.length || requests.length > 100
      || requests.some(request => request?.action !== 'download' || !request.url)) {
    return { ok: false, message: t('downloadListInvalid') };
  }

  const prepared = [];
  for (const request of requests) prepared.push(await withBrowserCookies(request));
  const taskId = crypto.randomUUID();
  const first = prepared[0];
  activeTask = {
    id: taskId,
    state: 'running',
    title: first.title || first.url,
    url: first.url,
    sourcePageUrl: first.sourcePageUrl || first.url,
    sourceTabId: first.sourceTabId,
    sourceAnalysisStartedAt: first.sourceAnalysisStartedAt,
    itemCount: prepared.length,
    itemIndex: 1,
    completedItems: 0,
    completedSourceItemKeys: [],
    currentSourceItemIndex: first.sourceItemIndex,
    currentSourceItemKey: first.sourceItemKey,
    itemPercent: 0,
    percent: 0,
    downloaded: '',
    total: '',
    speed: '',
    eta: '',
    message: t('downloadPreparing'),
    startedAt: Date.now(),
  };
  downloadQueue = prepared;
  completedOutputPaths = [];
  await publishTask(activeTask);
  await setActionState(activeTask.sourceTabId, 'downloading');
  await runNextDownload(taskId);
  return { ok: true, message: t('downloadStarted'), data: { task: activeTask } };
}

async function startDownload(request) {
  return startDownloadBatch([request]);
}

function oneShotNativeRequest(request, sendResponse) {
  chrome.runtime.sendNativeMessage(HOST, request, async response => {
    if (chrome.runtime.lastError) {
      sendResponse({ ok: false, message: chrome.runtime.lastError.message });
      return;
    }
    try {
      const directory = response?.data?.outputDirectory || response?.data?.directory;
      if (directory) {
        await chrome.storage.local.set({ v2DownloadDirectory: directory });
      }
      sendResponse(response || { ok: false, message: t('nativeNoData') });
    } catch (error) {
      sendResponse({ ok: false, message: error.message });
    }
  });
}

async function openGalleryAlbum(gallery, autoDownload = null) {
  const media = albumGalleryMedia(gallery);
  if (!isValidGalleryMedia(media)) {
    return { ok: false, message: 'Invalid gallery media.' };
  }
  if (autoDownload != null && !['files', 'zip'].includes(autoDownload)) {
    return { ok: false, message: 'Invalid gallery download mode.' };
  }
  const id = crypto.randomUUID();
  await chrome.storage.session.set({
    [`galleryAlbum:${id}`]: gallery,
    ...(autoDownload ? { [`galleryAutoDownload:${id}`]: autoDownload } : {}),
  });
  await chrome.tabs.create({ url: chrome.runtime.getURL(
    `album.html?id=${id}${autoDownload ? `&download=${autoDownload}` : ''}`,
  ) });
  return { ok: true };
}

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message?.type === 'GET_OPERATION_DIAGNOSTICS') {
    diagnosticJournal().then(rows => sendResponse({ ok: true, data: rows })).catch(() => sendResponse({ ok: false, message: 'Diagnostic journal unavailable.' }));
    return true;
  }
  if (message?.type === 'YT_DLP_GUI_TIMELINE_READ') {
    readTimeline(message.request).then(data => sendResponse({ ok: true, data }))
      .catch(error => sendResponse({ ok: false, message: error.message }));
    return true;
  }
  const browserAction = ({
    YT_DLP_GUI_BROWSER_DOWNLOAD_START: () => browserDownloads.start(message),
    YT_DLP_GUI_BROWSER_DOWNLOAD_STATUS: () => browserDownloads.status(),
    YT_DLP_GUI_BROWSER_DOWNLOAD_STOP: () => browserDownloads.stop(),
  })[message?.type];
  if (browserAction) {
    browserAction().then(sendResponse).catch(error => sendResponse({ ok: false, message: String(error?.message || error) }));
    return true;
  }
  if (message?.type === 'YT_DLP_GUI_OPEN_GALLERY') {
    openGalleryAlbum(message.gallery, message.autoDownload).then(sendResponse)
      .catch(error => sendResponse({ ok: false, message: error.message }));
    return true;
  }
  if (message?.type === 'YT_DLP_GUI_ANALYSIS_SNAPSHOT') {
    analysisSnapshot(message.request).then(sendResponse)
      .catch(error => sendResponse({ ok: false, message: String(error?.message || error) }));
    return true;
  }
  if (message?.type === 'YT_DLP_GUI_TASK_STATUS') {
    chrome.storage.session.get('activeTask').then(saved => {
      sendResponse({ ok: true, data: { task: saved.activeTask || activeTask } });
    });
    return true;
  }
  if (message?.type !== 'YT_DLP_GUI_NATIVE_REQUEST') return false;
  if (message.request?.action === 'cancelDownload') {
    cancelDownload().then(sendResponse)
      .catch(error => sendResponse({ ok: false, message: error.message || t('downloadStopFailed') }));
    return true;
  }
  if (message.request?.action === 'cancelAnalysis') {
    cancelAnalysis(message.request).then(sendResponse)
      .catch(error => sendResponse({ ok: false, message: error.message || t('analysisStopFailed') }));
    return true;
  }
  if (['analyze', 'analyzeSelected', 'analyzeSources', 'analyzeIndexedSources'].includes(message.request?.action)) {
    startAnalysis(message.request).then(sendResponse)
      .catch(error => sendResponse({ ok: false, message: error.message || t('analysisStartFailed') }));
    return true;
  }
  if (message.request?.action === 'download') {
    startDownload(message.request).then(sendResponse).catch(async error => {
      const port = nativePort;
      const failure = nativeConnectionFailure(error.message, 'nativeConnectFailed');
      await failTask(failure);
      if (port) port.disconnect();
      sendResponse({ ok: false, ...failure });
    });
    return true;
  }
  if (message.request?.action === 'downloadBatch') {
    startDownloadBatch(message.request.requests).then(sendResponse).catch(async error => {
      const port = nativePort;
      const failure = nativeConnectionFailure(error.message, 'nativeConnectFailed');
      await failTask(failure);
      if (port) port.disconnect();
      sendResponse({ ok: false, ...failure });
    });
    return true;
  }
  withBrowserCookies(message.request).then(request => oneShotNativeRequest(request, sendResponse))
    .catch(() => sendResponse({ ok: false, message: t('cookieReadFailed') }));
  return true;
});
