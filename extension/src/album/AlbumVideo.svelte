<script>
import { t } from '../shared/i18n.js';
  import { onMount, createEventDispatcher } from 'svelte';
  import { installAlbumVideoPreview, albumVideoPoster } from './album-video-preview.js';
  export let item;
  let video;
  let state = 'idle';
  const dispatch = createEventDispatcher();
  $: poster = albumVideoPoster(item);
  onMount(() => installAlbumVideoPreview(video.parentElement, video, item, value => { state = value; }));
</script>

<video class="album-video-preview" bind:this={video} {poster} muted loop playsinline preload="none" aria-label={t('albumMutedVideoPreview')} onloadedmetadata={event => dispatch('load', event)}></video>
{#if state !== 'playing'}
  <span class="album-video">▶ {String(item.extension).toUpperCase()}</span>
{/if}
