export function collectPageSnapshot(doc = document, currentUrl = location.href) {
  const cleanText = value => String(value || '').replace(/\s+/g, ' ').trim().slice(0, 500);
  const meta = selector => cleanText(doc.querySelector(selector)?.content);
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
  let structuredSiteName = '';
  const scripts = [...doc.querySelectorAll('script[type="application/ld+json"]')].slice(0, 20);
  for (const script of scripts) {
    if (structuredMedia || script.textContent.length > 1_000_000) break;
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
        if (!structuredSiteName) {
          if (types.some(type => ['website', 'organization', 'corporation', 'onlinebusiness'].includes(type))) {
            structuredSiteName = namedText(value.name);
          }
          structuredSiteName ||= namedText(value.publisher || value.provider || value.isPartOf);
        }
        if (types.some(type => type === 'videoobject' || type === 'audioobject')) {
          structuredMedia = value;
          break;
        }
        queue.push(...Object.values(value));
      }
    } catch {}
  }

  const videoCount = doc.querySelectorAll('video').length;
  const audioCount = doc.querySelectorAll('audio').length;
  const firstVideo = doc.querySelector('video');
  const firstMedia = firstVideo || doc.querySelector('audio');
  const ogType = meta('meta[property="og:type"]').toLowerCase();
  const structuredTypes = (Array.isArray(structuredMedia?.['@type'])
    ? structuredMedia['@type']
    : [structuredMedia?.['@type']]).map(type => String(type || '').toLowerCase());
  const hasVideo = videoCount > 0 || ogType.includes('video') || structuredTypes.includes('videoobject');
  const hasAudio = audioCount > 0 || ogType.includes('audio') || structuredTypes.includes('audioobject');
  const mediaKind = hasVideo && hasAudio ? 'mixed' : hasVideo ? 'video' : hasAudio ? 'audio' : '';

  return {
    schemaVersion: 3,
    url: httpUrl(currentUrl),
    canonicalUrl: httpUrl(doc.querySelector('link[rel="canonical"]')?.href),
    siteName: cleanText(
      meta('meta[property="og:site_name"]')
      || meta('meta[name="application-name"]')
      || structuredSiteName,
    ),
    iconUrl: httpUrl(
      doc.querySelector('link[rel~="icon"]')?.href
      || doc.querySelector('link[rel="apple-touch-icon"]')?.href,
    ),
    title: cleanText(
      meta('meta[property="og:title"]')
      || meta('meta[name="twitter:title"]')
      || structuredMedia?.name
      || structuredMedia?.headline
      || doc.title,
    ),
    thumbnailUrl: httpUrl(
      meta('meta[property="og:image:secure_url"]')
      || meta('meta[property="og:image"]')
      || meta('meta[name="twitter:image"]')
      || meta('meta[itemprop="thumbnailUrl"]')
      || structuredMedia?.thumbnailUrl
      || firstVideo?.poster
      || doc.querySelector('link[rel="image_src"]')?.href,
    ),
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
    mediaDurationSeconds: durationSeconds(firstMedia?.duration)
      ?? durationSeconds(structuredMedia?.duration),
    mediaWidth: positiveInteger(firstVideo?.videoWidth)
      || positiveInteger(meta('meta[property="og:video:width"]'))
      || positiveInteger(structuredMedia?.width),
    mediaHeight: positiveInteger(firstVideo?.videoHeight)
      || positiveInteger(meta('meta[property="og:video:height"]'))
      || positiveInteger(structuredMedia?.height),
    mediaKind,
    mediaElementCount: videoCount + audioCount,
  };
}
