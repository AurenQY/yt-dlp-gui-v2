import { mount } from 'svelte';
import Framework7 from 'framework7/lite-bundle';
import Framework7Svelte from 'framework7-svelte';
import 'framework7/css/bundle';
import './egui-shadow-theme.css';
import { installPopupHostSizing } from './popup-host.js';
import App from './App.svelte';

Framework7.use(Framework7Svelte);

const target = document.getElementById('app');
const disposePopupHostSizing = installPopupHostSizing(target);
mount(App, { target });

window.addEventListener('unload', disposePopupHostSizing, { once: true });
