import { t } from '../shared/i18n.js';
import { NATIVE_HOST_HELP_URL, isNativeHostMissingError } from '../shared/native-host-errors.js';
import { hostProtocolFrom, withClientProtocol } from '../shared/native-protocol.js';

const HOST = 'kannagi0303.yt_dlp_gui';
const TASK_EVENT = 'YT_DLP_GUI_TASK_EVENT';
const ANALYSIS_EVENT = 'YT_DLP_GUI_ANALYSIS_EVENT';
const ANALYSIS_CACHE_VERSION = 9;
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
const supportedHostsReady = fetch(chrome.runtime.getURL('resources/supported-hosts.json'))
  .then(response => {
    if (!response.ok) throw new Error('Could not load the yt-dlp host index.');
    return response.json();
  })
  .then(index => new Set(index.hosts));
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
      const supportedHosts = await supportedHostsReady;
      let hostname = parsed.hostname.toLowerCase();
      while (hostname) {
        if (supportedHosts.has(hostname)) {
          known = true;
          break;
        }
        const dot = hostname.indexOf('.');
        if (dot < 0) break;
        hostname = hostname.slice(dot + 1);
      }
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
  return ['analysis', 'choices', 'selection'].map(prefix => `${prefix}:tab:${tabId}:`);
}

async function clearLegacyPageState() {
  const legacyKey = key =>
    key === 'analysisCacheIndex'
    || /^(analysis|choices|selection):(current|playlist):/.test(key);
  for (const storage of [chrome.storage.session, chrome.storage.local]) {
    const saved = await storage.get(null);
    const keys = Object.keys(saved).filter(legacyKey);
    if (keys.length) await storage.remove(keys);
  }
}

async function clearTabDocumentState(tabId) {
  const prefixes = tabScopedStoragePrefixes(tabId);
  for (const [key, port] of analysisPorts) {
    if (!prefixes.some(prefix => key.startsWith(prefix))) continue;
    activeAnalyses.delete(key);
    analysisPorts.delete(key);
    try {
      port.postMessage({ action: 'cancelAnalysis' });
      port.disconnect();
    } catch {}
  }
  const saved = await chrome.storage.session.get(null);
  const keys = Object.keys(saved).filter(key =>
    key === `actionState:tab:${tabId}`
    || prefixes.some(prefix => key.startsWith(prefix)));
  if (keys.length) await chrome.storage.session.remove(keys);
}

async function updateActiveAction() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (tab?.id != null) await updateActionForTab(tab.id, tab.url);
}

chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  const documentChanged = Boolean(changeInfo.url) || changeInfo.status === 'loading';
  const update = async () => {
    if (documentChanged) await clearTabDocumentState(tabId);
    if (changeInfo.url || changeInfo.status === 'complete' || documentChanged) {
      await updateActionForTab(tabId, changeInfo.url || tab.url);
    }
  };
  update().catch(reportActionStateError);
});
chrome.tabs.onRemoved.addListener(tabId => {
  clearTabDocumentState(tabId).catch(reportActionStateError);
});
chrome.tabs.onActivated.addListener(({ tabId }) => {
  chrome.tabs.get(tabId).then(tab => updateActionForTab(tabId, tab.url)).catch(reportActionStateError);
});
chrome.windows?.onFocusChanged?.addListener(() => updateActiveAction().catch(reportActionStateError));
chrome.runtime.onInstalled.addListener(() => updateActiveAction().catch(reportActionStateError));
chrome.runtime.onStartup.addListener(() => updateActiveAction().catch(reportActionStateError));
clearLegacyPageState().catch(reportActionStateError);
updateActiveAction().catch(reportActionStateError);

async function withBrowserCookies(request) {
  request = withClientProtocol(request);
  if (!['analyze', 'download'].includes(request?.action)) return request;
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  const stores = await chrome.cookies.getAllCookieStores();
  const store = stores.find(value => value.tabIds.includes(tab?.id));
  const parsed = new URL(request.url);
  const host = parsed.hostname.toLowerCase().replace(/^www\./, '');
  const urls = [request.url];
  if (host === 'youtube.com' || host.endsWith('.youtube.com') || host === 'youtu.be') {
    urls.push('https://www.youtube.com/');
  }
  const batches = await Promise.all(urls.map(url =>
    chrome.cookies.getAll({ url, ...(store ? { storeId: store.id } : {}) })));
  const cookies = [...new Map(batches.flat().map(cookie => [
    `${cookie.domain}\n${cookie.path}\n${cookie.name}`,
    cookie,
  ])).values()];
  return { ...request, browserCookies: cookies.map(({ domain, hostOnly, path, secure, expirationDate, name, value }) =>
    ({ domain, hostOnly, path, secure, expirationDate, name, value })) };
}
let nativePort = null;
let activeTask = { state: 'idle' };
let downloadQueue = [];
let completedOutputPaths = [];
const activeAnalyses = new Map();
const analysisPorts = new Map();

function analysisStorageKey(request) {
  if (!Number.isInteger(request.sourceTabId)) {
    throw new Error(t('sourceTabMissing'));
  }
  const scope = request.playlistScope === 'playlist' ? 'playlist' : 'current';
  return `analysis:tab:${request.sourceTabId}:${scope}:${request.url}`;
}

async function publishAnalysis(key, response) {
  await chrome.storage.session.set({ [key]: response });
  chrome.runtime.sendMessage({ type: ANALYSIS_EVENT, key, response }).catch(() => {});
}

