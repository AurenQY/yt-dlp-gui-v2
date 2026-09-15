<script>
  import { Block, Link, Navbar, Page } from 'framework7-svelte';
  import { popupController as controller } from '../popup-runtime.js';
  import { detailViewModel } from '../popup-view-model.js';
  import {
    bitrateLabel,
    codecLabel,
    filesizeLabel,
    sampleRateLabel,
  } from '../../shared/format-display.js';
  import { t } from '../../shared/i18n.js';

  export let itemIndex;
  export let formatKind;

  $: index = Number(itemIndex);
  $: audio = formatKind === 'audio';
  $: state = $controller;
  $: vm = detailViewModel(state, index);
  $: formats = vm ? (audio ? vm.audioFormats : vm.videoFormats) : [];
  $: selectedId = vm
    ? String(audio ? vm.choice.audioFormat || '' : vm.choice.videoFormat || '')
    : '';
  $: pageTitle = t(audio ? 'audioFormat' : 'videoFormat');

  function cells(format) {
    if (audio) {
      return [
        String(format.language || '').trim() || '—',
        sampleRateLabel(format.sampleRate) || '—',
        codecLabel(format.audioCodec || format.codec) || '—',
        bitrateLabel(format.bitrate) || '—',
        filesizeLabel(format.filesize, format.filesizeApproximate) || '—',
      ];
    }
    const codecs = [
      codecLabel(format.codec),
      format.kind === 'muxed' ? codecLabel(format.audioCodec) : '',
    ].filter(Boolean).join(' + ');
    return [
      format.resolution || '—',
      format.fps ? String(format.fps) : '—',
      codecs || '—',
      filesizeLabel(format.filesize, format.filesizeApproximate) || '—',
    ];
  }

  function selectFormat(formatId) {
    controller.setChoice(index, {
      [audio ? 'audioFormat' : 'videoFormat']: String(formatId),
    });
  }
</script>

<Page name="format-grid">
  <Navbar title={pageTitle} backLink={true} />

  {#if formats.length}
    <div
      class="format-grid"
      class:audio-grid={audio}
      role="radiogroup"
      aria-label={pageTitle}
    >
      <div class="format-header" role="row">
        <span aria-hidden="true"></span>
        {#if audio}
          <span>{t('language')}</span>
          <span>{t('sampleRate')}</span>
          <span>{t('codec')}</span>
          <span>{t('bitrate')}</span>
          <span>{t('size')}</span>
        {:else}
          <span>{t('resolution')}</span>
          <span>{t('fps')}</span>
          <span>{t('codec')}</span>
          <span>{t('size')}</span>
        {/if}
      </div>

      {#each formats as format}
        {@const formatId = String(format.id)}
        {@const values = cells(format)}
        <Link
          href="#"
          back
          noLinkClass
          class={`format-row${formatId === selectedId ? ' selected' : ''}`}
          role="radio"
          aria-checked={formatId === selectedId}
          onClick={() => selectFormat(formatId)}
        >
          <span class="radio-cell">
            <input
              type="radio"
              name={`${formatKind}-format-${index}`}
              value={formatId}
              checked={formatId === selectedId}
              tabindex="-1"
              aria-hidden="true"
            />
          </span>
          {#each values as value}
            <span class="format-cell" title={value}>{value}</span>
          {/each}
        </Link>
      {/each}
    </div>
  {:else}
    <Block strong inset>{t('noFormats')}</Block>
  {/if}
</Page>

<style>
  .format-grid {
    --format-columns: 38px minmax(92px, 1.15fr) 48px minmax(104px, 1.35fr) minmax(72px, 0.9fr);
    width: 100%;
    min-width: 0;
  }

  .format-grid.audio-grid {
    --format-columns: 38px minmax(58px, 0.75fr) minmax(72px, 0.9fr) minmax(78px, 1fr) minmax(62px, 0.7fr) minmax(70px, 0.8fr);
  }

  .format-header,
  :global(.format-grid .format-row) {
    display: grid;
    grid-template-columns: var(--format-columns);
    align-items: center;
    min-width: 0;
  }

  .format-header {
    position: sticky;
    z-index: 5;
    top: 0;
    min-height: 42px;
    color: var(--f7-list-item-text-text-color, var(--f7-text-color));
    background: var(--f7-page-bg-color);
    border-bottom: 1px solid var(--f7-list-divider-border-color, rgba(127, 127, 127, 0.24));
    font-size: 12px;
    font-weight: 600;
  }

  :global(.format-grid .format-row) {
    min-height: 52px;
    border-bottom: 1px solid var(--f7-list-divider-border-color, rgba(127, 127, 127, 0.24));
    cursor: pointer;
  }

  :global(.format-grid .format-row.selected) {
    background: rgba(var(--f7-theme-color-rgb), 0.12);
  }

  .radio-cell {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .radio-cell input {
    width: 18px;
    height: 18px;
    margin: 0;
    accent-color: var(--f7-theme-color);
    pointer-events: none;
  }

  .format-header > span:not(:first-child),
  .format-cell {
    min-width: 0;
    padding-inline: 5px;
  }

  .format-cell {
    overflow: hidden;
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
