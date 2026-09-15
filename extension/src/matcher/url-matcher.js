import { createPCRE2, FLAGS } from 'pcre2-wasm';

import { t } from '../shared/i18n.js';
import { validateUrl } from '../shared/url-validation.js';

export { validateUrl } from '../shared/url-validation.js';

export async function createMatcher(index) {
  if (index.resource !== 'yt-dlp-url-matcher-index' || index.regexDialect !== 'python-re') throw new Error(t('invalidUrlIndex'));
  const engine = await createPCRE2();
  const failures = [];
  const compiled = [...index.extractors].sort((a, b) => a.order - b.order).flatMap(record => {
    try {
      // ANCHORED preserves Python re.match; Unicode classes follow Python str patterns.
      return [{ record, regex: engine.compile(record.validUrl, FLAGS.ANCHORED | FLAGS.UCP) }];
    } catch (error) {
      failures.push({ key: record.key, message: error.message });
      return [];
    }
  });
  return {
    compiled,
    failures,
    classify(input) {
      const url = validateUrl(input);
      const errors = [];
      for (const { record, regex } of compiled) {
        try {
          if (regex.test(url, { matchLimit: 10000, depthLimit: 500 })) {
            return { url, kind: 'known', key: record.key, name: record.name || record.key,
              working: record.working, approximate: record.overridesSuitable || failures.length > 0 || errors.length > 0,
              operation: 'generic' };
          }
        } catch { errors.push(record.key); }
      }
      return { url, kind: 'unknown', key: 'Generic', name: t('genericDownload'), operation: 'generic',
        approximate: failures.length > 0 || errors.length > 0 };
    },
    destroy() { for (const { regex } of compiled) regex.destroy(); },
  };
}
