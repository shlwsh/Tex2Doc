/**
 * Cloud Conversion Module
 *
 * Handles cloud conversion via API
 */

import { ApiClient } from '@/api/api-client';
import { createAndPollConversion } from '@/api/conversions';
import { uploadProjectZip } from '@/api/uploads';
import { downloadBytes } from '@/browser/downloads';
import { saveJob, updateJobStatus } from '@/state/job-store';
import type { JobRecord } from '@/shared/types';
import { QuotaExceededError, ConversionError } from '@/shared/errors';

export interface CloudConversionOptions {
  fileName: string;
  mainTex: string;
  profile: string;
  quality: string;
}

export interface CloudConversionResult {
  jobId: string;
  cloudJobId: string;
  report?: unknown;
  docxFilename?: string;
}

/**
 * Convert file via cloud API
 */
export async function convertCloud(
  zipBytes: Uint8Array,
  options: CloudConversionOptions,
  client: ApiClient,
  onProgress?: (progress: number, status: string) => void
): Promise<CloudConversionResult> {
  // Create job record
  const localJobId = crypto.randomUUID();
  const job: JobRecord = {
    id: localJobId,
    file_name: options.fileName,
    main_tex: options.mainTex,
    profile: options.profile,
    quality: options.quality,
    mode: 'cloud',
    status: 'pending',
    progress: 0,
    created_at: Date.now(),
    updated_at: Date.now(),
  };

  await saveJob(job);

  try {
    onProgress?.(5, 'Uploading file...');
    await updateJobStatus(localJobId, 'processing', 5);

    // Upload file
    const upload = await uploadProjectZip(client, zipBytes, options.fileName, (progress) => {
      onProgress?.(5 + progress.percentage * 0.25, `Uploading: ${progress.percentage}%`);
    });

    onProgress?.(30, 'Creating conversion job...');

    // Create and poll conversion
    const cloudJob = await createAndPollConversion(
      client,
      upload.upload_id,
      options.mainTex,
      options.profile,
      options.quality,
      (job) => {
        const progress =
          job.status === 'completed' ? 100 : job.status === 'processing' ? 50 : 30;
        onProgress?.(progress, `Status: ${job.status}`);
        updateJobStatus(localJobId, job.status as JobRecord['status'], progress);
      }
    );

    // Update job with cloud job ID
    await saveJob({
      ...job,
      job_id: cloudJob.job_id,
      status: 'completed',
      progress: 100,
      updated_at: Date.now(),
      docx_ready: cloudJob.docx_ready,
    });

    // Download DOCX
    let docxFilename: string | undefined;
    if (cloudJob.docx_ready) {
      onProgress?.(95, 'Downloading DOCX...');
      const docxBytes = await client.downloadConversionDocx(cloudJob.job_id);
      docxFilename = options.fileName.replace(/\.[^.]+$/, '') + '.docx';
      await downloadBytes(docxBytes, docxFilename);
    }

    onProgress?.(100, 'Complete');

    return {
      jobId: localJobId,
      cloudJobId: cloudJob.job_id,
      docxFilename,
    };
  } catch (error) {
    if (error instanceof QuotaExceededError) {
      await updateJobStatus(localJobId, 'failed');
      throw new ConversionError(
        `Quota exceeded: ${error.used}/${error.limit} conversions used`,
        'QUOTA_EXCEEDED',
        localJobId
      );
    }

    await updateJobStatus(localJobId, 'failed');
    throw error;
  }
}

/**
 * Determine conversion mode based on file size
 */
export function determineConversionMode(
  fileSize: number,
  preferredMode: 'auto' | 'local' | 'cloud',
  wasmLimit: number
): 'local' | 'cloud' {
  if (preferredMode === 'local') return 'local';
  if (preferredMode === 'cloud') return 'cloud';

  // Auto mode: use local for small files, cloud for large files
  if (fileSize <= wasmLimit) {
    return 'local';
  }
  return 'cloud';
}

/**
 * Get recommended mode for file
 */
export function getRecommendedMode(fileSize: number, wasmLimit: number): 'local' | 'cloud' {
  return fileSize <= wasmLimit ? 'local' : 'cloud';
}
