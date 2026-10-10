/** Retry one failed tool inside the existing source document. The other tool's
 * result, media and settings identity survive every native plan/result frame. */
import { ANALYSIS_CACHE_VERSION } from '../shared/analysis-cache.js';

export function analysisFrameToolNames(document) {
  return document.request.retryBackend === 'y' ? ['yt-dlp']
    : document.request.retryBackend === 'g' ? ['gallery-dl'] : ['yt-dlp', 'gallery-dl'];
}

export function retryAnalysisDocument(request, previous, startedAt) {
  if (request.retryBackend == null) return null;
  const backend = request.retryBackend;
  const name = backend === 'y' ? 'yt-dlp' : backend === 'g' ? 'gallery-dl' : null;
  if (!name || request.analysisTools?.length !== 1 || request.analysisTools[0] !== backend
      || previous?.data?.request?.schemaVersion !== ANALYSIS_CACHE_VERSION
      || !['failed', 'interrupted'].includes(previous.data[name]?.status)) {
    throw Error('Only a failed analyzer in the current source document can be retried.');
  }
  const document = structuredClone(previous.data);
  document.request = { ...document.request, status: 'analyzing', startedAt, updatedAt: startedAt,
    retryBackend: backend, analysisTools: request.analysisTools, routed: false };
  delete document.request.message;
  delete document.request.cachedAt;
  document[name] = { status: 'pending', info: {}, items: [] };
  return document;
}

export function mergedAnalysisStatus(document, incomingStatus) {
  if (!document.request.retryBackend || incomingStatus === 'stopped') return incomingStatus;
  const statuses = [document['yt-dlp']?.status, document['gallery-dl']?.status];
  const failed = statuses.some(status => ['failed', 'interrupted'].includes(status));
  if (statuses.some(status => ['found', 'collection'].includes(status))) return failed ? 'partial' : 'completed';
  if (failed) return 'failed';
  if (statuses.every(status => ['disabled', 'notMatched'].includes(status))) return 'notMatched';
  return 'notFound';
}
