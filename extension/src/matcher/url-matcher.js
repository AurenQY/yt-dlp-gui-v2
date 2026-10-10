import { createPCRE2, FLAGS } from 'pcre2-wasm';

import { t } from '../shared/i18n.js';
import { validateUrl } from '../shared/url-validation.js';
import { ytSourceScopes } from '../shared/extractor-source-scope.js';
import { suitabilityRules, conditionalScopes, evaluateSuitability } from '../shared/extractor-suitability.js';
import { compileSourceBranches, evaluateSourceBranch } from '../shared/extractor-source-branches.js';
import { registryMetadata } from '../shared/extractor-registry-metadata.js';

export { validateUrl } from '../shared/url-validation.js';

function validateIndex(index) {
  if (!index || !Array.isArray(index.y) || !Array.isArray(index.g)
    || !index.y.length || !index.g.length
    || [...index.y, ...index.g].some(row => !Array.isArray(row) || row.length !== 2
      || typeof row[0] !== 'string' || !row[0].trim()
      || typeof row[1] !== 'string' || !row[1])) throw new Error(t('invalidUrlIndex'));
}

function compileToolRules(engine, rows, tool, failures, scopes = [], metadata = new Map()) {
  return rows.flatMap(([name, pattern], index) => {
    try {
      // ANCHORED preserves Python re.match; Unicode classes follow Python str patterns.
      return [{ tool, name, index, metadata: metadata.get(index), sourceScope: scopes[index] || 'unknown', regex: engine.compile(pattern.replaceAll('\\ufeff', '\\x{FEFF}'), FLAGS.ANCHORED | FLAGS.UCP) }];
    } catch (error) {
      failures.push({ tool, name, message: error.message });
      return [];
    }
  });
}

function firstMatch(compiled, url, errors, predicates = new Map(), conditional = new Map(), branches = new Map()) {
  const byIndex = new Map(compiled.map(rule => [rule.index, rule]));
  const matched = new Map();
  const match = index => {
    if (!matched.has(index)) {
      const rule = byIndex.get(index);
      try { matched.set(index, rule ? rule.regex.test(url, { matchLimit: 10000, depthLimit: 500 }) : null); }
      catch { matched.set(index, null); errors.push({ tool: rule?.tool, name: rule?.name }); }
    }
    return matched.get(index);
  };
  for (const { tool, name, index, sourceScope, metadata } of compiled) {
    try {
      const candidate = match(index);
      if (candidate === false) continue;
      if (candidate == null) return { tool, name, sourceScope: 'unknown', approximate: true };
      const suitable = evaluateSuitability(predicates.get(index) || [1, index], match, url);
      if (suitable === false) continue;
      const playlist = (conditional.get(index) || []).some(key => new URL(url).searchParams.getAll(key).some(value => value !== ''));
      let resolvedScope = sourceScope;
      if (branches.has(index)) {
        try {
          resolvedScope = evaluateSourceBranch(branches.get(index), url);
          if (resolvedScope === 'unknown' && sourceScope === 'any') resolvedScope = 'any';
        }
        catch { errors.push({ tool, name }); return { tool, name, sourceScope: 'unknown', approximate: true }; }
      }
      return { tool, name, sourceScope: suitable == null ? 'unknown' : playlist ? 'playlist' : resolvedScope,
        extractorKey: metadata?.key, working: metadata?.working ?? null, metadata, ruleIndex: index, approximate: suitable == null };
    } catch {
      errors.push({ tool, name });
    }
  }
  return null;
}

export async function createMatcher(index) {
  validateIndex(index);
  const scopes = ytSourceScopes(index);
  const galleryScopes = ytSourceScopes(index, 'g');
  const ytMetadata = registryMetadata(index, 'y'), galleryMetadata = registryMetadata(index, 'g');
  const predicates = suitabilityRules(index);
  const conditional = conditionalScopes(index);
  const engine = await createPCRE2();
  const branches = compileSourceBranches(index, pattern => engine.compile(pattern, FLAGS.ANCHORED | FLAGS.UCP));
  const failures = [];
  const ytDlp = compileToolRules(engine, index.y, 'y', failures, scopes, ytMetadata);
  const galleryDl = compileToolRules(engine, index.g, 'g', failures, galleryScopes, galleryMetadata);
  const compiled = [...ytDlp, ...galleryDl];

  return {
    compiled,
    failures,
    classify(input) {
      const url = validateUrl(input);
      const errors = [];
      const y = firstMatch(ytDlp, url, errors, predicates, conditional, branches.rules);
      if (y && failures.some(failure => failure.tool === 'y')) {
        y.sourceScope = 'unknown';
        y.approximate = true;
      }
      const g = firstMatch(galleryDl, url, errors);
      const matched = Boolean(y || g);
      const primary = y || g;
      return {
        url,
        kind: matched ? 'known' : 'unknown',
        key: primary?.name || 'Generic',
        name: primary?.name || t('genericDownload'),
        y,
        g,
        tools: { y: Boolean(y), g: Boolean(g) },
        approximate: Boolean(y?.approximate || failures.length > 0 || errors.length > 0),
        operation: 'generic',
      };
    },
    destroy() { for (const { regex } of compiled) regex.destroy(); branches.destroy(); },
  };
}
