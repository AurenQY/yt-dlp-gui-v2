import { pageCacheUrl } from './page-cache-key.js';
import { t } from './i18n.js';

/** Convert gallery-dl's media stream into source items (timeline) or post items.
 * Keep original media rows for the album; never infer a post by image count. */
export function galleryPostGroups(items = [], sourceUrl = '') {
  const groups = new Map();
  for (const [sourceIndex, media] of items.entries()) {
    if (!media?.url) continue;
    const inputPostUrl = String(sourceUrl || '');
    const timeline = media.sourceScope === 'timeline';
    const directInstagramPost = /^https?:\/\/(?:www\.)?instagram\.com\/(?:p|reels?|tv)\/[^/?#]+/i.test(inputPostUrl);
    const sourceIdentifiesPostOrTimeline = directInstagramPost
      || /^https?:\/\/(?:www\.)?(?:x\.com|twitter\.com)\/[^/?#]+\/(?:timeline|status\/\d+)(?:[/?#]|$)/i.test(inputPostUrl);
    const id = String(media.postId || '').trim();
    const sourceIsXAccount = /^https?:\/\/(?:www\.)?(?:x\.com|twitter\.com)\/[^/?#]+(?:[/?#]|$)/i.test(inputPostUrl);
    const postUrl = (sourceIdentifiesPostOrTimeline || (sourceIsXAccount && /^\d+$/.test(id)))
      ? inputPostUrl : String(media.canonicalMetadata?.source_url || media.sourcePageUrl || sourceUrl || '');
    const stablePostUrl = directInstagramPost ? pageCacheUrl(postUrl) : postUrl;
    // A single Instagram post is one item even if individual slides expose
    // different media-level IDs in the extractor metadata.
    const key = timeline ? `timeline:${pageCacheUrl(media.analysisSourceUrl || sourceUrl)}`
      : Number.isInteger(media.analysisPostIndex) ? `probe-post:${media.analysisPostIndex}`
      : directInstagramPost ? `instagram-post:${stablePostUrl}` : id ? `post:${id}` : stablePostUrl || media.url;
    let group = groups.get(key);
    if (!group) {
      group = {
        key, sourceScope: media.sourceScope || 'unknown', postId: id, title: media.title || media.filename || postUrl,
        creator: media.canonicalMetadata?.creator_handle || media.creator || '', date: media.date || '',
        canonicalTitle: timeline ? '' : media.canonicalMetadata?.title || '',
        description: media.description || '', thumbnail: media.thumbnail || media.url,
        sourcePageUrl: timeline ? media.analysisSourceUrl || sourceUrl : postUrl,
        downloadUrl: timeline ? media.analysisSourceUrl || sourceUrl : galleryPostDownloadUrl(postUrl, id),
        sourceItemIndices: [], media: [],
      };
      groups.set(key, group);
    }
    group.sourceItemIndices.push(sourceIndex);
    const item = {
      url: media.url, thumbnail: media.thumbnail || media.url,
      fallbackUrls: Array.isArray(media.fallbackUrls) ? media.fallbackUrls : [],
      filename: media.filename || '', extension: media.extension || '',
      relativePath: media.relativePath ?? null,
      mediaType: media.mediaType || '', width: media.width, height: media.height,
      archiveKey: media.archiveKey || '', canonicalMetadata: media.canonicalMetadata || null,
    };
    if (timeline) {
      group.probeComplete = (group.probeComplete ?? true) && media.analysisProbeComplete !== false;
      group.probePostCount = Math.max(group.probePostCount || 0, media.analysisProbePostCount || 0);
      group.previewPosts ||= [];
      const index = Number.isInteger(media.analysisPostIndex) ? media.analysisPostIndex : 0;
      let post = group.previewPosts.find(post => post.probeIndex === index);
      if (!post) {
        post = { probeIndex: index, postId: id, title: media.canonicalMetadata?.title || media.title || '',
          creator: media.canonicalMetadata?.creator_handle || media.creator || '', date: media.date || '',
          description: media.description || '', sourcePageUrl: media.canonicalMetadata?.source_url || media.sourcePageUrl || '', media: [] };
        group.previewPosts.push(post);
      }
      post.media.push(item);
      // Do not inflate the homepage cover count with the second seed post.
      if (index !== group.previewPosts[0].probeIndex) continue;
    }
    group.media.push(item);
  }
  return [...groups.values()];
}

/** Limit a timeline download to the selected post, never accidentally the entire account. */
export function galleryPostDownloadUrl(sourceUrl, postId) {
  try {
    const url = new URL(sourceUrl);
    if (!['http:', 'https:'].includes(url.protocol)) return '';
    const host = url.hostname.toLowerCase().replace(/^www\./, '');
    // A bounded analysis of an Instagram profile produces post cards, not
    // authorization to pass the full account URL to a native download job.
    if (host === 'instagram.com' && !/^\/(?:p|reels?|tv)\/[^/?#]+(?:\/|$)/i.test(url.pathname)) return '';
    if ((host === 'x.com' || host === 'twitter.com') && postId && /^\d+$/.test(postId)) {
      const handle = url.pathname.split('/').filter(Boolean)[0];
      if (handle && handle !== 'i' && !['status', 'statuses'].includes(handle.toLowerCase())) {
        return `${url.origin}/${handle}/status/${postId}`;
      }
      return ''; // No verified account handle: do not download a full timeline.
    }
    return url.href;
  } catch { return ''; }
}

/** Assemble an album without losing post boundaries or media order. */
export function galleryAlbumPayload(groups, siteName = '', sourceUrl = '') {
  if (!Array.isArray(groups) || !groups.length) throw Error(t('albumEmpty'));
  const posts = groups.map(group => ({
    postId: group.postId || '', title: group.title || '',
    creator: group.creator || '', date: group.date || '',
    description: group.description || '', sourcePageUrl: group.sourcePageUrl || '',
    siteName, media: group.media || [],
  }));
  // Preserve the existing one-post payload format for existing consumers.
  if (posts.length === 1) {
    const group = groups[0];
    if (group.sourceScope !== 'timeline') return posts[0];
    const seed = group.previewPosts?.map(({ probeIndex, ...post }) => ({ ...post, siteName }));
    return { ...posts[0], ...(seed?.length ? { posts: seed } : {}),
      timeline: { url: group.sourcePageUrl, postStart: 1, hasMore: true,
        ...(group.probePostCount ? { seedNextStart: group.probePostCount + 1,
          seedHasMore: group.probeComplete === false || group.probePostCount >= 2 } : {}) } };
  }
  return { siteName, sourcePageUrl: sourceUrl, posts };
}
