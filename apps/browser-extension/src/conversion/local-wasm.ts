/**
 * Local WASM Conversion Module
 *
 * Handles local conversion using the browser WASM engine
 */

import {
  initWasm,
  isWasmReady,
  convertZipToDocxBytes,
  validateDocx,
  isWithinSizeLimit,
  getFileSizeLimit,
  getFileSizeLimitDisplay,
  type WasmConvertResult,
} from '@/workers/wasm-worker';
import { downloadBytes } from '@/browser/downloads';
import { saveJob, updateJobStatus } from '@/state/job-store';
import type { JobRecord } from '@/shared/types';

export interface TexFileInfo {
  path: string;
  name: string;
  size: number;
}

export interface ZipAnalysis {
  isValid: boolean;
  texFiles: TexFileInfo[];
  detectedMainTex: string | null;
  hasDocumentClass: boolean;
}

export interface LocalConversionOptions {
  fileName: string;
  mainTex: string;
  bibStyle?: 'numeric' | 'author-year';
}

export interface LocalConversionResult {
  jobId: string;
  docxBytes: Uint8Array;
  docxFilename: string;
  warnings: string[];
  elapsedMs: number;
}

/**
 * Convert ZIP file locally using WASM
 */
export async function convertLocal(
  zipBytes: Uint8Array,
  options: LocalConversionOptions,
  onProgress?: (progress: number) => void
): Promise<LocalConversionResult> {
  const startTime = Date.now();

  // Validate file size
  if (!isWithinSizeLimit(zipBytes.length)) {
    throw new Error(
      `File too large. Maximum size is ${getFileSizeLimitDisplay()}.`
    );
  }

  // Create job record
  const jobId = crypto.randomUUID();
  const job: JobRecord = {
    id: jobId,
    file_name: options.fileName,
    main_tex: options.mainTex,
    profile: 'local',
    quality: 'balanced',
    mode: 'local',
    status: 'pending',
    progress: 0,
    created_at: Date.now(),
    updated_at: Date.now(),
  };

  await saveJob(job);
  await updateJobStatus(jobId, 'processing', 10);

  onProgress?.(10);

  try {
    // Initialize WASM if needed
    if (!isWasmReady()) {
      await initWasm();
    }

    onProgress?.(30);

    // Perform conversion
    const result = await convertZipToDocxBytes(
      zipBytes,
      options.mainTex,
      { bib_style: options.bibStyle }
    );

    onProgress?.(80);

    // Validate result
    if (!validateDocx(result)) {
      throw new Error('Conversion produced invalid DOCX file');
    }

    await updateJobStatus(jobId, 'completed', 100);

    onProgress?.(100);

    const elapsedMs = Date.now() - startTime;

    return {
      jobId,
      docxBytes: result,
      docxFilename: options.fileName.replace(/\.[^.]+$/, '') + '.docx',
      warnings: [], // WASM doesn't return warnings in simplified mode
      elapsedMs,
    };
  } catch (error) {
    await updateJobStatus(jobId, 'failed');
    throw error;
  }
}

/**
 * Convert and download locally
 */
export async function convertAndDownload(
  zipBytes: Uint8Array,
  options: LocalConversionOptions,
  onProgress?: (progress: number) => void
): Promise<void> {
  const result = await convertLocal(zipBytes, options, onProgress);
  await downloadBytes(result.docxBytes, result.docxFilename);
}

/**
 * Check if local conversion is available
 */
export async function isLocalConversionAvailable(): Promise<boolean> {
  try {
    await initWasm();
    return true;
  } catch {
    return false;
  }
}

/**
 * Get local conversion file size limit
 */
export function getLocalConversionSizeLimit(): number {
  return getFileSizeLimit();
}

/**
 * Analyze a ZIP file and detect tex files inside
 */
