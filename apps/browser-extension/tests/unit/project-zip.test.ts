import { describe, it, expect, beforeEach } from 'vitest';
import { validateZipFile } from '@/conversion/project-zip';

describe('project-zip', () => {
  describe('validateZipFile', () => {
    it('should validate a valid ZIP file', async () => {
      // Create a minimal ZIP file (just headers)
      const zipBytes = new Uint8Array([0x50, 0x4b, 0x03, 0x04]);
      const blob = new Blob([zipBytes]);
      const file = new File([blob], 'test.zip', { type: 'application/zip' });

      const result = await validateZipFile(file);
      expect(result.valid).toBe(true);
    });

    it('should reject non-ZIP files', async () => {
      const textBytes = new Uint8Array([0x74, 0x65, 0x73, 0x74]); // "test"
      const blob = new Blob([textBytes]);
      const file = new File([blob], 'test.txt', { type: 'text/plain' });

      const result = await validateZipFile(file);
      expect(result.valid).toBe(false);
    });
  });
});
