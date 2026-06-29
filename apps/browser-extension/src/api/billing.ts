/**
 * Billing API module
 *
 * Handles plans, checkout, and billing portal
 */

import { ApiClient } from './api-client';
import type { PlanSummary } from '@/shared/types';
import { openUrl } from '@/browser/compat';

export interface CheckoutOptions {
  planId: string;
  successUrl?: string;
  cancelUrl?: string;
}

export interface PortalOptions {
  returnUrl?: string;
}

/**
 * Get available plans
 */
export async function getPlans(client: ApiClient): Promise<PlanSummary[]> {
  return client.plans();
}

/**
 * Create checkout session and open it
 */
export async function startCheckout(client: ApiClient, options: CheckoutOptions): Promise<void> {
  const session = await client.createCheckout({
    plan_id: options.planId,
    success_url: options.successUrl ?? window.location.origin + '/billing/success',
    cancel_url: options.cancelUrl ?? window.location.origin + '/billing/cancel',
  });

  if (session.url) {
    await openUrl(session.url);
  }
}

/**
 * Create billing portal session and open it
 */
export async function openBillingPortal(client: ApiClient, options?: PortalOptions): Promise<void> {
  const session = await client.createBillingPortal({
    return_url: options?.returnUrl ?? window.location.origin + '/account',
  });

  if (session.url) {
    await openUrl(session.url);
  }
}

/**
 * Format price for display
 */
export function formatPrice(cents: number, currency: string): string {
  const amount = cents / 100;
  const symbols: Record<string, string> = {
    USD: '$',
    CNY: '¥',
    EUR: '€',
    GBP: '£',
  };
  const symbol = symbols[currency] ?? currency + ' ';
  return `${symbol}${amount.toFixed(2)}`;
}

/**
 * Get plan features as list
 */
export function getPlanFeatures(plan: PlanSummary): string[] {
  const features: string[] = [];

  if (plan.monthly_conversions > 0) {
    features.push(`${plan.monthly_conversions} conversions/month`);
  }

  if (plan.features) {
    features.push(...plan.features);
  }

  return features;
}
