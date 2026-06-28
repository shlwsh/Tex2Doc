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
 *
 * 在 service worker（没有 window / URL.createObjectURL）里也能用。
 * 通过 `URL.createObjectURL` 在普通页面里创建 blob URL，或者在 service worker
 * 里 fallback 到 `data:` URL（base64 编码）。`chrome.downloads.download` 同时
 * 接受这两种 URL 形式。
 */
export async function downloadBytes(
  data: Uint8Array,
  filename: string,
  mimeType = 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
): Promise<DownloadResult> {
  const url = createBlobUrl(data, mimeType);
  try {
    return await downloadFile({ url, filename });
  } finally {
    setTimeout(() => revokeBlobUrl(url), 60_000);
  }
}

function createBlobUrl(data: Uint8Array, mimeType: string): string {
  // 1) 优先用 URL.createObjectURL（popup / options / sidepanel 等 DOM 上下文）
  if (typeof URL !== 'undefined' && typeof URL.createObjectURL === 'function') {
    try {
      const blob = new Blob([new Uint8Array(data)], { type: mimeType });
      return URL.createObjectURL(blob);
    } catch {
      // 一些受限上下文里 Blob 也不可用，继续走 data URL
    }
  }

  // 2) Fallback：base64 data URL（service worker / headless 场景）
  //    `chrome.downloads.download` 接受 `data:` URL，最长 1 GiB，对 docx 绰绰有余。
  const bytes = new Uint8Array(data);
  let binary = '';
  const chunkSize = 0x8000;
  for (let i = 0; i < bytes.length; i += chunkSize) {
    const chunk = bytes.subarray(i, i + chunkSize);
    binary += String.fromCharCode.apply(null, Array.from(chunk));
  }
  const base64 = btoa(binary);
  return `data:${mimeType};base64,${base64}`;
}

function revokeBlobUrl(url: string): void {
  if (url.startsWith('blob:') && typeof URL !== 'undefined' && typeof URL.revokeObjectURL === 'function') {
    try {
      URL.revokeObjectURL(url);
    } catch {
      // 忽略 revoke 错误
    }
  }
  // data: URL 不需要 revoke
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
