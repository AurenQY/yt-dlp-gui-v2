/** Isolated-world DOM capture; keep self-contained for executeScript. */
export function collectTwitterPostThumbnail(doc = document, currentUrl = location.href) {
  const postIdFromUrl = value => {
    if (typeof value !== 'string' || !value) return '';
    try {
      const url = new URL(value, currentUrl);
      return /(^|\.)(?:x|twitter)\.com$/i.test(url.hostname)
        ? url.pathname.match(/\/(?:status|statuses)\/(\d+)(?:\/|$)/)?.[1] || '' : '';
    } catch { return ''; }
  };
  const postId = postIdFromUrl(currentUrl);
  if (!postId) return null;
  const mediaUrl = value => {
    if (typeof value !== 'string' || !value || value.length > 4096) return '';
    try {
      const url = new URL(value, currentUrl);
      return url.protocol === 'https:' && url.hostname === 'pbs.twimg.com'
        && /^\/(?:media|ext_tw_video_thumb|amplify_video_thumb|tweet_video_thumb)\//.test(url.pathname)
        ? url.href : '';
    } catch { return ''; }
  };
  const ownNode = (node, article) => {
    if (node.closest('article') !== article
        || node.closest('[data-testid="quoteTweet"], [data-testid="card.wrapper"]')) return false;
    // Quotes may use a role=link wrapper without quoteTweet test ID. Their
    // author block identifies that wrapper; ordinary media anchors do not.
    for (let parent = node.parentElement; parent && parent !== article; parent = parent.parentElement) {
      if (parent.tagName !== 'A' && parent.getAttribute('role') === 'link'
          && parent.querySelector('[data-testid="User-Name"], [data-testid="Tweet-User-Avatar"]')) return false;
    }
    return true;
  };
  const articles = [...doc.querySelectorAll('article[data-testid="tweet"], article[role="article"]')].slice(0, 100);
  const matching = articles.filter(article => [...article.querySelectorAll('a[href]')].slice(0, 80)
    .some(link => ownNode(link, article) && postIdFromUrl(link.href) === postId
      && (link.querySelector('time') || link.querySelector('[data-testid="tweetPhoto"], [data-testid="videoPlayer"], video[poster]'))));
  // A timestamp permalink is stronger ownership evidence than a media link.
  const article = matching.find(candidate => [...candidate.querySelectorAll('time')]
    .some(time => ownNode(time, candidate) && postIdFromUrl(time.closest('a[href]')?.href) === postId)) || matching[0];
  const candidates = [];
  if (article) {
    const covers = [...article.querySelectorAll(
      'video[poster], [data-testid="tweetPhoto"] img, [data-testid="videoPlayer"] img',
    )].slice(0, 24).filter(node => ownNode(node, article));
    const selected = new URL(currentUrl).pathname.match(/\/(photo|video)\/(\d+)\/?$/);
    const selectedPath = selected ? `/${selected[1]}/${selected[2]}` : '';
    covers.sort((left, right) => {
      const matches = node => {
        const href = node.closest('a[href]')?.href;
        return href && postIdFromUrl(href) === postId && new URL(href, currentUrl).pathname.endsWith(selectedPath);
      };
      return selectedPath ? Number(Boolean(matches(right))) - Number(Boolean(matches(left))) : 0;
    });
    for (const node of covers) {
      const url = mediaUrl(node.poster || node.currentSrc || node.src);
      if (url) candidates.push(url);
    }
  }
  const declaredUrls = [doc.querySelector('meta[property="og:url"]')?.content,
    doc.querySelector('link[rel="canonical"]')?.href].filter(Boolean);
  const metadataVerified = declaredUrls.length > 0 && declaredUrls.every(value => postIdFromUrl(value) === postId);
  const ownsMedia = article && [...article.querySelectorAll('video, [data-testid="tweetPhoto"], [data-testid="videoPlayer"]')]
    .some(node => ownNode(node, article));
  // For a mounted text-only post, do not promote a quote/card's share image.
  // Before the target mounts, only explicitly current-post share tags qualify.
  if (!candidates.length && metadataVerified && (!article || ownsMedia)) {
    for (const selector of ['meta[property="og:image:secure_url"]', 'meta[property="og:image"]', 'meta[name="twitter:image"]']) {
      const url = mediaUrl(doc.querySelector(selector)?.content);
      if (url) candidates.push(url);
    }
  }
  return { url: currentUrl, postId, thumbnailCandidates: [...new Set(candidates)].slice(0, 8) };
}

export function twitterPagePostId(value) {
  try {
    const url = new URL(value);
    return /(^|\.)(?:x|twitter)\.com$/i.test(url.hostname)
      ? url.pathname.match(/\/(?:status|statuses)\/(\d+)(?:\/|$)/)?.[1] || '' : '';
  } catch { return ''; }
}

