#!/usr/bin/env bun

import { promises as fs } from "fs";
import * as path from "path";

type Range = { start: number; end: number };

type TransformResult = {
  text: string;
  changed: boolean;
};

const root = process.cwd();

const args = new Set(process.argv.slice(2));
const checkMode = args.delete("--check");

if (args.size > 0) {
  console.error("Unknown arguments:", [...args].join(", "));
  process.exit(1);
}

function isForbiddenNeighbor(char: string | undefined): boolean {
  if (!char) {
    return false;
  }
  return /[A-Za-z0-9@._\/-]/.test(char);
}

function buildInlineRanges(text: string): Range[] {
  const ranges: Range[] = [];
  const pattern = /`[^`]*`/g;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    const start = match.index;
    if (start === undefined) {
      continue;
    }
    ranges.push({ start, end: start + match[0].length });
  }

  return ranges;
}

function replaceWithGuard(text: string, ranges: Range[]): TransformResult {
  let cursor = 0;
  let changed = false;
  let output = "";

  while (cursor < text.length) {
    const index = text.indexOf("blz", cursor);
    if (index === -1) {
      output += text.slice(cursor);
      break;
    }

    const inMaskedRange = ranges.some((range) => index >= range.start && index < range.end);
    if (inMaskedRange) {
      output += text.slice(cursor, index + 3);
      cursor = index + 3;
      continue;
    }

    const before = index === 0 ? undefined : text[index - 1];
    const after = index + 3 >= text.length ? undefined : text[index + 3];

    if (isForbiddenNeighbor(before) || isForbiddenNeighbor(after)) {
      output += text.slice(cursor, index + 3);
      cursor = index + 3;
      continue;
    }

    changed = true;
    output += text.slice(cursor, index) + "BLZ";
    cursor = index + 3;
  }

  return { text: output, changed };
}

function transformMarkdown(content: string): TransformResult {
  const lines = content.split("\n");
  let inFence = false;
  let changed = false;

  const transformedLines = lines.map((line) => {
    const fenceMatch = line.match(/^\s*(`{3,}|~{3,})/);
    if (fenceMatch) {
      inFence = !inFence;
      return line;
    }

    if (inFence) {
      return line;
    }

    const trimmed = line.trimStart();
    if (trimmed.startsWith("#")) {
      const processedHeading = line.replace(/`blz`/g, "BLZ").replace(/`BLZ`/g, "BLZ");
      if (processedHeading !== line) {
        changed = true;
        line = processedHeading;
      }
    }

    const ranges = buildInlineRanges(line);
    const { text, changed: lineChanged } = replaceWithGuard(line, ranges);
    if (lineChanged) {
      changed = true;
    }
    return text;
  });

  return { text: transformedLines.join("\n"), changed };
}

function transformRust(content: string): TransformResult {
  const lines = content.split("\n");
  let changed = false;
  let inFence = false;

  const transformedLines = lines.map((line) => {
    const trimmed = line.trimStart();
    if (!trimmed.startsWith("//")) {
      inFence = false;
      return line;
    }

    const whitespaceLength = line.length - trimmed.length;
    const markerLength = trimmed.startsWith("///") || trimmed.startsWith("//!") ? 3 : 2;
    const prefixEnd = whitespaceLength + markerLength;
    const prefix = line.slice(0, prefixEnd);
    const rest = line.slice(prefixEnd);

    const trimmedRest = rest.trimStart();
    const fenceMatch = trimmedRest.match(/^(`{3,}|~{3,})/);
    if (fenceMatch) {
      inFence = !inFence;
      return line;
    }

    if (inFence) {
      return line;
    }

    const ranges = buildInlineRanges(rest);
    const { text, changed: lineChanged } = replaceWithGuard(rest, ranges);

    if (lineChanged) {
      changed = true;
      return prefix + text;
    }

    return line;
  });

  return { text: transformedLines.join("\n"), changed };
}

function transformShell(content: string): TransformResult {
  const lines = content.split("\n");
  let changed = false;

  const transformedLines = lines.map((line) => {
    const trimmed = line.trimStart();
    if (!trimmed.startsWith("#") || trimmed.startsWith("#!")) {
      return line;
    }

    const whitespaceLength = line.length - trimmed.length;
    const prefixEnd = whitespaceLength + 1;
    const prefix = line.slice(0, prefixEnd);
    const rest = line.slice(prefixEnd);

    const ranges = buildInlineRanges(rest);
    const { text, changed: lineChanged } = replaceWithGuard(rest, ranges);

    if (lineChanged) {
      changed = true;
      return prefix + text;
    }

    return line;
  });

  return { text: transformedLines.join("\n"), changed };
}

async function collect(patterns: string[]): Promise<string[]> {
  const result = new Set<string>();

  for (const pattern of patterns) {
    const glob = new Bun.Glob(pattern);
    for await (const match of glob.scan({ cwd: root, dot: true })) {
      if (match.startsWith("target/") || match.startsWith("npm/")) {
        continue;
      }
      result.add(match);
    }
  }

  return [...result];
}

function transformForFile(filePath: string, content: string): TransformResult {
  if (filePath.endsWith(".md")) {
    return transformMarkdown(content);
  }
  if (filePath.endsWith(".rs")) {
    return transformRust(content);
  }
  if (filePath.endsWith(".sh")) {
    return transformShell(content);
  }

  return { text: content, changed: false };
}

async function main(): Promise<void> {
  const markdownFiles = await collect([
    "*.md",
    "docs/**/*.md",
    "crates/**/*.md",
    "scripts/**/*.md",
  ]);
  const rustFiles = await collect(["crates/**/*.rs"]);
  const shellFiles = await collect(["scripts/**/*.sh"]);

  const files = [...new Set([...markdownFiles, ...rustFiles, ...shellFiles])].sort();

  const modified: string[] = [];

  for (const relativePath of files) {
    const absolutePath = path.join(root, relativePath);
    const original = await fs.readFile(absolutePath, "utf8");
    const { text, changed } = transformForFile(relativePath, original);

    if (!changed) {
      continue;
    }

    modified.push(relativePath);

    if (!checkMode) {
      await fs.writeFile(absolutePath, text, "utf8");
    }
  }

  if (modified.length === 0) {
    if (!checkMode) {
      console.log("No stylization updates needed.");
    }
    return;
  }

  if (checkMode) {
    console.error("Stylization updates required for:");
    for (const file of modified) {
      console.error(`  ${file}`);
    }
    process.exit(1);
  }

  console.log("Updated BLZ stylization in:");
  for (const file of modified) {
    console.log(`  ${file}`);
  }
}

await main();
