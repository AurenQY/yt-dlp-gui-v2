export const NATIVE_HOST_HELP_URL = 'https://github.com/kannagi0303/yt-dlp-gui-v2/releases/latest';

export function isNativeHostMissingError(message) {
  const normalized = String(message || '').toLowerCase();
  return normalized.includes('native messaging host not found')
    || normalized.includes('specified native messaging host not found')
    || normalized.includes('failed to start native messaging host');
}
