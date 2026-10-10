/** Final policy shared by every site's pre-analysis preview. No network work. */
export function selectPageThumbnail(snapshot) {
  if (!snapshot) return snapshot;
  const imageIdentity = value => {
    try { const url = new URL(value); return `${url.origin}${url.pathname}`; } catch { return ''; }
  };
  const branding = new Set([snapshot.iconUrl, ...(snapshot.brandingImageUrls || [])]
    .map(imageIdentity).filter(Boolean));
  const candidates = [snapshot.thumbnailUrl, ...(snapshot.thumbnailCandidates || [])];
  const thumbnailCandidates = [...new Set(candidates.filter(value => {
    if (typeof value !== 'string' || !value || value.length > 4096) return false;
    try {
      const url = new URL(value);
      if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) return false;
      if (branding.has(imageIdentity(value))) return false;
      // X's static share campaign/default-avatar assets are branding, not the
      // current account/post. Do not infer this from arbitrary filename words.
      if (url.hostname === 'abs.twimg.com'
          && /^\/(?:rweb\/|sticky\/default_profile_images\/)/.test(url.pathname)) return false;
      return true;
    } catch { return false; }
  }))].slice(0, 12);
  return { ...snapshot, thumbnailUrl: thumbnailCandidates[0] || '', thumbnailCandidates };
}
