/** Popup presentation rules only. The analysis itself belongs to the
 * background and chrome.storage.session, never to the popup. */
export function pendingBackendAnalyses(backends, itemLimit) {
  const pending = { y: null, g: null };
  for (const backend of backends) {
    if (backend === 'y' || backend === 'g') {
      pending[backend] = {
        items: [], analyzing: true, phase: 'preparing',
        ...(backend === 'y' ? { itemLimit } : {}),
      };
    }
  }
  return pending;
}

export function shouldAcceptBackendUpdate(current, backend, incoming) {
  if (!['y', 'g'].includes(backend) || !incoming || !Array.isArray(incoming.items)) return false;
  const previous = current.analysisByBackend?.[backend];
  if (!previous) return true;
  const oldStart = Number(previous.startedAt || 0);
  const newStart = Number(incoming.startedAt || 0);
  if (oldStart && newStart && oldStart !== newStart) return newStart > oldStart;
  if (!previous.analyzing && incoming.analyzing && oldStart === newStart) return false;
  return true;
}

export function completeBackendFailure(current, backend, rawMessage) {
  const previous = current.analysisByBackend?.[backend];
  const retained = previous ? { ...previous, analyzing: false } : null;
  const analysisByBackend = { ...current.analysisByBackend, [backend]: retained };
  const analysisErrors = { ...current.analysisErrors, [backend]: String(rawMessage || '') };
  const other = backend === 'y' ? 'g' : 'y';
  const otherResult = analysisByBackend[other];
  return {
    analysisByBackend,
    analysisErrors,
    analysisRunning: Object.values(analysisByBackend).some(value => Boolean(value?.analyzing)),
    switchTo: otherResult?.items?.length || otherResult?.analyzing ? other : null,
  };
}
