import { invoke } from '@tauri-apps/api/core';
import { isOnline, getApiConfig } from './apiService';
import type { EndpointKey } from '../types/api';

function getHeaders(endpointKey: EndpointKey): Record<string, string> {
  const config = getApiConfig();
  if (!config) return { 'Content-Type': 'application/json' };
  const ep = config.endpoints[endpointKey];
  const headers: Record<string, string> = { ...(ep?.headers || {}), 'Content-Type': 'application/json' };
  if (config.bearer_token) headers['Authorization'] = `Bearer ${config.bearer_token}`;
  return headers;
}

async function apiFetch<T>(endpointKey: EndpointKey, body?: any, urlParams?: Record<string, string>, queryParams?: Record<string, string>): Promise<T | null> {
  const config = getApiConfig();
  if (!config) return null;
  const ep = config.endpoints[endpointKey];
  if (!ep?.enabled || !ep.url) return null;

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

  const fetchOptions: RequestInit = { method: ep.method, headers: getHeaders(endpointKey) };
  if (body && ep.method !== 'GET') fetchOptions.body = JSON.stringify(body);

  const res = await fetch(url, fetchOptions);
  if (!res.ok) throw new Error(`API ${endpointKey}: HTTP ${res.status}`);
  return res.json();
}

// ─── Auth ─────────────────────────────────────────────────────────

export async function proxyLogin(payload: { username: string; password: string }): Promise<{ token: string; user: any }> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ token: string; user: any }>('auth_login', payload);
      if (result) return result;
    } catch {}
  }
  return invoke('cmd_login', { payload });
}

export async function proxyRegisterCompany(payload: {
  company_name: string; admin_username: string; admin_display_name: string; admin_password: string;
}): Promise<{ token: string; user: any }> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ token: string; user: any }>('auth_register', payload);
      if (result) return result;
    } catch {}
  }
  return invoke('cmd_register_company', { payload });
}

export async function proxyValidateSession(token: string): Promise<any> {
  if (isOnline()) {
    try {
      const result = await apiFetch<any>('auth_validate', undefined, undefined, { token });
      if (result) return result.user || result;
    } catch {}
  }
  return invoke('cmd_validate_session', { token });
}

export async function proxyLogout(token: string): Promise<void> {
  if (isOnline()) {
    try {
      await apiFetch<any>('auth_logout', { token });
      return;
    } catch {}
  }
  await invoke('cmd_logout', { token });
}

// ─── Admin: Users ─────────────────────────────────────────────────

export async function proxyGetCompanyUsers(companyId: string): Promise<any[]> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ users: any[] }>('admin_users_list');
      if (result?.users) return result.users;
    } catch {}
  }
  return invoke('cmd_get_company_users', { companyId });
}

export async function proxyCreateUser(companyId: string, payload: any): Promise<any> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ user: any }>('admin_users_create', payload);
      if (result?.user) return result.user;
    } catch {}
  }
  return invoke('cmd_create_user', { companyId, payload });
}

// ─── Admin: Stats ─────────────────────────────────────────────────

export async function proxyGetAdminStats(companyId: string): Promise<any[]> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ stats: any[] }>('admin_stats');
      if (result?.stats) return result.stats;
    } catch {}
  }
  return invoke('cmd_get_admin_stats', { companyId });
}

// ─── Admin: Drill-down ────────────────────────────────────────────

export async function proxyGetUserScreenshots(userId: string, from: string, to: string, limit: number, offset: number): Promise<any[]> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ screenshots: any[] }>('admin_user_screenshots', undefined, { userId }, { from, to, limit: String(limit), offset: String(offset) });
      if (result?.screenshots) return result.screenshots;
    } catch {}
  }
  return invoke('cmd_get_user_screenshots', { userId, from, to, limit, offset });
}

export async function proxyGetUserTimeLogs(userId: string, from: string, to: string, limit: number, offset: number): Promise<any[]> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ logs: any[] }>('admin_user_time_logs', undefined, { userId }, { from, to, limit: String(limit), offset: String(offset) });
      if (result?.logs) return result.logs;
    } catch {}
  }
  return invoke('cmd_get_user_time_logs', { userId, from, to, limit, offset });
}

