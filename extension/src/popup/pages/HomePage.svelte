<script>
  import {
    Block,
    Button,
    Link,
    List,
    ListInput,
    ListItem,
    Navbar,
    NavRight,
    NavTitle,
    Page,
    Preloader,
    Segmented,
    Toolbar,
  } from 'framework7-svelte';
  import { popupController as controller } from '../popup-runtime.js';
  import {
    analysisCountText,
    analysisStatusText,
    itemTaskState,
    pageSnapshotSummary,
    selectedFormatDetails,
  } from '../popup-view-model.js';
  import { durationLabel } from '../../shared/format-display.js';
  import { t } from '../../shared/i18n.js';

  $: state = $controller;
  $: items = state.analysis?.items || [];
  $: selectedCount = controller.selectedDownloadIndices(state).length;
  $: snapshot = state.sourcePageSnapshot;
  $: snapshotDuration = durationLabel(snapshot?.mediaDurationSeconds);
  $: snapshotDetails = pageSnapshotSummary(state);
  $: overallPercent = Math.max(0, Math.min(100, Number(state.currentTask?.percent) || 0));
  $: downloadDisabled = !items.length
    || (items.length > 1 && selectedCount === 0)
    || state.busy
    || state.downloadRunning;
  $: sourceErrorVisible = !state.recognitionPending && !state.recognized;

  function toggleItem(index, event) {
    controller.toggleSelection(index, event.currentTarget.checked);
  }

  function openItem(index) {
    controller.prepareDetail(index);
  }


</script>

