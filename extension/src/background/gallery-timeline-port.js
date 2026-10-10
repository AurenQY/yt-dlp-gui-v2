/** UI browsing is a streaming port, not a one-shot message response.
 * Disconnect aborts this read only, including its independent native port. */
export function installTimelineBrowsePort(readTimeline) {
  chrome.runtime.onConnect.addListener(port => {
    if (port.name !== 'yt-dlp-gui-timeline-browse') return;
    const aborter = new AbortController(); let started = false;
    const post = value => {
      if (aborter.signal.aborted) return;
      try { port.postMessage(value); }
      catch { aborter.abort(new DOMException('Album disconnected', 'AbortError')); }
    };
    port.onDisconnect.addListener(() => aborter.abort(new DOMException('Album disconnected', 'AbortError')));
    port.onMessage.addListener(message => {
      if (started) return;
      started = true;
      readTimeline(message.request, aborter.signal, page => post({ event: 'post', page }))
        .then(page => post({ event: 'complete', page }))
        .catch(error => { if (!aborter.signal.aborted) post({ event: 'error', message: String(error.message || error) }); });
    });
  });
}
