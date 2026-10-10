/** Append without renumbering already-visible media/selection. Never use CDN URLs as identity. */
import { t } from './i18n.js';
function postKey(post) {
  return post.postId || post.media?.map(item => item.archiveKey || item.relativePath).join('\n') || post.sourcePageUrl;
}
export function appendTimelinePage(gallery, page, { allowSameCursor = false } = {}) {
  if (!Array.isArray(page?.posts) || typeof page.hasMore !== 'boolean' || !page.timelineRange) throw Error('Incomplete timeline page response.');
  const previous = gallery.timeline?.postStart > 1 ? gallery.posts || [] : [];
  const seen = new Set(previous.map(postKey));
  const added = page.posts.filter(post => { const key = postKey(post); if (!key || seen.has(key)) return false; seen.add(key); return true; });
  const cursor = gallery.timeline?.postStart || 1;
  if (!Number.isInteger(page.nextStart) || page.nextStart < cursor || (page.hasMore && page.nextStart === cursor && !allowSameCursor)) throw Error('Invalid timeline continuation cursor.');
  return { ...gallery, media: undefined, posts: [...previous, ...added],
    timeline: { ...gallery.timeline, range: page.timelineRange, postStart: page.nextStart, hasMore: Boolean(page.hasMore) } };
}
export function timelineReadRequest(gallery) {
  const timeline = gallery.timeline;
  return { url: timeline.url, sourceTabId: timeline.sourceTabId, siteName: gallery.siteName,
    timelineRange: timeline.range, postStart: timeline.postStart || 1,
    ...(timeline.pageSize != null ? { postCount: timeline.pageSize } : {}) };
}
