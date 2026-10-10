const ALL_WEB_ORIGINS = ['http://*/*', 'https://*/*'];

/** Called only by user-initiated flows. Existing grants never prompt again. */
export async function ensureSitePermissions(origins) {
  if (!origins.length || await chrome.permissions.contains({ origins })) return true;
  // The first missing host asks once for every web host, not one site at a time.
  return chrome.permissions.request({ origins: ALL_WEB_ORIGINS });
}
