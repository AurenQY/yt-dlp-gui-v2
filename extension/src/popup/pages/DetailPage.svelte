<script>
  import {
    Block,
    Button,
    List,
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
  import SourcePreview from '../components/SourcePreview.svelte';
  import ActionIcon from '../components/ActionIcon.svelte';

  export let itemIndex;
  let fileNameInput;

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
    const fileName = event.currentTarget.value.replace(/[\r\n]+/g, ' ');
    if (fileName !== event.currentTarget.value) event.currentTarget.value = fileName;
    controller.setChoice(index, { fileName }, false);
  }

  function fileNameKeyDown(event) {
    if (event.key === 'Enter' && !event.isComposing) event.preventDefault();
  }

  function clearFileName() {
    controller.setChoice(index, { fileName: '' }, false);
    fileNameInput?.focus();
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
    <article class="source-card">
      <SourcePreview sources={[vm.item.thumbnail].filter(Boolean)} siteIcon={state.sourceSiteIconUrl} />
      <div class="source-card-content"><div class="source-card-title">{vm.item.title || vm.item.url}</div><span class="result-date">{durationLabel(vm.item.duration)}</span></div>
    </article>

    {#if vm.contentKinds.length > 1}
      <Block inset>
        <Segmented strong>
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

    <List strong inset class="option-list">
      <ListItem class="file-name-option" title={t('fileName')}>
        {#snippet after()}
          <div class="file-name-value">
            <div class="file-name-editor">
              <textarea bind:this={fileNameInput} id={`file-name-${index}`} aria-label={t('fileName')} aria-describedby={`file-name-hint-${index}`} rows="3" wrap="soft" maxlength="200" spellcheck="false" value={vm.choice.fileName} oninput={setFileName} onkeydown={fileNameKeyDown}></textarea>
              <button type="button" class="file-name-clear" aria-label={t('clearFileName')} aria-controls={`file-name-${index}`} disabled={!vm.choice.fileName} onclick={clearFileName}><ActionIcon name="close" /></button>
            </div>
            <span id={`file-name-hint-${index}`} class="file-name-hint">{t('fileNameBaseHint')}</span>
          </div>
        {/snippet}
      </ListItem>

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
