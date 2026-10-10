/** Same tagged intent as V2 GalleryDownloadRange; no extractor expressions in UI. */
import { t } from './i18n.js';
export function timelineRange(value = { kind: 'posts', count: 10 }) {
  if (value.kind === 'all') return { kind: 'all' };
  if (value.kind === 'posts' && Number.isInteger(Number(value.count)) && Number(value.count) > 0 && Number(value.count) <= 4294967295) return { kind: 'posts', count: Number(value.count) };
  if (value.kind === 'recent' && Number.isInteger(Number(value.days)) && Number(value.days) > 0 && Number(value.days) <= 36500) return { kind: 'recent', days: Number(value.days), ...(value.since != null && value.until != null ? { since: value.since, until: value.until } : {}) };
  if (value.kind === 'since' && /^\d{4}-\d{2}-\d{2}$/.test(value.date) && !value.date.startsWith('0000') && !Number.isNaN(Date.parse(`${value.date}T00:00:00Z`)) && new Date(`${value.date}T00:00:00Z`).toISOString().slice(0,10) === value.date) return { kind: 'since', date: value.date };
  throw Error(t('rangeInvalid'));
}
export function timelineRangeSummary(value) {
  const range = timelineRange(value);
  return range.kind === 'posts' ? t('rangeLatest', { count: range.count }) : range.kind === 'recent' ? range.days === 1 ? t('rangeRecent24h') : t('rangeRecentDays', { days: range.days }) : range.kind === 'since' ? t('rangeSince', { date: range.date }) : t('rangeAll');
}
export function rangeForTimeline(group, state) { try { return timelineRange(state.galleryRanges?.[group.key]); } catch { return timelineRange(); } }
