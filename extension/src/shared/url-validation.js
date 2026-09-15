import { t } from './i18n.js';

export function validateUrl(input) {
  if (typeof input !== 'string') throw new Error(t('invalidUrlType'));
  const value = input.trim();
  if (value.length > 8192 || /[\u0000-\u0020\u007f]/u.test(value)) {
    throw new Error(t('invalidUrlLength'));
  }
  let url;
  try {
    url = new URL(value);
  } catch {
    throw new Error(t('invalidUrlComplete'));
  }
  if (!['http:', 'https:'].includes(url.protocol) || !url.hostname) {
    throw new Error(t('invalidUrlProtocol'));
  }
  if (url.username || url.password) throw new Error(t('invalidUrlCredentials'));
  return value;
}
