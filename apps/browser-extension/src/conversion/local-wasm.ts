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
