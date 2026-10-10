import HomePage from './pages/HomePage.svelte';
import DetailPage from './pages/DetailPage.svelte';
import GalleryDetailPage from './pages/GalleryDetailPage.svelte';
import GalleryFormatPage from './pages/GalleryFormatPage.svelte';
import GalleryRangePage from './pages/GalleryRangePage.svelte';
import FormatPage from './pages/FormatPage.svelte';
import TimeRangePage from './pages/TimeRangePage.svelte';
import ChapterRangePage from './pages/ChapterRangePage.svelte';
import SettingsPage from './pages/SettingsPage.svelte';

export const routes = [
  { path: '/', component: HomePage },
  { path: '/item/:itemIndex/', component: DetailPage },
  { path: '/gallery/:galleryIndex/', component: GalleryDetailPage },
  { path: '/gallery/:galleryIndex/format/', component: GalleryFormatPage },
  { path: '/gallery/:galleryIndex/range/', component: GalleryRangePage },
  { path: '/item/:itemIndex/format/:formatKind/', component: FormatPage },
  { path: '/item/:itemIndex/time-range/', component: TimeRangePage },
  { path: '/item/:itemIndex/chapter-range/', component: ChapterRangePage },
  { path: '/settings/', component: SettingsPage },
];
