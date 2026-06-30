/**
 * Project ZIP Utilities
 *
 * Handles ZIP file operations for project conversion
 */

import JSZip from 'jszip';

/**
 * Read file as ArrayBuffer
 */
export function readFileAsArrayBuffer(file: File): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      const buffer = e.target?.result as ArrayBuffer;
      resolve(new Uint8Array(buffer));
    };
    reader.onerror = () => reject(new Error('Failed to read file'));
    reader.readAsArrayBuffer(file);
  });
}

/**
 * Read file as text
 */
export function readFileAsText(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      resolve(e.target?.result as string);
    };
    reader.onerror = () => reject(new Error('Failed to read file'));
    reader.readAsText(file);
  });
}

/**
 * Extract main tex files from ZIP
 */
export async function extractTexFiles(zipBytes: Uint8Array): Promise<string[]> {
  const zip = await JSZip.loadAsync(zipBytes);
  const texFiles: string[] = [];

  zip.forEach((path, file) => {
    if (!file.dir && path.endsWith('.tex')) {
      texFiles.push(path);
    }
  });

  return texFiles;
}

/**
 * Find main tex file (common patterns)
 */
export async function findMainTex(zipBytes: Uint8Array): Promise<string | null> {
  const texFiles = await extractTexFiles(zipBytes);

  // Common main file names in order of preference
  const mainCandidates = [
    'main.tex',
    'main-jos.tex',
    'main-article.tex',
    'main-paper.tex',
    'minimal.tex',
    'paper.tex',
    'article.tex',
    'thesis.tex',
    'report.tex',
    'document.tex',
  ];

  for (const candidate of mainCandidates) {
    if (texFiles.some((f) => f.toLowerCase() === candidate.toLowerCase())) {
      return candidate;
    }
  }

  // Return the first .tex file if no main candidate found
  return texFiles.length > 0 ? texFiles[0] : null;
}

/**
 * Get ZIP file info
 */
export async function getZipInfo(
  zipBytes: Uint8Array
): Promise<{
  fileCount: number;
  texCount: number;
  mainTexCandidates: string[];
}> {
  const zip = await JSZip.loadAsync(zipBytes);
  const files: string[] = [];
  const texFiles: string[] = [];

  zip.forEach((path, file) => {
    if (!file.dir) {
      files.push(path);
      if (path.endsWith('.tex')) {
        texFiles.push(path);
      }
    }
  });

  // Find main candidates
  const mainCandidates = texFiles.filter((f) => {
    const name = f.split('/').pop()?.toLowerCase() ?? '';
    return (
      name.startsWith('main') ||
      name === 'minimal.tex' ||
      name === 'paper.tex' ||
      name === 'article.tex'
    );
  });

  return {
    fileCount: files.length,
    texCount: texFiles.length,
    mainTexCandidates: mainCandidates,
  };
}

/**
 * Validate ZIP contents
 */
export async function validateZipContents(
  zipBytes: Uint8Array
): Promise<{ valid: boolean; error?: string }> {
  try {
    const zip = await JSZip.loadAsync(zipBytes);
    const files = Object.keys(zip.files);

    if (files.length === 0) {
      return { valid: false, error: 'ZIP file is empty' };
    }

    // Check for .tex files
    const hasTex = files.some((f) => f.endsWith('.tex'));
    if (!hasTex) {
      return { valid: false, error: 'No .tex files found in ZIP' };
    }

    return { valid: true };
  } catch (error) {
    return {
      valid: false,
      error: `Invalid ZIP file: ${error instanceof Error ? error.message : 'Unknown error'}`,
    };
  }
}

/**
 * Format file size
 */
export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * Check if file is ZIP
 */
export function isZipFile(bytes: Uint8Array): boolean {
  return bytes.length >= 2 && bytes[0] === 0x50 && bytes[1] === 0x4b;
}

/**
 * Get file extension
 */
export function getFileExtension(filename: string): string {
  const parts = filename.split('.');
  return parts.length > 1 ? parts.pop()!.toLowerCase() : '';
}

/**
 * Check if file is a ZIP by name or magic bytes
 */
export async function isZipByContent(file: File): Promise<boolean> {
  return new Promise((resolve) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      const bytes = new Uint8Array(e.target?.result as ArrayBuffer);
      resolve(isZipFile(bytes));
    };
    reader.onerror = () => resolve(false);
    reader.readAsArrayBuffer(file.slice(0, 2));
  });
}
