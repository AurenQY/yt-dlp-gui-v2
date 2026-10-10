<script>
  import { Block, Link, List, ListInput, Navbar, Page, Toolbar } from 'framework7-svelte';
  import { popupController as controller } from '../popup-runtime.js';
  import { galleryPostGroups } from '../../shared/gallery-post-groups.js';
  import { rangeForTimeline, timelineRange } from '../../shared/gallery-timeline-range.js';
  import { t } from '../../shared/i18n.js';
  export let galleryIndex;
  let initialized = false;
  let kind = 'posts'; let count = 10; let days = 7; let date = '';
  $: state = $controller;
  $: item = galleryPostGroups(state.analysisByBackend.g?.items || [], state.sourceUrl)[Number(galleryIndex)];
  $: if (item && !initialized) { const range = rangeForTimeline(item, state); kind = range.kind; count = range.count || 10; days = range.days || 7; date = range.date || ''; initialized = true; }
  $: draft = kind === 'posts' ? {kind,count:Number(count)} : kind === 'recent' ? {kind,days:Number(days)} : kind === 'since' ? {kind,date} : {kind};
  $: valid = validRange(draft);
  function validRange(value) { try { timelineRange(value); return true; } catch { return false; } }
</script>
<Page name="gallery-range">
  <Navbar title={t('downloadRange')} backLink={true} />
  {#if item?.sourceScope === 'timeline'}
    <List strong inset class="option-list">
      <ListInput label={t('timelineRangeMethod')} type="select" value={kind} onChange={event => kind = event.currentTarget.value}>
        <option value="posts">{t('rangePosts')}</option><option value="recent">{t('rangeTime')}</option><option value="since">{t('rangeSinceChoice')}</option><option value="all">{t('rangeAll')}</option>
      </ListInput>
      {#if kind === 'posts'}
        <ListInput label={t('rangeCount')} type="number" min="1" step="1" value={count} onInput={event => count = event.currentTarget.value} />
      {:else if kind === 'recent'}
        <ListInput label={t('rangeDays')} type="number" min="1" max="36500" step="1" value={days} onInput={event => days = event.currentTarget.value} />
      {:else if kind === 'since'}
        <ListInput label={t('rangeDate')} type="date" value={date} onInput={event => date = event.currentTarget.value} />
      {/if}
    </List>
    {#if kind === 'posts'}
      <Block inset class="range-actions range-presets">{#each [10,50,100] as n}<Link class={Number(count) === n ? 'button button-fill' : 'button button-outline'} onClick={() => count = n}>{t('rangeLatest', { count: n })}</Link>{/each}</Block>
    {:else if kind === 'recent'}
      <Block inset class="range-actions range-presets">{#each [1,7,30] as n}<Link class={Number(days) === n ? 'button button-fill' : 'button button-outline'} onClick={() => days = n}>{t('rangeRecentDays', { days: n })}</Link>{/each}</Block>
    {/if}
    {#if !valid}<Block class="range-error">{t('rangeInvalid')}</Block>{/if}
    <Toolbar bottom class="range-toolbar"><div class="range-actions"><Link back href="#" class="button button-fill" disabled={!valid ? true : undefined} onClick={() => valid && controller.setGalleryRange(item.key, draft)}>{t('confirm')}</Link></div></Toolbar>
  {/if}
</Page>
