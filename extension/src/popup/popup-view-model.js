import {
  audioFormatSelectorVisible,
  availableContentKinds,
  formatsForContent,
  itemAudioPresence,
  outputFormatsForChoice,
  resolvedOutputFormat,
  subtitleChoices,
} from '../shared/media-download-options.js';
import { codecLabel, filesizeLabel, formatSummary, sampleRateLabel } from '../shared/format-display.js';
import {
  downloadTimeRangeFromChoice,
  formatTimecode,
} from '../shared/download-time-range.js';
import { selectedChapterIndicesFromChoice } from '../shared/download-chapter-range.js';
import { t } from '../shared/i18n.js';
import { protocolSupports } from '../shared/native-protocol.js';

export function analysisStatusText(state) {
  const key = {
    preparing: 'analysisPrepareBrowser',
    connectingHost: 'analysisConnectHost',
    validatingHost: 'analysisValidateTools',
    startingYtDlp: 'analysisLaunchYtDlp',
    startingGalleryDl: 'analysisLaunchGalleryDl',
    readingMedia: 'analysisReadMedia',
    readingPlaylist: 'analysisReadPlaylist',
  }[state.analysisPhase] || 'analysisPrepareBrowser';
  return state.analysisPhase === 'readingPlaylist'
    ? `${t(key)} · ${t('analysisPlaylistLimitDetail', { count: state.playlistItemLimit })}`
    : t(key);
}

export function analysisCountText(state) {
  const data = Object.values(state.analysisByBackend).find(result => result?.analyzing)
    || state.analysisByBackend.y || state.analysisByBackend.g;
  if (!data) return '';
  // gallery-dl media count is independent of the yt-dlp playlist limit.
  if (data.backend === 'g') {
    if (data.stopped) return t('stoppedCount', { count: data.items.length });
    return data.items.length ? t('galleryMediaCount', { count: data.items.length }) : '';
  }
  const limit = data.itemLimit || state.playlistItemLimit || 32;
  if (data.analyzing) return t('analyzedCount', { count: data.items.length, limit });
  if (data.stopped) return t('stoppedCount', { count: data.items.length });
  if (data.truncated) return t('showFirstItems', { count: limit });
  return data.items.length > 1 ? t('mediaCountLimit', { count: data.items.length, limit }) : '';
}

export function pageSnapshotSummary(state) {
  const snapshot = state.sourcePageSnapshot;
  if (!snapshot) return '';
  const width = Number(snapshot.mediaWidth);
  const height = Number(snapshot.mediaHeight);
  const resolution = Number.isFinite(width) && width > 0
    && Number.isFinite(height) && height > 0
    ? `${Math.round(width)}×${Math.round(height)}`
    : '';
  return [snapshot.creator, resolution].filter(Boolean).join(' · ');
}

export function selectedFormatDetails(item, saved, storedChoice) {
  const choice = storedChoice(item, saved);
  const formats = item.formats || [];
  const video = formats.find(format => format.id === choice.videoFormatId);
  const audio = formats.find(format => format.id === choice.audioFormatId);
  const container = resolvedOutputFormat(item, choice).toUpperCase();

  if (choice.contentKind === 'audio') {
    const stream = [
      String(audio?.language || '').trim(),
      sampleRateLabel(audio?.sampleRate),
      codecLabel(audio?.audioCodec || audio?.codec),
    ]
      .filter(Boolean).join(' · ') || t('audioSummary');
    return {
      stream,
      container,
      size: filesizeLabel(audio?.filesize, audio?.filesizeApproximate),
    };
  }

  const selectedAudio = video?.kind === 'muxed' ? video : audio;
  const videoPart = [video?.resolution, codecLabel(video?.codec)].filter(Boolean).join(' · ');
  const audioPart = [
    String(selectedAudio?.language || '').trim(),
    sampleRateLabel(selectedAudio?.sampleRate),
    codecLabel(selectedAudio?.audioCodec || (selectedAudio?.kind === 'audio' ? selectedAudio?.codec : '')),
  ].filter(Boolean).join(' · ');
  const stream = audioPart
    ? `${videoPart} + ${audioPart}`
    : `${videoPart} · ${itemAudioPresence(item) === 'absent' ? t('noAudio') : t('audioUnknown')}`;
  const separateAudio = video?.kind !== 'muxed' && audio;
  const totalSize = video?.filesize && (!separateAudio || audio?.filesize)
    ? video.filesize + (separateAudio ? audio.filesize : 0)
    : 0;
  const approximate = Boolean(video?.filesizeApproximate || (separateAudio && audio?.filesizeApproximate));
  return {
    stream,
    container,
    size: filesizeLabel(totalSize, approximate),
  };
}

export function selectedFormatSummary(item, saved, storedChoice) {
  const details = selectedFormatDetails(item, saved, storedChoice);
  return `${details.stream}\n${[details.container, details.size].filter(Boolean).join(' · ')}`;
}

