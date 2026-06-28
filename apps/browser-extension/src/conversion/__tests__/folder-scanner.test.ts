import { describe, it, expect, vi, beforeEach } from 'vitest';
import { shouldExclude, MAX_DEPTH, MAX_FILE_COUNT, MAX_TOTAL_SIZE } from '@/conversion/folder-types';

// ── Helper ────────────────────────────────────────────────────────────────────
function makeFile(name: string, size = 100): File {
  return new File(['x'.repeat(size)], name, { type: 'text/plain' });
}

function makeFileList(files: File[]): FileList {
  const dt = new DataTransfer();
  for (const f of files) dt.items.add(f);
  return dt.files;
}

// ── shouldExclude unit tests ─────────────────────────────────────────────────
describe('folder-types', () => {
  describe('shouldExclude', () => {
    it('returns false for a normal .tex file', () => {
      expect(shouldExclude('main.tex')).toBe(false);
      expect(shouldExclude('chapters/intro.tex')).toBe(false);
      expect(shouldExclude('sub/chapters/deep/intro.tex')).toBe(false);
    });

    it('excludes node_modules directories', () => {
      expect(shouldExclude('node_modules/lodash/index.js')).toBe(true);
      expect(shouldExclude('node_modules/.package-lock.json')).toBe(true);
      expect(shouldExclude('a/node_modules/b/package.json')).toBe(true);
    });

    it('excludes .git directories', () => {
      expect(shouldExclude('.git/config')).toBe(true);
      expect(shouldExclude('a/b/.git/HEAD')).toBe(true);
    });

    it('excludes dist / build / out / target', () => {
      expect(shouldExclude('dist/bundle.js')).toBe(true);
      expect(shouldExclude('build/output.png')).toBe(true);
      expect(shouldExclude('out/result.pdf')).toBe(true);
      expect(shouldExclude('target/debug/binary')).toBe(true);
    });

    it('excludes __pycache__', () => {
      expect(shouldExclude('__pycache__/cache.pyc')).toBe(true);
    });

    it('excludes .venv / venv', () => {
      expect(shouldExclude('.venv/lib/module.py')).toBe(true);
      expect(shouldExclude('venv/site-packages/app.py')).toBe(true);
    });

    it('excludes .next / .cache', () => {
      expect(shouldExclude('.next/cache/file.js')).toBe(true);
      expect(shouldExclude('.cache/data.json')).toBe(true);
    });

    it('excludes compiled output extensions', () => {
      expect(shouldExclude('main.aux')).toBe(true);
      expect(shouldExclude('main.log')).toBe(true);
      expect(shouldExclude('main.out')).toBe(true);
      expect(shouldExclude('main.toc')).toBe(true);
      expect(shouldExclude('main.bbl')).toBe(true);
      expect(shouldExclude('main.bcf')).toBe(true);
      expect(shouldExclude('main.blg')).toBe(true);
      expect(shouldExclude('main.synctex.gz')).toBe(true);
      expect(shouldExclude('main.fls')).toBe(true);
      expect(shouldExclude('main.fdb_latexmk')).toBe(true);
      expect(shouldExclude('main.nav')).toBe(true);
      expect(shouldExclude('main.snm')).toBe(true);
      expect(shouldExclude('main.vrb')).toBe(true);
      expect(shouldExclude('main.run.xml')).toBe(true);
      expect(shouldExclude('main.synctex')).toBe(true);
      expect(shouldExclude('main.xdv')).toBe(true);
      expect(shouldExclude('main.pdf')).toBe(true);
    });

    it('excludes whole-file compiled outputs', () => {
      expect(shouldExclude('.DS_Store')).toBe(true);
      expect(shouldExclude('Thumbs.db')).toBe(true);
      expect(shouldExclude('desktop.ini')).toBe(true);
    });

    it('allows .tex files inside excluded dirs that are not excluded by name', () => {
      // a node_modules dir should exclude all its children regardless of extension
      expect(shouldExclude('node_modules/pkg/main.tex')).toBe(true);
    });

    it('allows .tex files with spaces and unicode', () => {
      expect(shouldExclude('我的 文档/main.tex')).toBe(false);
      expect(shouldExclude('chapter 1/intro.tex')).toBe(false);
      expect(shouldExclude('你好世界/paper.tex')).toBe(false);
    });

    it('allows .cls and .sty files (LaTeX support files)', () => {
      expect(shouldExclude('cls/article.cls')).toBe(false);
      expect(shouldExclude('styIEEEtran.sty')).toBe(false);
    });

    it('allows .bib, .bst, .cls, .sty, .cfg, .def, .fd files', () => {
      expect(shouldExclude('ref.bib')).toBe(false);
      expect(shouldExclude('style.bst')).toBe(false);
      expect(shouldExclude('myclass.cls')).toBe(false);
      expect(shouldExclude('mystyle.sty')).toBe(false);
      expect(shouldExclude('config.cfg')).toBe(false);
      expect(shouldExclude('definitions.def')).toBe(false);
    });

    it('returns true for paths deeper than MAX_DEPTH', () => {
      const deep = 'a/b/c/d/e/f/g/h/i/main.tex';
      expect(shouldExclude(deep)).toBe(true);
      // exactly MAX_DEPTH segments should be allowed
      const ok = 'a/b/c/d/e/f/g/h/main.tex';
      expect(shouldExclude(ok)).toBe(false);
    });
  });
});
