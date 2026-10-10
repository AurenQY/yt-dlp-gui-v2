import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('../', import.meta.url));
const result = spawnSync('cargo', ['build', '--manifest-path', 'xmp-wasm/Cargo.toml', '--target', 'wasm32-unknown-unknown', '--release', '--locked'], { cwd: root, stdio: 'inherit' });
if (result.error) throw result.error;
if (result.status !== 0) process.exit(result.status || 1);
mkdirSync(new URL('../chromium/resources/', import.meta.url), { recursive: true });
copyFileSync(new URL('../xmp-wasm/target/wasm32-unknown-unknown/release/gallery_xmp_wasm.wasm', import.meta.url), new URL('../chromium/resources/gallery-xmp.wasm', import.meta.url));
