import { pageCacheUrl } from '../shared/page-cache-key.js';

/** Own the current tab/route and discard captures completed after navigation. */
export function createPopupPageSource({ captureSnapshot, onPage, onSnapshot, onError }) {
  let revision = 0;
  let disposed = false;
  let activeTab = null;
  let refreshTimer;
  let settleTimer;

  const samePage = (left, right) => left?.id === right?.id
    && pageCacheUrl(left?.url) === pageCacheUrl(right?.url);

  async function capture(tab, expectedRevision) {
    const snapshot = await captureSnapshot(tab.id);
    if (disposed || expectedRevision !== revision) return;
    const latestTab = await chrome.tabs.get(tab.id);
    if (disposed || expectedRevision !== revision) return;
    if (!samePage(tab, latestTab)) {
      await load(latestTab);
      return;
    }
    // executeScript observes location.href, not a potentially older tabs query.
    if (snapshot?.url && pageCacheUrl(snapshot.url) !== pageCacheUrl(latestTab.url)) return;
    onSnapshot(latestTab, snapshot);
  }

  async function load(tab, documentReset = false) {
    if (disposed) return;
    const expectedRevision = ++revision;
    clearTimeout(settleTimer);
    const changed = documentReset || !samePage(activeTab, tab);
    activeTab = tab;
    // Invalidate the old preview before waiting for script execution/storage.
    onPage(tab, changed, documentReset);
    try {
      if (tab.status === 'loading') return; // Capture the new DOM after completion.
      await capture(tab, expectedRevision);
      // SPA title/head/player updates can lag behind the history change. One
      // bounded follow-up also handles reopening while the new player mounts.
      if (!disposed && expectedRevision === revision) {
        settleTimer = setTimeout(() => {
          capture(tab, expectedRevision).catch(error => {
            if (!disposed && expectedRevision === revision) onError(error);
          });
        }, 600);
      }
    } catch (error) {
      if (!disposed && expectedRevision === revision) onError(error);
    }
  }

  async function refresh() {
    const expectedRevision = ++revision;
    clearTimeout(refreshTimer);
    clearTimeout(settleTimer);
    try {
      const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
      if (!disposed && expectedRevision === revision && Number.isInteger(tab?.id)) await load(tab);
    } catch (error) {
      if (!disposed && expectedRevision === revision) onError(error);
    }
  }

  function updated(tabId, change, tab) {
    if (disposed || tabId !== activeTab?.id) return;
    if (change.status === 'loading') {
      void load(tab, true);
    } else if (change.url) {
      void load(tab);
    } else if (change.title !== undefined || change.status === 'complete') {
      clearTimeout(refreshTimer);
      refreshTimer = setTimeout(() => { void refresh(); }, 100);
    }
  }

  function activated(info) {
    if (!activeTab || info.windowId === activeTab.windowId) void refresh();
  }

  return {
    async start() {
      chrome.tabs.onUpdated.addListener(updated);
      chrome.tabs.onActivated.addListener(activated);
      await refresh();
    },
    refresh,
    destroy() {
      disposed = true;
      revision += 1;
      clearTimeout(refreshTimer);
      clearTimeout(settleTimer);
      chrome.tabs.onUpdated.removeListener(updated);
      chrome.tabs.onActivated.removeListener(activated);
    },
  };
}