export function detailViewModel(state, index) {
  const item = state.analysis?.items?.[index];
  if (!item) return null;
  const choice = state.choices[index] || {};
  const contentKind = choice.contentKind || 'video';
  const videoFormats = formatsForContent(item, 'video');
  const audioFormats = contentKind === 'audio'
    ? formatsForContent(item, 'audio')
    : formatsForContent(item, 'audio').filter(format => format.kind === 'audio');
  const selectedVideo = videoFormats.find(format => format.id === choice.videoFormat);
  const selectedAudio = audioFormats.find(format => format.id === choice.audioFormat);
  const requestChoice = {
    contentKind,
    videoFormatId: choice.videoFormat || '',
    audioFormatId: choice.audioFormat || '',
    outputFormat: choice.outputFormat || '',
  };
  const timeRange = downloadTimeRangeFromChoice(choice);
  const chapterIndices = selectedChapterIndicesFromChoice(item, choice);
  return {
    item,
    choice,
    contentKinds: availableContentKinds(item),
    videoFormats,
    audioFormats,
    selectedVideo,
    selectedAudio,
    showAudioFormat: audioFormatSelectorVisible(contentKind, audioFormats, selectedVideo),
    outputs: outputFormatsForChoice(item, requestChoice),
    subtitles: subtitleChoices(item),
    timeRange,
    timeRangeSummary: timeRange
      ? `${formatTimecode(timeRange.startSeconds)}–${formatTimecode(timeRange.endSeconds)}`
      : t('fullVideo'),
    chapterIndices,
    chapterRangeSummary: chapterIndices.length
      ? t('selectedChapters', { count: chapterIndices.length })
      : t('fullVideo'),
    videoSummary: formatSummary(selectedVideo, false),
    audioSummary: formatSummary(selectedAudio, true),
  };
}

/** Main cards only expose active extras; off/full-file defaults stay in Detail. */
export function itemSettingsSummary(state, index, storedChoice) {
  const vm = detailViewModel(state, index);
  if (!vm) return [];
  const choice = storedChoice(vm.item, vm.choice);
  const host = state.analysis?.host;
  const rows = [];
  const modeLabel = (prefix, mode) => t(`${prefix}${mode === 'embed' ? 'Embed' : mode === 'download' ? 'Download' : 'Off'}`);
  if (['download', 'embed'].includes(choice.thumbnailMode)
      && protocolSupports(host, 'postprocess.thumbnail-mode.v1')) {
    rows.push({ label: t('thumbnailMode'), value: modeLabel('thumbnail', choice.thumbnailMode) });
  }
  if (choice.contentKind === 'video' && vm.subtitles.length
      && ['download', 'embed'].includes(choice.subtitleMode)) {
    const subtitle = vm.subtitles.find(item => item.language === choice.subtitleLanguage);
    rows.push({ label: t('subtitles'), value: `${subtitle?.label || choice.subtitleLanguage} · ${modeLabel('subtitle', choice.subtitleMode)}` });
  }
  if (vm.item.chapterCount > 0 && ['download', 'embed'].includes(choice.chapterMode)
      && protocolSupports(host, 'postprocess.chapter-mode.v1')) {
    rows.push({ label: t('chapterMode'), value: modeLabel('chapter', choice.chapterMode) });
  }
  if (choice.chapterIndices?.length || vm.timeRange) {
    rows.push({ label: t('downloadRange'), value: choice.chapterIndices?.length ? vm.chapterRangeSummary : vm.timeRangeSummary });
  }
  return rows;
}

export function itemTaskState(state, index, key = `y:${index}`) {
  const browserTask = state.browserDownloadTask;
  const task = key.startsWith('g:') && browserTask
    && (key === browserTask.currentSourceItemKey || browserTask.completedSourceItemKeys?.includes(key))
    ? browserTask : state.currentTask;
  const applies = Number.isInteger(task.sourceTabId)
    ? task.sourceTabId === state.sourceTabId
    : task.sourcePageUrl === state.sourceUrl;
  if (!applies) return null;
  if (task.sourceAnalysisStartedAt
      && task.sourceAnalysisStartedAt !== state.unifiedAnalysis?.request?.startedAt) return null;
  const active = ['running', 'stopping'].includes(task.state);
  const completed = new Set(task.completedSourceItemKeys || []);
  if (completed.has(key)) {
    return { state: 'completed', percent: 100, label: t('completed') };
  }
  if (key === task.currentSourceItemKey && (active || ['failed', 'stopped'].includes(task.state))) {
    const percent = Math.max(0, Math.min(100, Number(task.itemPercent) || 0));
    const label = task.state === 'failed' ? t('downloadFailed')
      : task.state === 'stopped' ? t('downloadStopped')
      : task.state === 'stopping' ? t('stopping')
      : [
          `${percent.toFixed(0)}%`,
          task.speed,
          task.eta ? t('remaining', { eta: task.eta }) : '',
        ].filter(Boolean).join(' · ');
    return { state: task.state, percent, label };
  }
  return null;
}

export function sourceScopeLabel(state) {
  return state.scopeLabel || t('currentPage');
}
