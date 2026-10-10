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
            <span class="format-cell">{value}</span>
          {/each}
        </Link>
      {/each}
    </div>
  {:else}
    <Block strong inset>{t('noFormats')}</Block>
  {/if}
</Page>
