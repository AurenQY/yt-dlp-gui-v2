<script>
  import {
    Block,
    Button,
    List,
    ListInput,
    ListItem,
    Navbar,
    Page,
    Segmented,
    Toggle,
  } from 'framework7-svelte';
  import { popupController as controller } from '../popup-runtime.js';
  import { detailViewModel } from '../popup-view-model.js';
  import { durationLabel } from '../../shared/format-display.js';
  import { t } from '../../shared/i18n.js';
  import { protocolSupports } from '../../shared/native-protocol.js';

  export let itemIndex;

  $: index = Number(itemIndex);
  $: state = $controller;
  $: vm = detailViewModel(state, index);
  $: thumbnailModeSupported = protocolSupports(
    state.analysis?.host,
    'postprocess.thumbnail-mode.v1',
  );
  $: subtitleModeSupported = protocolSupports(
    state.analysis?.host,
    'postprocess.subtitle-mode.v1',
  );
  $: chapterModeSupported = protocolSupports(
    state.analysis?.host,
    'postprocess.chapter-mode.v1',
  );

  function setContentKind(kind) {
    controller.setChoice(index, { contentKind: kind });
  }

  function setFileName(event) {
    controller.setChoice(index, { fileName: event.currentTarget.value }, false);
  }

  function setOutput(event) {
    controller.setChoice(index, { outputFormat: event.currentTarget.value });
  }

  function setThumbnailMode(event) {
    controller.setChoice(index, { thumbnailMode: event.currentTarget.value });
  }

  function setSubtitle(event) {
    const subtitleLanguage = event.currentTarget.value;
    const subtitle = vm?.subtitles.find(value => value.language === subtitleLanguage);
    const configuredMode = subtitleModeSupported
      && ['download', 'embed'].includes(state.analysis?.subtitleMode)
      ? state.analysis.subtitleMode
      : 'download';
    const activeMode = ['download', 'embed'].includes(vm?.choice.subtitleMode)
      ? vm.choice.subtitleMode
      : configuredMode;
    controller.setChoice(index, {
      subtitleLanguage,
      subtitleAutomatic: Boolean(subtitle?.automatic),
      subtitleMode: subtitleLanguage ? activeMode : 'off',
    }, false);
  }

  function setSubtitleMode(event) {
    const subtitleMode = event.currentTarget.value;
    controller.setChoice(index, {
      subtitleMode,
      ...(subtitleMode === 'off'
        ? { subtitleLanguage: '', subtitleAutomatic: false }
        : {}),
    }, false);
  }

  function setChapters(event) {
    const split = event.currentTarget.checked;
    controller.setChoice(index, {
      chapterMode: split ? 'download' : 'off',
    });
  }

  function setChapterMode(event) {
    controller.setChoice(index, { chapterMode: event.currentTarget.value });
  }

</script>