export async function proxyGetUserActivity(userId: string, from: string, to: string, limit: number, offset: number): Promise<any[]> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ events: any[] }>('admin_user_activity', undefined, { userId }, { from, to, limit: String(limit), offset: String(offset) });
      if (result?.events) return result.events;
    } catch {}
  }
  return invoke('cmd_get_user_activity', { userId, from, to, limit, offset });
}

export async function proxyGetUserUrls(userId: string, from: string, to: string, limit: number, offset: number): Promise<any[]> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ urls: any[] }>('admin_user_urls', undefined, { userId }, { from, to, limit: String(limit), offset: String(offset) });
      if (result?.urls) return result.urls;
    } catch {}
  }
  return invoke('cmd_get_user_urls', { userId, from, to, limit, offset });
}

export async function proxyGetUserInputStats(userId: string, from: string, to: string): Promise<any> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ keyboard: number; mouse: number; idle: number }>('admin_user_input_stats', undefined, { userId }, { from, to });
      if (result) return result;
    } catch {}
  }
  return invoke('cmd_get_user_input_stats', { userId, from, to });
}

// ─── Own Data ─────────────────────────────────────────────────────

export async function proxyGetDashboardData(): Promise<any> {
  if (isOnline()) {
    try {
      const result = await apiFetch<any>('dashboard_today');
      if (result) return result;
    } catch {}
  }
  return invoke('cmd_get_dashboard_data');
}

export async function proxyGetFilteredDashboardData(from: string, to: string): Promise<any> {
  if (isOnline()) {
    try {
      const result = await apiFetch<any>('dashboard_range', undefined, undefined, { from, to });
      if (result) return result;
    } catch {}
  }
  return invoke('cmd_get_filtered_dashboard_data', { from, to });
}

export async function proxyGetTimeLogsRange(from: string, to: string, limit: number, offset: number): Promise<any[]> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ logs: any[] }>('time_logs_get', undefined, undefined, { from, to, limit: String(limit), offset: String(offset) });
      if (result?.logs) return result.logs;
    } catch {}
  }
  return invoke('cmd_get_time_logs_range', { from, to, limit, offset });
}

export async function proxyGetUrlsRange(from: string, to: string, limit: number, offset: number): Promise<any[]> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ urls: any[] }>('urls_get', undefined, undefined, { from, to, limit: String(limit), offset: String(offset) });
      if (result?.urls) return result.urls;
    } catch {}
  }
  return invoke('cmd_get_urls_range', { from, to, limit, offset });
}

export async function proxyGetScreenshotsRange(from: string, to: string, limit: number, offset: number): Promise<any[]> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ screenshots: any[] }>('screenshots_get', undefined, undefined, { from, to, limit: String(limit), offset: String(offset) });
      if (result?.screenshots) return result.screenshots;
    } catch {}
  }
  return invoke('cmd_get_screenshots_range', { from, to, limit, offset });
}

// ─── Categories ───────────────────────────────────────────────────

export async function proxyGetAllAppCategories(): Promise<any[]> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ categories: any[] }>('app_categories_get');
      if (result?.categories) return result.categories;
    } catch {}
  }
  return invoke('cmd_get_all_app_categories');
}

export async function proxyUpdateAppCategory(appName: string, category: string): Promise<void> {
  if (isOnline()) {
    try {
      await apiFetch<any>('app_categories_update', { app_name: appName, category });
      return;
    } catch {}
  }
  await invoke('cmd_update_app_category', { appName, category });
}

// ─── Calendar ───────────────────────────────────────────────────

export async function proxyGetCalendarMonth(from: string, to: string): Promise<any[]> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ days: any[] }>('calendar_month', undefined, undefined, { from, to });
      if (result?.days) return result.days;
    } catch {}
  }
  return invoke('cmd_get_calendar_month', { from, to });
}

// ─── Settings ────────────────────────────────────────────────────

export async function proxyGetSettings(): Promise<any> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ settings: any }>('settings_get');
      if (result?.settings) return result.settings;
    } catch {}
  }
  return invoke('get_settings');
}

export async function proxyUpdateSettings(settings: any): Promise<void> {
  if (isOnline()) {
    try {
      await apiFetch<any>('settings_update', { settings });
      return;
    } catch {}
  }
  await invoke('update_settings', { settings });
}

