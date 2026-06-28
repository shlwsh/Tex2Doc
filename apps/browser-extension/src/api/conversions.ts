/**
 * Conversions API module
 */

import { ApiClient } from './api-client';
import type { ConversionJob, ConversionReport, JobStatus } from '@/shared/types';
import { ConversionError } from '@/shared/errors';
import { downloadBytes } from '@/browser/downloads';

export interface ConversionResult {
  job: ConversionJob;
  report?: ConversionReport;
  docxBytes?: Uint8Array;
  docxFilename?: string;
}

/**
 * Create and poll a conversion job
 */
export async function createAndPollConversion(
  client: ApiClient,
  uploadId: string,
  mainTex: string,
  profile: string,
  quality: string,
  onProgress?: (job: ConversionJob) => void,
  options: {
    pollInterval?: number;
    maxPolls?: number;
  } = {}
): Promise<ConversionJob> {
  const { pollInterval = 2000, maxPolls = 120 } = options;

  const job = await client.createConversion({
    upload_id: uploadId,
    main_tex: mainTex,
    profile,
    quality,
  }) as ConversionJob;

  if (onProgress) {
    onProgress(job);
  }

  for (let i = 0; i < maxPolls; i++) {
    const currentJob = await client.getConversion(job.job_id) as ConversionJob;

    if (onProgress) {
      onProgress(currentJob);
    }

    if (currentJob.status === 'completed' && currentJob.docx_ready) {
      return currentJob;
    }

    if (currentJob.status === 'failed' || currentJob.status === 'expired') {
      throw new ConversionError(
        currentJob.error || 'Conversion failed',
        currentJob.error_code || 'CONVERSION_FAILED',
        currentJob.job_id
      );
    }

    await new Promise((resolve) => setTimeout(resolve, pollInterval));
  }

  throw new ConversionError('Conversion timed out', 'TIMEOUT', job.job_id);
}

/**
 * Poll an existing cloud job until it reaches a terminal state.
 *
 * Used by SW recovery: after the SW restarts, the original upload/create steps
 * already happened server-side, so we just need to keep polling until done.
 *
 * Unlike `createAndPollConversion`, this assumes the cloud job already exists
 * and only retries the GET endpoint. It also tolerates `404 / expired` by
 * throwing a non-retryable `ConversionError` so the caller can mark the job failed.
 */
export async function pollCloudConversion(
  client: ApiClient,
  cloudJobId: string,
  onProgress?: (job: ConversionJob) => void,
  options: {
    pollInterval?: number;
    maxPolls?: number;
  } = {}
): Promise<ConversionJob> {
  const { pollInterval = 2000, maxPolls = 120 } = options;

  for (let i = 0; i < maxPolls; i++) {
    let currentJob: ConversionJob;
    try {
      currentJob = await client.getConversion(cloudJobId) as ConversionJob;
    } catch (error) {
      if (error instanceof ConversionError && error.code === 'NOT_FOUND') {
        throw new ConversionError(
          `Cloud job ${cloudJobId} not found after SW restart; original upload may have expired`,
          'JOB_NOT_FOUND_AFTER_RESTART',
          cloudJobId
        );
      }
      throw error;
    }

    if (onProgress) {
      onProgress(currentJob);
    }

    if (currentJob.status === 'completed' && currentJob.docx_ready) {
      return currentJob;
    }

    if (currentJob.status === 'failed' || currentJob.status === 'expired') {
      throw new ConversionError(
        currentJob.error || 'Conversion failed',
        currentJob.error_code || 'CONVERSION_FAILED',
        currentJob.job_id
      );
    }

    await new Promise((resolve) => setTimeout(resolve, pollInterval));
  }

  throw new ConversionError('Conversion timed out after SW restart', 'TIMEOUT', cloudJobId);
}

/**
 * Get all conversion jobs
 */
export async function getConversions(client: ApiClient): Promise<ConversionJob[]> {
  return (await client.conversions()) as ConversionJob[];
}

/**
 * Get a single conversion job
 */
export async function getConversion(client: ApiClient, jobId: string): Promise<ConversionJob> {
  return (await client.getConversion(jobId)) as ConversionJob;
}

/**
 * Download conversion DOCX
 */
export async function downloadConversionDocx(
  client: ApiClient,
  jobId: string,
  filename?: string
): Promise<Uint8Array> {
  const docxBytes = await client.downloadConversionDocx(jobId);
  await downloadBytes(docxBytes, filename ?? `conversion_${jobId}.docx`);
  return docxBytes;
}

/**
 * Get conversion report
 */
export async function getConversionReport(
  client: ApiClient,
  jobId: string
): Promise<ConversionReport> {
  return (await client.getConversionReport(jobId)) as ConversionReport;
}

/**
 * Get full conversion result including report
 */
export async function getFullConversionResult(
  client: ApiClient,
  jobId: string
): Promise<ConversionResult> {
  const job = (await client.getConversion(jobId)) as ConversionJob;
  let report: ConversionReport | undefined;
  let docxBytes: Uint8Array | undefined;

  if (job.report_ready) {
    try {
      report = await getConversionReport(client, jobId);
    } catch {
      // Ignore
    }
  }

  if (job.docx_ready) {
    try {
      docxBytes = await client.downloadConversionDocx(jobId);
    } catch {
      // Ignore
    }
  }

  return {
    job,
    report,
    docxBytes,
    docxFilename: docxBytes ? `conversion_${jobId}.docx` : undefined,
  };
}

/**
 * Get status display text
 */
export function getStatusDisplay(status: JobStatus): string {
  const statusMap: Record<JobStatus, string> = {
    pending: 'Pending',
    processing: 'Processing',
    completed: 'Completed',
    failed: 'Failed',
    expired: 'Expired',
  };
  return statusMap[status] ?? status;
}

/**
 * Is job in terminal state
 */
export function isJobTerminal(status: JobStatus): boolean {
  return status === 'completed' || status === 'failed' || status === 'expired';
}

/**
 * Format job for display
 */
export function formatJobSummary(job: ConversionJob): string {
  const parts: string[] = [];
  parts.push(`Job: ${job.job_id.slice(0, 8)}...`);
  parts.push(`Status: ${getStatusDisplay(job.status)}`);
  if (job.main_tex) parts.push(`Main: ${job.main_tex}`);
  if (job.profile) parts.push(`Profile: ${job.profile}`);
  if (job.error) parts.push(`Error: ${job.error}`);
  return parts.join(' | ');
}
