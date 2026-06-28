// User & Auth Types
export interface UserProfile {
  id: string;
  email: string;
  display_name: string | null;
  plan_id: string;
}

export interface AuthResponse {
  access_token: string;
  refresh_token: string;
  user: UserProfile;
}

export interface UsageSummary {
  plan_id: string;
  cloud_conversions_used: number;
  cloud_conversions_limit: number;
  count_balance: number;
  date_valid_until: string | null;
  storage_bytes_used: number;
  storage_bytes_limit: number;
  period_start: string;
  period_end: string;
}

export interface Session {
  access_token: string;
  refresh_token: string;
  user: UserProfile;
  usage: UsageSummary | null;
  expires_at: number;
}

// Plans & Billing
export interface PlanSummary {
  id: string;
  name: string;
  price_cents: number;
  currency: string;
  monthly_conversions: number;
  features: string[];
}

export interface BillingSession {
  url: string;
  expires_at: string;
}

// Redeem Codes
export interface RedeemCodeResult {
  redeem_id: string;
  recharge_id: string;
  package_id: string;
  package_name: string;
  recharge_type: string;
  quantity: number;
  count_balance: number;
  date_valid_until: string | null;
  redeemed_at: string;
  // Auto-account fields: present when the redeem code also provisions an account
  is_new_account?: boolean;
  access_token?: string;
  refresh_token?: string;
  user?: UserProfile;
}

export interface RedeemCodeRequest {
  code: string;
}

// Conversions
export type JobStatus = 'pending' | 'processing' | 'completed' | 'failed' | 'expired';

export interface ConversionJob {
  job_id: string;
  upload_id: string;
  main_tex: string | null;
  profile: string | null;
  quality: string | null;
  engine: string | null;
  status: JobStatus;
  created_at: string;
  updated_at: string;
  docx_ready: boolean;
  report_ready: boolean;
  error_code: string | null;
  error: string | null;
}

export interface ConversionReport {
  job_id: string;
  main_tex: string;
  profile: string;
  quality_score: number;
  backend: string | null;
  docx_bytes: number | null;
  warnings: string[];
  errors: string[];
  quality_gate: QualityGate | null;
  active_profile: ActiveProfile | null;
}

export interface QualityGate {
  status: 'passed' | 'warning' | 'failed';
  score: number;
  passed_checks: string[];
  failed_checks: string[];
  warnings: string[];
}

export interface ActiveProfile {
  id: string;
  name: string;
  description: string;
}

// Feedback
export type FeedbackType = 'issue' | 'requirement' | 'other';

export interface FeedbackThread {
  thread_id: string;
  title: string;
  feedback_type: FeedbackType;
  status: 'open' | 'in_progress' | 'resolved' | 'closed';
  priority: 'low' | 'normal' | 'high' | 'urgent';
  message_count: number | null;
  latest_message_at: string | null;
  created_at: string;
  updated_at: string | null;
  conversion_job_id: string | null;
  automation_status: string | null;
  automation_request_id: string | null;
}

export interface CreateFeedbackRequest {
  title: string;
  feedback_type: FeedbackType;
  content: string;
  conversion_job_id?: string;
  priority?: 'low' | 'normal' | 'high' | 'urgent';
}

// Storage Types
export interface StoredSession {
  refresh_token: string;
  user: UserProfile;
  usage: UsageSummary | null;
  stored_at: number;
}

export interface ExtensionSettings {
  api_base_url: string;
  default_profile: string;
  default_quality: string;
  default_mode: 'auto' | 'local' | 'cloud';
  wasm_file_size_limit: number;
  language: 'en' | 'zh';
  theme: 'light' | 'dark' | 'system';
  polling_interval: number;
}

// Conversion Types
export interface JobRecord {
  id: string;
  job_id?: string;
  file_name: string;
  main_tex: string;
  profile: string;
  quality: string;
  mode: 'local' | 'cloud';
  status: JobStatus;
  progress: number;
  created_at: number;
  updated_at: number;
  error_code?: string;
  error_message?: string;
  report?: ConversionReport;
  docx_ready?: boolean;
}

// Event Log
export interface EventLogEntry {
  id: string;
  timestamp: number;
  type: 'info' | 'warning' | 'error';
  message: string;
  details?: Record<string, unknown>;
  job_id?: string;
}
