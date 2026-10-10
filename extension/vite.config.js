import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

function source(relativePath) {
  return fileURLToPath(new URL(relativePath, import.meta.url));
}

export default defineConfig({
  plugins: [svelte()],
  build: {
    emptyOutDir: true,
    outDir: source('./chromium/build'),
    lib: {
      entry: {
        popup: source('./src/popup/main.js'),
        album: source('./src/album/album.js'),
        background: source('./src/background/background.js'),
        'matcher-worker': source('./src/matcher/matcher-worker.js'),
        'xmp-worker': source('./src/image/xmp-worker.js'),
      },
      formats: ['es'],
      fileName: (_format, entryName) => entryName + '.js',
      cssFileName: 'popup',
    },
    rollupOptions: {
      output: {
        chunkFileNames: 'chunks/[name]-[hash].js',
      },
    },
  },
});
