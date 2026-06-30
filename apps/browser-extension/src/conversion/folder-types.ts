/**
 * Folder Upload Type Definitions
 *
 * Shared types for folder scanning and packaging.
 * These are used by the popup UI and the folder scanner/packager modules.
 */

/** POSIX-style relative path from the selected root directory. */
export interface FolderEntry {
  path: string;
  size: number;
  file: File;
}

export interface ScanResult {
  entries: FolderEntry[];
  texFiles: Array<{ path: string; size: number }>;
  detectedMainTex: string | null;
  excludedCount: number;
  totalSize: number;
  truncated: boolean;
}

export const MAX_DEPTH = 8;
export const MAX_FILE_COUNT = 5000;
export const MAX_TOTAL_SIZE = 100 * 1024 * 1024; // 100 MB

const EXCLUDED_DIR_NAMES = new Set([
  'node_modules',
  '.git',
  '.svn',
  '.hg',
  'dist',
  'build',
  'out',
  'target',
  '.next',
  '.cache',
  '__pycache__',
  '.venv',
  'venv',
]);

const EXCLUDED_FILE_EXTENSIONS = new Set([
  '.aux',
  '.log',
  '.out',
  '.toc',
  '.bbl',
  '.bcf',
  '.blg',
  '.synctex.gz',
  '.fls',
  '.fdb_latexmk',
  '.nav',
  '.snm',
  '.vrb',
  '.run.xml',
  '.synctex',
  '.xdv',
  '.pdf',
]);

const EXCLUDED_WHOLE_NAMES = new Set([
  '.DS_Store',
  'Thumbs.db',
  'desktop.ini',
]);

/**
 * Check if a relative path should be excluded from the ZIP package.
 * Rules:
 * - Directories exceeding MAX_DEPTH are excluded
 * - Known VCS / build-tool / cache directories are excluded
 * - Known compiled output file names are excluded
 * - Files with known build-artifact extensions are excluded
 * - Double-extension patterns like .synctex.gz are handled
 */
export function shouldExclude(relativePath: string): boolean {
  const segments = relativePath.split('/');
  const depth = segments.length - 1;

  if (depth > MAX_DEPTH) return true;

  // Check each path segment for excluded directory names
  for (const seg of segments) {
    if (EXCLUDED_DIR_NAMES.has(seg)) return true;
  }

  const name = segments[segments.length - 1];
  if (!name) return false;

  // Check whole-file exclusions
  if (EXCLUDED_WHOLE_NAMES.has(name)) return true;

  // Check extension-based exclusions
  const dot = name.lastIndexOf('.');
  if (dot < 0) return false;

  const ext = name.slice(dot).toLowerCase();
  if (EXCLUDED_FILE_EXTENSIONS.has(ext)) return true;

  // Handle multi-part extensions like .synctex.gz and .run.xml
  if (dot > 0) {
    const innerDot = name.lastIndexOf('.', dot - 1);
    if (innerDot >= 0) {
      const innerExt = name.slice(innerDot).toLowerCase();
      if (EXCLUDED_FILE_EXTENSIONS.has(innerExt)) return true;
    }
  }

  return false;
}
