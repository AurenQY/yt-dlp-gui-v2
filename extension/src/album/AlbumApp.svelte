<script>
import { t } from '../shared/i18n.js';
  import { extensionTouchPolicy } from '../shared/framework7-interaction-policy.js';
  import { onMount, tick } from 'svelte';
  import { App, View, Page, Navbar, NavRight, NavTitle, Button, Preloader } from 'framework7-svelte';
  import { loadAlbum, isMediaUrl, isVideo, safeFileName } from './album-data.js';
  import { openAlbumViewer, closeAlbumViewer } from './album-viewer.js';
  import { justifiedAlbumRows, imageAspectRatio } from './album-justified-layout.js';
  import { createAlbumStream } from './album-post-stream.js';
  import { downloadAlbumSelection } from './album-download.js';
  import { consumeAlbumAutoDownload } from './album-auto-download.js';
  import { createGalleryImageSession } from '../image/gallery-image-session.js';
  import AlbumImage from './AlbumImage.svelte';
  import AlbumVideo from './AlbumVideo.svelte';
  import { observeAlbumWheelMode } from '../shared/album-preferences.js';
  import { appendTimelinePage, timelineReadRequest } from '../shared/gallery-timeline-session.js';
  import { albumBrowseSession, observeAlbumContinuation } from './album-timeline-browse.js';
  import { albumReadTarget, albumContentHeight, forecastAlbumPostCount } from './album-page-forecast.js';
  import { readAlbumTimeline } from './album-timeline-client.js';
  const session = createGalleryImageSession();
  const themePreference = matchMedia('(prefers-color-scheme: dark)');
  let dark = themePreference.matches;
  let wheelMode = 'zoom';
  let downloadAborter;

  let gallery = null;
  let media = [];
  let posts = [];
  let streamTiles = [];
  let postByMediaIndex = [];
  let activePost = null;
  let postDialog;
  let chosen = new Set();
  let status = '';
  let busy = false;
  let gridElement;
  let albumRows = [];
  let measuredRatios = new Map();
  let albumKey;
  let reading = false;
  let readError = '';
  let readAborter;
  let alive = true;

  function applyGallery(value) {
    gallery = value;
    const stream = createAlbumStream(value);
    media = stream.media; posts = stream.posts; streamTiles = stream.tiles;
    postByMediaIndex = stream.postByMediaIndex;
    updateLayout();
  }

  async function continueReading() {
    if (!alive || reading || busy || !gallery?.timeline?.hasMore) return;
    reading = true; readError = '';
    readAborter = new AbortController();
    const loadedHeight = albumContentHeight(streamTiles, gridElement?.clientWidth, measuredRatios);
    const targetHeight = albumReadTarget(gridElement, loadedHeight);
    async function publishPage(page, final = false) {
      if (!alive || readAborter.signal.aborted) return;
      const next = appendTimelinePage(gallery, page, { allowSameCursor: final });
      applyGallery(next);
      await tick();
      if (alive && !readAborter.signal.aborted) await chrome.storage.session.set({ [albumKey]: next });
    }
    try {
      do {
        const request = { ...timelineReadRequest(gallery), postCount: forecastAlbumPostCount({ tiles: streamTiles,
          width: gridElement?.clientWidth, targetHeight, measuredRatios }) };
        const page = await readAlbumTimeline(request, readAborter.signal, page => publishPage(page));
        await publishPage(page, true);
      } while (alive && !document.hidden && gallery.timeline.hasMore &&
        albumContentHeight(streamTiles, gridElement?.clientWidth, measuredRatios) < targetHeight);
    } catch (error) { if (alive) readError = String(error?.message || error); }
    finally { if (alive) reading = false; }
  }

  function updateLayout() {
    if (!gridElement) return;
    albumRows = justifiedAlbumRows(streamTiles, gridElement.clientWidth, measuredRatios);
  }

  function onThumbnailLoad(index, event) {
    // The first layout uses gallery-dl metadata. When it is missing or the
    // thumbnail differs, correct the ratio once the browser knows its dimensions.
    const image = event.currentTarget;
    const width = image.naturalWidth || image.videoWidth;
    const height = image.naturalHeight || image.videoHeight;
    if (!width || !height) return;
    const ratio = imageAspectRatio(null, width / height);
    const current = measuredRatios.get(index);
    if (current != null && Math.abs(current - ratio) < 0.01) return;
    const entry = streamTiles[index];
    if (entry?.kind !== 'media') return;
    const item = media[entry.mediaIndex];
    if (item?.width > 0 && item?.height > 0) return; // Trust original image metadata over cropped preview.
    measuredRatios = new Map(measuredRatios).set(index, ratio);
    updateLayout();
  }

  onMount(() => {
    const releasePreferences = observeAlbumWheelMode(chrome.storage,
      value => { wheelMode = value; }, error => { status = t('albumPreferencesFailed', { message: String(error?.message || error) }); });
    const updateTheme = event => { dark = event.matches; };
    themePreference.addEventListener('change', updateTheme);
    const observer = new ResizeObserver(updateLayout);
    if (gridElement) observer.observe(gridElement);
    let mounted = true;
    loadAlbum(chrome.storage.session, location.href)
      .then(async result => {
        if (!mounted) return;
        albumKey = result.key;
        applyGallery(albumBrowseSession(result.gallery));
        const mode = await consumeAlbumAutoDownload(chrome.storage.session, location.href);
        if (!mounted || !mode) return;
        chosen = new Set(media.map((_, index) => index));
        busy = true;
        downloadAborter = new AbortController();
        status = t('albumDownloadAuthorized');
        try {
          await downloadAlbumSelection({
            gallery: result.gallery,
            media,
            selected: chosen,
            asZip: mode === 'zip',
            permissionsAlreadyGranted: true,
            session,
            signal: downloadAborter.signal,
            progress: value => { if (mounted) status = value; },
          });
        } catch (error) {
          if (mounted) status = error?.name === 'AbortError' ? t('downloadStopped') : t('browserDownloadFailed', { message: String(error?.message || error) });
        } finally { if (mounted) busy = false; }
      })
      .catch(error => { if (mounted) status = String(error?.message || error); });
    return () => {
      mounted = false;
      alive = false;
      releasePreferences();
      observer.disconnect();
      themePreference.removeEventListener('change', updateTheme);
      downloadAborter?.abort();
      readAborter?.abort(new DOMException('Album closed', 'AbortError'));
      closeAlbumViewer();
      session.dispose();
    };
  });

  function toggle(index) {
    const updated = new Set(chosen);
    if (updated.has(index)) updated.delete(index);
    else updated.add(index);
    chosen = updated;
  }

  function selectAll() { chosen = new Set(media.map((_, index) => index)); }
  function clearSelection() { chosen = new Set(); }

  async function open(index) {
    try {
      openAlbumViewer(media, index, gallery, postByMediaIndex, { session, wheelMode: () => wheelMode, report: value => { status = value; } });
      status = '';
    } catch (error) {
      status = t('albumViewerFailed', { message: String(error?.message || error) });
    }
  }

  function showPost(post) {
    activePost = post;
    postDialog?.showModal();
  }

  async function download(asZip) {
    if (busy || reading || !chosen.size) return;
    busy = true;
    downloadAborter = new AbortController();
    status = t('downloadPreparing');
    try {
      await downloadAlbumSelection({ gallery, media, selected: chosen, asZip, session, signal: downloadAborter.signal, progress: value => { status = value; } });
    } catch (error) {
      status = error?.name === 'AbortError' ? t('downloadStopped') : String(error?.message || error || t('downloadFailed'));
    } finally { busy = false; }
  }
