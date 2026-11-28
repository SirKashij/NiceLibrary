// Builds the app icons in src-tauri/icons/ from src-tauri/icon.svg: node scripts/build-icons.ts
// `tauri icon` renders every size from the SVG. The 16 px image of icon.ico, the one the Tray and the
// title bar use, comes from the simplified src-tauri/icon-small.svg instead, which stays legible there.
// Only the files the app ships are kept; the store, iOS and Android sizes `tauri icon` adds are not.
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { withIcoImage } from './ico.ts';

const ROOT = join(import.meta.dirname, '..');
const ICONS = join(ROOT, 'src-tauri', 'icons');
const SHIPPED = ['32x32.png', '128x128.png', '128x128@2x.png', 'icon.ico', 'icon.icns', 'icon.png'];
const SMALL_SIZE = 16;

function tauriIcon(svg: string, out: string, ...options: string[]): void {
  const cli = join(ROOT, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');
  execFileSync(process.execPath, [cli, 'icon', join(ROOT, 'src-tauri', svg), '-o', out, ...options], {
    stdio: 'inherit',
  });
}

const work = mkdtempSync(join(tmpdir(), 'nicelibrary-icons-'));
try {
  tauriIcon('icon.svg', join(work, 'full'));
  tauriIcon('icon-small.svg', join(work, 'small'), '-p', String(SMALL_SIZE));
  for (const file of SHIPPED) copyFileSync(join(work, 'full', file), join(ICONS, file));
  const small = readFileSync(join(work, 'small', `${SMALL_SIZE}x${SMALL_SIZE}.png`));
  const ico = join(ICONS, 'icon.ico');
  writeFileSync(ico, withIcoImage(readFileSync(ico), SMALL_SIZE, small));
} finally {
  rmSync(work, { recursive: true, force: true });
}