async function startAnalysis(request) {
  const key = analysisStorageKey(request);
  if (activeAnalyses.has(key)) {
    return { ok: true, message: t('analysisAlreadyRunning'), data: activeAnalyses.get(key).data };
  }
  const prepared = await withBrowserCookies(request);
  const response = {
    ok: true,
    message: t('analysisStarted'),
    data: {
      items: [],
      itemLimit: Math.max(1, Math.min(100, Number(request.itemLimit) || 32)),
      truncated: false,
      analyzing: true,
      phase: 'connectingHost',
      schemaVersion: ANALYSIS_CACHE_VERSION,
      startedAt: Date.now(),
    },
  };
  activeAnalyses.set(key, response);
  await setActionState(request.sourceTabId, 'analyzing');
  await publishAnalysis(key, response);

  const port = chrome.runtime.connectNative(HOST);
  analysisPorts.set(key, port);
  port.onMessage.addListener(async nativeResponse => {
    const current = activeAnalyses.get(key);
    if (!current) return;
    if (!nativeResponse?.ok) {
      const failed = { ok: false, message: nativeResponse?.message || t('nativeAnalysisFailed') };
      activeAnalyses.delete(key);
      await setActionState(request.sourceTabId, 'failed');
      await publishAnalysis(key, failed);
      port.disconnect();
      return;
    }
    const event = nativeResponse.data || {};
    const host = hostProtocolFrom(event);
    if (host.advertised) current.data.host = host;
    if (['off', 'download', 'embed'].includes(event.thumbnailMode)) {
      current.data.thumbnailMode = event.thumbnailMode;
    }
    if (['off', 'download', 'embed'].includes(event.subtitleMode)) {
      current.data.subtitleMode = event.subtitleMode;
    }
    if (['off', 'download', 'embed'].includes(event.chapterMode)) {
      current.data.chapterMode = event.chapterMode;
    }
    if (typeof event.useAria2 === 'boolean') current.data.useAria2 = event.useAria2;
    if (typeof event.aria2Available === 'boolean') {
      current.data.aria2Available = event.aria2Available;
    }
    if (event.outputDirectory) {
      await chrome.storage.local.set({ v2DownloadDirectory: event.outputDirectory });
    }
    if ((event.event === 'started' || event.event === 'phase') && event.phase) {
      current.data.phase = event.phase;
      await publishAnalysis(key, structuredClone(current));
    } else if (event.event === 'item' && event.item) {
      current.data.items[event.index ?? current.data.items.length] = event.item;
      await publishAnalysis(key, structuredClone(current));
    } else if (event.event === 'complete') {
      current.data = {
        ...current.data,
        itemLimit: event.itemLimit || current.data.itemLimit,
        truncated: Boolean(event.truncated),
        analyzing: false,
        cachedAt: Date.now(),
      };
      activeAnalyses.delete(key);
      analysisPorts.delete(key);
      await publishAnalysis(key, structuredClone(current));
      await setActionState(request.sourceTabId, 'analyzed', current.data.items.length);
    }
  });
  port.onDisconnect.addListener(async () => {
    if (!activeAnalyses.has(key)) return;
    const failed = {
      ok: false,
      ...nativeConnectionFailure(chrome.runtime.lastError?.message, 'analysisDisconnected'),
    };
    activeAnalyses.delete(key);
    analysisPorts.delete(key);
    await publishAnalysis(key, failed);
    await setActionState(request.sourceTabId, 'failed');
  });
  port.postMessage(prepared);
  return response;
}

async function cancelAnalysis(request) {
  const key = analysisStorageKey(request);
  const current = activeAnalyses.get(key);
  const port = analysisPorts.get(key);
  if (!current || !port) {
    return { ok: false, message: t('analysisNotRunning') };
  }
  current.data = {
    ...current.data,
    analyzing: false,
    stopped: true,
    cachedAt: Date.now(),
  };
  activeAnalyses.delete(key);
  analysisPorts.delete(key);
  await publishAnalysis(key, structuredClone(current));
  await setActionState(
    request.sourceTabId,
    current.data.items.length ? 'analyzed' : null,
    current.data.items.length,
  );
  port.postMessage({ action: 'cancelAnalysis' });
  return { ok: true, message: t('analysisStoppedStatus'), data: current.data };
}

async function publishTask(next) {
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
    url: request.url,
    itemIndex,
    currentSourceItemIndex: request.sourceItemIndex,
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
      const completedSourceItemIndices = Number.isInteger(request.sourceItemIndex)
        ? [...activeTask.completedSourceItemIndices, request.sourceItemIndex]
        : activeTask.completedSourceItemIndices;
      if (downloadQueue.length) {
        await publishTask({
          completedItems,
          percent: (completedItems / activeTask.itemCount) * 100,
          outputPaths: [...completedOutputPaths],
          completedSourceItemIndices,
        });
        await runNextDownload(taskId);
      } else {
        await publishTask({
          state: 'completed',
          completedItems,
          percent: 100,
          outputPath: completedOutputPaths.at(-1) || '',
          outputPaths: [...completedOutputPaths],
          completedSourceItemIndices,
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
    itemCount: prepared.length,
    itemIndex: 1,
    completedItems: 0,
    completedSourceItemIndices: [],
    currentSourceItemIndex: first.sourceItemIndex,
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

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
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
  if (message.request?.action === 'analyze') {
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
