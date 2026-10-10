<script>
  import video from '../../../../assets/icons/PlayBoxOutline.svg?raw';
  import playlist from '../../../../assets/icons/PlaylistPlay.svg?raw';
  import gallery from '../../../../assets/icons/ImageMultipleOutline.svg?raw';
  import media from '../../../../assets/icons/Multimedia.svg?raw';
  import { sourceCapabilities } from '../../shared/source-capabilities.js';
  export let recognized;
  export let detection;
  const shapes = { video, playlist, gallery, media };
  $: icons = sourceCapabilities(recognized, detection);
</script>

<span class="source-capabilities">
  {#each icons as icon}
    <span class="source-capability capability-{icon.kind}" role="img" aria-label={icon.kind === 'gallery' ? 'gallery-dl' : 'yt-dlp'}>
      {@html shapes[icon.name].replace(/<\?xml[^?]*\?>\s*/, '')}
    </span>
  {/each}
</span>
