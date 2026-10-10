<script>
  import { Link } from 'framework7-svelte';
  import SourcePreview from './SourcePreview.svelte';
  import ActionIcon from './ActionIcon.svelte';
  import { popupController as controller } from '../popup-runtime.js';
  import { itemTaskState, itemSettingsSummary, selectedFormatDetails } from '../popup-view-model.js';
  import { popupResultSelected } from '../popup-result-list.js';
  import { durationLabel, formatSummary } from '../../shared/format-display.js';
  import { audioFormatSelectorVisible, formatsForContent } from '../../shared/media-download-options.js';
  import { t } from '../../shared/i18n.js';
  import { galleryDownloadModeLabelKey } from '../../shared/gallery-download-mode.js';
  import { rangeForTimeline, timelineRangeSummary } from '../../shared/gallery-timeline-range.js';
  export let result;
  export let state;
  export let count;
  $: item = result.item;
  $: gallery = result.backend === 'g';
  $: locked = state.busy || state.analysisRunning || state.downloadRunning;
  $: task = itemTaskState(state, result.index, result.key);
  $: details = gallery ? null : selectedFormatDetails(item, state.choices[result.index] || {}, controller.storedDownloadChoice);
  $: choice = gallery ? null : controller.storedDownloadChoice(item, state.choices[result.index] || {});
  $: settings = gallery ? [] : itemSettingsSummary(state, result.index, controller.storedDownloadChoice);
  $: selected = popupResultSelected(state, result, count);
  $: video = item.formats?.find(format => format.id === choice?.videoFormatId);
  $: audio = item.formats?.find(format => format.id === choice?.audioFormatId);
  $: showAudio = !gallery && audioFormatSelectorVisible(choice.contentKind, formatsForContent(item, 'audio').filter(format => format.kind === 'audio'), video);
  $: hasAudioRow = !gallery && (showAudio || choice.contentKind === 'audio');
  $: route = gallery ? `/gallery/${result.index}/` : `/item/${result.index}/`;
  $: mode = gallery ? controller.galleryModeForGroup(item, state) : '';
  $: modeLabel = gallery ? t(galleryDownloadModeLabelKey(mode)) : '';
  $: title = gallery ? item.canonicalTitle || [state.sourceSiteName, item.creator && `@${item.creator.replace(/^@/, '')}`].filter(Boolean).join(' · ') || item.title : item.title || item.url;
  $: date = String(item.date || '').replace('T', ' ').replace(/:\d\d(?:\.\d+)?(?:Z|[+-]\d\d:\d\d)?$/, '');
  function select(event) {
    if (gallery) controller.toggleGalleryGroup(result.index, event.currentTarget.checked);
    else controller.toggleSelection(result.index, event.currentTarget.checked);
  }
  function prepare() { if (!gallery) controller.prepareDetail(result.index); }
</script>

<article class="result-card" aria-label={`${gallery ? 'gallery-dl' : 'yt-dlp'} · ${title}`}>
  <div class="result-cover">
    <div class="result-preview">
    <SourcePreview sources={[item.thumbnail].filter(Boolean)} siteIcon={state.sourceSiteIconUrl} />
    {#if count > 1}
      <label class="result-select-toggle">
        <input type="checkbox" aria-label={title} checked={selected} disabled={locked} onchange={select} />
        {#if selected}<ActionIcon name="check" />{/if}
      </label>
    {/if}
    {#if task?.state === 'completed'}<span class="cover-status" aria-label={t('completed')}>✓</span>{/if}
    {#if gallery || item.duration}<span class="cover-count">{gallery ? t('galleryMediaCount', { count: item.media.length }) : durationLabel(item.duration)}</span>{/if}
    </div>
    <span class="backend-label" class:backend-gallery={gallery} class:backend-video={!gallery}>{gallery ? 'gallery-dl' : 'yt-dlp'}</span>
  </div>
  <div class="result-content">
    <Link href={route} class="result-summary" disabled={locked ? true : undefined} tabindex={locked ? -1 : undefined} onClick={prepare} aria-label={`${t('downloadContent')} · ${title}`}>
      <span class="result-title" class:has-site-icon={Boolean(state.sourceSiteIconUrl)}>{#if state.sourceSiteIconUrl}<span class="result-title-icon"><img class="site-icon" src={state.sourceSiteIconUrl} alt="" /></span>{/if}<span class="result-title-text">{title}</span></span>
      <div class="result-summary-details" class:result-media-summary={!gallery}>
      {#if gallery}
        <span class="result-date">{date}</span><span class="result-format-summary">{modeLabel}</span>
        {#if item.sourceScope === 'timeline'}<span class="result-format-summary">{timelineRangeSummary(rangeForTimeline(item, state))}</span>{/if}
      {:else}
        {#if choice.contentKind !== 'audio'}
          <div class="result-media-row"><span class="result-media-label">{t('summaryVideo')}</span><span>{formatSummary(video, false)}</span><span class="result-output">{hasAudioRow ? '' : details.container}</span></div>
        {/if}
        {#if hasAudioRow}
          <div class="result-media-row"><span class="result-media-label">{t('summaryAudio')}</span><span>{formatSummary(audio, true)}</span><span class="result-output">{details.container}</span></div>
        {/if}
      {/if}
      </div>
    </Link>
    {#if gallery}
      <div class="result-album-actions">
        <button class="result-album-button" disabled={locked} onclick={() => controller.openGalleryAlbum(result.index)}><span>{t('album')}</span><ActionIcon name="external" /></button>
      </div>
    {/if}
    {#if settings.length}
      <dl class="result-options">
        {#each settings as setting}<div><dt>{setting.label}</dt><dd>{setting.value}</dd></div>{/each}
      </dl>
    {/if}
    {#if task && task.state !== 'completed'}
      <div class="result-task" role="status">{gallery && task.state === 'running' ? t('downloading') : task.label}</div>
      {#if !gallery}<progress max="100" value={task.percent} aria-label={task.label}></progress>{/if}
    {/if}
  </div>
</article>