</script>

<App name="yt-dlp-gui Album" id="me.kannagi.ytdlpgui.album" theme="md" touch={extensionTouchPolicy} {dark}>
  <View main>
    <Page name="gallery-album" class="album-page">
      <Navbar>
        <NavTitle>
          <span class="album-navbar-title">{gallery?.siteName || 'Gallery'} · {t('album')}</span>
        </NavTitle>
        <NavRight>
          <div class="album-navbar-actions" aria-label={t('albumActions')}>
            <span class="album-selected-count" aria-live="polite">{t('albumSelected', { count: chosen.size, total: media.length })}</span>
            <Button small outline disabled={busy || reading || !media.length || chosen.size === media.length}
              onClick={selectAll}>{t('selectAll')}</Button>
            <Button small outline disabled={busy || reading || chosen.size === 0} onClick={clearSelection}>{t('clearSelection')}</Button>
            <Button small fill class="primary-action" disabled={busy || reading || chosen.size === 0} onClick={() => download(false)}>{t('download')}</Button>
            <Button small outline disabled={busy || reading || chosen.size === 0} onClick={() => download(true)}>ZIP</Button>
            {#if busy}<Button small fill onClick={() => downloadAborter?.abort()}>{t('stop')}</Button>{/if}
          </div>
        </NavRight>
      </Navbar>

      <section class="album-content" aria-label={t('albumMedia')}>
        {#if status}
          <div class="album-status" role="status" aria-live="polite">{status}</div>
        {/if}

        <div class="album-grid" bind:this={gridElement} aria-label={t('albumGrid')}>
          {#each albumRows as row (row.id)}
            <div class={row.justified ? 'album-row justified' : 'album-row'} style={`height: ${row.height}px`}>
              {#each row.items as tile (tile.index)}
                {@const entry = streamTiles[tile.index]}
                {#if entry.kind === 'post'}
                  {@const post = posts[entry.postIndex]}
                  <article class="album-post-card" style={row.justified ? `flex: ${tile.ratio} 1 0%;` : `flex: 0 0 ${tile.width}px;`}>
                    <button type="button" class="album-post-card-open" onclick={() => showPost(post)}>
                      <span class="album-post-card-date-line">
                        {#if post.dateOnly}<time class="album-post-card-date" datetime={post.dateOnly}>{post.dateOnly}</time>{/if}
                        {#if post.timeOnly}<span class="album-post-card-time">{post.timeOnly}</span>{/if}
                      </span>
                      <span class="album-post-card-heading">{post.creator || post.title || post.siteName || t('post')}</span>
                      <span class="album-post-card-content">{post.description || post.title || t('postViewInfo')}</span>
                      <span class="album-post-card-meta">{t('galleryMediaCount', { count: post.count })}</span>
                    </button>
                  </article>
                {:else if entry.kind === 'continue'}
                  <article class="album-post-card" use:observeAlbumContinuation={{ load: continueReading,
                    enabled: alive && gallery.timeline.hasMore && !reading && !busy && !readError }}
                    style={row.justified ? `flex: ${tile.ratio} 1 0%;` : `flex: 0 0 ${tile.width}px;`}>
                    <button type="button" class="album-post-card-open album-continuation" disabled={reading || busy || !gallery.timeline.hasMore} onclick={continueReading}>
                      {#if reading}<Preloader size={32} />{/if}
                      <span class="album-post-card-heading">{reading ? t('albumReading') : !gallery.timeline.hasMore ? t('albumReadDone') : readError ? t('albumReadRetry') : t('albumReadMore')}</span>
                      <span class="album-post-card-content" role="status">{readError || (gallery.timeline.hasMore ? t('albumReadHint') : t('albumNoMore'))}</span>
                    </button>
                  </article>
                {:else}
                  {@const index = entry.mediaIndex}
                  {@const item = media[index]}
                  <article class={chosen.has(index) ? 'album-tile selected' : 'album-tile'}
                    style={row.justified ? `flex: ${tile.ratio} 1 0%;` : `flex: 0 0 ${tile.width}px;`}>
                    <div class="album-image-frame">
                      <button type="button" class="album-preview" aria-label={t('albumViewItem', { number: index + 1 })} onclick={() => open(index)}>
                        {#if isVideo(item)}
                          <AlbumVideo {item} on:load={event => onThumbnailLoad(tile.index, event.detail)} />
                        {:else}
                          <AlbumImage {item} {session} {index} on:load={event => onThumbnailLoad(tile.index, event.detail)} />
                        {/if}
                      </button>
                      <label class="album-select-toggle">
                        <input type="checkbox" checked={chosen.has(index)} disabled={busy || (reading && gallery.timeline?.postStart === 1)} aria-label={t('albumSelectItem', { number: index + 1 })} onchange={() => toggle(index)} />
                      </label>
                    </div>
                    <div class="album-tile-footer">
                      <span title={safeFileName(item, index)}>{safeFileName(item, index)}</span>
                    </div>
                  </article>
                {/if}
              {/each}
            </div>
          {/each}
        </div>
      </section>
      <dialog class="album-post-dialog" bind:this={postDialog} onclose={() => activePost = null}>
        {#if activePost}
          <header class="album-post-dialog-header">
            <strong>{activePost.creator || activePost.siteName || t('postInfo')}</strong>
            <button type="button" onclick={() => postDialog?.close()} aria-label={t('postCloseInfo')}>✕</button>
          </header>
          <p class="album-post-dialog-meta">{[activePost.date, t('galleryMediaCount', { count: activePost.count })].filter(Boolean).join(' · ')}</p>
          <p class="album-post-dialog-text">{activePost.description || activePost.title || t('postEmpty')}</p>
          {#if isMediaUrl(activePost.sourcePageUrl)}
            <a href={activePost.sourcePageUrl} target="_blank" rel="noopener noreferrer">{t('postOpenOriginal')} ↗</a>
          {/if}
        {/if}
      </dialog>
    </Page>
  </View>
</App>
