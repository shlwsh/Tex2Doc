/**
 * Permissions utilities
 */

import type Browser from 'webextension-polyfill';

export interface PermissionStatus {
  granted: boolean;
  permissions: string[];
  origins: string[];
}

/**
 * Check if a permission is granted
 */
export async function hasPermission(permission: string): Promise<boolean> {
  return browser.permissions.contains({ permissions: [permission] });
}

/**
 * Check if host permissions are granted
 */
export async function hasHostPermission(origin: string): Promise<boolean> {
  return browser.permissions.contains({ origins: [origin] });
}

/**
 * Get current permission status
 */
export async function getPermissionStatus(): Promise<PermissionStatus> {
  const result = await browser.permissions.getAll();
  return {
    granted: true,
    permissions: result.permissions ?? [],
    origins: result.origins ?? [],
  };
}

/**
 * Request optional permissions
 */
export async function requestPermissions(
  permissions?: string[],
  origins?: string[]
): Promise<boolean> {
  if (!permissions?.length && !origins?.length) {
    return true;
  }

  const request: Browser.Permissions.Permissions = {};
  if (permissions?.length) {
    request.permissions = permissions as Browser.Permissions.Permissions['permissions'];
  }
  if (origins?.length) {
    request.origins = origins;
  }

  return browser.permissions.request(request);
}

/**
 * Request optional host permission for a domain
 */
export async function requestDomainPermission(domain: string): Promise<boolean> {
  const origin = domain.startsWith('http') ? domain : `https://${domain}/*`;
  return requestPermissions(undefined, [origin]);
}

/**
 * Remove permissions
 */
export async function removePermissions(
  permissions?: string[],
  origins?: string[]
): Promise<boolean> {
  if (!permissions?.length && !origins?.length) {
    return true;
  }

  const request: Browser.Permissions.Permissions = {};
  if (permissions?.length) {
    request.permissions = permissions as Browser.Permissions.Permissions['permissions'];
  }
  if (origins?.length) {
    request.origins = origins;
  }

  return browser.permissions.remove(request);
}

/**
 * Common permission groups
 */
export const PermissionGroups = {
  overleaf: ['https://*.overleaf.com/*'],
  arxiv: ['https://arxiv.org/*', 'https://*.arxiv.org/*'],
} as const;

/**
 * Request Overleaf permission
 */
export async function requestOverleafPermission(): Promise<boolean> {
  return requestPermissions(undefined, [...PermissionGroups.overleaf]);
}

/**
 * Request arXiv permission
 */
export async function requestArxivPermission(): Promise<boolean> {
  return requestPermissions(undefined, [...PermissionGroups.arxiv]);
}

/**
 * Check if Overleaf permission is granted
 */
export async function hasOverleafPermission(): Promise<boolean> {
  const origin = PermissionGroups.overleaf[0];
  return browser.permissions.contains({ origins: [origin] });
}
