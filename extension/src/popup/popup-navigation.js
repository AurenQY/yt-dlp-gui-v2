import { pageCacheUrl } from '../shared/page-cache-key.js';

const ROUTE_KEY = 'popupRoute';
const GALLERY_ROUTE = /^\/gallery\/(0|[1-9]\d*)\/(?:format\/)?$/;
const ITEM_ROUTE = /^\/item\/(0|[1-9]\d*)\/(?:format\/(video|audio)\/|time-range\/|chapter-range\/)?$/;

/** Only restore actual popup routes; dynamic smart-select pages are not routable on a fresh popup. */
export function popupRoute(url) {
  if (url === '/' || url === '/settings/') return url;
  return typeof url === 'string' && (ITEM_ROUTE.test(url) || GALLERY_ROUTE.test(url)) ? url : '/';
}

export function popupRouteKey(tabId, sourceUrl) {
  return `${ROUTE_KEY}:tab:${tabId}:${pageCacheUrl(sourceUrl)}`;
}

/** The route is UI state of a source tab, not a new copy of the analysis result. */
export async function restorePopupNavigation(tabs, session, waitForDocument = async () => {}) {
  const [tab] = await tabs.query({ active: true, currentWindow: true });
  const tabId = Number.isInteger(tab?.id) ? tab.id : null;
  const sourceUrl = pageCacheUrl(tab?.url || '');
  if (tabId === null || !sourceUrl) return { key: null, route: '/' };
  await waitForDocument(tabId, sourceUrl);
  const key = popupRouteKey(tabId, sourceUrl);
  if (tab.status === 'loading') return { key, route: '/' };
  const saved = (await session.get(key))[key];
  return { key, route: popupRoute(saved?.route) };
}

export function savePopupNavigation(session, navigation, route) {
  if (!navigation?.key || popupRoute(route) !== route) return Promise.resolve();
  return session.set({ [navigation.key]: { route: popupRoute(route) } });
}
