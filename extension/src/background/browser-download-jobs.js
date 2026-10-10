import { albumGalleryMedia, isValidGalleryMedia } from '../album/album-data.js';
import { t } from '../shared/i18n.js';
import { recordDiagnostic, operationTraceId } from '../shared/operation-diagnostics.js';
import { albumDownloadPath } from '../album/album-download-path.js';
import { downloadAlbumSelection } from '../album/album-download.js';
import { prepareGalleryImage } from '../image/gallery-image-processor.js';
import { instantiateXmp } from '../image/xmp-wasm-runtime.js';
import { saveBrowserDownload } from './browser-download-save.js';
import { appendTimelinePage, timelineReadRequest } from '../shared/gallery-timeline-session.js';

const KEY = 'browserDownloadTask';
export const BROWSER_DOWNLOAD_EVENT = 'YT_DLP_GUI_BROWSER_DOWNLOAD_EVENT';
const active = task => ['running', 'stopping'].includes(task?.state);

/** One bounded foreground-requested batch, independent of popup and album lifetimes. */
export function createBrowserDownloadJobs({ readTimeline } = {}) {
  let task = { state: 'idle' };
  let aborter;
  let starting = false;
  let runtime;
  let writes = Promise.resolve();
  const ready = chrome.storage.session.get(KEY).then(async saved => {
    task = saved[KEY] || task;
    if (active(task)) {
      // No automatic replay: Chrome may already have saved the current file.
      await publish({ state: 'failed', message: t('browserJobInterrupted') });
    }
  });
  async function publish(update) {
    if (update.state && update.state !== task.state) recordDiagnostic('browser-download.state', { traceId: task.traceId, status: update.state, completedItems: task.completedItems || 0 });
    task = { ...task, ...update, revision: (task.revision || 0) + 1 };
    const snapshot = { ...task };
    const write = writes.catch(() => {}).then(async () => {
      await chrome.storage.session.set({ [KEY]: snapshot });
      await chrome.runtime.sendMessage({ type: BROWSER_DOWNLOAD_EVENT, task: snapshot }).catch(() => {});
    });
    writes = write;
    await write;
  }
  async function writer() {
    runtime ||= fetch(chrome.runtime.getURL('resources/gallery-xmp.wasm'))
      .then(response => { if (!response.ok) throw Error('XMP WASM unavailable'); return response.arrayBuffer(); })
      .then(instantiateXmp).catch(error => { runtime = null; throw error; });
    return runtime;
  }
  async function run(jobs, signal) {
    // Chrome's documented task-scoped keepalive; never run this when idle.
    const keepAlive = setInterval(() => chrome.runtime.getPlatformInfo().catch(() => {}), 25000);
    let completed = 0;
    try {
      for (const job of jobs) {
        signal.throwIfAborted();
        await publish({ currentSourceItemKey: job.key, title: job.gallery.title || 'gallery-dl' });
        let gallery = job.gallery;
        if (gallery.timeline) {
          if (!readTimeline) throw Error('Timeline reader unavailable.');
          while (gallery.timeline.hasMore) {
            signal.throwIfAborted();
            await publish({ message: t('timelineReadProgress', { number: gallery.timeline.postStart }) });
            gallery = appendTimelinePage(gallery, await readTimeline(timelineReadRequest(gallery), signal));
          }
        }
        const media = albumGalleryMedia(gallery);
        if (!media?.length) throw Error(t('browserDownloadNoMedia'));
        await downloadAlbumSelection({ gallery, media,
          selected: new Set(media.map((_, index) => index)), asZip: job.mode === 'browser-zip',
          permissionsAlreadyGranted: true, signal,
          session: { prepare: async (item, options) => {
            const result = await prepareGalleryImage(item, { ...options,
              writeXmp: async (bytes, metadata) => (await writer())(bytes, metadata) });
            return { blob: new Blob([result.bytes], { type: result.mime }), xmp: result.xmp };
          } },
          save: (blob, name, abortSignal) => saveBrowserDownload(blob, name, abortSignal,
            id => publish({ downloadIds: [...task.downloadIds, id] })),
          progress: message => publish({ message }),
        });
        completed++;
        await publish({ completedItems: completed,
          completedSourceItemKeys: [...task.completedSourceItemKeys, job.key] });
      }
      await publish({ state: 'completed' });
    } catch (error) {
      await publish({ state: signal.aborted ? 'stopped' : 'failed',
        message: signal.aborted ? t('downloadStopped') : String(error?.message || error) });
    } finally { clearInterval(keepAlive); aborter = null; }
  }
  async function start(request) {
    await ready;
    if (starting || aborter || active(task)) throw Error(t('downloadAlreadyRunning'));
    starting = true;
    try {
      const jobs = request.jobs;
      if (!Array.isArray(jobs) || !jobs.length || jobs.length > 100) throw Error(t('downloadListInvalid'));
      for (const job of jobs) {
        const media = albumGalleryMedia(job.gallery);
        if (!['browser-files', 'browser-zip'].includes(job.mode) || !isValidGalleryMedia(media)) throw Error('Invalid browser download item.');
        media.forEach(albumDownloadPath);
        const origins = [...new Set(media.map(item => new URL(item.url).origin + '/*'))];
        if (!await chrome.permissions.contains({ origins })) throw Error(t('mediaPermissionRequired'));
      }
      aborter = new AbortController();
      task = { state: 'running', revision: task.revision || 0, sourceTabId: request.sourceTabId, sourcePageUrl: request.sourcePageUrl,
        traceId: operationTraceId(),
        sourceAnalysisStartedAt: request.sourceAnalysisStartedAt, itemCount: jobs.length,
        completedItems: 0, completedSourceItemKeys: [], downloadIds: [], message: t('downloadPreparing') };
      recordDiagnostic('browser-download.start', { traceId: task.traceId, itemCount: jobs.length });
      try { await publish({}); }
      catch (error) { aborter = null; task = { ...task, state: 'failed', message: t('browserJobStateFailed') }; throw error; }
      const accepted = { ...task };
      // Respond immediately; the user-facing popup is never the job owner.
      run(jobs, aborter.signal).catch(error => console.error('Browser download state persistence failed:', error.name));
      return { ok: true, task: accepted };
    } finally { starting = false; }
  }
  async function stop() {
    await ready;
    if (aborter) {
      await publish({ state: 'stopping' });
      aborter.abort(new DOMException('Download stopped', 'AbortError'));
    }
    return { ok: true, task: { ...task } };
  }
  return { start, stop, status: async () => { await ready; return { ok: true, task: { ...task } }; } };
}
