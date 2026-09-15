import { t } from './i18n.js';
import { downloadTimeRangeFromChoice } from './download-time-range.js';
import { chapterDownloadRangesFromChoice } from './download-chapter-range.js';

function resolutionPixels(format) {
  const match = String(format.resolution || '').match(/(\d+)\s*x\s*(\d+)/i);
  return match ? Number(match[1]) * Number(match[2]) : 0;
}

function numericQuality(format, field) {
  const value = Number(format[field]);
  return Number.isFinite(value) ? value : 0;
}

function compareVideoQuality(left, right) {
  return resolutionPixels(right) - resolutionPixels(left)
    || numericQuality(right, 'fps') - numericQuality(left, 'fps')
    || numericQuality(right, 'bitrate') - numericQuality(left, 'bitrate')
    || numericQuality(right, 'filesize') - numericQuality(left, 'filesize');
}

function compareAudioQuality(left, right) {
  return numericQuality(right, 'bitrate') - numericQuality(left, 'bitrate')
    || numericQuality(right, 'sampleRate') - numericQuality(left, 'sampleRate')
    || numericQuality(right, 'filesize') - numericQuality(left, 'filesize');
}

export function formatsForContent(item, contentKind) {
  const formats = item.formats || [];
  if (contentKind === 'audio') {
    const audioOnly = formats.filter(format =>
      format.kind === 'audio' && formatHasAudio(format));
    const muxed = formats.filter(format =>
      format.kind === 'muxed' && formatHasAudio(format));
    return (audioOnly.length ? audioOnly : muxed)
      .slice()
      .sort(compareAudioQuality);
  }
  return formats
    .filter(format => format.kind === 'video' || format.kind === 'muxed')
    .slice()
    .sort(compareVideoQuality);
}

export function defaultFormatId(item, contentKind) {
  const formats = formatsForContent(item, contentKind);
  const preferred = contentKind === 'audio' ? item.defaultAudio : item.defaultVideo;
  return formats.some(format => format.id === preferred) ? preferred : formats.at(0)?.id || '';
}

export function availableContentKinds(item) {
  return ['video', 'audio'].filter(kind => formatsForContent(item, kind).length > 0);
}

export function subtitleChoices(item) {
  return (item.subtitles || []).filter(subtitle =>
    subtitle.language?.toLowerCase() !== 'live_chat');
}

export function itemHasAudio(item) {
  return itemAudioPresence(item) === 'present';
}

function formatHasAudio(format) {
  return formatAudioPresence(format) === 'present';
}

export function formatAudioPresence(format) {
  if (['present', 'absent', 'unknown'].includes(format.audioPresence)) {
    return format.audioPresence;
  }
  const codec = String(format.audioCodec || (format.kind === 'audio' ? format.codec : ''))
    .trim().toLowerCase();
  if ((codec && codec !== 'none') || Number(format.sampleRate) > 0) return 'present';
  return format.kind === 'video' ? 'unknown' : 'absent';
}

export function itemAudioPresence(item) {
  const states = (item.formats || []).map(formatAudioPresence);
  if (states.includes('present')) return 'present';
  return states.includes('unknown') ? 'unknown' : 'absent';
}

export function audioFormatSelectorVisible(contentKind, audioFormats, selectedVideo) {
  if (contentKind === 'video') {
    return selectedVideo?.kind !== 'muxed' && audioFormats.length > 0;
  }
  return !(audioFormats.length === 1 && audioFormats[0].kind === 'muxed');
}

function codecStartsWith(codec, prefixes) {
  const normalized = String(codec || '').trim().toLowerCase();
  return prefixes.some(prefix => normalized.startsWith(prefix));
}

function unique(values) {
  return [...new Set(values.filter(Boolean))];
}

function nativeAudioOutput(format) {
  const codec = format?.audioCodec || format?.codec;
  if (codecStartsWith(codec, ['mp4a', 'aac'])) return 'm4a';
  if (codecStartsWith(codec, ['opus'])) return 'opus';
  if (codecStartsWith(codec, ['vorbis'])) return 'vorbis';
  if (codecStartsWith(codec, ['mp3'])) return 'mp3';
  return ['m4a', 'opus', 'aac', 'mp3', 'flac', 'vorbis', 'alac']
    .includes(format?.audioExt || format?.ext)
    ? format.audioExt || format.ext
    : 'm4a';
}

