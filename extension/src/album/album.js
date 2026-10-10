import { mount } from 'svelte';
import Framework7 from 'framework7/lite-bundle';
import Framework7Svelte from 'framework7-svelte';
import 'framework7/css/bundle';
import '../popup/egui-shadow-theme.css';
import 'spotlight.js/dist/css/spotlight.min.css';
import AlbumApp from './AlbumApp.svelte';

Framework7.use(Framework7Svelte);
mount(AlbumApp, { target: document.getElementById('app') });
