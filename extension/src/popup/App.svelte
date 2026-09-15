<script>
  import { onMount } from 'svelte';
  import { App, View } from 'framework7-svelte';
  import { popupController } from './popup-runtime.js';
  import { routes } from './routes.js';

  const f7params = {
    name: 'yt-dlp-gui',
    id: 'me.kannagi.ytdlpgui.extension',
    theme: 'auto',
    routes,
    smartSelect: {
      openIn: 'page',
      closeOnSelect: true,
      scrollToSelectedItem: true,
      searchbar: false,
    },
  };

  onMount(() => {
    popupController.init();
    return () => popupController.destroy();
  });
</script>

<App {...f7params}>
  <View main url="/" />
</App>
