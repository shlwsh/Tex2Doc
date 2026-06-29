import { describe, it, expect, vi } from 'vitest';
import { buildZipFromFolder } from '@/conversion/folder-packager';
import type { FolderEntry } from '@/conversion/folder-types';

// ── Helper ────────────────────────────────────────────────────────────────────
function makeFile(name: string, content = 'Hello world'): File {
  return new File([content], name, { type: 'text/plain' });
}

function makeEntry(path: string, content = 'content'): FolderEntry {
  return { path, size: content.length, file: makeFile(path, content) };
}

// ── Tests ─────────────────────────────────────────────────────────────────────
describe('folder-packager', () => {
  describe('buildZipFromFolder', () => {
    it('produces a valid ZIP with PK\\x03\\x04 magic bytes', async () => {
      const entries = [makeEntry('main.tex', '\\documentclass{article}')];
      const result = await buildZipFromFolder(entries);
      expect(result[0]).toBe(0x50); // P
      expect(result[1]).toBe(0x4b); // K
      expect(result[2]).toBe(0x03);
      expect(result[3]).toBe(0x04);
    });

    it('produces a non-empty ZIP', async () => {
      const entries = [makeEntry('hello.txt', 'test')];
      const result = await buildZipFromFolder(entries);
      expect(result.byteLength).toBeGreaterThan(0);
    });

    it('calls onProgress for reading and packing phases', async () => {
      const onProgress = vi.fn();
      const entries = [
        makeEntry('a.txt'),
        makeEntry('b.txt'),
        makeEntry('c.txt'),
      ];
      await buildZipFromFolder(entries, { onProgress });
      const phases = onProgress.mock.calls.map((c) => c[0]);
      expect(phases).toContain('reading');
      expect(phases).toContain('packing');
    });

    it('reports correct total counts on progress', async () => {
      const onProgress = vi.fn();
      const entries = Array.from({ length: 10 }, (_, i) =>
        makeEntry(`file${i}.txt`),
      );
      await buildZipFromFolder(entries, { onProgress });

      // Last reading progress should report total=10
      const readingCalls = onProgress.mock.calls.filter((c) => c[0] === 'reading');
      const lastReading = readingCalls[readingCalls.length - 1];
      expect(lastReading[2]).toBe(10); // total
    });

    it('handles files with unicode paths', async () => {
      const entries = [
        makeEntry('中文/文档.tex'),
        makeEntry(' capítulo/main.tex'),
      ];
      const result = await buildZipFromFolder(entries);
      // Should produce valid ZIP without throwing
      expect(result[0]).toBe(0x50);
      expect(result[1]).toBe(0x4b);
    });

    it('handles nested directory paths', async () => {
      const entries = [
        makeEntry('src/chapters/intro.tex'),
        makeEntry('src/chapters/conclusion.tex'),
        makeEntry('ref.bib'),
      ];
      const result = await buildZipFromFolder(entries);
      expect(result.byteLength).toBeGreaterThan(0);
    });

    it('throws AbortError when signal is aborted', async () => {
      const controller = new AbortController();
      controller.abort();
      const entries = [makeEntry('a.txt')];
      await expect(
        buildZipFromFolder(entries, { signal: controller.signal }),
      ).rejects.toThrow(DOMException);
    });

    it('produces different sizes for STORE (level 0) compression', async () => {
      // Note: ZipPassThrough uses level 0 by default, so this tests the
      // output is still a valid zip regardless of compression settings.
      const longContent = 'A'.repeat(5000);
      const entries = [makeEntry('large.txt', longContent)];
      const result = await buildZipFromFolder(entries);
      expect(result.byteLength).toBeGreaterThan(0);
      // The result should start with ZIP local file header
      expect(result[0]).toBe(0x50);
      expect(result[1]).toBe(0x4b);
    });

    it('handles empty entry list gracefully', async () => {
      const entries: FolderEntry[] = [];
      const result = await buildZipFromFolder(entries);
      // Empty ZIP is valid (just end of central directory)
      expect(result[0]).toBe(0x50);
      expect(result[1]).toBe(0x4b);
    });

    it('does not throw on zero-size file', async () => {
      const emptyFile = new File([''], 'empty.tex', { type: 'text/plain' });
      const entries: FolderEntry[] = [
        { path: 'empty.tex', size: 0, file: emptyFile },
      ];
      const result = await buildZipFromFolder(entries);
      expect(result.byteLength).toBeGreaterThan(0);
    });
  });
});
