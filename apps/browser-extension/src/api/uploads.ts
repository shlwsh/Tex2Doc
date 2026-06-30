/**
 * Uploads API module
 */

import { ApiClient } from './api-client';
import type { UploadResult } from './api-client';

export interface UploadProgress {
  loaded: number;
  total: number;
  percentage: number;
}

export type UploadProgressCallback = (progress: UploadProgress) => void;

/**
 * Upload a project ZIP file with progress tracking
 */
export async function uploadProjectZip(
  client: ApiClient,
  zipBytes: Uint8Array,
  filename: string,
  onProgress?: UploadProgressCallback
): Promise<UploadResult> {
  if (onProgress) {
    onProgress({
      loaded: 0,
      total: zipBytes.length,
      percentage: 0,
    });

    return new Promise((resolve, reject) => {
      const xhr = new XMLHttpRequest();
      const url = `${client['baseUrl']}/v1/uploads`;

      xhr.open('POST', url);
      xhr.setRequestHeader('Authorization', `Bearer ${client['apiKey']}`);

      xhr.upload.onprogress = (event) => {
        if (event.lengthComputable && onProgress) {
          onProgress({
            loaded: event.loaded,
            total: event.total,
            percentage: Math.round((event.loaded / event.total) * 100),
          });
        }
      };

      xhr.onload = () => {
        if (xhr.status >= 200 && xhr.status < 300) {
          try {
            const result = JSON.parse(xhr.responseText);
            resolve(result);
          } catch {
            reject(new Error('Invalid response'));
          }
        } else {
          reject(new Error(`Upload failed: ${xhr.status}`));
        }
      };

      xhr.onerror = () => reject(new Error('Network error'));

      const blob = new Blob([new Uint8Array(zipBytes)], { type: 'application/zip' });
      const formData = new FormData();
      formData.append('file', blob, filename);

      xhr.send(formData);
    });
  }

  return client.uploadProjectZip(zipBytes, filename);
}

/**
 * Validate file is a valid ZIP
 */
export function validateZipFile(file: File): Promise<boolean> {
  return new Promise((resolve) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      const bytes = new Uint8Array(e.target?.result as ArrayBuffer);
      resolve(bytes.length >= 2 && bytes[0] === 0x50 && bytes[1] === 0x4b);
    };
    reader.onerror = () => resolve(false);
    reader.readAsArrayBuffer(file.slice(0, 4));
  });
}

/**
 * Get file size limit message
 */
export function formatFileSize(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