export function outputFormatsForChoice(item, choice) {
  const formats = item.formats || [];
  if (choice.contentKind === 'audio') {
    const audio = formats.find(format => format.id === choice.audioFormatId);
    return unique([nativeAudioOutput(audio), 'mp3', 'm4a', 'flac', 'opus', 'aac', 'vorbis', 'alac']);
  }
  const video = formats.find(format => format.id === choice.videoFormatId);
  const audio = video?.kind === 'muxed'
    ? video
    : formats.find(format => format.id === choice.audioFormatId);
  const compatible = [];
  if (codecStartsWith(video?.codec, ['h264', 'h.264', 'avc', 'hevc', 'h265', 'h.265', 'av1', 'av01'])
      && codecStartsWith(audio?.audioCodec || audio?.codec, ['aac', 'mp4a', 'mp3'])) {
    compatible.push('mp4');
  }
  if (codecStartsWith(video?.codec, ['vp8', 'vp9', 'vp09', 'av1', 'av01'])
      && codecStartsWith(audio?.audioCodec || audio?.codec, ['opus', 'vorbis'])) {
    compatible.push('webm');
  }
  if (video?.kind === 'muxed' && ['mp4', 'webm', 'mkv'].includes(video.ext)) {
    compatible.unshift(video.ext);
  }
  if (!audio && ['mp4', 'webm', 'mkv'].includes(video?.ext)) {
    compatible.unshift(video.ext);
  }
  return unique([...compatible, 'mkv']);
}

export function resolvedOutputFormat(item, choice) {
  if (choice.outputFormat) return choice.outputFormat;
  return outputFormatsForChoice(item, choice).at(0) ||
    (choice.contentKind === 'audio' ? 'm4a' : 'mkv');
}

export function recommendedDownloadChoice(item, contentKind) {
  const choice = {
    contentKind,
    videoFormatId: contentKind === 'video' ? defaultFormatId(item, 'video') : '',
    audioFormatId: defaultFormatId(item, 'audio'),
    subtitleLanguage: '',
    subtitleAutomatic: false,
    subtitleMode: 'off',
    chapterMode: 'off',
    useAria2: false,
    splitChapters: false,
    rangeStartSeconds: null,
    rangeEndSeconds: null,
    chapterIndices: [],
    outputFormat: '',
  };
  choice.outputFormat = resolvedOutputFormat(item, choice);
  return choice;
}

export function pageDownloadScope(url) {
  const parsed = new URL(url);
  const host = parsed.hostname.toLowerCase().replace(/^www\./, '');
  const youtube = host === 'youtu.be' || host === 'youtube.com' || host.endsWith('.youtube.com');
  if (!youtube || !parsed.searchParams.get('list')) {
    return { value: 'current', selectable: false };
  }
  const hasCurrentVideo = host === 'youtu.be'
    || parsed.searchParams.has('v')
    || /^\/(shorts|live)\//.test(parsed.pathname);
  return hasCurrentVideo
    ? { value: 'current', selectable: true }
    : { value: 'playlist', selectable: false };
}

export function buildDownloadRequest(item, choice) {
  const audioOnly = choice.contentKind === 'audio';
  const formats = item.formats || [];
  const video = audioOnly ? null : formats.find(value => value.id === choice.videoFormatId);
  const audio = formats.find(value => value.id === choice.audioFormatId);
  if (audioOnly && !audio) throw new Error(t('selectAudioFormat'));
  if (!audioOnly && !video) throw new Error(t('selectVideoFormat'));
  const hasSeparateAudio = formats.some(format =>
    format.kind === 'audio' && formatHasAudio(format));
  if (!audioOnly && video.kind === 'video' && hasSeparateAudio
      && (!audio || audio.kind !== 'audio')) {
    throw new Error(t('selectMergeAudioFormat'));
  }
  const muxed = !audioOnly && video.kind === 'muxed';
  const downloadTimeRange = downloadTimeRangeFromChoice(choice);
  const downloadChapterRanges = chapterDownloadRangesFromChoice(item, choice);
  const chapterMode = ['off', 'download', 'embed'].includes(choice.chapterMode)
    ? choice.chapterMode
    : Boolean(choice.splitChapters) ? 'download' : 'off';
  return {
    action: 'download',
    url: item.url,
    title: item.title,
    fileName:
      typeof choice.fileName === 'string' && choice.fileName.trim()
        ? choice.fileName.trim()
        : item.title,
    contentKind: choice.contentKind,
    videoFormatId: audioOnly ? '' : video.id,
    videoExt: audioOnly ? '' : video.ext || '',
    videoIsMuxed: muxed,
    audioFormatId: audioOnly ? audio.id : muxed ? '' : audio?.id || '',
    audioExt: audioOnly ? audio.audioExt || audio.ext || '' : muxed ? '' : audio?.ext || '',
    subtitleLanguage: audioOnly ? '' : choice.subtitleLanguage || '',
    subtitleAutomatic: !audioOnly && Boolean(choice.subtitleAutomatic),
    subtitleMode: !audioOnly
      && choice.subtitleLanguage
      && ['download', 'embed'].includes(choice.subtitleMode)
      ? choice.subtitleMode
      : 'off',
    thumbnailMode: ['off', 'download', 'embed'].includes(choice.thumbnailMode)
      ? choice.thumbnailMode
      : 'off',
    chapterMode,
    splitChapters: chapterMode !== 'off',
    useAria2: Boolean(choice.useAria2),
    downloadTimeRange,
    downloadChapterRanges,
    outputFormat: choice.outputFormat || resolvedOutputFormat(item, choice),
    playlistScope: 'current',
  };
}
