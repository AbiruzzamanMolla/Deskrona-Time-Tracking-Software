import { invoke } from '@tauri-apps/api/core';
import { isOnline, enqueueRequest, getApiConfig } from './apiService';

const SYNC_STATE_KEY = 'deskrona_sync_state';
const SYNC_INTERVAL_MS = 30000; // 30 seconds between sync cycles

interface SyncState {
  lastSyncTime: string;
  syncedTimeLogIds: string[];
  syncedUrlIds: string[];
  syncedActivityIds: string[];
  lastSessionId: string | null;
}

function getDefaultSyncState(): SyncState {
  return {
    lastSyncTime: new Date(0).toISOString(),
    syncedTimeLogIds: [],
    syncedUrlIds: [],
    syncedActivityIds: [],
    lastSessionId: null,
  };
}

function loadSyncState(): SyncState {
  try {
    const raw = localStorage.getItem(SYNC_STATE_KEY);
    return raw ? { ...getDefaultSyncState(), ...JSON.parse(raw) } : getDefaultSyncState();
  } catch {
    return getDefaultSyncState();
  }
}

function saveSyncState(state: SyncState) {
  localStorage.setItem(SYNC_STATE_KEY, JSON.stringify(state));
}

async function syncTimeLogs(state: SyncState): Promise<SyncState> {
  try {
    const config = getApiConfig();
    if (!config?.endpoints.time_logs_sync?.enabled) return state;

    const from = state.lastSyncTime;
    const to = new Date().toISOString();
    const logs = await invoke<any[]>('cmd_get_time_logs_range', {
      from,
      to,
      limit: 500,
      offset: 0,
    });

    const newLogs = logs.filter((l: any) => !state.syncedTimeLogIds.includes(l.id));
    if (newLogs.length === 0) return state;

    // Send in batches of 50
    for (let i = 0; i < newLogs.length; i += 50) {
      const batch = newLogs.slice(i, i + 50);
      enqueueRequest('time_logs_sync', { logs: batch });
    }

    state.syncedTimeLogIds = [
      ...state.syncedTimeLogIds,
      ...newLogs.map((l: any) => l.id),
    ].slice(-1000); // Keep last 1000 IDs to avoid unbounded growth
    state.lastSyncTime = to;
    saveSyncState(state);
  } catch (e) {
    console.error('Sync time logs failed:', e);
  }
  return state;
}

async function syncUrls(state: SyncState): Promise<SyncState> {
  try {
    const config = getApiConfig();
    if (!config?.endpoints.urls_sync?.enabled) return state;

    const from = state.lastSyncTime;
    const to = new Date().toISOString();
    const urls = await invoke<any[]>('cmd_get_urls_range', {
      from,
      to,
      limit: 500,
      offset: 0,
    });

    const newUrls = urls.filter((u: any) => !state.syncedUrlIds.includes(u.id));
    if (newUrls.length === 0) return state;

    for (let i = 0; i < newUrls.length; i += 100) {
      const batch = newUrls.slice(i, i + 100);
      enqueueRequest('urls_sync', {
        urls: batch.map((u: any) => ({ url: u.url, title: u.activity_status || '', timestamp: u.timestamp })),
      });
    }

    state.syncedUrlIds = [
      ...state.syncedUrlIds,
      ...newUrls.map((u: any) => u.id),
    ].slice(-1000);
    saveSyncState(state);
  } catch (e) {
    console.error('Sync URLs failed:', e);
  }
  return state;
}

async function syncActivity(state: SyncState): Promise<SyncState> {
  try {
    const config = getApiConfig();
    if (!config?.endpoints.activity_sync?.enabled) return state;

    const from = state.lastSyncTime;
    const to = new Date().toISOString();
    const userId = 'default_user';

    const events = await invoke<any[]>('cmd_get_user_activity', {
      userId,
      from,
      to,
      limit: 500,
      offset: 0,
    });

    if (events.length === 0) return state;

    enqueueRequest('activity_sync', { events });

    state.lastSyncTime = to;
    saveSyncState(state);
  } catch (e) {
    console.error('Sync activity failed:', e);
  }
  return state;
}

async function syncSession(state: SyncState): Promise<SyncState> {
  try {
    const config = getApiConfig();
    if (!config?.endpoints.session_start?.enabled && !config?.endpoints.session_stop?.enabled) return state;

    const activeSession = await invoke<any | null>('cmd_get_active_session');

    if (activeSession && activeSession.id !== state.lastSessionId) {
      enqueueRequest('session_start', {
        session_id: activeSession.id,
        start_time: activeSession.start_time,
      });
      state.lastSessionId = activeSession.id;
      saveSyncState(state);
    }

    if (!activeSession && state.lastSessionId) {
      enqueueRequest('session_stop', {
        session_id: state.lastSessionId,
      });
      state.lastSessionId = null;
      saveSyncState(state);
    }
  } catch (e) {
    console.error('Sync session failed:', e);
  }
  return state;
}

async function syncTrackingStatus() {
  try {
    const config = getApiConfig();
    if (!config?.endpoints.tracking_status?.enabled) return;

    const status = await invoke<string>('cmd_get_tracking');
    enqueueRequest('tracking_status', { status });
  } catch (e) {
    console.error('Sync tracking status failed:', e);
  }
}

export async function runSyncCycle() {
  if (!isOnline()) return;

  let state = loadSyncState();

  state = await syncTimeLogs(state);
  state = await syncUrls(state);
  state = await syncActivity(state);
  await syncSession(state);
  await syncTrackingStatus();

  state.lastSyncTime = new Date().toISOString();
  saveSyncState(state);
}

let syncInterval: ReturnType<typeof setInterval> | null = null;

export function startSync() {
  stopSync();
  if (!isOnline()) return;
  // Run immediately
  runSyncCycle();
  // Then periodically
  syncInterval = setInterval(runSyncCycle, SYNC_INTERVAL_MS);
}

export function stopSync() {
  if (syncInterval) {
    clearInterval(syncInterval);
    syncInterval = null;
  }
}

export function restartSync() {
  if (isOnline()) startSync();
  else stopSync();
}
