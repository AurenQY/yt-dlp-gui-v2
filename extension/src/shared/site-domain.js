let publicSuffixRulesPromise;

function normalizeHostname(value) {
  return String(value || '').trim().toLowerCase().replace(/^\.+|\.+$/g, '');
}

function isIpAddress(hostname) {
  return /^\d{1,3}(?:\.\d{1,3}){3}$/.test(hostname) || hostname.includes(':');
}

async function loadPublicSuffixRules() {
  if (!publicSuffixRulesPromise) {
    publicSuffixRulesPromise = fetch(chrome.runtime.getURL('resources/public-suffix-list.dat'))
      .then(response => {
        if (!response.ok) throw new Error(`Could not load Public Suffix List (${response.status})`);
        return response.text();
      })
      .then(text => {
        const exact = new Set();
        const wildcard = new Set();
        const exception = new Set();
        for (const raw of text.split(/\r?\n/)) {
          const rule = raw.trim().toLowerCase();
          if (!rule || rule.startsWith('//')) continue;
          if (rule.startsWith('!')) exception.add(rule.slice(1));
          else if (rule.startsWith('*.')) wildcard.add(rule.slice(2));
          else exact.add(rule);
        }
        return { exact, wildcard, exception };
      });
  }
  return publicSuffixRulesPromise;
}

export async function registrableDomain(hostname) {
  hostname = normalizeHostname(hostname);
  if (!hostname || isIpAddress(hostname) || !hostname.includes('.')) return null;

  const labels = hostname.split('.');
  const { exact, wildcard, exception } = await loadPublicSuffixRules();

  let exceptionLength = 0;
  let publicSuffixLength = 1; // PSL's implicit "*" rule.

  for (let index = 0; index < labels.length; index += 1) {
    const suffix = labels.slice(index).join('.');
    const length = labels.length - index;
    if (exception.has(suffix) && length > exceptionLength) exceptionLength = length;
    if (exact.has(suffix) && length > publicSuffixLength) publicSuffixLength = length;
    if (index > 0 && wildcard.has(suffix) && length + 1 > publicSuffixLength) {
      publicSuffixLength = length + 1;
    }
  }

  if (exceptionLength) publicSuffixLength = exceptionLength - 1;
  if (labels.length <= publicSuffixLength) return null;
  return labels.slice(-(publicSuffixLength + 1)).join('.');
}

export async function siteDomainFromUrl(value) {
  const url = new URL(value);
  const hostname = normalizeHostname(url.hostname);
  return await registrableDomain(hostname) || hostname;
}

export async function sitePermissionOrigins(value) {
  const url = new URL(value);
  const hostname = normalizeHostname(url.hostname);
  const domain = await registrableDomain(hostname);
  if (!domain) return [`${url.origin}/*`];
  const scheme = url.protocol;
  return [...new Set([
    `${scheme}//${domain}/*`,
    `${scheme}//*.${domain}/*`,
  ])];
}
