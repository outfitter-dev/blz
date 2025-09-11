#!/usr/bin/env node
// Postinstall: download the platform-specific blz binary into npm/bin
// Skips if BLZ_SKIP_POSTINSTALL=1

import { mkdirSync, chmodSync, createWriteStream, existsSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';
import { arch, platform } from 'os';
import { pipeline } from 'stream/promises';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const log = (...args) => console.log('[blz postinstall]', ...args);
const warn = (...args) => console.warn('[blz postinstall]', ...args);
const error = (...args) => console.error('[blz postinstall]', ...args);

try {
  if (process.env.BLZ_SKIP_POSTINSTALL === '1') {
    log('Skipping binary download (BLZ_SKIP_POSTINSTALL=1)');
    process.exit(0);
  }

  const pkg = await import(join(dirname(__dirname), 'package.json'), { assert: { type: 'json' } });
  const version = pkg.default.version;
  const p = platform();
  const a = arch();

  // Map Node platform/arch to asset naming
  // Asset names expected on GitHub Releases: blz-<platform>-<arch>[.exe]
  // platform: darwin | linux | win32
  // arch: x64 | arm64
  const isWin = p === 'win32';
  const assetName = isWin ? `blz-${p}-${a}.exe` : `blz-${p}-${a}`;
  const outputName = isWin ? 'blz.exe' : `blz-${p}-${a}`;

  const binDir = join(__dirname, 'bin');
  const dest = join(binDir, outputName);

  if (!existsSync(binDir)) mkdirSync(binDir, { recursive: true });
  if (existsSync(dest)) {
    log(`Binary already present: ${dest}`);
    process.exit(0);
  }

  const base = process.env.BLZ_BINARY_BASE_URL || `https://github.com/outfitter-dev/blz/releases/download/v${version}`;
  const url = `${base}/${assetName}`;

  log(`Downloading ${url}`);

  // Use global fetch (Node >=18)
  const res = await fetch(url, { redirect: 'follow' });
  if (!res.ok) {
    error(`Failed to download binary (${res.status} ${res.statusText}).`);
    error('You can set BLZ_SKIP_POSTINSTALL=1 to skip, or BLZ_BINARY_BASE_URL to a custom source.');
    process.exit(1);
  }

  const fileStream = createWriteStream(dest, { mode: 0o755 });
  // Use pipeline to properly propagate errors from both source and destination
  await pipeline(res.body, fileStream);

  if (!isWin) {
    chmodSync(dest, 0o755);
  }

  log(`Installed blz binary: ${dest}`);
} catch (e) {
  error('Postinstall failed:', e?.message || e);
  process.exit(1);
}
