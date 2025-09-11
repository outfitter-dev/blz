#!/usr/bin/env node
// Prepack: prepare the npm package contents before publishing.
// Currently ensures npm/bin exists and is clean.

import { mkdirSync, rmSync, existsSync, writeFileSync, copyFileSync, chmodSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';
import { arch, platform } from 'os';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const binDir = join(__dirname, 'bin');

try {
  if (existsSync(binDir)) {
    // Remove any stale files to keep package clean; keep directory
    rmSync(binDir, { recursive: true, force: true });
  }
  mkdirSync(binDir, { recursive: true });
  // Ensure directory is kept in the tarball even if empty
  writeFileSync(join(binDir, '.gitkeep'), '');

  // Optional: bundle local binary for current platform
  // Enable with BLZ_PREPACK_LOCAL=1 (e.g., after `cargo build --release`)
  if (process.env.BLZ_PREPACK_LOCAL === '1') {
    const p = platform();
    const a = arch();
    const isWin = p === 'win32';
    const src = join(dirname(__dirname), 'target', 'release', isWin ? 'blz.exe' : 'blz');
    const destName = isWin ? 'blz.exe' : `blz-${p}-${a}`;
    const dest = join(binDir, destName);
    if (existsSync(src)) {
      copyFileSync(src, dest);
      if (!isWin) chmodSync(dest, 0o755);
      console.log(`[blz prepack] Bundled local binary -> ${dest}`);
    } else {
      console.warn('[blz prepack] BLZ_PREPACK_LOCAL=1 but no local binary found at', src);
    }
  }

  console.log('[blz prepack] Prepared npm/bin');
} catch (e) {
  console.error('[blz prepack] Failed:', e?.message || e);
  process.exit(1);
}
