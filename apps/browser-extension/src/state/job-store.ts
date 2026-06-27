/**
 * IndexedDB Store for Job Records
 *
 * Handles persistent storage of conversion job records
 */

import { openDB, type DBSchema, type IDBPDatabase } from 'idb';
import type { JobRecord } from '@/shared/types';
import { DB_NAME, DB_VERSION, STORES } from '@/shared/constants';

interface Tex2DocDB extends DBSchema {
  [STORES.JOBS]: {
    key: string;
    value: JobRecord;
    indexes: {
      'by-status': string;
      'by-created': number;
    };
  };
  [STORES.EVENTS]: {
    key: string;
    value: {
      id: string;
      timestamp: number;
      type: 'info' | 'warning' | 'error';
      message: string;
      details?: unknown;
    };
  };
}

let dbInstance: IDBPDatabase<Tex2DocDB> | null = null;

async function getDB(): Promise<IDBPDatabase<Tex2DocDB>> {
  if (dbInstance) {
    return dbInstance;
  }

  dbInstance = await openDB<Tex2DocDB>(DB_NAME, DB_VERSION, {
    upgrade(db) {
      // Jobs store
      if (!db.objectStoreNames.contains(STORES.JOBS)) {
        const jobStore = db.createObjectStore(STORES.JOBS, { keyPath: 'id' });
        jobStore.createIndex('by-status', 'status');
        jobStore.createIndex('by-created', 'created_at');
      }

      // Events store (ring buffer for diagnostics)
      if (!db.objectStoreNames.contains(STORES.EVENTS)) {
        db.createObjectStore(STORES.EVENTS, { keyPath: 'id' });
      }
    },
  });

  return dbInstance;
}

// ============================================
// Job Store Operations
// ============================================

/**
 * Add or update a job record
 */
export async function saveJob(job: JobRecord): Promise<void> {
  const db = await getDB();
  await db.put(STORES.JOBS, job);
}

/**
 * Get a job by ID
 */
export async function getJob(id: string): Promise<JobRecord | undefined> {
  const db = await getDB();
  return db.get(STORES.JOBS, id);
}

/**
 * Get a job by cloud job ID
 */
export async function getJobByCloudId(jobId: string): Promise<JobRecord | undefined> {
  const db = await getDB();
  const jobs = await db.getAllFromIndex(STORES.JOBS, 'by-created');
  return jobs.find((j) => j.job_id === jobId);
}

/**
 * Get all jobs
 */
export async function getAllJobs(): Promise<JobRecord[]> {
  const db = await getDB();
  const jobs = await db.getAllFromIndex(STORES.JOBS, 'by-created');
  return jobs.reverse(); // Most recent first
}

/**
 * Get jobs by status
 */
export async function getJobsByStatus(status: JobRecord['status']): Promise<JobRecord[]> {
  const db = await getDB();
  return db.getAllFromIndex(STORES.JOBS, 'by-status', status);
}

/**
 * Get recent jobs (limit)
 */
export async function getRecentJobs(limit = 10): Promise<JobRecord[]> {
  const db = await getDB();
  const jobs = await db.getAllFromIndex(STORES.JOBS, 'by-created');
  return jobs.reverse().slice(0, limit);
}

/**
 * Delete a job
 */
export async function deleteJob(id: string): Promise<void> {
  const db = await getDB();
  await db.delete(STORES.JOBS, id);
}

/**
 * Delete all completed jobs
 */
export async function clearCompletedJobs(): Promise<void> {
  const db = await getDB();
  const completedJobs = await db.getAllFromIndex(STORES.JOBS, 'by-status', 'completed');
  const tx = db.transaction(STORES.JOBS, 'readwrite');
  await Promise.all(completedJobs.map((job) => tx.store.delete(job.id)));
  await tx.done;
}

/**
 * Clear all jobs
 */
export async function clearAllJobs(): Promise<void> {
  const db = await getDB();
  await db.clear(STORES.JOBS);
}

/**
 * Update job status
 */
export async function updateJobStatus(
  id: string,
  status: JobRecord['status'],
  progress?: number
): Promise<void> {
  const job = await getJob(id);
  if (job) {
    job.status = status;
    if (progress !== undefined) {
      job.progress = progress;
    }
    job.updated_at = Date.now();
    await saveJob(job);
  }
}

/**
 * Update job with report
 */
export async function updateJobReport(id: string, report: JobRecord['report']): Promise<void> {
  const job = await getJob(id);
  if (job) {
    job.report = report;
    job.updated_at = Date.now();
    await saveJob(job);
  }
}

/**
 * Count jobs by status
 */
export async function countJobsByStatus(status: JobRecord['status']): Promise<number> {
  const db = await getDB();
  return db.countFromIndex(STORES.JOBS, 'by-status', status);
}

// ============================================
// Event Log Operations
// ============================================

const MAX_EVENTS = 1000;

/**
 * Add event to log
 */
export async function addEvent(
  type: 'info' | 'warning' | 'error',
  message: string,
  details?: Record<string, unknown>,
  jobId?: string
): Promise<void> {
  const db = await getDB();
  const event = {
    id: crypto.randomUUID(),
    timestamp: Date.now(),
    type,
    message,
    details,
    job_id: jobId,
  };
  await db.put(STORES.EVENTS, event);

  // Trim old events
  const count = await db.count(STORES.EVENTS);
  if (count > MAX_EVENTS) {
    const tx = db.transaction(STORES.EVENTS, 'readwrite');
    const events = await tx.store.getAll();
    events.sort((a, b) => a.timestamp - b.timestamp);
    const toDelete = events.slice(0, count - MAX_EVENTS);
    await Promise.all(toDelete.map((e) => tx.store.delete(e.id)));
    await tx.done;
  }
}

/**
 * Get events (optionally filtered by type)
 */
export async function getEvents(
  type?: 'info' | 'warning' | 'error',
  limit = 100
): Promise<unknown[]> {
  const db = await getDB();
  const events = await db.getAll(STORES.EVENTS);
  let filtered = type ? events.filter((e) => e.type === type) : events;
  filtered.sort((a, b) => b.timestamp - a.timestamp);
  return filtered.slice(0, limit);
}

/**
 * Clear event log
 */
export async function clearEvents(): Promise<void> {
  const db = await getDB();
  await db.clear(STORES.EVENTS);
}

/**
 * Export events for diagnostics
 */
export async function exportEvents(): Promise<string> {
  const events = await getEvents(undefined, 10000);
  return JSON.stringify(events, null, 2);
}
