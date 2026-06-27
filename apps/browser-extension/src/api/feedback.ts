/**
 * Feedback API module
 */

import { ApiClient } from './api-client';
import type { FeedbackThread, CreateFeedbackRequest, FeedbackType } from '@/shared/types';
import { RedeemCodeSchema } from '@/shared/schemas';
import { RedeemCodeResult } from '@/shared/types';
import { ApiError } from '@/shared/errors';

/**
 * Get all feedback threads
 */
export async function getFeedbackThreads(client: ApiClient): Promise<FeedbackThread[]> {
  return (await client.feedbackThreads()) as FeedbackThread[];
}

/**
 * Create a new feedback thread
 */
export async function createFeedbackThread(
  client: ApiClient,
  request: CreateFeedbackRequest
): Promise<FeedbackThread> {
  return (await client.createFeedbackThread(request)) as FeedbackThread;
}

/**
 * Submit feedback with validation
 */
export async function submitFeedback(
  client: ApiClient,
  options: {
    title: string;
    feedbackType: FeedbackType;
    content: string;
    conversionJobId?: string;
    priority?: 'low' | 'normal' | 'high' | 'urgent';
  }
): Promise<FeedbackThread> {
  const request: CreateFeedbackRequest = {
    title: options.title.trim(),
    feedback_type: options.feedbackType,
    content: options.content.trim(),
    conversion_job_id: options.conversionJobId,
    priority: options.priority ?? 'normal',
  };

  return createFeedbackThread(client, request);
}

/**
 * Redeem a code
 */
export async function redeemCode(client: ApiClient, code: string): Promise<RedeemCodeResult> {
  const parsed = RedeemCodeSchema.safeParse({ code: code.trim().toUpperCase() });
  if (!parsed.success) {
    throw new ApiError('Invalid code format', 'INVALID_CODE', 400);
  }

  return client.redeemCode({ code: parsed.data.code });
}

/**
 * Get feedback type display text
 */
export function getFeedbackTypeDisplay(type: FeedbackType): string {
  const typeMap: Record<FeedbackType, string> = {
    issue: 'Issue',
    requirement: 'Feature Request',
    other: 'Other',
  };
  return typeMap[type] ?? type;
}

/**
 * Get feedback priority display text
 */
export function getPriorityDisplay(priority: string): string {
  const priorityMap: Record<string, string> = {
    low: 'Low',
    normal: 'Normal',
    high: 'High',
    urgent: 'Urgent',
  };
  return priorityMap[priority] ?? priority;
}

/**
 * Get feedback status display text
 */
export function getStatusDisplay(status: string): string {
  const statusMap: Record<string, string> = {
    open: 'Open',
    in_progress: 'In Progress',
    resolved: 'Resolved',
    closed: 'Closed',
  };
  return statusMap[status] ?? status;
}

/**
 * Format feedback thread for display
 */
export function formatThreadSummary(thread: FeedbackThread): string {
  const parts: string[] = [];
  parts.push(`[${getStatusDisplay(thread.status)}]`);
  parts.push(thread.title);
  if (thread.message_count && thread.message_count > 0) {
    parts.push(`(${thread.message_count} messages)`);
  }
  return parts.join(' ');
}
