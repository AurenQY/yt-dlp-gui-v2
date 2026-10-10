/** Read only an already loaded YouTube player's thumbnail data, without fetch. */
export function collectYouTubePlayerThumbnail() {
  // Self-contained for executeScript in MAIN: player methods/data are not
  // available in the isolated world used by the general DOM collector.
  const url = new URL(location.href);
  if (!/(^|\.)youtube\.com$/i.test(url.hostname)) return null;
  const videoId = url.pathname === '/watch' ? url.searchParams.get('v')
    : url.pathname.match(/^\/(?:shorts|live|embed)\/([^/]+)/)?.[1];
  if (!/^[\w-]{11}$/.test(videoId || '')) return null;
  const thumbnails = [];
  const addImage = (item, ownerId = '') => {
    if (typeof item?.url !== 'string' || item.url.length > 4096) return;
    try {
      const image = new URL(item.url, location.href);
      if (image.protocol !== 'https:' || !/(^|\.)ytimg\.com$/i.test(image.hostname)) return;
      const imageVideoId = image.pathname.match(/\/(?:vi|vi_webp)\/([^/]+)\//)?.[1];
      if (imageVideoId ? imageVideoId !== videoId : ownerId !== videoId) return;
      thumbnails.push({ url: image.href, width: Number(item.width) || 0 });
    } catch {}
  };
  const addResponse = response => {
    const details = response?.videoDetails;
    if (details?.videoId !== videoId || !Array.isArray(details.thumbnail?.thumbnails)) return;
    for (const item of details.thumbnail.thumbnails.slice(0, 16)) {
      addImage(item, videoId);
    }
  };
  try { addResponse(document.getElementById('movie_player')?.getPlayerResponse?.()); } catch {}
  // Shorts keeps multiple reels/players mounted; DOM order is not current-video
  // order. Examine bounded candidates and trust each response's own video ID.
  const players = [...new Set([
    ...document.querySelectorAll(`[is-active] .html5-video-player, [video-id="${videoId}"] .html5-video-player`),
    ...document.querySelectorAll('.html5-video-player'),
  ])].slice(0, 12);
  for (const player of players) {
    try { addResponse(player.getPlayerResponse?.()); } catch {}
  }
  if (url.pathname.startsWith('/shorts/')) {
    // Read only known renderer thumbnail/navigation branches, not arbitrary
    // page state. Image URLs with explicit IDs remain usable before hydration.
    const keys = ['data', 'videoDetails', 'playerResponse', 'playerData',
      'reelWatchEndpoint', 'watchEndpoint', 'navigationEndpoint', 'onTap',
      'innertubeCommand', 'command', 'endpoint', 'thumbnail', 'thumbnails',
      'thumbnailViewModel', 'image', 'sources', 'reelItemRenderer', 'shortsLockupViewModel'];
    const queue = [];
    const reels = [...new Set([
      ...document.querySelectorAll('ytd-reel-video-renderer[is-active], ytd-shorts-video-renderer[is-active], shorts-video[is-active]'),
      ...document.querySelectorAll('ytd-reel-video-renderer, ytd-shorts-video-renderer, ytm-reel-video-renderer, shorts-video'),
    ])].slice(0, 12);
    for (const reel of reels) {
      try {
        const ownerId = reel.getAttribute('video-id') || '';
        queue.push({ value: reel.data, ownerId }, { value: reel.playerResponse, ownerId },
          { value: reel.playerData, ownerId });
      } catch {}
    }
    const visited = new Set();
    for (let count = 0; queue.length && count < 400; count += 1) {
      const { value, ownerId } = queue.shift();
      if (!value || typeof value !== 'object' || visited.has(value)) continue;
      visited.add(value);
      const nodeId = value.videoId || value.videoDetails?.videoId || ownerId;
      if (nodeId && nodeId !== videoId) continue;
      addImage(value, nodeId);
      if (Array.isArray(value)) {
        for (const item of value.slice(0, 16)) queue.push({ value: item, ownerId: nodeId });
      } else {
        for (const key of keys) {
          try { if (value[key]) queue.push({ value: value[key], ownerId: nodeId }); } catch {}
        }
      }
    }
  }
  // Current Shorts cover images/posters are useful even if share tags still
  // refer to the first reel. Only an explicit matching ytimg ID admits these.
  for (const element of [...document.querySelectorAll(
    'img[src*="ytimg.com"], video[poster], .ytp-cued-thumbnail-overlay-image',
  )].slice(0, 64)) {
    const background = element.style?.backgroundImage?.match(/url\(["']?([^"')]+)["']?\)/)?.[1];
    for (const source of [element.currentSrc, element.src, element.poster, background]) {
      if (source) addImage({ url: source, width: element.naturalWidth || 0 });
    }
  }
  for (const element of [...document.querySelectorAll(
    'meta[property="og:image"], meta[property="og:image:secure_url"], meta[name="twitter:image"], link[itemprop="thumbnailUrl"]',
  )].slice(0, 8)) {
    addImage({ url: element.content || element.href });
  }
  // Initial response is only a fallback, never trusted without its video ID.
  try { addResponse(window.ytInitialPlayerResponse); } catch {}
  thumbnails.sort((left, right) => right.width - left.width);
  return { url: location.href, videoId, thumbnailCandidates: [...new Set(thumbnails.map(item => item.url))].slice(0, 8) };
}

export function youtubePageVideoId(value) {
  try {
    const url = new URL(value);
    if (!/(^|\.)youtube\.com$/i.test(url.hostname)) return '';
    const id = url.pathname === '/watch' ? url.searchParams.get('v')
      : url.pathname.match(/^\/(?:shorts|live|embed)\/([^/]+)/)?.[1];
    return /^[\w-]{11}$/.test(id || '') ? id : '';
  } catch { return ''; }
}

/** Last-resort current-ID cover while Shorts' hydrated data is still pending. */
export function withYouTubeShortsThumbnailFallback(snapshot) {
  const videoId = youtubePageVideoId(snapshot?.url);
  if (!videoId || new URL(snapshot.url).pathname.replace(/\/+$/, '') !== `/shorts/${videoId}`) return snapshot;
  // These are image candidates loaded by SourcePreview's existing image/error
  // path, not an API request or a guessed video ID. Page-provided covers win.
  const currentImages = [
    ...(snapshot.thumbnailCandidates || []),
    ...(snapshot.thumbnailUrl ? [snapshot.thumbnailUrl] : []),
  ].filter(value => {
    if (typeof value !== 'string' || value.length > 4096) return false;
    try {
      const image = new URL(value);
      return image.protocol === 'https:' && /(^|\.)ytimg\.com$/i.test(image.hostname)
        && image.pathname.match(/\/(?:vi|vi_webp)\/([^/]+)\//)?.[1] === videoId;
    } catch { return false; }
  });
  const thumbnailCandidates = [...new Set([
    ...currentImages,
    `https://i.ytimg.com/vi/${videoId}/hqdefault.jpg`,
    `https://i.ytimg.com/vi/${videoId}/mqdefault.jpg`,
  ])];
  return { ...snapshot, thumbnailUrl: thumbnailCandidates[0], thumbnailCandidates };
}

/** Treat MAIN-world data as untrusted and retain all existing non-image fields. */
export function mergeYouTubePlayerThumbnail(snapshot, preview) {
  const videoId = youtubePageVideoId(snapshot?.url);
  if (!videoId || videoId !== preview?.videoId || videoId !== youtubePageVideoId(preview?.url)) return snapshot;
  const images = (Array.isArray(preview.thumbnailCandidates) ? preview.thumbnailCandidates : [])
    .slice(0, 8).filter(value => {
      if (typeof value !== 'string' || value.length > 4096) return false;
      try {
        const image = new URL(value);
        const imageVideoId = image.pathname.match(/\/(?:vi|vi_webp)\/([^/]+)\//)?.[1];
        return image.protocol === 'https:' && /(^|\.)ytimg\.com$/i.test(image.hostname)
          && (!imageVideoId || imageVideoId === videoId);
      } catch { return false; }
    });
  if (!images.length) return snapshot;
  const thumbnailCandidates = [...new Set([...images, ...(snapshot.thumbnailCandidates || [])])];
  return { ...snapshot, thumbnailUrl: thumbnailCandidates[0], thumbnailCandidates };
}