// ─── Pomodoro ────────────────────────────────────────────────────

export async function proxyPomodoroStart(): Promise<any> {
  if (isOnline()) {
    try {
      const result = await apiFetch<any>('pomodoro_start', {});
      if (result) return result;
    } catch {}
  }
  return invoke('cmd_pomodoro_start');
}

export async function proxyPomodoroSkip(): Promise<any> {
  if (isOnline()) {
    try {
      const result = await apiFetch<any>('pomodoro_skip', {});
      if (result) return result;
    } catch {}
  }
  return invoke('cmd_pomodoro_skip');
}

export async function proxyPomodoroStop(): Promise<any> {
  if (isOnline()) {
    try {
      const result = await apiFetch<any>('pomodoro_stop', {});
      if (result) return result;
    } catch {}
  }
  return invoke('cmd_pomodoro_stop');
}

export async function proxyPomodoroStatus(): Promise<any> {
  if (isOnline()) {
    try {
      const result = await apiFetch<any>('pomodoro_status');
      if (result) return result;
    } catch {}
  }
  return invoke<any>('cmd_pomodoro_status');
}

// ─── Autostart ───────────────────────────────────────────────────

export async function proxySetAutostart(enabled: boolean): Promise<void> {
  if (isOnline()) {
    try {
      await apiFetch<any>('autostart_set', { enabled });
      return;
    } catch {}
  }
  await invoke('set_autostart', { enabled });
}

// ─── Backup ──────────────────────────────────────────────────────

export async function proxyExportDb(path: string): Promise<void> {
  if (isOnline()) {
    try {
      await apiFetch<any>('backup_export', { path });
    } catch {}
  }
  await invoke('cmd_export_db', { path });
}

export async function proxyImportDb(path: string): Promise<void> {
  if (isOnline()) {
    try {
      await apiFetch<any>('backup_import', { path });
    } catch {}
  }
  await invoke('cmd_import_db', { path });
}

// ─── App Config ──────────────────────────────────────────────────

export async function proxyGetAppConfig(): Promise<any> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ config: any }>('config_get');
      if (result?.config) return result.config;
    } catch {}
  }
  return invoke('cmd_get_app_config');
}

export async function proxySaveAppConfig(cfg: any): Promise<void> {
  if (isOnline()) {
    try {
      await apiFetch<any>('config_save', { config: cfg });
      return;
    } catch {}
  }
  await invoke('cmd_save_app_config', { cfg });
}

// ─── Reset ───────────────────────────────────────────────────────

export async function proxyResetApp(): Promise<void> {
  if (isOnline()) {
    try {
      await apiFetch<any>('reset_app', {});
    } catch {}
  }
  await invoke('cmd_reset_app');
}

// ─── Session ─────────────────────────────────────────────────────

export async function proxyStartSession(): Promise<any> {
  if (isOnline()) {
    try {
      const result = await apiFetch<any>('session_start', {});
      if (result?.session) return result.session;
    } catch {}
  }
  return invoke('cmd_start_session');
}

export async function proxyStopSession(sessionId: string): Promise<any> {
  if (isOnline()) {
    try {
      const result = await apiFetch<any>('session_stop', { session_id: sessionId });
      if (result?.session) return result.session;
    } catch {}
  }
  return invoke('cmd_stop_session', { sessionId });
}

export async function proxyGetActiveSession(): Promise<any> {
  if (isOnline()) {
    try {
      const result = await apiFetch<any>('session_active');
      if (result?.session !== undefined) return result.session;
    } catch {}
  }
  return invoke('cmd_get_active_session');
}

// ─── Tracking Control ────────────────────────────────────────────

export async function proxySetTracking(status: string): Promise<void> {
  if (isOnline()) {
    try {
      await apiFetch<any>('tracking_status', { status });
    } catch {}
  }
  await invoke('cmd_set_tracking', { status });
}

export async function proxyGetTracking(): Promise<string> {
  if (isOnline()) {
    try {
      const result = await apiFetch<{ status: string }>('tracking_status');
      if (result?.status) return result.status;
    } catch {}
  }
  return invoke<string>('cmd_get_tracking');
}
