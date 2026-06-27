/**
 * Downloads utilities
 */

export interface DownloadOptions {
  url: string;
  filename?: string;
  conflictAction?: 'overwrite' | 'prompt' | 'uniquify';
  saveAs?: boolean;
}

export interface DownloadResult {
  id: number;
  filename?: string;
  url?: string;
}

/**
 * Download a file
 */
export async function downloadFile(options: DownloadOptions): Promise<DownloadResult> {
  const { url, filename, conflictAction = 'uniquify', saveAs = true } = options;

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const downloadOptions: any = { url, conflictAction, saveAs };
  if (filename) downloadOptions.filename = filename;

  const id = await browser.downloads.download(downloadOptions);
  return { id };
}

/**
 * Download bytes as a file
 */
export async function downloadBytes(
  data: Uint8Array,
  filename: string,
  mimeType = 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
): Promise<DownloadResult> {
  const blob = new Blob([new Uint8Array(data)], { type: mimeType });
  const url = URL.createObjectURL(blob);

  try {
    return await downloadFile({ url, filename });
  } finally {
    setTimeout(() => URL.revokeObjectURL(url), 60000);
  }
}

/**
 * Download a blob as a file
 */
export async function downloadBlob(blob: Blob, filename: string): Promise<DownloadResult> {
  const url = URL.createObjectURL(blob);

  try {
    return await downloadFile({ url, filename });
  } finally {
    setTimeout(() => URL.revokeObjectURL(url), 60000);
  }
}

/**
 * Get download by ID
 */
export async function getDownload(id: number) {
  const downloads = await browser.downloads.search({ id });
  return downloads[0] || null;
}

/**
 * Cancel a download
 */
export async function cancelDownload(id: number): Promise<void> {
  await browser.downloads.cancel(id);
}

/**
 * Search downloads
 */
export async function searchDownloads(query: {
  id?: number;
  state?: string;
}): Promise<browser.Downloads.DownloadItem[]> {
  return browser.downloads.search(query);
}

/**
 * Get download filename from blob
 */
export function getFilenameFromBlob(_blob: Blob, extension: string): string {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
  return `Tex2Doc_${timestamp}.${extension}`;
}

/**
 * Is download active
 */
export function isDownloadActive(state: string): boolean {
  return state === 'in_progress' || state === 'interrupted';
}

/**
 * Is download complete
 */
export function isDownloadComplete(state: string): boolean {
  return state === 'complete';
}
