<script>
  import { Block, Button, Navbar, NavRight, NavTitle, Page, Preloader, Segmented, Toolbar } from 'framework7-svelte';
  import { popupController as controller } from '../popup-runtime.js';
  import { analysisCountText, analysisStatusText, pageSnapshotSummary } from '../popup-view-model.js';
  import { popupResultItems, popupDownloadPresentation } from '../popup-result-list.js';
  import { galleryPostGroups } from '../../shared/gallery-post-groups.js';
  import { t } from '../../shared/i18n.js';
  import SourcePreview from '../components/SourcePreview.svelte';
  import ResultCard from '../components/ResultCard.svelte';
  import SourceCapabilities from '../components/SourceCapabilities.svelte';
  $: state = $controller;
  $: items = popupResultItems(state);
  $: download = popupDownloadPresentation(state);
  $: snapshot = state.sourcePageSnapshot;
  $: groups = galleryPostGroups(state.analysisByBackend.g?.items || [], state.sourceUrl);
</script>

<Page name="home" class="home-page">
  <Navbar class="home-navbar">
    <NavTitle><span class="site-identity">
      {#if state.sourceSiteIconUrl}<img class="site-icon" src={state.sourceSiteIconUrl} alt="" />{/if}
      <span class="site-name">{state.sourceSiteName || 'yt-dlp-gui'}</span>
    </span></NavTitle>
    <NavRight>
      <Button small href="/settings/" disabled={state.busy || state.analysisRunning || state.downloadRunning}>{t('settings')}</Button>
      {#if state.analysisRunning}
        <Button small fill disabled={state.busy} onClick={() => controller.stopAnalysis()}><Preloader /> {t('stop')}</Button>
      {:else}
        <Button small fill disabled={!state.preferencesLoaded || state.toolDetectionRunning || state.toolPreferenceSaving || (!state.analysisTools.y && !state.analysisTools.g) || !state.recognized || state.recognitionPending || state.busy || state.downloadRunning} onClick={() => controller.analyze()}>{t('analyze')}</Button>
      {/if}
    </NavRight>
  </Navbar>
  {#if state.preferencesLoaded && !state.analysisTools.y && !state.analysisTools.g}<Block inset>{t('selectAnalysisTool')}</Block>{/if}
  {#each ['y', 'g'] as backend}
    {#if state.analysisErrors[backend]}
      <Block strong inset>
        <div class="analysis-error-actions">
          <span>{backend === 'y' ? 'yt-dlp' : 'gallery-dl'}</span>
          <Button small outline disabled={state.busy || state.analysisRunning || state.downloadRunning || state.recognitionPending}
            onClick={() => controller.retryAnalysis(backend)}>{t('retryAnalysis')}</Button>
        </div>
        <div>{backend === 'g' && state.analysisByBackend.g?.items?.length
          ? t('galleryAnalysisWarning', { message: state.analysisErrors[backend] })
          : state.analysisErrors[backend]}</div>
      </Block>
    {/if}
  {/each}
  {#if !state.recognitionPending && !state.recognized}
    <Block inset role="alert">{state.recognitionText}</Block>
  {/if}
  {#if !items.length && (state.recognized || snapshot?.title || snapshot?.thumbnailUrl || snapshot?.thumbnailCandidates?.length)}
    <article class="source-card">
      <SourcePreview sources={snapshot?.thumbnailCandidates?.length ? snapshot.thumbnailCandidates : [snapshot?.thumbnailUrl]} siteIcon={state.sourceSiteIconUrl} />
      <div class="source-card-content">
        <div class="source-card-heading">
          <div class="source-card-title">{snapshot?.title || state.sourceSiteName}</div>
          <SourceCapabilities recognized={state.recognized} detection={state.toolDetection} />
        </div>
        <span class="result-date">{pageSnapshotSummary(state)}</span>
      </div>
    </article>
  {/if}
  {#if state.playlistSelectable}
    <Block inset><Segmented strong>
      <Button active={state.playlistScope === 'current'} onClick={() => controller.setPlaylistScope('current')}>{t('currentVideo')}</Button>
      <Button active={state.playlistScope === 'playlist'} onClick={() => controller.setPlaylistScope('playlist')}>{t('fullPlaylist')}</Button>
    </Segmented></Block>
  {/if}
  {#if groups.length > 1}<Block inset><Button outline disabled={state.busy || state.analysisRunning} onClick={() => controller.openGalleryAlbumAll()}>{t('openAllPostsInNewTab')}</Button></Block>{/if}
  {#if items.length || state.analysisRunning}
  <div class="result-card-stack" aria-busy={state.analysisRunning}>
    {#each items as result (result.key)}<ResultCard {result} {state} count={items.length} />{/each}
    {#if state.analysisRunning}
      <div class="analysis-status" role="status" aria-live="polite"><Preloader /><span>{analysisStatusText(state)}{#if analysisCountText(state)} · {analysisCountText(state)}{/if}</span></div>
    {/if}
  </div>
  {/if}
  {#if state.messageText && (state.messageError || !['running', 'stopping', 'completed'].includes(state.currentTask?.state))}
    <Block strong inset role="status">{state.messageText}
      {#if state.helpUrl}<Button href={state.helpUrl} target="_blank" external>{t('nativeHostGitHub')}</Button>{/if}
    </Block>
  {/if}
  <Toolbar bottom class="download-toolbar-shell">
    <div class="download-toolbar">
      <Button small fill disabled={state.busy || state.analysisRunning || state.downloadRunning || download.browserOnly} onClick={() => controller.chooseDirectory()}>{t('chooseFolder')}</Button>
      <button class="download-location result-field" disabled={state.busy || state.analysisRunning || state.downloadRunning || download.browserOnly || !state.outputDirectory} aria-label={t('openDownloadFolder')} onclick={() => controller.openDirectory()}>{download.browserOnly ? t('browserDownloadDestination') : state.outputDirectory || t('downloadLocation')}</button>
      {#if state.downloadRunning}
        <Button small fill disabled={state.busy || state.currentTask?.state === 'stopping'} onClick={() => controller.stopDownload()}>{state.currentTask?.state === 'stopping' ? t('stopping') : t('stop')}</Button>
      {:else}<Button small fill class="primary-action" disabled={download.disabled} onClick={() => controller.downloadSelected()}>{t('download')}</Button>{/if}
    </div>
  </Toolbar>
</Page>
