<script>
  import { onMount } from 'svelte';
  import { App, View, f7ready } from 'framework7-svelte';
  import { savePopupNavigation } from './popup-navigation.js';
  import { popupController } from './popup-runtime.js';
  import { routes } from './routes.js';
  import { extensionTouchPolicy } from '../shared/framework7-interaction-policy.js';

  export let navigation = { key: null, route: '/' };
  const themePreference = matchMedia('(prefers-color-scheme: dark)');
  let dark = themePreference.matches;

  const f7params = {
    name: 'yt-dlp-gui',
    id: 'me.kannagi.ytdlpgui.extension',
    theme: 'md',
    touch: extensionTouchPolicy,
    routes,
    smartSelect: {
      openIn: 'page',
      closeOnSelect: true,
      scrollToSelectedItem: true,
      searchbar: false,
    },
  };

  onMount(() => {
    const updateTheme = event => { dark = event.matches; };
    themePreference.addEventListener('change', updateTheme);
    let disposed = false;
    let router = null;
    let pageResetRevision = 0;
    const unsubscribe = popupController.subscribe(state => {
      if (state.pageResetRevision === pageResetRevision) return;
      pageResetRevision = state.pageResetRevision;
      navigation.route = '/';
      if (router && router.currentRoute?.url !== '/') router.back('/', { force: true, animate: false });
    });
    const rememberRoute = route => {
      if (!disposed) savePopupNavigation(chrome.storage.session, navigation, route?.url).catch(() => {});
    };

    f7ready(app => {
      if (disposed) return;
      router = app.views.main?.router;
      if (!router) return;
      // The View starts at / so Framework7 has a home page in its back history.
      // Restore the last page without animation, then track subsequent navigation.
      if (navigation.route !== '/') router.navigate(navigation.route, { animate: false });
      router.on('routeChange', rememberRoute);
    });

    popupController.init();
    return () => {
      themePreference.removeEventListener('change', updateTheme);
      disposed = true;
      unsubscribe();
      router?.off('routeChange', rememberRoute);
      popupController.destroy();
    };
  });
</script>

<App {...f7params} {dark}>
  <View main url="/" class="popup-view" />
</App>
