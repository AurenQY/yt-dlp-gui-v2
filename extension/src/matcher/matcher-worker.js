import { createMatcher } from './url-matcher.js';
import { t } from '../shared/i18n.js';

const ready = fetch(chrome.runtime.getURL('resources/extractor-index.json'))
  .then(response => { if (!response.ok) throw new Error(t('matcherIndexLoadFailed')); return response.json(); })
  .then(async index => ({ matcher: await createMatcher(index), version: index.generatedFrom.version }));
// Attach a rejection handler immediately; requests receive the actual failure below.
ready.catch(() => {});
self.onmessage = async ({ data }) => {
  try {
    const { matcher, version } = await ready;
    self.postMessage({ id: data.id, result: matcher.classify(data.url), version, skipped: matcher.failures.length });
  } catch (error) {
    self.postMessage({ id: data.id, error: error.message });
  }
};
