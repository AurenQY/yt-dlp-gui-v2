import catalog from '../../../assets/site-icons/catalog.json' with { type: 'json' };

/** Branding only, never URL support/source-kind inference. No favicon fetching. */
export function builtinSiteIconPath(input) {
  let hostname;
  try {
    const url = new URL(input);
    if (!['http:', 'https:'].includes(url.protocol)) return '';
    hostname = url.hostname.toLowerCase();
  } catch { return ''; }
  const site = catalog.find(site => site.hosts.some(host => hostname === host || hostname.endsWith(`.${host}`)));
  return site ? `resources/site-icons/${site.file}` : '';
}
