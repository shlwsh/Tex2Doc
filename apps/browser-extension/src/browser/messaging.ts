/**
 * Messaging utilities
 */

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type MessageHandler = (message: any, sender: browser.Runtime.MessageSender) => Promise<any> | any;

export interface MessageOptions {
  expectResponse?: boolean;
  timeout?: number;
}

const DEFAULT_TIMEOUT = 30000;

/**
 * Send message to background script
 */
export async function sendToBackground<T = unknown>(
  message: Record<string, unknown>,
  options: MessageOptions = {}
): Promise<T> {
  const { timeout = DEFAULT_TIMEOUT } = options;

  return new Promise((resolve, reject) => {
    const timeoutId = setTimeout(() => {
      reject(new Error(`Message timeout: ${JSON.stringify(message)}`));
    }, timeout);

    browser.runtime
      .sendMessage(message)
      .then((response) => {
        clearTimeout(timeoutId);
        resolve(response as T);
      })
      .catch((error) => {
        clearTimeout(timeoutId);
        reject(error);
      });
  });
}

/**
 * Send message to specific content script
 */
export async function sendToContentScript<T = unknown>(
  tabId: number,
  message: Record<string, unknown>
): Promise<T> {
  return browser.tabs.sendMessage(tabId, message) as Promise<T>;
}

/**
 * Create a message listener
 */
export function createMessageListener(handlers: Record<string, MessageHandler>): () => void {
  const listener = (
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    message: any,
    sender: browser.Runtime.MessageSender,
    sendResponse: () => void
  ) => {
    const handler = handlers[message.type as string];
    if (!handler) {
      sendResponse();
      return;
    }

    const result = handler(message, sender);
    if (result instanceof Promise) {
      result.then(() => sendResponse()).catch(() => sendResponse());
      return;
    }

    sendResponse();
  };

  browser.runtime.onMessage.addListener(listener);
  return () => browser.runtime.onMessage.removeListener(listener);
}

/**
 * Message types for type safety
 */
export const MessageTypes = {
  AUTH_LOGIN: 'AUTH_LOGIN',
  AUTH_REGISTER: 'AUTH_REGISTER',
  AUTH_LOGOUT: 'AUTH_LOGOUT',
  AUTH_REFRESH: 'AUTH_REFRESH',
  AUTH_GET_SESSION: 'AUTH_GET_SESSION',
  CONVERSION_START: 'CONVERSION_START',
  CONVERSION_CANCEL: 'CONVERSION_CANCEL',
  CONVERSION_STATUS: 'CONVERSION_STATUS',
  CONVERSION_DOWNLOAD: 'CONVERSION_DOWNLOAD',
  WASM_CONVERT: 'WASM_CONVERT',
  WASM_CHECK: 'WASM_CHECK',
  JOBS_GET: 'JOBS_GET',
  JOBS_CLEAR: 'JOBS_CLEAR',
  USAGE_GET: 'USAGE_GET',
  SETTINGS_GET: 'SETTINGS_GET',
  SETTINGS_UPDATE: 'SETTINGS_UPDATE',
  UI_OPEN_POPUP: 'UI_OPEN_POPUP',
  UI_OPEN_TAB: 'UI_OPEN_TAB',
} as const;
