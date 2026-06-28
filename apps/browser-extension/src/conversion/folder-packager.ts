/**
 * Folder Packager
 *
 * Takes FolderEntry[] and produces a standard ZIP Uint8Array using fflate.
 * The output is a valid PKZip stream consumable by the WASM engine and
 * by the cloud upload pipeline without any changes to either.
 */

import { Zip, ZipPassThrough } from 'fflate';
import type { FolderEntry } from './folder-types';

export interface PackagerOptions {
  onProgress?: (phase: 'reading' | 'packing', current: number, total: number) => void;
  level?: 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9;
  signal?: AbortSignal;
}

/**
 * Build a ZIP byte array from a list of folder entries.
 *
 * Phase 1 (reading): streams each File as an ArrayBuffer, yielding the
 *   main thread every 50 files to avoid blocking the UI.
 * Phase 2 (packing): feeds all buffers into fflate's Zip + ZipPassThrough
 *   streams synchronously.
 *
 * @throws DOMException with code 'AbortError' if signal is aborted.
 */
export async function buildZipFromFolder(
  entries: FolderEntry[],
  options: PackagerOptions = {},
): Promise<Uint8Array> {
  const { onProgress, signal } = options;
  const filesMap: Record<string, Uint8Array> = {};

  // ── Phase 1: stream read ────────────────────────────────────────────────
  for (let i = 0; i < entries.length; i++) {
    if (signal?.aborted) {
      throw new DOMException('Aborted', 'AbortError');
    }

    const buf = await entries[i].file.arrayBuffer();
    filesMap[entries[i].path] = new Uint8Array(buf);

    if (onProgress && i % 50 === 0) {
      onProgress('reading', i + 1, entries.length);
      // Yield the main thread so the popup stays responsive
      await new Promise<void>((resolve) => setTimeout(resolve, 0));
    }
  }
  onProgress?.('reading', entries.length, entries.length);

  // ── Phase 2: pack ──────────────────────────────────────────────────────
  onProgress?.('packing', 0, 1);

  return new Promise((resolve, reject) => {
    const chunks: Uint8Array[] = [];

    const zip = new Zip((err, data, final) => {
      if (err) {
        reject(err);
        return;
      }
      if (data) chunks.push(data);
      if (final) {
        // Concatenate all chunks into a single contiguous Uint8Array
        const totalLen = chunks.reduce((s, c) => s + c.byteLength, 0);
        const out = new Uint8Array(totalLen);
        let offset = 0;
        for (const c of chunks) {
          out.set(c, offset);
          offset += c.byteLength;
        }
        onProgress?.('packing', 1, 1);
        resolve(out);
      }
    });

    for (const [path, data] of Object.entries(filesMap)) {
      // ZipPassThrough stores files without additional compression (level 0).
      // This is appropriate for already-compressed assets (images, PDFs) and
      // avoids the CPU cost of re-compressing during the build step.
      const entry = new ZipPassThrough(path);
      zip.add(entry);
      entry.push(data, true);
    }

    zip.end();
  });
}
