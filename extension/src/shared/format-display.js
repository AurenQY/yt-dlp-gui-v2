import { t } from './i18n.js';

export function codecLabel(codec) {
  const normalized = String(codec || '').trim().toLowerCase();
  if (!normalized) return '';
  if (/^(av01|av1)/.test(normalized)) return 'AV1';
  if (/^(avc|h\.264|h264)/.test(normalized)) return 'H.264';
  if (/^(hev|hvc|h\.265|h265)/.test(normalized)) return 'H.265';
  if (/^(vp09|vp9)/.test(normalized)) return 'VP9';
  if (/^(vp08|vp8)/.test(normalized)) return 'VP8';
  if (/^(mp4a|aac)/.test(normalized)) return 'AAC';
  if (normalized.startsWith('opus')) return 'Opus';
  if (normalized.startsWith('vorbis')) return 'Vorbis';
  if (normalized.startsWith('mp3')) return 'MP3';
  return normalized.split('.')[0].toUpperCase();
}

export function sampleRateLabel(sampleRate) {
  const khz = Number(sampleRate) / 1000;
  if (!Number.isFinite(khz) || khz <= 0) return '';
  return `${Number.isInteger(khz) ? khz.toFixed(0) : khz.toFixed(1)} kHz`;
}

export function durationLabel(duration) {
  const raw = String(duration ?? '').trim();
  if (!raw) return '';

  let totalSeconds;
  if (!raw.includes(':')) {
    const secondsValue = Number(raw);
    if (!Number.isFinite(secondsValue) || secondsValue < 0) return raw;
    totalSeconds = Math.round(secondsValue);
  } else {
    const parts = raw.split(':').map(part => Number(part.trim()));
    if (parts.length < 2
        || parts.length > 3
        || parts.some(part => !Number.isInteger(part) || part < 0)) {
      return raw;
    }
    totalSeconds = parts.reduce((total, part) => total * 60 + part, 0);
  }

  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
  }
  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
}

export function bitrateLabel(bitrate) {
  const value = Number(bitrate);
  return Number.isFinite(value) && value > 0 ? `${Math.round(value)} kbps` : '';
}

export function filesizeLabel(bytes, approximate = false) {
  const value = Number(bytes);
  if (!Number.isFinite(value) || value <= 0) return '';
  const size = value >= 1073741824
    ? `${(value / 1073741824).toFixed(1)} GB`
    : `${(value / 1048576).toFixed(value >= 104857600 ? 0 : 1)} MB`;
  return approximate ? t('approximate', { value: size }) : size;
}

export function formatColumns(format, audio) {
  if (audio) {
    return [
      format.kind === 'muxed' ? 'V+A' : '',
      String(format.language || '').trim() || '—',
      sampleRateLabel(format.sampleRate) || '—',
      codecLabel(format.audioCodec || format.codec) || '—',
      bitrateLabel(format.bitrate) || '—',
      filesizeLabel(format.filesize, format.filesizeApproximate) || '—',
    ];
  }
  return [
    format.kind === 'muxed' ? 'V+A' : '',
    format.resolution || '—',
    format.fps ? `${format.fps} fps` : '—',
    codecLabel(format.codec) || '—',
    format.kind === 'muxed' ? codecLabel(format.audioCodec) || '—' : '—',
    filesizeLabel(format.filesize, format.filesizeApproximate) || '—',
  ];
}

export function formatSummary(format, audio) {
  if (!format) return t('noFormats');
  if (audio) {
    return [
      format.kind === 'muxed' ? 'V+A' : '',
      String(format.language || '').trim(),
      sampleRateLabel(format.sampleRate),
      codecLabel(format.audioCodec || format.codec),
      bitrateLabel(format.bitrate),
      filesizeLabel(format.filesize, format.filesizeApproximate),
    ].filter(Boolean).join(' · ') || format.id;
  }
  const codecs = [
    codecLabel(format.codec),
    format.kind === 'muxed' ? codecLabel(format.audioCodec) : '',
  ].filter(Boolean).join(' + ');
  return [
    format.kind === 'muxed' ? 'V+A' : '',
    format.resolution,
    format.fps ? `${format.fps} fps` : '',
    codecs,
    filesizeLabel(format.filesize, format.filesizeApproximate),
  ].filter(Boolean).join(' · ') || format.id;
}
