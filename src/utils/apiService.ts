import { invoke } from '@tauri-apps/api/core';
import type { ApiConfigFile, EndpointKey } from '../types/api';

interface QueueJob {
  id: string;
  endpointKey: EndpointKey;
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: any;
  queryParams?: Record<string, string>;
  urlParams?: Record<string, string>;
  retriesLeft: number;
  maxRetries: number;
  createdAt: number;
  status: 'pending' | 'processing' | 'completed' | 'failed';
  error?: string;
}

const QUEUE_STORAGE_KEY = 'deskrona_api_queue';
const MAX_RETRIES = 5;
const RETRY_DELAY_MS = 5000;

let apiConfig: ApiConfigFile | null = null;
let processing = false;

function getBearerToken(): string {
  return apiConfig?.bearer_token || '';
}

function generateId(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8);
}

function loadQueue(): QueueJob[] {
  try {
    const raw = localStorage.getItem(QUEUE_STORAGE_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

function saveQueue(queue: QueueJob[]) {
  localStorage.setItem(QUEUE_STORAGE_KEY, JSON.stringify(queue));
}

function emitQueueEvent() {
  const queue = loadQueue();
  const pending = queue.filter(j => j.status === 'pending' || j.status === 'processing').length;
  const failed = queue.filter(j => j.status === 'failed').length;
  window.dispatchEvent(new CustomEvent('api-queue-update', {
    detail: { total: queue.length, pending, failed, jobs: queue },
  }));
}

export async function loadApiConfig(): Promise<ApiConfigFile> {
  if (!apiConfig) {
    apiConfig = await invoke<ApiConfigFile>('cmd_get_api_config');
  }
  return apiConfig;
}

export async function saveApiConfig(config: ApiConfigFile): Promise<void> {
  apiConfig = config;
  await invoke('cmd_save_api_config', { config });
}

export function getApiConfig(): ApiConfigFile | null {
  return apiConfig;
}

export function isOnline(): boolean {
  return apiConfig?.mode === 'online';
}

function getEffectiveUrl(endpointKey: EndpointKey, urlParams?: Record<string, string>, queryParams?: Record<string, string>): string {
  if (!apiConfig) return '';
  const ep = apiConfig.endpoints[endpointKey];
  if (!ep) return '';

  let url = ep.url;
  if (urlParams) {
    for (const [k, v] of Object.entries(urlParams)) {
      url = url.replace(`{${k}}`, encodeURIComponent(v));
    }
  }
  if (queryParams) {
    const qs = Object.entries(queryParams)
      .filter(([, v]) => v !== undefined && v !== null && v !== '')
      .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`)
      .join('&');
    if (qs) url += (url.includes('?') ? '&' : '?') + qs;
  }
  return url;
}

export function enqueueRequest(
  endpointKey: EndpointKey,
  body?: any,
  urlParams?: Record<string, string>,
  queryParams?: Record<string, string>,
): string | null {
  if (!apiConfig) return null;
  const ep = apiConfig.endpoints[endpointKey];
  if (!ep?.enabled || !ep.url) return null;

  const headers: Record<string, string> = { ...ep.headers };
  const token = getBearerToken();
  if (token) headers['Authorization'] = `Bearer ${token}`;

  const url = getEffectiveUrl(endpointKey, urlParams, queryParams);

  const job: QueueJob = {
    id: generateId(),
    endpointKey,
    method: ep.method,
    url,
    headers,
    body: body || undefined,
    queryParams,
    urlParams,
    retriesLeft: MAX_RETRIES,
    maxRetries: MAX_RETRIES,
    createdAt: Date.now(),
    status: 'pending',
  };

  const queue = loadQueue();
  queue.push(job);
  saveQueue(queue);
  emitQueueEvent();

  processQueue();
  return job.id;
}

async function executeJob(job: QueueJob): Promise<boolean> {
  try {
    const fetchOptions: RequestInit = {
      method: job.method,
      headers: job.headers,
    };

    if (job.body && job.method !== 'GET') {
      fetchOptions.body = JSON.stringify(job.body);
    }

    const response = await fetch(job.url, fetchOptions);

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    return true;
  } catch (error: any) {
    job.error = error.message || 'Unknown error';
    return false;
  }
}

function getRetryDelay(retriesUsed: number): number {
  return RETRY_DELAY_MS * Math.pow(2, MAX_RETRIES - retriesUsed);
}

async function processQueue() {
  if (processing) return;
  processing = true;

  while (true) {
    const queue = loadQueue();
    const pendingJob = queue.find(j => j.status === 'pending');

    if (!pendingJob) break;

    pendingJob.status = 'processing';
    saveQueue(queue);
    emitQueueEvent();

    const success = await executeJob(pendingJob);

    if (success) {
      pendingJob.status = 'completed';
      const remaining = loadQueue().filter(j => j.id !== pendingJob.id);
      saveQueue(remaining);
      emitQueueEvent();
      window.dispatchEvent(new CustomEvent('api-job-completed', {
        detail: { jobId: pendingJob.id, endpointKey: pendingJob.endpointKey },
      }));
    } else {
      pendingJob.retriesLeft--;
      if (pendingJob.retriesLeft <= 0) {
        pendingJob.status = 'failed';
        saveQueue(queue);
        emitQueueEvent();
        window.dispatchEvent(new CustomEvent('api-job-failed', {
          detail: { jobId: pendingJob.id, endpointKey: pendingJob.endpointKey, error: pendingJob.error },
        }));
      } else {
        pendingJob.status = 'pending';
        saveQueue(queue);
        emitQueueEvent();
        await new Promise(r => setTimeout(r, getRetryDelay(pendingJob.retriesLeft)));
      }
    }
  }

  processing = false;
}

export function getQueueStats(): { total: number; pending: number; failed: number; jobs: QueueJob[] } {
  const queue = loadQueue();
  return {
    total: queue.length,
    pending: queue.filter(j => j.status === 'pending' || j.status === 'processing').length,
    failed: queue.filter(j => j.status === 'failed').length,
    jobs: queue,
  };
}

export function retryFailedJobs() {
  const queue = loadQueue();
  let changed = false;
  for (const job of queue) {
    if (job.status === 'failed') {
      job.status = 'pending';
      job.retriesLeft = MAX_RETRIES;
      job.error = undefined;
      changed = true;
    }
  }
  if (changed) {
    saveQueue(queue);
    emitQueueEvent();
    processQueue();
  }
}

export function clearCompletedJobs() {
  const queue = loadQueue().filter(j => j.status === 'pending' || j.status === 'failed' || j.status === 'processing');
  saveQueue(queue);
  emitQueueEvent();
}

export function clearAllJobs() {
  saveQueue([]);
  emitQueueEvent();
}

export async function apiCall<T = any>(
  endpointKey: EndpointKey,
  body?: any,
  urlParams?: Record<string, string>,
  queryParams?: Record<string, string>,
  skipQueue = false,
): Promise<T> {
  if (skipQueue) {
    if (!apiConfig) throw new Error('API config not loaded');
    const ep = apiConfig.endpoints[endpointKey];
    if (!ep?.enabled) throw new Error(`Endpoint ${endpointKey} is disabled`);
    const headers: Record<string, string> = { ...ep.headers };
    const token = getBearerToken();
    if (token) headers['Authorization'] = `Bearer ${token}`;
    const url = getEffectiveUrl(endpointKey, urlParams, queryParams);
    const fetchOptions: RequestInit = { method: ep.method, headers };
    if (body && ep.method !== 'GET') fetchOptions.body = JSON.stringify(body);
    const res = await fetch(url, fetchOptions);
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return res.json();
  }

  const jobId = enqueueRequest(endpointKey, body, urlParams, queryParams);
  if (!jobId) throw new Error(`Endpoint ${endpointKey} is disabled or not configured`);

  return new Promise<T>((resolve, reject) => {
    const onCompleted = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail.jobId === jobId) {
        cleanup();
        resolve({} as T);
      }
    };
    const onFailed = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail.jobId === jobId) {
        cleanup();
        reject(new Error(detail.error || 'Job failed'));
      }
    };
    const cleanup = () => {
      window.removeEventListener('api-job-completed', onCompleted);
      window.removeEventListener('api-job-failed', onFailed);
    };
    window.addEventListener('api-job-completed', onCompleted);
    window.addEventListener('api-job-failed', onFailed);
  });
}
