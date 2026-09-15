import { mkdir, copyFile, readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const extensionRoot = fileURLToPath(new URL('../', import.meta.url));
const packageRoot = path.join(extensionRoot, 'chromium');
const source = path.resolve(process.argv[2] ?? path.join(extensionRoot, '../../yt-dlp-url-indexer/extractor-index.json'));
const index = JSON.parse(await readFile(source, 'utf8'));
if (index.resource !== 'yt-dlp-url-matcher-index' || index.regexDialect !== 'python-re') {
  throw new Error('Unexpected index format');
}
await mkdir(path.join(packageRoot, 'resources'), { recursive: true });
await copyFile(source, path.join(packageRoot, 'resources/extractor-index.json'));
const hosts = new Set();
let coveredExtractors = 0;
for (const extractor of index.extractors) {
  if (extractor.working === false) continue;
  const extractorHosts = new Set();
  for (const value of extractor.testUrls) {
    try {
      const url = new URL(value);
      if (!['http:', 'https:'].includes(url.protocol)) continue;
      const hostname = url.hostname.toLowerCase();
      extractorHosts.add(hostname);
      if (hostname.startsWith('www.')) extractorHosts.add(hostname.slice(4));
    } catch {}
  }
  if (extractorHosts.size > 0) coveredExtractors++;
  for (const hostname of extractorHosts) hosts.add(hostname);
}
await writeFile(
  path.join(packageRoot, 'resources/supported-hosts.json'),
  JSON.stringify({
    resource: 'yt-dlp-supported-host-index',
    generatedFrom: index.generatedFrom,
    coveredExtractors,
    hosts: [...hosts].sort(),
  }),
);
const bundledLicenses = [
  ['Framework7', 'node_modules/framework7/LICENSE'],
  ['Framework7 Svelte', 'node_modules/framework7-svelte/LICENSE'],
  ['Svelte', 'node_modules/svelte/LICENSE.md'],
  ['pcre2-wasm', 'node_modules/pcre2-wasm/LICENSE'],
  ['PCRE2 notice', 'node_modules/pcre2-wasm/NOTICE'],
];
const noticeSections = await Promise.all(bundledLicenses.map(async ([name, relativePath]) =>
  '===============================================================================\n'
  + name + '\n'
  + '===============================================================================\n'
  + await readFile(path.join(extensionRoot, relativePath), 'utf8')));
await writeFile(
  path.join(packageRoot, 'THIRD-PARTY-NOTICES.txt'),
  noticeSections.join('\n\n'),
);
console.log(`Synced ${index.extractors.length} extractors from yt-dlp ${index.generatedFrom.version}`);
