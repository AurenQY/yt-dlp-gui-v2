import HomePage from './pages/HomePage.svelte';
import DetailPage from './pages/DetailPage.svelte';
import FormatPage from './pages/FormatPage.svelte';
import TimeRangePage from './pages/TimeRangePage.svelte';
import ChapterRangePage from './pages/ChapterRangePage.svelte';

export const routes = [
  { path: '/', component: HomePage },
  { path: '/item/:itemIndex/', component: DetailPage },
  { path: '/item/:itemIndex/format/:formatKind/', component: FormatPage },
  { path: '/item/:itemIndex/time-range/', component: TimeRangePage },
  { path: '/item/:itemIndex/chapter-range/', component: ChapterRangePage },
];
