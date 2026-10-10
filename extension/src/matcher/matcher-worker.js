import { createMatcher } from './url-matcher.js';
import { t } from '../shared/i18n.js';

// This is a dedicated Worker, not the extension background service worker.
// The popup resolves the packaged resource; no extension API is needed here.
let ready;
self.onmessage = async ({ data }) => {
  try {
    ready ??= fetch(data.indexUrl)
      .then(response => { if (!response.ok) throw new Error(t('matcherIndexLoadFailed')); return response.json(); })
      .then(createMatcher);
    const matcher = await ready;
    self.postMessage({ id: data.id, result: matcher.classify(data.url), skipped: matcher.failures.length });
  } catch (error) {
    self.postMessage({ id: data.id, error: error.message });
  }
};
