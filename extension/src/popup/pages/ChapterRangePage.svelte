<script>
  import {
    Block,
    Button,
    Link,
    List,
    ListItem,
    Navbar,
    Page,
    Segmented,
    Toolbar,
  } from 'framework7-svelte';
  import { popupController as controller } from '../popup-runtime.js';
  import { detailViewModel } from '../popup-view-model.js';
  import { formatTimecode } from '../../shared/download-time-range.js';
  import { t } from '../../shared/i18n.js';

  export let itemIndex;

  let initialized = false;
  let selected = new Set();

  $: index = Number(itemIndex);
  $: state = $controller;
  $: vm = detailViewModel(state, index);
  $: chapters = vm?.item.chapters || [];
  $: if (vm && !initialized) {
    selected = new Set(vm.chapterIndices);
    initialized = true;
  }

  function chapterTime(chapter) {
    return `${formatTimecode(Math.floor(chapter.startMillis / 1_000))}–${formatTimecode(Math.floor(chapter.endMillis / 1_000))}`;
  }

  function toggleChapter(chapterIndex, event) {
    const next = new Set(selected);
    if (event.currentTarget.checked) next.add(chapterIndex);
    else next.delete(chapterIndex);
    selected = next;
  }

  function selectAll() {
    selected = new Set(chapters.map((_, chapterIndex) => chapterIndex));
  }

  function clearDraft() {
    selected = new Set();
  }

  function applyChapters() {
    if (!selected.size) return;
    controller.setChoice(index, {
      chapterIndices: [...selected].sort((left, right) => left - right),
      rangeStartSeconds: null,
      rangeEndSeconds: null,
      chapterMode: 'off',
      chapters: 'full',
    }, false);
  }

  function useFullVideo() {
    controller.setChoice(index, {
      chapterIndices: [],
    }, false);
  }
</script>

<Page name="chapter-range">
  <Navbar title={t('chapterRange')} backLink={true} />

  <Block strong inset>{t('chapterRangeHint')}</Block>

  <Block inset>
    <Segmented raised round>
      <Button onClick={selectAll}>{t('selectAll')}</Button>
      <Button onClick={clearDraft}>{t('clearSelection')}</Button>
    </Segmented>
  </Block>

  <List strong dividers outline inset>
    {#each chapters as chapter, chapterIndex}
      <ListItem
        checkbox
        checked={selected.has(chapterIndex)}
        title={chapter.title || t('chapterNumber', { number: chapterIndex + 1 })}
        after={chapterTime(chapter)}
        onChange={(event) => toggleChapter(chapterIndex, event)}
      />
    {/each}
  </List>

  <Toolbar bottom class="chapter-range-toolbar">
    <div class="chapter-range-actions">
      <Link href="#" back class="button button-outline" onClick={useFullVideo}>
        {t('fullVideo')}
      </Link>
      {#if selected.size}
        <Link href="#" back class="button button-fill" onClick={applyChapters}>
          {t('confirm')}
        </Link>
      {:else}
        <span class="button button-fill disabled" aria-disabled="true">
          {t('confirm')}
        </span>
      {/if}
    </div>
  </Toolbar>
</Page>

<style>
  :global(.chapter-range-toolbar .toolbar-inner) {
    padding-inline: var(--shadow-action-inset);
  }

  .chapter-range-actions {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 10px;
    width: 100%;
  }

  :global([data-name="chapter-range"] .item-after) {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
</style>
