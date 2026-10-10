export function collectPageSnapshot(doc = document, currentUrl = location.href) {
  const cleanText = value => String(value || '').replace(/\s+/g, ' ').trim().slice(0, 500);
  const rawMeta = selector => cleanText(doc.querySelector(selector)?.content);
  // Keep this function self-contained: executeScript serializes it into the
  // page's isolated world. Share tags may still describe the initial SPA route.
  const pageIdentity = value => {
    try {
      const url = new URL(value, currentUrl);
      const host = url.hostname.replace(/^www\./, '');
      const path = url.pathname.replace(/\/+$/, '') || '/';
      if (host === 'youtube.com' || host.endsWith('.youtube.com')) {
        return `youtube.com${path}?v=${url.searchParams.get('v') || ''}&list=${url.searchParams.get('v') ? '' : url.searchParams.get('list') || ''}`;
      }
      const xPost = (host === 'x.com' || host === 'twitter.com') && path.match(/\/(?:status|statuses)\/(\d+)(?:\/|$)/);
      if (xPost) return `x.com/status/${xPost[1]}`;
      if (host === 'instagram.com' && /^\/(?:p|reel|reels|tv)\/[^/]+$/.test(path)) {
        return `${host}${path}`;
      }
      for (const key of [...url.searchParams.keys()]) {
        if (/^utm_/i.test(key) || ['fbclid', 'gclid'].includes(key)) url.searchParams.delete(key);
      }
      url.searchParams.sort();
      return `${host}${path}${url.search}`;
    } catch { return ''; }
  };
  const canonicalUrl = doc.querySelector('link[rel="canonical"]')?.href || '';
  const declaredPageUrls = [rawMeta('meta[property="og:url"]'), canonicalUrl].filter(Boolean);
  const metadataMatchesPage = declaredPageUrls.every(url => pageIdentity(url) === pageIdentity(currentUrl));
  const meta = selector => metadataMatchesPage ? rawMeta(selector) : '';
  const namedText = value => {
    const candidate = Array.isArray(value) ? value[0] : value;
    return cleanText(typeof candidate === 'object' && candidate ? candidate.name : candidate);
  };
  const httpUrl = value => {
    const candidate = Array.isArray(value) ? value[0] : value;
    const raw = typeof candidate === 'object' && candidate
      ? candidate.url || candidate.contentUrl
      : candidate;
    if (!raw) return '';
    try {
      const resolved = new URL(String(raw), currentUrl);
      return ['http:', 'https:'].includes(resolved.protocol) ? resolved.href.slice(0, 4096) : '';
    } catch {
      return '';
    }
  };
  const positiveInteger = value => {
    const number = Number(value);
    return Number.isFinite(number) && number > 0 ? Math.round(number) : null;
  };
  const durationSeconds = value => {
    const raw = String(value ?? '').trim();
    if (!raw) return null;
    const numeric = Number(raw);
    if (Number.isFinite(numeric) && numeric >= 0) return Math.round(numeric);
    const match = raw.match(
      /^P(?:(\d+(?:\.\d+)?)D)?(?:T(?:(\d+(?:\.\d+)?)H)?(?:(\d+(?:\.\d+)?)M)?(?:(\d+(?:\.\d+)?)S)?)?$/i,
    );
    if (!match || !match.slice(1).some(Boolean)) return null;
    const total = Number(match[1] || 0) * 86400
      + Number(match[2] || 0) * 3600
      + Number(match[3] || 0) * 60
      + Number(match[4] || 0);
    return Number.isFinite(total) ? Math.round(total) : null;
  };

  let structuredMedia = null;
  const structuredImages = [];
  const brandingImageUrls = [];
  let structuredSiteName = '';
  const scripts = [...doc.querySelectorAll('script[type="application/ld+json"]')].slice(0, 20);
  for (const script of scripts) {
    if (script.textContent.length > 1_000_000) continue;
    try {
      const queue = [JSON.parse(script.textContent)];
      for (let visited = 0; queue.length && visited < 500; visited += 1) {
        const value = queue.shift();
        if (!value || typeof value !== 'object') continue;
        if (Array.isArray(value)) {
          queue.push(...value);
          continue;
        }
        const types = (Array.isArray(value['@type']) ? value['@type'] : [value['@type']])
          .map(type => String(type || '').toLowerCase());
        const logo = httpUrl(value.logo);
        if (logo && brandingImageUrls.length < 16) brandingImageUrls.push(logo);
        if (!structuredSiteName) {
          if (types.some(type => ['website', 'organization', 'corporation', 'onlinebusiness'].includes(type))) {
            structuredSiteName = namedText(value.name);
          }
          structuredSiteName ||= namedText(value.publisher || value.provider || value.isPartOf);
        }
        // mainEntityOfPage identifies the HTML page; VideoObject.url can instead
        // identify a CDN media file, and is not a navigation identity.
        let declaredUrl = value.mainEntityOfPage?.['@id'] || value.mainEntityOfPage;
        if (!declaredUrl) {
          const ownUrl = value.url || value['@id'];
          try {
            const parsed = new URL(typeof ownUrl === 'string' ? ownUrl : '', currentUrl);
            if (ownUrl && parsed.origin === new URL(currentUrl).origin
                && !/\.(?:mp4|webm|m4a|mp3|jpe?g|png|webp)$/i.test(parsed.pathname)) declaredUrl = ownUrl;
          } catch {}
        }
        const belongsToPage = declaredUrl
          ? typeof declaredUrl === 'string' && pageIdentity(declaredUrl) === pageIdentity(currentUrl)
          : metadataMatchesPage;
        if (!structuredMedia && belongsToPage && types.some(type => type === 'videoobject' || type === 'audioobject')) {
          structuredMedia = value;
        }
        const contentType = types.some(type => ['article', 'newsarticle', 'blogposting',
          'socialmediaposting', 'photograph', 'profilepage', 'person', 'imagegallery'].includes(type));
        const contentUrl = declaredUrl || value.url || value['@id'];
        const contentMatches = contentUrl ? typeof contentUrl === 'string'
          && pageIdentity(contentUrl) === pageIdentity(currentUrl) : metadataMatchesPage;
        const authorOnly = types.includes('person') && !contentUrl;
        if (contentType && contentMatches && !authorOnly && structuredImages.length < 12) {
          const images = value.image || value.primaryImageOfPage || value.mainEntity?.image;
          for (const image of (Array.isArray(images) ? images : [images]).slice(0, 8)) {
            const resolved = httpUrl(image);
            if (resolved) structuredImages.push(resolved);
          }
        }
        queue.push(...Object.values(value));
      }
    } catch {}
  }

  const videoCount = doc.querySelectorAll('video').length;
  const audioCount = doc.querySelectorAll('audio').length;
  const youtubeUrl = (() => {
    try {
      const url = new URL(currentUrl);
      return /(^|\.)youtube\.com$/i.test(url.hostname) && url.pathname === '/watch' ? url : null;
    } catch { return null; }
  })();
  const youtubeWatch = youtubeUrl ? doc.querySelector('ytd-watch-flexy[video-id]') : null;
  const youtubeWatchMatches = youtubeWatch
    && youtubeWatch.getAttribute('video-id') === youtubeUrl.searchParams.get('v');
  const youtubePlayerStale = Boolean(youtubeWatch && !youtubeWatchMatches);
  if (youtubePlayerStale) structuredMedia = null;
  const youtubeTitle = youtubeWatchMatches ? cleanText(
    youtubeWatch.querySelector('ytd-watch-metadata h1, #title h1')?.textContent,
  ) : '';
  // A player still mounted for the preceding video must not contribute its
  // poster, dimensions or duration to the new URL's preview.
  const firstVideo = youtubeWatch
    ? (youtubeWatchMatches ? youtubeWatch.querySelector('video') : null)
    : doc.querySelector('video');
  const firstMedia = firstVideo || doc.querySelector('audio');
  const ogType = meta('meta[property="og:type"]').toLowerCase();
  const structuredTypes = (Array.isArray(structuredMedia?.['@type'])
    ? structuredMedia['@type']
    : [structuredMedia?.['@type']]).map(type => String(type || '').toLowerCase());
  const hasVideo = videoCount > 0 || ogType.includes('video') || structuredTypes.includes('videoobject');
  const hasAudio = audioCount > 0 || ogType.includes('audio') || structuredTypes.includes('audioobject');
  const mediaKind = hasVideo && hasAudio ? 'mixed' : hasVideo ? 'video' : hasAudio ? 'audio' : '';

  // A document may contain sidebar ads, replies and suggested videos. Never
  // make its first arbitrary video poster the page's content cover.
  const eligibleVideos = [...doc.querySelectorAll('main video, [role="main"] video, article video')]
    .filter(video => !video.closest?.('aside, nav, footer, header, [aria-hidden="true"]'));
  const primaryVideo = youtubeWatchMatches ? firstVideo
    : metadataMatchesPage && eligibleVideos.length === 1 ? eligibleVideos[0]
      : metadataMatchesPage && videoCount === 1 && (structuredMedia || ogType.includes('video')) ? firstVideo : null;

  // Instagram's og:image often points at a stale or restricted preview. Prefer
  // an image already loaded in the post itself, but do not mistake avatars or
  // other small UI images for the post's cover image.
  const isInstagram = /^https?:\/\/(?:www\.)?instagram\.com\//i.test(currentUrl);
  const isInstagramPost = isInstagram && /^https?:\/\/[^/]+\/(?:p|reels?|tv)\/[^/?#]+/i.test(currentUrl);
  const instagramRoot = doc.querySelector('[role="dialog"]') || doc;
  const loadedPostImages = isInstagramPost && instagramRoot.querySelectorAll('article').length <= 1
    ? [...instagramRoot.querySelectorAll('article img, main img, [role="main"] img')]
      .filter(image => image.complete && image.naturalWidth >= 160 && image.naturalHeight >= 160)
      .map(image => httpUrl(image.currentSrc || image.src))
      .filter(Boolean)
      .slice(0, 4)
    : [];
  const ogUrl = rawMeta('meta[property="og:url"]');
  const ogMatches = ogUrl ? pageIdentity(ogUrl) === pageIdentity(currentUrl) : metadataMatchesPage;
  const twitterUrl = rawMeta('meta[name="twitter:url"]');
  const twitterMatches = twitterUrl ? pageIdentity(twitterUrl) === pageIdentity(currentUrl) : ogMatches;
  const shareImages = [
    ...(ogMatches ? [rawMeta('meta[property="og:image:secure_url"]'), rawMeta('meta[property="og:image"]')] : []),
    ...(twitterMatches ? [rawMeta('meta[name="twitter:image"]')] : []),
    ...(metadataMatchesPage ? [meta('meta[itemprop="thumbnailUrl"]'), doc.querySelector('link[rel="image_src"]')?.href] : []),
  ].map(httpUrl).filter(Boolean);
  const mediaImages = structuredMedia?.thumbnailUrl || structuredMedia?.image;
  const thumbnailMetadataCandidates = [...new Set([
    ...(Array.isArray(mediaImages) ? mediaImages : [mediaImages]).slice(0, 8).map(httpUrl),
    ...structuredImages, ...shareImages,
  ].filter(Boolean))];
  let thumbnailCandidates = [...new Set([...loadedPostImages,
    ...thumbnailMetadataCandidates, httpUrl(primaryVideo?.poster)].filter(Boolean))];
  if (youtubeUrl) {
    // Validate images independently. A stale og:url must not discard a current
    // og:image, and one stale first candidate must not hide a valid later one.
    const imageSources = [firstVideo?.poster,
      rawMeta('meta[property="og:image:secure_url"]'),
      rawMeta('meta[property="og:image"]'), rawMeta('meta[name="twitter:image"]'),
      rawMeta('meta[itemprop="thumbnailUrl"]'), doc.querySelector('link[itemprop="thumbnailUrl"]')?.href,
      structuredMedia?.thumbnailUrl, doc.querySelector('link[rel="image_src"]')?.href];
    thumbnailCandidates = [...new Set(imageSources.map(httpUrl).filter(Boolean))].filter(value => {
      const image = new URL(value);
      const imageVideoId = image.pathname.match(/\/(?:vi|vi_webp)\/([^/]+)\//)?.[1];
      if (imageVideoId && /(^|\.)ytimg\.com$/i.test(image.hostname)) {
        return imageVideoId === youtubeUrl.searchParams.get('v');
      }
      return metadataMatchesPage && !youtubePlayerStale;
    });
  }

  return {
    schemaVersion: 3,
    url: httpUrl(currentUrl),
    canonicalUrl: metadataMatchesPage ? httpUrl(canonicalUrl) : '',
    siteName: cleanText(
      rawMeta('meta[property="og:site_name"]')
      || rawMeta('meta[name="application-name"]')
      || structuredSiteName,
    ),
    iconUrl: httpUrl(
      doc.querySelector('link[rel~="icon"]')?.href
      || doc.querySelector('link[rel="apple-touch-icon"]')?.href,
    ),
    title: youtubePlayerStale ? '' : cleanText(
      youtubeTitle
      || (youtubeUrl ? doc.title : '')
      || meta('meta[property="og:title"]')
      || meta('meta[name="twitter:title"]')
      || structuredMedia?.name
      || structuredMedia?.headline
      || doc.title,
    ),
    thumbnailUrl: thumbnailCandidates[0] || '',
    thumbnailCandidates,
    thumbnailMetadataCandidates,
    brandingImageUrls: [...new Set(brandingImageUrls)],
    creator: namedText(
      meta('meta[name="author"]')
      || structuredMedia?.author
      || structuredMedia?.creator
      || structuredMedia?.publisher,
    ),
    uploadDate: cleanText(structuredMedia?.uploadDate || structuredMedia?.datePublished),
    description: cleanText(
      meta('meta[property="og:description"]')
      || meta('meta[name="twitter:description"]')
      || meta('meta[name="description"]')
      || structuredMedia?.description,
    ),
    contentUrl: httpUrl(structuredMedia?.contentUrl),
    embedUrl: httpUrl(structuredMedia?.embedUrl),
    mediaDurationSeconds: durationSeconds(primaryVideo?.duration ?? (videoCount === 0 ? firstMedia?.duration : null))
      ?? durationSeconds(structuredMedia?.duration),
    mediaWidth: positiveInteger(primaryVideo?.videoWidth)
      || positiveInteger(meta('meta[property="og:video:width"]'))
      || positiveInteger(structuredMedia?.width),
    mediaHeight: positiveInteger(primaryVideo?.videoHeight)
      || positiveInteger(meta('meta[property="og:video:height"]'))
      || positiveInteger(structuredMedia?.height),
    mediaKind,
    mediaElementCount: videoCount + audioCount,
  };
}