<Page name="home">
  <Navbar class="home-navbar">
    <NavTitle>
      <span class="site-identity">
        {#if state.sourceSiteIconUrl}
          <img
            class="site-icon"
            src={state.sourceSiteIconUrl}
            alt=""
            onerror={(event) => { event.currentTarget.hidden = true; }}
          />
        {/if}
        <span class="site-name">{state.sourceSiteName || 'yt-dlp-gui'}</span>
      </span>
    </NavTitle>
    <NavRight>
      {#if state.analysisRunning}
        <Button
          small
          fill
          color="red"
          disabled={state.busy}
          onClick={() => controller.stopAnalysis()}
        >{t('stop')}</Button>
      {:else}
        <Button
          small
          fill
          disabled={!state.recognized || state.busy || state.downloadRunning}
          onClick={() => controller.analyze()}
        >{t('analyze')}</Button>
      {/if}
    </NavRight>
  </Navbar>

  {#if state.analysisRunning && !items.length}
    <List strong dividers outline inset>
      <ListItem
        title={analysisStatusText(state)}
        footer={analysisCountText(state) || t('analyzingPage')}
      >
        {#snippet media()}<Preloader />{/snippet}
      </ListItem>
    </List>
  {:else if !items.length}
    <List strong outline inset class="source-preview">
      {#if snapshot?.title || snapshot?.thumbnailUrl}
        <ListItem
          mediaItem={Boolean(snapshot?.thumbnailUrl || snapshotDuration)}
          title={snapshot.title || state.sourceSiteName}
          footer={snapshotDetails}
        >
          {#snippet media()}
            {#if snapshot?.thumbnailUrl || snapshotDuration}
              <div class="result-media" class:without-thumbnail={!snapshot?.thumbnailUrl}>
                {#if snapshot?.thumbnailUrl}
                  <img src={snapshot.thumbnailUrl} width="120" height="68" alt="" />
                {/if}
                {#if snapshotDuration}
                  <span class="result-duration">{snapshotDuration}</span>
                {/if}
              </div>
            {/if}
          {/snippet}
        </ListItem>
      {:else if sourceErrorVisible}
        <ListItem
          title={t('unavailable')}
          footer={state.recognitionText}
        />
      {/if}
    </List>
  {/if}

  {#if state.playlistSelectable}
    <Block strong inset>
      <Segmented strong>
        <Button
          active={state.playlistScope === 'current'}
          onClick={() => controller.setPlaylistScope('current')}
        >{t('currentVideo')}</Button>
        <Button
          active={state.playlistScope === 'playlist'}
          onClick={() => controller.setPlaylistScope('playlist')}
        >{t('fullPlaylist')}</Button>
      </Segmented>
    </Block>
  {/if}

  {#if state.playlistScope === 'playlist'}
    <List strong dividers outline inset>
      <ListInput
        label={t('playlistLimitCompact')}
        type="number"
        min="1"
        max="100"
        value={String(state.playlistItemLimit)}
        info={t('itemsUnit')}
        onChange={(event) => controller.setPlaylistItemLimit(event.currentTarget.value)}
      />
    </List>
  {/if}

  {#if items.length}
    <List mediaList strong dividers outline inset class="analysis-results">
      {#each items as item, index}
        {@const saved = state.choices[index] || {}}
        {@const task = itemTaskState(state, index)}
        {@const details = selectedFormatDetails(item, saved, controller.storedDownloadChoice)}
        {@const duration = durationLabel(item.duration)}

        {#if items.length > 1}
          <ListItem
            mediaItem
            checkbox
            noChevron
            checked={state.selectedIndices.has(index)}
            subtitle={details.stream}
            onChange={(event) => toggleItem(index, event)}
          >
            {#snippet title()}
              <span class="result-title">{item.title || item.url}</span>
            {/snippet}
            {#snippet text()}
              <div>{details.container}</div>
              {#if task && task.state !== 'completed'}
                <div class="result-task-label">{task.label}</div>
                <div class="result-progress" aria-label={task.label}>
                  <span style={`width: ${task.percent}%`}></span>
                </div>
              {/if}
            {/snippet}
            {#snippet media()}
              {#if item.thumbnail || duration}
                <div class="result-media" class:without-thumbnail={!item.thumbnail}>
                  {#if item.thumbnail}
                    <img src={item.thumbnail} width="120" height="68" alt="" />
                  {/if}
                  {#if task?.state === 'completed'}
                    <span class="result-complete" aria-label={t('completed')}>✓</span>
                  {/if}
                  {#if duration}
                    <span class="result-duration">{duration}</span>
                  {/if}
                </div>
              {/if}
            {/snippet}
            {#snippet after()}
              <Link
                href={`/item/${index}/`}
                text={t('downloadContent')}
                onClick={(event) => {
                  event.stopPropagation();
                  openItem(index);
                }}
              />
            {/snippet}
          </ListItem>
        {:else}
          <ListItem
            mediaItem
            chevronCenter
            link={`/item/${index}/`}
            subtitle={details.stream}
            onClick={() => openItem(index)}
          >
            {#snippet title()}
              <span class="result-title">{item.title || item.url}</span>
            {/snippet}
            {#snippet text()}
              <div>{details.container}</div>
              {#if task && task.state !== 'completed'}
                <div class="result-task-label">{task.label}</div>
                <div class="result-progress" aria-label={task.label}>
                  <span style={`width: ${task.percent}%`}></span>
                </div>
              {/if}
            {/snippet}
            {#snippet media()}
              {#if item.thumbnail || duration}
                <div class="result-media" class:without-thumbnail={!item.thumbnail}>
                  {#if item.thumbnail}
                    <img src={item.thumbnail} width="120" height="68" alt="" />
                  {/if}
                  {#if task?.state === 'completed'}
                    <span class="result-complete" aria-label={t('completed')}>✓</span>
                  {/if}
                  {#if duration}
                    <span class="result-duration">{duration}</span>
                  {/if}
                </div>
              {/if}
            {/snippet}
          </ListItem>
        {/if}
      {/each}
    </List>
  {/if}

  {#if state.messageText && (state.messageError || !['running', 'stopping', 'completed'].includes(state.currentTask?.state))}
    <Block strong inset>
      {state.messageText}
      {#if state.helpUrl}
        <Button href={state.helpUrl} target="_blank" external>{t('nativeHostGitHub')}</Button>
      {/if}
    </Block>
  {/if}

  <Toolbar bottom class="download-toolbar-shell">
    {#if state.downloadRunning}
      <div class="overall-progress" aria-label={`${overallPercent.toFixed(0)}%`}>
        <span style={`width: ${overallPercent}%`}></span>
      </div>
    {/if}
    <div class="download-toolbar">
      <Button
        small
        fill
        disabled={state.busy || state.analysisRunning || state.downloadRunning}
        onClick={() => controller.chooseDirectory()}
      >{t('chooseFolder')}</Button>
      <button
        type="button"
        class="download-location"
        title={state.outputDirectory ? t('openDownloadFolder') : t('downloadLocation')}
        aria-label={t('openDownloadFolder')}
        disabled={
          state.busy
          || state.analysisRunning
          || state.downloadRunning
          || !state.outputDirectory
        }
        onclick={() => controller.openDirectory()}
      >
        {state.outputDirectory || t('downloadLocation')}
      </button>
      {#if state.downloadRunning}
        <Button
          small
          fill
          color="red"
          disabled={state.busy || state.currentTask?.state === 'stopping'}
          onClick={() => controller.stopDownload()}
        >{state.currentTask?.state === 'stopping' ? t('stopping') : t('stop')}</Button>
      {:else}
        <Button
          small
          fill
          disabled={downloadDisabled}
          onClick={() => controller.downloadSelected()}
        >{t('download')}</Button>
      {/if}
    </div>
  </Toolbar>
</Page>

<style>
  .site-identity {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 8px;
  }

  .site-icon {
    width: 22px;
    height: 22px;
    flex: 0 0 auto;
    border-radius: 4px;
    object-fit: contain;
  }

  .site-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .result-title {
    display: -webkit-box;
    overflow: hidden;
    white-space: normal;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
  }

  :global(.home-navbar .navbar-inner),
  :global(.download-toolbar-shell .toolbar-inner) {
    padding-inline: var(--shadow-action-inset);
  }

  :global(.analysis-results .item-media),
  :global(.source-preview .item-media) {
    align-self: flex-start;
  }

  .result-media {
    position: relative;
    width: 120px;
    min-height: 68px;
    overflow: hidden;
    background: var(--egui-extreme);
    border-radius: 6px;
  }

  .result-media img {
    display: block;
    width: 120px;
    height: 68px;
    object-fit: cover;
  }

  .result-media.without-thumbnail {
    background: transparent;
  }

  .result-complete {
    position: absolute;
    top: 4px;
    right: 4px;
    display: grid;
    width: 18px;
    height: 18px;
    color: var(--egui-text);
    background: var(--egui-selection);
    border-radius: 50%;
    font-size: 12px;
    font-weight: 700;
    line-height: 1;
    place-items: center;
  }

  .result-duration {
    position: absolute;
    right: 4px;
    bottom: 4px;
    padding: 1px 4px;
    color: var(--egui-text);
    background: rgb(10 10 10 / 88%);
    border: 1px solid rgb(60 60 60 / 80%);
    border-radius: 3px;
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    line-height: 1.35;
  }

  .result-media.without-thumbnail .result-duration {
    position: static;
    display: inline-block;
  }

  :global(.analysis-results.media-list .item-inner) {
    display: grid;
    grid-template-areas:
      "title after"
      "subtitle after"
      "text after";
    grid-template-columns: minmax(0, 1fr) auto;
    align-content: center;
    column-gap: 10px;
  }

  :global(.analysis-results .item-title-row) {
    display: contents;
  }

  :global(.analysis-results .item-title) {
    grid-area: title;
    font-size: 15px;
    line-height: 1.35;
  }

  :global(.analysis-results .item-subtitle) {
    grid-area: subtitle;
    font-size: 12px;
    line-height: 1.3;
  }

  :global(.analysis-results .item-text) {
    grid-area: text;
    font-size: 11px;
    line-height: 1.3;
  }

  .result-task-label {
    overflow: hidden;
    margin-top: 2px;
    color: var(--egui-selection-text);
    font-variant-numeric: tabular-nums;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .result-progress,
  .overall-progress {
    overflow: hidden;
    background: var(--egui-widget);
  }

  .result-progress {
    height: 3px;
    margin-top: 3px;
    border-radius: 2px;
  }

  .result-progress > span,
  .overall-progress > span {
    display: block;
    height: 100%;
    background: var(--egui-selection);
    transition: width 160ms ease-out;
  }

  :global(.analysis-results .item-after) {
    grid-area: after;
    align-self: center;
  }

  .overall-progress {
    position: absolute;
    top: 0;
    right: 0;
    left: 0;
    height: 3px;
  }

  .download-toolbar {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    width: 100%;
  }

  .download-location {
    appearance: none;
    width: 100%;
    min-width: 0;
    overflow: hidden;
    padding: 0;
    border: 0;
    color: var(--f7-text-color);
    background: transparent;
    cursor: pointer;
    font: inherit;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .download-location:hover:not(:disabled),
  .download-location:focus-visible {
    color: var(--egui-selection-text);
  }

  .download-location:disabled {
    cursor: default;
    opacity: 0.5;
  }
</style>
