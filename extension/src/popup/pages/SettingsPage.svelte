<script>
  import {
    Block,
    Button,
    List,
    ListInput,
    ListItem,
    Navbar,
    Page,
    Preloader,
  } from 'framework7-svelte';
  import { popupController as controller } from '../popup-runtime.js';
  import { t } from '../../shared/i18n.js';
  import { exportOperationDiagnostics } from '../export-operation-diagnostics.js';

  let diagnosticMessage = '';
  async function exportDiagnostics() {
    try { await exportOperationDiagnostics(); diagnosticMessage = 'Diagnostic export requested.'; }
    catch { diagnosticMessage = 'Could not export diagnostics.'; }
  }

  const extensionVersion = chrome.runtime.getManifest().version;
  $: state = $controller;
  $: toolsLocked = !state.preferencesLoaded || state.busy || state.analysisRunning || state.downloadRunning || state.toolDetectionRunning || state.toolPreferenceSaving;
</script>

<Page name="settings">
  <Navbar title={t('settings')} backLink={true} />

  <Block inset class="tool-settings">
    <div class="tool-settings-heading">{t('analysisTools')}</div>
    {#each ['y', 'g'] as tool}
      <label class="tool-settings-row">
        <input type="checkbox" checked={state.analysisTools[tool]} disabled={toolsLocked}
          onchange={event => controller.setAnalysisTool(tool, event.currentTarget.checked)} />
        <span>{tool === 'y' ? 'yt-dlp' : 'gallery-dl'}</span>
        <span class="tool-availability">{t(state.toolDetection?.[tool] === true ? 'toolAvailable' : state.toolDetection?.[tool] === false ? 'toolUnavailable' : 'toolNotDetected')}</span>
      </label>
    {/each}
    <p class="tool-settings-hint">{t('analysisToolsHint')}</p>
    <Button small fill disabled={toolsLocked} onClick={() => controller.detectAnalysisTools()}>
      {#if state.toolDetectionRunning}<Preloader />{/if}{t('detectTools')}
    </Button>
    {#if state.toolDetection?.checkedAt}<p class="tool-settings-hint">{t('lastToolDetection', { date: new Date(state.toolDetection.checkedAt).toLocaleString() })}</p>{/if}
    {#if state.toolSettingsMessage}<p role="status" class:error-message={state.toolSettingsError}>{state.toolSettingsMessage}</p>{/if}
  </Block>

  <Block inset class="tool-settings">
    <div class="tool-settings-heading">yt-dlp</div>
    <List strong dividers outline class="option-list">
      <ListInput label={t('ytDlpFetchLimit')} type="number" min="1" max="100" step="1"
        disabled={toolsLocked} value={String(state.playlistItemLimit)}
        info={t('ytDlpFetchLimitHint')}
        onChange={event => controller.setPlaylistItemLimit(event.currentTarget.value)} />
    </List>
  </Block>

  <Block inset class="tool-settings">
    <div class="tool-settings-heading">{t('albumSettings')}</div>
    <div>{t('albumWheelOperation')}</div>
    {#each ['zoom', 'navigate'] as mode}
      <label class="tool-settings-row">
        <input type="radio" name="album-wheel" value={mode} checked={state.albumWheelMode === mode}
          disabled={!state.preferencesLoaded || state.albumPreferenceSaving}
          onchange={() => controller.setAlbumWheelMode(mode)} />
        <span>{t(mode === 'zoom' ? 'albumWheelZoom' : 'albumWheelNavigate')}</span>
      </label>
    {/each}
    <p class="tool-settings-hint">{t('albumWheelHint')}</p>
    {#if state.albumSettingsMessage}<p role="status" class="error-message">{state.albumSettingsMessage}</p>{/if}
  </Block>

  <List strong dividers outline inset>
    <ListItem
      title={t('saveSiteCookies')}
      footer={t('saveSiteCookiesHint')}
    >
      {#snippet after()}
        {#if state.cookieSaveRunning}
          <Preloader />
        {:else}
          <Button
            small
            fill
            disabled={state.busy || state.analysisRunning || state.downloadRunning}
            onClick={() => controller.saveSiteCookies()}
          >{t('save')}</Button>
        {/if}
      {/snippet}
    </ListItem>
  </List>

  <Block inset class="extension-version">v{extensionVersion} · F7 + Spotlight album</Block>

  <Block inset class="tool-settings">
    <div class="tool-settings-heading">Diagnostics</div>
    <Button small fill onClick={exportDiagnostics}>Export diagnostics</Button>
    {#if diagnosticMessage}<p role="status">{diagnosticMessage}</p>{/if}
  </Block>

  {#if state.cookieSaveMessage}
    <Block strong inset>
      <span class:error-message={state.cookieSaveError}>{state.cookieSaveMessage}</span>
    </Block>
  {/if}
</Page>
