<script>
  import { List, ListItem, Navbar, Page } from 'framework7-svelte';
  import { popupController as controller } from '../popup-runtime.js';
  import { galleryPostGroups } from '../../shared/gallery-post-groups.js';
  import { t } from '../../shared/i18n.js';
  import { galleryDownloadModeLabelKey } from '../../shared/gallery-download-mode.js';
  import SourcePreview from '../components/SourcePreview.svelte';
  import { rangeForTimeline, timelineRangeSummary } from '../../shared/gallery-timeline-range.js';

  export let galleryIndex;
  $: index = Number(galleryIndex);
  $: state = $controller;
  $: groups = galleryPostGroups(state.analysisByBackend.g?.items || [], state.sourceUrl);
  $: item = groups[index];
  $: method = item ? controller.galleryModeForGroup(item, state) : 'browser-files';
</script>

<Page name="gallery-detail">
  <Navbar title={t('downloadContent')} backLink={true} />
  {#if item}
    <article class="source-card">
      <SourcePreview sources={[item.thumbnail].filter(Boolean)} siteIcon={state.sourceSiteIconUrl} />
      <div class="source-card-content"><div class="source-card-title">{item.title || item.sourcePageUrl}</div><span class="result-date">{[item.date, t('galleryMediaCount', { count: item.media.length })].filter(Boolean).join(' · ')}</span></div>
    </article>
    <List strong inset class="option-list">
      {#if item.sourceScope === 'timeline'}
        <ListItem title={t('downloadRange')} link={`/gallery/${index}/range/`} disabled={state.busy || state.downloadRunning || state.analysisRunning}>
          {#snippet after()}<span class="format-choice-summary">{timelineRangeSummary(rangeForTimeline(item, state))}</span>{/snippet}
        </ListItem>
      {/if}
      <ListItem title={t('galleryDownloadMode')} link={`/gallery/${index}/format/`}
        disabled={state.busy || state.downloadRunning || state.analysisRunning}>
        {#snippet after()}
          <span class="format-choice-summary">{t(galleryDownloadModeLabelKey(method))}</span>
        {/snippet}
      </ListItem>
    </List>
  {:else}
    <List strong dividers outline inset><ListItem title={t('noFormats')} /></List>
  {/if}
</Page>
