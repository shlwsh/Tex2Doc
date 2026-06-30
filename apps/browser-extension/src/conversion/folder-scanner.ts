/**
 * Folder Scanner
 *
 * Reads the FileList from a webkitdirectory input and produces a structured
 * ScanResult ready for the packager.  Handles exclusion filtering, limits,
 * and delegates main-tex detection to the existing local-wasm logic.
 */

import { shouldExclude, MAX_FILE_COUNT, MAX_TOTAL_SIZE } from './folder-types';
import type { FolderEntry, ScanResult } from './folder-types';
import { detectMainTex } from './local-wasm';
import type { TexFileInfo } from './local-wasm';

/**
 * Scan a FileList from a webkitdirectory input.
 *
 * @param files  FileList from `<input webkitdirectory multiple>`
 * @param options.onProgress  Called every 200 files with the running count
 */
export async function scanFolder(
  files: FileList,
  options?: { onProgress?: (scanned: number) => void },
): Promise<ScanResult> {
  const arr = Array.from(files);
  const entries: FolderEntry[] = [];
  let excludedCount = 0;
  let totalSize = 0;
  let truncated = false;

  for (let i = 0; i < arr.length; i++) {
    const file = arr[i];

    // webkitRelativePath is the relative path inside the selected directory.
    // Fallback to just the file name if the browser does not provide it.
    const rel = file.webkitRelativePath || file.name;

    if (shouldExclude(rel)) {
      excludedCount++;
      continue;
    }

    // Respect global limits
    if (entries.length >= MAX_FILE_COUNT) {
      truncated = true;
      break;
    }
    if (totalSize + file.size > MAX_TOTAL_SIZE) {
      truncated = true;
      break;
    }

    entries.push({
      path: rel.replace(/\\/g, '/'), // Normalize Windows backslashes to forward slashes
      size: file.size,
      file,
    });
    totalSize += file.size;

    if (options?.onProgress && i % 200 === 0) {
      options.onProgress(i + 1);
    }
  }

  // Filter to .tex files for main-tex detection
  const texFiles: Array<{ path: string; size: number }> = entries
    .filter((e) => e.path.toLowerCase().endsWith('.tex'))
    .map((e) => ({ path: e.path, size: e.size }));

  const detectedMainTex = detectMainTexFromList(texFiles.map((t) => t.path));

  return {
    entries,
    texFiles,
    detectedMainTex,
    excludedCount,
    totalSize,
    truncated,
  };
}

/**
 * Delegate main-tex detection to the shared local-wasm logic.
 * Adapts a plain string[] to the TexFileInfo[] expected by detectMainTex.
 */
export function detectMainTexFromList(texPaths: string[]): string | null {
  if (texPaths.length === 0) return null;
  const infos: TexFileInfo[] = texPaths.map((p) => ({
    path: p,
    name: p.split('/').pop() || p,
    size: 0,
  }));
  return detectMainTex(infos);
}
