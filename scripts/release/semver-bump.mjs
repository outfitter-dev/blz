#!/usr/bin/env node

import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';

function usage(message) {
  if (message) {
    console.error(message);
  }
  console.error(`Usage:
  semver-bump.mjs next --mode <patch|minor|major|canary|set> --current <version> [--value <version>] [--meta <path>] [--write-meta]
  semver-bump.mjs sync --version <version> [--repo-root <path>]
  semver-bump.mjs check --expect <version> [--repo-root <path>]
`);
  process.exit(1);
}

function parseFlags(args) {
  const flags = { _: [] };
  for (let i = 0; i < args.length; i += 1) {
    const token = args[i];
    if (token.startsWith('--')) {
      const key = token.slice(2);
      const next = args[i + 1];
      if (next && !next.startsWith('--')) {
        flags[key] = next;
        i += 1;
      } else {
        flags[key] = true;
      }
    } else {
      flags._.push(token);
    }
  }
  return flags;
}

function parseSemver(version) {
  const match = /^([0-9]+)\.([0-9]+)\.([0-9]+)(?:-([0-9A-Za-z.-]+))?(?:\+([0-9A-Za-z.-]+))?$/.exec(version);
  if (!match) {
    usage(`Invalid semver string: ${version}`);
  }
  const prerelease = match[4] ? match[4].split('.') : [];
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease,
    build: match[5] ?? null,
  };
}

function compareIdentifiers(a, b) {
  const numA = /^\d+$/.test(a);
  const numB = /^\d+$/.test(b);
  if (numA && numB) {
    return Number(a) - Number(b);
  }
  if (numA) {
    return -1;
  }
  if (numB) {
    return 1;
  }
  return a.localeCompare(b);
}

function compareSemver(a, b) {
  if (a.major !== b.major) {
    return a.major - b.major;
  }
  if (a.minor !== b.minor) {
    return a.minor - b.minor;
  }
  if (a.patch !== b.patch) {
    return a.patch - b.patch;
  }
  const aPre = a.prerelease ?? [];
  const bPre = b.prerelease ?? [];
  if (aPre.length === 0 && bPre.length === 0) {
    return 0;
  }
  if (aPre.length === 0) {
    return 1;
  }
  if (bPre.length === 0) {
    return -1;
  }
  const len = Math.max(aPre.length, bPre.length);
  for (let i = 0; i < len; i += 1) {
    const aId = aPre[i];
    const bId = bPre[i];
    if (aId === undefined) {
      return -1;
    }
    if (bId === undefined) {
      return 1;
    }
    const diff = compareIdentifiers(aId, bId);
    if (diff !== 0) {
      return diff;
    }
  }
  return 0;
}

function formatSemver({ major, minor, patch, prerelease = [], build = null }) {
  let version = `${major}.${minor}.${patch}`;
  if (prerelease.length > 0) {
    version += `-${prerelease.join('.')}`;
  }
  if (build) {
    version += `+${build}`;
  }
  return version;
}

function readMeta(metaPath) {
  if (!metaPath) {
    return {};
  }
  if (!existsSync(metaPath)) {
    return {};
  }
  const raw = readFileSync(metaPath, 'utf8');
  try {
    return JSON.parse(raw);
  } catch (error) {
    throw new Error(`Failed to parse meta file ${metaPath}: ${error.message}`);
  }
}

function writeMeta(metaPath, meta) {
  if (!metaPath) {
    return;
  }
  writeFileSync(metaPath, `${JSON.stringify(meta, null, 2)}\n`);
}