export async function analyzeZip(zipBytes: Uint8Array): Promise<ZipAnalysis> {
  const texFiles: TexFileInfo[] = [];
  let hasDocumentClass = false;
  let detectedMainTex: string | null = null;

  try {
    // Parse ZIP using JSZip-like manual parsing
    // ZIP format: local file headers start with 0x04034b50 (PK\x03\x04)
    let offset = 0;
    const bytes = zipBytes;

    while (offset < bytes.length - 4) {
      // Check for local file header signature
      if (bytes[offset] === 0x50 && bytes[offset + 1] === 0x4b &&
          bytes[offset + 2] === 0x03 && bytes[offset + 3] === 0x04) {

        // Parse local file header
        const headerSize = 30; // minimum header size
        if (offset + headerSize > bytes.length) break;

        // Skip: version needed (2), general purpose flag (2), compression method (2)
        // compression method is at offset + 8
        const compressionMethod = bytes[offset + 8] | (bytes[offset + 9] << 8);

        // Skip: mod time (2), mod date (2), CRC32 (4), compressed size (4), uncompressed size (4)
        // Uncompressed size is at offset + 18 (4 bytes)
        const compressedSize = bytes[offset + 18] | (bytes[offset + 19] << 8) |
                             (bytes[offset + 20] << 16) | (bytes[offset + 21] << 24);
        const uncompressedSize = bytes[offset + 22] | (bytes[offset + 23] << 8) |
                                (bytes[offset + 24] << 16) | (bytes[offset + 25] << 24);

        // File name length at offset + 26 (2 bytes)
        const fileNameLen = bytes[offset + 26] | (bytes[offset + 27] << 8);

        // Extra field length at offset + 28 (2 bytes)
        const extraFieldLen = bytes[offset + 28] | (bytes[offset + 29] << 8);

        // File name starts at offset + 30
        const nameStart = offset + 30;
        const nameEnd = nameStart + fileNameLen;

        if (nameEnd > bytes.length) break;

        // Decode filename (UTF-8)
        let fileName = '';
        for (let i = nameStart; i < nameEnd; i++) {
          fileName += String.fromCharCode(bytes[i]);
        }

        // Check if it's a tex file
        if (fileName.toLowerCase().endsWith('.tex') && !fileName.endsWith('/')) {
          // Try to read the file content to check for \documentclass
          let content = '';
          if (compressionMethod === 0 && uncompressedSize > 0 && uncompressedSize < 1024 * 1024) {
            // Stored (no compression)
            const dataStart = nameEnd + extraFieldLen;
            const dataEnd = dataStart + uncompressedSize;
            if (dataEnd <= bytes.length) {
              const decoder = new TextDecoder('utf-8', { fatal: false });
              content = decoder.decode(bytes.slice(dataStart, dataEnd));

              // Check for documentclass
              if (content.includes('\\documentclass') || content.includes('\\documentstyle')) {
                hasDocumentClass = true;
              }
            }
          }

          texFiles.push({
            path: fileName.replace(/\\/g, '/'), // Normalize path separators
            name: fileName.split('/').pop() || fileName,
            size: uncompressedSize || compressedSize,
          });
        }

        // Move to next entry
        const dataStart = nameEnd + extraFieldLen;
        const dataEnd = dataStart + (compressionMethod === 0 ? uncompressedSize : compressedSize);
        offset = dataEnd;

        // Handle data descriptor (if bit 3 of flags is set)
        // For simplicity, we skip this edge case

      } else if (bytes[offset] === 0x50 && bytes[offset + 1] === 0x4b &&
                 bytes[offset + 2] === 0x01 && bytes[offset + 3] === 0x02) {
        // Central directory header - stop parsing
        break;
      } else {
        offset++;
      }
    }
  } catch (error) {
    console.error('[Tex2Doc] ZIP parsing error:', error);
  }

  // Detect main tex file
  if (texFiles.length > 0) {
    detectedMainTex = detectMainTex(texFiles);
  }

  return {
    isValid: texFiles.length > 0,
    texFiles,
    detectedMainTex,
    hasDocumentClass,
  };
}

/**
 * Detect the main tex file from a list of tex files
 * Priority:
 * 1. main.tex or main-page.tex (exact match)
 * 2. File with \documentclass (not in subdirectory)
 * 3. Shortest path file
 * 4. First alphabetically
 */
function detectMainTex(texFiles: TexFileInfo[]): string {
  // Priority 1: Check for common main file names
  const commonNames = ['main.tex', 'main-page.tex', 'paper.tex', 'article.tex', 'thesis.tex'];

  for (const name of commonNames) {
    const exact = texFiles.find(f => f.name.toLowerCase() === name.toLowerCase());
    if (exact) return exact.path;
  }

  // Priority 2: Check for file with \documentclass in root or shallow directories
  // We'll filter tex files that are in root or one level deep
  const shallowFiles = texFiles.filter(f => {
    const depth = f.path.split('/').length - 1;
    return depth <= 1;
  });

  if (shallowFiles.length > 0) {
    // Sort by name length (shorter = more likely to be main)
    shallowFiles.sort((a, b) => a.name.length - b.name.length);
    return shallowFiles[0].path;
  }

  // Priority 3: Return first tex file alphabetically
  texFiles.sort((a, b) => a.path.localeCompare(b.path));
  return texFiles[0].path;
}

/**
 * Format file size for display
 */
export function formatFileSizeForDisplay(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * Validate ZIP file from File object
 */
export async function validateZipFile(file: File): Promise<{ valid: boolean; error?: string }> {
  // Check file extension
  if (!file.name.toLowerCase().endsWith('.zip')) {
    return { valid: false, error: 'Please select a ZIP file' };
  }

  // Check file size
  if (!isWithinSizeLimit(file.size)) {
    return {
      valid: false,
      error: `File too large. Maximum size is ${getFileSizeLimitDisplay()}.`,
    };
  }

  // Check magic bytes
  return new Promise((resolve) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      const bytes = new Uint8Array(e.target?.result as ArrayBuffer);
      if (bytes.length >= 2 && bytes[0] === 0x50 && bytes[1] === 0x4b) {
        resolve({ valid: true });
      } else {
        resolve({ valid: false, error: 'Invalid ZIP file' });
      }
    };
    reader.onerror = () => resolve({ valid: false, error: 'Failed to read file' });
    reader.readAsArrayBuffer(file.slice(0, 4));
  });
}