<Page name="detail">
  <Navbar title={t('downloadContent')} backLink={true} />

  {#if vm}
    <List mediaList strong dividers outline inset>
      <ListItem
        mediaItem
        title={vm.item.title || vm.item.url}
        subtitle={durationLabel(vm.item.duration)}
      >
        {#snippet media()}
          {#if vm.item.thumbnail}
            <img class="detail-thumbnail" src={vm.item.thumbnail} width="96" height="54" alt="" />
          {/if}
        {/snippet}
      </ListItem>
    </List>

    {#if vm.contentKinds.length > 1}
      <Block inset>
        <Segmented raised round>
          <Button
            active={vm.choice.contentKind === 'video'}
            onClick={() => setContentKind('video')}
          >{t('video')}</Button>
          <Button
            active={vm.choice.contentKind === 'audio'}
            onClick={() => setContentKind('audio')}
          >{t('audioOnly')}</Button>
        </Segmented>
      </Block>
    {/if}

    <List strong inset>
  <ListInput
    class="file-name-input"
    label={t('fileName')}
        type="text"
        value={vm.choice.fileName}
        maxlength="200"
        clearButton
        info={t('fileNameBaseHint')}
        onInput={setFileName}
      />

      {#if vm.choice.contentKind === 'video'}
        <ListItem title={t('videoFormat')} link={`/item/${index}/format/video/`}>
          {#snippet after()}
            <span class="format-choice-summary">{vm.videoSummary}</span>
          {/snippet}
        </ListItem>
      {/if}

      {#if vm.choice.contentKind === 'audio' || vm.showAudioFormat}
        <ListItem title={t('audioFormat')} link={`/item/${index}/format/audio/`}>
          {#snippet after()}
            <span class="format-choice-summary">{vm.audioSummary}</span>
          {/snippet}
        </ListItem>
      {/if}

      <ListItem title={t('output')} smartSelect smartSelectParams={{ openIn: 'page', pageTitle: t('output'), searchbar: false, scrollToSelectedItem: true }}>
        <select name={`output-format-${index}`} value={vm.choice.outputFormat} onchange={setOutput}>
          {#each vm.outputs as output}
            <option value={output}>{output.toUpperCase()}</option>
          {/each}
        </select>
      </ListItem>

      {#if thumbnailModeSupported}
        <ListItem title={t('thumbnailMode')} smartSelect smartSelectParams={{ openIn: 'page', pageTitle: t('thumbnailMode'), searchbar: false, scrollToSelectedItem: true }}>
          <select name={`thumbnail-mode-${index}`} value={vm.choice.thumbnailMode} onchange={setThumbnailMode}>
            <option value="off">{t('thumbnailOff')}</option>
            <option value="download">{t('thumbnailDownload')}</option>
            <option value="embed">{t('thumbnailEmbed')}</option>
          </select>
        </ListItem>
      {/if}

      {#if vm.choice.contentKind === 'video' && vm.subtitles.length > 0}
        <ListItem
          title={t('subtitles')}
          smartSelect
          smartSelectParams={{ openIn: 'page', pageTitle: t('subtitles'), searchbar: false, scrollToSelectedItem: true }}
        >
          <select name={`subtitle-${index}`} value={vm.choice.subtitleLanguage || ''} onchange={setSubtitle}>
            <option value="">{t('noSubtitles')}</option>
            {#each vm.subtitles as subtitle}
              <option value={subtitle.language}>{subtitle.label || subtitle.language}{subtitle.automatic ? ` · ${t('automatic')}` : ''}</option>
            {/each}
          </select>
        </ListItem>

        {#if subtitleModeSupported && vm.choice.subtitleLanguage}
          <ListItem
            title={t('subtitleMode')}
            smartSelect
            smartSelectParams={{ openIn: 'page', pageTitle: t('subtitleMode'), searchbar: false, scrollToSelectedItem: true }}
          >
            <select name={`subtitle-mode-${index}`} value={vm.choice.subtitleMode} onchange={setSubtitleMode}>
              <option value="off">{t('subtitleOff')}</option>
              <option value="download">{t('subtitleDownload')}</option>
              <option value="embed">{t('subtitleEmbed')}</option>
            </select>
          </ListItem>
        {/if}
      {/if}

      <ListItem title={t('downloadRange')} link={`/item/${index}/time-range/`}>
        {#snippet after()}
          <span class="format-choice-summary">{vm.timeRangeSummary}</span>
        {/snippet}
      </ListItem>

      {#if vm.item.chapters?.length}
        <ListItem title={t('chapterRange')} link={`/item/${index}/chapter-range/`}>
          {#snippet after()}
            <span class="format-choice-summary">{vm.chapterRangeSummary}</span>
          {/snippet}
        </ListItem>
      {/if}

      {#if vm.item.chapterCount > 0}
        {#if chapterModeSupported}
          <ListItem title={t('chapterMode')} smartSelect smartSelectParams={{ openIn: 'page', pageTitle: t('chapterMode'), searchbar: false, scrollToSelectedItem: true }}>
            <select name={`chapter-mode-${index}`} value={vm.choice.chapterMode} onchange={setChapterMode}>
              <option value="off">{t('chapterOff')}</option>
              <option value="download">{t('chapterDownload')}</option>
              <option value="embed">{t('chapterEmbed')}</option>
            </select>
          </ListItem>
        {:else}
          <ListItem title={t('chapters')}>
            {#snippet after()}
              <Toggle checked={vm.choice.chapters === 'split'} onChange={setChapters} />
            {/snippet}
          </ListItem>
        {/if}
      {/if}

    </List>
  {:else}
    <List strong dividers outline inset>
      <ListItem title={t('noFormats')} />
    </List>
  {/if}
</Page>

<style>
  :global(.file-name-input) {
    --file-name-clear-action-width: 44px;
  }

  :global(.file-name-input .item-input-wrap input) {
    padding-inline-end: var(--file-name-clear-action-width);
  }

  :global(.file-name-input .input-clear-button) {
    width: var(--file-name-clear-action-width);
  }

  .detail-thumbnail {
    display: block;
    width: 96px;
    height: 54px;
    border-radius: 6px;
    object-fit: cover;
  }

  .format-choice-summary {
    display: block;
    max-width: 240px;
    overflow: hidden;
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
