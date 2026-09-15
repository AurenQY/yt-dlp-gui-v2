function wholeNonNegativeSeconds(value) {
  const seconds = Number(value);
  return Number.isSafeInteger(seconds) && seconds >= 0 ? seconds : null;
}

export function downloadTimeRangeFromChoice(choice = {}) {
  const startSeconds = wholeNonNegativeSeconds(choice.rangeStartSeconds);
  const endSeconds = wholeNonNegativeSeconds(choice.rangeEndSeconds);
  return startSeconds !== null && endSeconds !== null && endSeconds > startSeconds
    ? { startSeconds, endSeconds }
    : null;
}

export function parseTimecode(value) {
  const text = String(value || '').trim();
  if (!text) return null;
  const parts = text.split(':');
  if (parts.length < 1 || parts.length > 3 || parts.some(part => !/^\d+$/.test(part))) {
    return null;
  }
  const values = parts.map(Number);
  if (values.some(number => !Number.isSafeInteger(number))) return null;
  if (parts.length >= 2 && values.at(-1) >= 60) return null;
  if (parts.length === 3 && values[1] >= 60) return null;
  if (parts.length === 1) return values[0];
  if (parts.length === 2) return values[0] * 60 + values[1];
  return values[0] * 3600 + values[1] * 60 + values[2];
}

export function formatTimecode(value) {
  const totalSeconds = wholeNonNegativeSeconds(value) || 0;
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return [hours, minutes, seconds].map(part => String(part).padStart(2, '0')).join(':');
}