/** Never leave the general collector's unrelated first-video poster on X. */
export function mergeTwitterPostThumbnail(snapshot, preview) {
  const postId = twitterPagePostId(snapshot?.url);
  if (!postId) return snapshot;
  const verified = preview?.postId === postId && twitterPagePostId(preview?.url) === postId;
  const thumbnailCandidates = verified && Array.isArray(preview.thumbnailCandidates)
    ? [...new Set(preview.thumbnailCandidates.slice(0, 8).filter(value => {
      if (typeof value !== 'string' || value.length > 4096) return false;
      try {
        const url = new URL(value);
        return url.protocol === 'https:' && url.hostname === 'pbs.twimg.com'
          && /^\/(?:media|ext_tw_video_thumb|amplify_video_thumb|tweet_video_thumb)\//.test(url.pathname);
      } catch { return false; }
    }))] : [];
  return { ...snapshot, thumbnailUrl: thumbnailCandidates[0] || '', thumbnailCandidates };
}

export function twitterPageProfileHandle(value) {
  try {
    const url = new URL(value);
    if (!/(^|\.)(?:x|twitter)\.com$/i.test(url.hostname)) return '';
    const handle = url.pathname.match(/^\/([\w]{1,15})(?:\/(?:media|with_replies|highlights))?\/?$/)?.[1]?.toLowerCase();
    return handle && !['home', 'explore', 'notifications', 'messages', 'search', 'settings',
      'compose', 'i', 'login', 'signup', 'tos', 'privacy'].includes(handle) ? handle : '';
  } catch { return ''; }
}

/** Profile ownership is the handle/header, not any tweet in the account feed. */
export function collectTwitterProfileThumbnail(doc = document, currentUrl = location.href) {
  // Deliberately local: injection cannot reference module-level helpers.
  const handleFromUrl = value => {
    if (typeof value !== 'string' || !value) return '';
    try {
      const url = new URL(value, currentUrl);
      return /(^|\.)(?:x|twitter)\.com$/i.test(url.hostname)
        ? url.pathname.match(/^\/([\w]{1,15})(?:\/(?:media|with_replies|highlights|photo|header_photo))?\/?$/)?.[1]?.toLowerCase() || '' : '';
    } catch { return ''; }
  };
  const handle = handleFromUrl(currentUrl);
  if (!handle) return null;
  const imageUrl = value => {
    if (typeof value !== 'string' || value.length > 4096) return '';
    try {
      const url = new URL(value, currentUrl);
      return url.protocol === 'https:' && url.hostname === 'pbs.twimg.com'
        && /^\/(?:profile_images|profile_banners)\//.test(url.pathname) ? url.href : '';
    } catch { return ''; }
  };
  const root = doc.querySelector('[data-testid="primaryColumn"]') || doc.querySelector('main') || doc;
  const candidates = [];
  const links = [...root.querySelectorAll('a[href]')].slice(0, 200).filter(link => !link.closest('article')
    && !link.closest('[data-testid="UserCell"]') && handleFromUrl(link.href) === handle);
  // Avatar represents the account first; banner is another verified fallback.
  for (const suffix of ['/photo', '/header_photo']) {
    for (const link of links.filter(link => new URL(link.href, currentUrl).pathname.replace(/\/+$/, '').endsWith(suffix))) {
      for (const image of [...link.querySelectorAll('img')].slice(0, 4)) {
        const value = imageUrl(image.currentSrc || image.src);
        if (value) candidates.push(value);
      }
    }
  }
  for (const image of [...root.querySelectorAll(`[data-testid="UserAvatar-Container-${handle}"] img`)].slice(0, 4)) {
    if (image.closest('article') || image.closest('[data-testid="UserCell"]')) continue;
    const value = imageUrl(image.currentSrc || image.src);
    if (value) candidates.push(value);
  }
  const pageUrls = [doc.querySelector('meta[property="og:url"]')?.content,
    doc.querySelector('link[rel="canonical"]')?.href].filter(Boolean);
  const metadataCandidates = [];
  if (pageUrls.length && pageUrls.every(value => handleFromUrl(value) === handle)) {
    for (const selector of ['meta[property="og:image"]', 'meta[name="twitter:image"]']) {
      const value = imageUrl(doc.querySelector(selector)?.content);
      if (value) metadataCandidates.push(value);
    }
  }
  return { url: currentUrl, profileHandle: handle,
    thumbnailCandidates: [...new Set([...metadataCandidates, ...candidates])].slice(0, 8) };
}

export function mergeTwitterProfileThumbnail(snapshot, preview) {
  const handle = twitterPageProfileHandle(snapshot?.url);
  if (!handle) return snapshot;
  const verified = preview?.profileHandle === handle && twitterPageProfileHandle(preview?.url) === handle;
  const thumbnailCandidates = [...new Set([
    ...(snapshot.thumbnailMetadataCandidates || []),
    ...(verified && Array.isArray(preview.thumbnailCandidates) ? preview.thumbnailCandidates : []),
  ])].slice(0, 20).filter(value => {
      if (typeof value !== 'string' || value.length > 4096) return false;
      try {
        const url = new URL(value);
        return url.protocol === 'https:' && url.hostname === 'pbs.twimg.com'
          && /^\/(?:profile_images|profile_banners)\//.test(url.pathname);
      } catch { return false; }
    }).slice(0, 8);
  return { ...snapshot, thumbnailUrl: thumbnailCandidates[0] || '', thumbnailCandidates };
}
