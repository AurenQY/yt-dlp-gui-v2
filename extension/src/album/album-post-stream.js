import { isMediaUrl } from './album-data.js';

/**
 * Flatten post-ordered data into one justified grid and one global media list.
 * The UI consumes `tiles`; Spotlight and downloads consume `media` only.
 * A post card never receives a media index or participates in the media count.
 *
 * Single-post payloads use `gallery.media`; timeline pages append `gallery.posts`.
 */
export function createAlbumStream(gallery = {}) {
  const inputPosts = Array.isArray(gallery.posts) && gallery.posts.length
    ? gallery.posts : [gallery];
  const posts = [];
  const media = [];
  const postByMediaIndex = [];
  const tiles = [];

  for (const input of inputPosts) {
    const postMedia = (Array.isArray(input?.media) ? input.media : [])
      .filter(item => isMediaUrl(item?.url));
    if (!postMedia.length) continue;
    const postIndex = posts.length;
    const post = {
      title: input.title || '', creator: input.creator || '',
      description: input.description || '', date: input.date || '',
      ...albumPostDate(input.date),
      siteName: input.siteName || gallery.siteName || '',
      sourcePageUrl: input.sourcePageUrl || gallery.sourcePageUrl || '',
      postId: input.postId || '', count: postMedia.length,
    };
    posts.push(post);
    // A post card should occupy roughly one portrait thumbnail's width, not two.
    tiles.push({ kind: 'post', postIndex, width: 0.9, height: 1 });

    for (const item of postMedia) {
      const mediaIndex = media.length;
      media.push(item);
      postByMediaIndex.push(post);
      tiles.push({ kind: 'media', postIndex, mediaIndex,
        width: item.width, height: item.height });
    }
  }

  if (gallery.timeline) tiles.push({ kind: 'continue', width: 0.9, height: 1 });
  return { posts, media, tiles, postByMediaIndex };
}

/** Display the extractor's date/time as supplied; do not invent a time zone. */
export function albumPostDate(value = '') {
  const match = /^(\d{4}-\d{2}-\d{2})(?:[T ](\d{2}:\d{2}(?::\d{2})?))?/.exec(String(value));
  return { dateOnly: match?.[1] || '', timeOnly: match?.[2] || '' };
}
