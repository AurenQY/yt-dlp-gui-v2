<script>
  import {
    Block,
    Link,
    List,
    ListInput,
    Navbar,
    Page,
    Toolbar,
  } from 'framework7-svelte';
  import { popupController as controller } from '../popup-runtime.js';
  import { detailViewModel } from '../popup-view-model.js';
  import {
    formatTimecode,
    parseTimecode,
  } from '../../shared/download-time-range.js';
  import { t } from '../../shared/i18n.js';

  export let itemIndex;

  let initialized = false;
  let startText = '00:00:00';
  let endText = '';

  $: index = Number(itemIndex);
  $: state = $controller;
  $: vm = detailViewModel(state, index);
  $: if (vm && !initialized) {
    startText = formatTimecode(vm.timeRange?.startSeconds || 0);
    const durationSeconds = parseTimecode(vm.item.duration);
    endText = vm.timeRange
      ? formatTimecode(vm.timeRange.endSeconds)
      : durationSeconds === null ? '' : formatTimecode(durationSeconds);
    initialized = true;
  }
  $: startSeconds = parseTimecode(startText);
  $: endSeconds = parseTimecode(endText);
  $: durationSeconds = parseTimecode(vm?.item.duration);
  $: validationKey = startSeconds === null || endSeconds === null
    ? 'timeRangeInvalid'
    : endSeconds <= startSeconds
      ? 'timeRangeOrderInvalid'
      : durationSeconds !== null && endSeconds > durationSeconds
        ? 'timeRangeBeyondDuration'
        : '';
  $: valid = !validationKey;

  function setStart(event) {
    startText = event.currentTarget.value;
  }

  function setEnd(event) {
    endText = event.currentTarget.value;
  }

  function applyRange() {
    if (!valid) return;
    controller.setChoice(index, {
      rangeStartSeconds: startSeconds,
      rangeEndSeconds: endSeconds,
      chapterIndices: [],
      chapterMode: 'off',
      chapters: 'full',
    }, false);
  }

  function clearRange() {
    controller.setChoice(index, {
      rangeStartSeconds: null,
      rangeEndSeconds: null,
    }, false);
  }
</script>

<Page name="time-range">
  <Navbar title={t('downloadRange')} backLink={true} />

  <Block strong inset>{t('timeRangeHint')}</Block>

  <List strong inset>
    <ListInput
      label={t('rangeStart')}
      type="text"
      inputmode="numeric"
      value={startText}
      placeholder="00:00:00"
      onInput={setStart}
    />
    <ListInput
      label={t('rangeEnd')}
      type="text"
      inputmode="numeric"
      value={endText}
      placeholder="00:00:00"
      onInput={setEnd}
    />
  </List>

  {#if validationKey}
    <Block inset class="range-error">{t(validationKey)}</Block>
  {/if}

  <Toolbar bottom class="range-toolbar">
    <div class="range-actions">
      <Link href="#" back class="button button-outline" onClick={clearRange}>
        {t('fullVideo')}
      </Link>
      {#if valid}
        <Link href="#" back class="button button-fill" onClick={applyRange}>
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
  :global(.range-error) {
    color: var(--f7-color-red);
  }

  :global(.range-toolbar .toolbar-inner) {
    padding-inline: var(--shadow-action-inset);
  }

  .range-actions {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 10px;
    width: 100%;
  }
</style>
