import { mount } from 'svelte';
import Framework7 from 'framework7/lite-bundle';
import Framework7Svelte from 'framework7-svelte';
import 'framework7/css/bundle';
import './egui-shadow-theme.css';
import './popup-controls.css';
import { installPopupHostSizing } from './popup-host.js';
import App from './App.svelte';
import { restorePopupNavigation } from './popup-navigation.js';

Framework7.use(Framework7Svelte);

const target = document.getElementById('app');
const disposePopupHostSizing = installPopupHostSizing(target);
const navigation = await restorePopupNavigation(chrome.tabs, chrome.storage.session,
  (sourceTabId, url) => chrome.runtime.sendMessage({ type: 'YT_DLP_GUI_ANALYSIS_SNAPSHOT',
    request: { sourceTabId, url, playlistScope: 'current' } }))
  .catch(() => ({ key: null, route: '/' }));
mount(App, { target, props: { navigation } });

window.addEventListener('unload', disposePopupHostSizing, { once: true });
