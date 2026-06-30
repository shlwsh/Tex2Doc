export type JsonRecord = Record<string, unknown>;

export interface UserProfile {
  user_id?: string;
  id?: string;
  email: string;
  role?: string;
  display_name?: string | null;
  plan_id?: string | null;
}

export interface AuthResponse {
  access_token: string;
  refresh_token?: string;
  user: UserProfile;
  is_new_account?: boolean;
}

export interface AdminProfile {
  user: UserProfile;
  permissions?: string[];
}

export interface UsageSummary {
  plan_id?: string | null;
  cloud_conversions_used?: number;
  cloud_conversions_limit?: number;
  count_balance?: number;
  date_valid_until?: string | null;
  [key: string]: unknown;
}

export interface PlanSummary {
  package_id?: string;
  id?: string;
  name?: string;
  quantity?: number;
  price_cents?: number;
  [key: string]: unknown;
}

export interface RechargeOptions {
  purchase_url?: string;
  plans?: PlanSummary[];
  packages?: PlanSummary[];
  [key: string]: unknown;
}

export interface RedeemCodeOptions {
  packages?: PlanSummary[];
  [key: string]: unknown;
}

export interface RedeemCodeResult {
  plan_id?: string;
  package_id?: string;
  quantity?: number;
  count_balance?: number;
  access_token?: string;
  refresh_token?: string;
  user?: UserProfile;
  [key: string]: unknown;
}

export interface RedeemCodeRecord {
  id?: string;
  code_preview?: string;
  package_id?: string;
  quantity?: number;
  created_at?: string;
  redeemed_at?: string;
  [key: string]: unknown;
}

export interface RechargeRecord {
  recharge_id?: string;
  id?: string;
  recharge_type?: string;
  package_id?: string;
  quantity?: number;
  amount_cents?: number;
  currency?: string;
  status?: string;
  provider?: string;
  created_at?: string;
  [key: string]: unknown;
}

export interface UploadResponse {
  upload_id: string;
  filename?: string;
  size?: number;
}

export type ConversionStatus = 'queued' | 'running' | 'completed' | 'failed' | 'expired' | string;

export interface ConversionJob {
  id?: string;
  job_id?: string;
  upload_id?: string;
  main_tex?: string;
  profile?: string;
  quality?: string;
  status: ConversionStatus;
  error_code?: string | null;
  error_message?: string | null;
  created_at?: string;
  updated_at?: string;
  [key: string]: unknown;
}

export interface LocalQuotaCheckResponse {
  allowed: boolean;
  reason?: string;
  remaining?: number;
  count_balance?: number;
  valid_until_active?: boolean;
  [key: string]: unknown;
}

export interface LocalQuotaConsumeResponse {
  consumed: boolean;
  remaining?: number;
  [key: string]: unknown;
}

export interface FeedbackThread {
  id: string;
  thread_id?: string;
  title: string;
  feedback_type: string;
  priority?: string;
  status?: string;
  message_count?: number;
  conversion_job_id?: string | null;
  automation_status?: string | null;
  automation_request_id?: string | null;
  created_at?: string;
  updated_at?: string;
}

export interface FeedbackMessage {
  id?: string;
  author_role?: string;
  content: string;
  is_internal?: boolean;
  created_at?: string;
}

export interface FeedbackThreadDetail extends FeedbackThread {
  messages?: FeedbackMessage[];
}

export interface CreateFeedbackResponse {
  thread?: FeedbackThread;
  message?: FeedbackMessage;
  id?: string;
  [key: string]: unknown;
}

export interface AdminDashboardSummary {
  counts?: Record<string, number>;
  modules?: string[];
  release_channels?: string[];
  updated_at?: string;
  [key: string]: unknown;
}

export interface RedeemCodeBatch {
  id: string;
  batch_id?: string;
  batch_no?: string;
  package_id?: string;
  package_name?: string;
  recharge_type?: string;
  quantity?: number;
  generated_count?: number;
  status?: string;
  channel?: string;
  note?: string | null;
  exported_count?: number;
  expires_at?: string | null;
  codes?: Array<string | { id?: string; code_id?: string; code?: string; code_preview?: string }>;
  created_at?: string;
}

export interface AdminRedeemCode {
  id: string;
  code_id?: string;
  batch_id?: string;
  batch_no?: string;
  code_preview?: string;
  package_id?: string;
  package_name?: string;
  recharge_type?: string;
  quantity?: number;
  status?: string;
  stock_status?: string;
  stocked_by?: string | null;
  stocked_at?: string | null;
  redeemed_by?: string | null;
  redeemed_recharge_id?: string | null;
  redeemed_at?: string | null;
  restocked_by?: string | null;
  restocked_at?: string | null;
  expires_at?: string | null;
  created_at?: string;
}

export interface AdminRedeemCodeListResult {
  items?: AdminRedeemCode[];
  codes?: AdminRedeemCode[];
  records?: AdminRedeemCode[];
  total?: number;
  page?: number;
  page_size?: number;
}

export interface ReleaseManifest {
  id?: string;
  channel?: string;
  platform?: string;
  arch?: string;
  version?: string;
  release_title?: string;
  download_url?: string;
  sha256?: string;
  is_prerelease?: boolean;
  rolled_back_at?: string | null;
  active?: boolean;
  created_at?: string;
  published_at?: string;
}

export interface AutomationRequest {
  id: string;
  short_id?: string;
  title?: string;
  status?: string;
  risk_level?: string;
  request_type?: string;
  source_type?: string;
  assigned_agent_id?: string | null;
  branch_name?: string | null;
  pr_url?: string | null;
  ai_summary?: string | null;
  updated_at?: string;
  priority?: number;
}

export interface AutomationAgent {
  id: string;
  status?: string;
  hostname?: string;
  agent_version?: string;
  last_heartbeat_at?: string;
  completed_count?: number;
  failed_count?: number;
  current_request_id?: string | null;
  capabilities?: string[];
}

export interface AutomationEvent {
  id?: string;
  event_type?: string;
  message?: string;
  created_at?: string;
}
