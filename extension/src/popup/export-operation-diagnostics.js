/** Bounded JSON bytes travel directly to Chrome; popup may close safely. */
export async function exportOperationDiagnostics() {
  const response = await chrome.runtime.sendMessage({ type: 'GET_OPERATION_DIAGNOSTICS' });
  if (!response?.ok || !Array.isArray(response.data)) throw Error('Diagnostic journal unavailable.');
  const url = 'data:application/json;charset=utf-8,' + encodeURIComponent(JSON.stringify(response.data, null, 2));
  await chrome.downloads.download({ url, filename: 'yt-dlp-gui-extension-diagnostics.json', saveAs: true });
}
