/** A tab's post identity should not change when sites alter tracking or image-index query strings. */
export function pageCacheUrl(value) {
  try {
    const url = new URL(value);
    url.hash = '';
    const host = url.hostname.toLowerCase().replace(/^www\./, '');
    if (host === 'instagram.com' && /^\/(?:p|reel|reels|tv)\/[^/?#]+\/?$/i.test(url.pathname)) {
      // Instagram can switch between www/bare host and with/without trailing
      // slash without changing the actual post. All those URLs share a cache.
      url.hostname = 'www.instagram.com';
      url.pathname = `${url.pathname.replace(/\/+$/, '')}/`;
      url.search = '';
    } else if ((host === 'x.com' || host === 'twitter.com')
        && /^\/[^/?#]+\/status\/\d+\/?$/i.test(url.pathname)) {
      url.search = '';
    }
    return url.href;
  } catch {
    return String(value || '');
  }
}
