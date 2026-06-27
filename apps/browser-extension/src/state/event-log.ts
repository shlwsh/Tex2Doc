import type { EventLogEntry } from '@/shared/types';

const MAX_EVENTS = 500;
const events: EventLogEntry[] = [];

export function addEvent(type: 'info' | 'warning' | 'error', message: string, details?: Record<string, unknown>): void {
  const entry: EventLogEntry = {
    id: crypto.randomUUID(),
    timestamp: Date.now(),
    type,
    message,
    details,
    job_id: undefined,
  };
  events.push(entry);
  while (events.length > MAX_EVENTS) events.shift();
}

export function getEvents(): EventLogEntry[] {
  return [...events];
}

export function getEventsByType(type: 'info' | 'warning' | 'error'): EventLogEntry[] {
  return events.filter((e) => e.type === type);
}

export function getEventsForJob(jobId: string): EventLogEntry[] {
  return events.filter((e) => e.job_id === jobId);
}

export function getRecentEvents(count = 50): EventLogEntry[] {
  return events.slice(-count);
}

export function clearEvents(): void {
  events.length = 0;
}

export function exportEvents(): string {
  return JSON.stringify(events, null, 2);
}

export function logInfo(message: string, details?: Record<string, unknown>): void {
  addEvent('info', message, details);
}

export function logWarning(message: string, details?: Record<string, unknown>): void {
  addEvent('warning', message, details);
}

export function logError(message: string, details?: Record<string, unknown>): void {
  addEvent('error', message, details);
}