function nextVersion({ mode, current, value, metaPath, writeMetaFlag }) {
  if (!mode) {
    usage('Missing --mode');
  }
  if (!current) {
    usage('Missing --current');
  }
  const parsed = parseSemver(current);
  const base = { ...parsed, prerelease: [], build: null };

  if (mode === 'set') {
    if (!value) {
      usage('Missing --value for mode set');
    }
    const target = parseSemver(value);
    if (compareSemver(target, parsed) <= 0) {
      throw new Error(`Target version ${value} must be greater than current ${current}`);
    }
    return formatSemver({ ...target, prerelease: target.prerelease ?? [] });
  }

  if (mode === 'major') {
    return formatSemver({ major: base.major + 1, minor: 0, patch: 0, prerelease: [], build: null });
  }
  if (mode === 'minor') {
    return formatSemver({ major: base.major, minor: base.minor + 1, patch: 0, prerelease: [], build: null });
  }
  if (mode === 'patch') {
    return formatSemver({ major: base.major, minor: base.minor, patch: base.patch + 1, prerelease: [], build: null });
  }
  if (mode === 'canary') {
    const meta = readMeta(metaPath);
    const baseId = `${base.major}.${base.minor}.${base.patch}`;
    let sequence = 1;
    if (meta.lastCanary && meta.lastCanary.base === baseId) {
      sequence = meta.lastCanary.sequence + 1;
    }
    const next = formatSemver({
      major: base.major,
      minor: base.minor,
      patch: base.patch,
      prerelease: [`canary`, String(sequence)],
      build: null,
    });
    if (writeMetaFlag) {
      const nextMeta = {
        ...meta,
        lastCanary: {
          base: baseId,
          sequence,
        },
      };
      writeMeta(metaPath, nextMeta);
    }
    return next;
  }

  throw new Error(`Unsupported mode: ${mode}`);
}

function syncPackageJson({ version, repoRoot }) {
  const root = repoRoot ? resolve(repoRoot) : process.cwd();
  const pkgPath = resolve(root, 'package.json');
  if (existsSync(pkgPath)) {
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
    pkg.version = version;
    writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);
  }
  const lockPath = resolve(root, 'package-lock.json');
  if (existsSync(lockPath)) {
    const lock = JSON.parse(readFileSync(lockPath, 'utf8'));
    lock.version = version;
    if (lock.packages && lock.packages['']) {
      lock.packages[''].version = version;
    }
    writeFileSync(lockPath, `${JSON.stringify(lock, null, 2)}\n`);
  }
}

function checkSync({ expect, repoRoot }) {
  const root = repoRoot ? resolve(repoRoot) : process.cwd();
  const pkgPath = resolve(root, 'package.json');
  if (existsSync(pkgPath)) {
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
    if (pkg.version !== expect) {
      throw new Error(`package.json version ${pkg.version} does not match ${expect}`);
    }
  }
  const lockPath = resolve(root, 'package-lock.json');
  if (existsSync(lockPath)) {
    const lock = JSON.parse(readFileSync(lockPath, 'utf8'));
    if (lock.version !== expect) {
      throw new Error(`package-lock.json version ${lock.version} does not match ${expect}`);
    }
    if (lock.packages && lock.packages[''] && lock.packages[''].version !== expect) {
      throw new Error(`Root package entry in package-lock.json is ${lock.packages[''].version}, expected ${expect}`);
    }
  }
}

function main(argv) {
  if (argv.length === 0) {
    usage();
  }
  const [command, ...rest] = argv;
  const flags = parseFlags(rest);

  try {
    if (command === 'next') {
      const mode = flags.mode;
      const current = flags.current;
      const value = flags.value;
      const metaPath = flags.meta;
      const writeMetaFlag = Boolean(flags['write-meta']);
      const next = nextVersion({ mode, current, value, metaPath, writeMetaFlag });
      process.stdout.write(next);
      return;
    }

    if (command === 'sync') {
      const version = flags.version;
      if (!version) {
        usage('Missing --version for sync');
      }
      syncPackageJson({ version, repoRoot: flags['repo-root'] });
      return;
    }

    if (command === 'check') {
      const expect = flags.expect;
      if (!expect) {
        usage('Missing --expect for check');
      }
      checkSync({ expect, repoRoot: flags['repo-root'] });
      return;
    }
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }

  usage(`Unknown command: ${command}`);
}

main(process.argv.slice(2));
