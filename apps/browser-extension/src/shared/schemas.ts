import { z } from 'zod';

// Auth Schemas
export const LoginSchema = z.object({
  email: z.string().email('Invalid email address'),
  password: z.string().min(6, 'Password must be at least 6 characters'),
});

export const RegisterSchema = z.object({
  email: z.string().email('Invalid email address'),
  password: z.string().min(6, 'Password must be at least 6 characters'),
  display_name: z.string().min(1, 'Display name is required').optional(),
});

// Conversion Schemas
export const CreateConversionSchema = z.object({
  upload_id: z.string().min(1, 'Upload ID is required'),
  main_tex: z.string().min(1, 'Main TeX file is required'),
  profile: z.enum(['standard', 'academic', 'publication']).default('standard'),
  quality: z.enum(['preview', 'balanced', 'strict']).default('balanced'),
});

export const WasmConversionSchema = z.object({
  file_name: z.string().min(1, 'File name is required'),
  main_tex: z.string().min(1, 'Main TeX file is required'),
  options: z
    .object({
      bib_style: z.enum(['numeric', 'author-year']).optional(),
    })
    .optional(),
});

// Feedback Schemas
export const CreateFeedbackSchema = z.object({
  title: z.string().min(1, 'Title is required').max(200, 'Title is too long'),
  feedback_type: z.enum(['issue', 'requirement', 'other']).default('issue'),
  content: z.string().min(10, 'Please provide more details').max(5000),
  conversion_job_id: z.string().optional(),
  priority: z.enum(['low', 'normal', 'high', 'urgent']).default('normal'),
});

// Settings Schemas
export const SettingsSchema = z.object({
  api_base_url: z.string().url('Invalid API URL'),
  default_profile: z.enum(['standard', 'academic', 'publication']).default('standard'),
  default_quality: z.enum(['preview', 'balanced', 'strict']).default('balanced'),
  default_mode: z.enum(['auto', 'local', 'cloud']).default('auto'),
  wasm_file_size_limit: z.number().min(1024).max(50 * 1024 * 1024).default(10 * 1024 * 1024),
  language: z.enum(['en', 'zh']).default('en'),
  theme: z.enum(['light', 'dark', 'system']).default('system'),
  polling_interval: z.number().min(1000).max(30000).default(2000),
});

// Redeem Code Schema
export const RedeemCodeSchema = z.object({
  code: z.string().min(1, 'Code is required').regex(/^[A-Z0-9-]+$/, 'Invalid code format'),
});

// Type exports
export type LoginInput = z.infer<typeof LoginSchema>;
export type RegisterInput = z.infer<typeof RegisterSchema>;
export type CreateConversionInput = z.infer<typeof CreateConversionSchema>;
export type WasmConversionInput = z.infer<typeof WasmConversionSchema>;
export type CreateFeedbackInput = z.infer<typeof CreateFeedbackSchema>;
export type SettingsInput = z.infer<typeof SettingsSchema>;
export type RedeemCodeInput = z.infer<typeof RedeemCodeSchema>;
