<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch, computed } from "vue";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useI18n } from "vue-i18n";
import { Pie } from 'vue-chartjs';
import { Chart as ChartJS, Title, Tooltip, Legend, ArcElement } from 'chart.js';

ChartJS.register(Title, Tooltip, Legend, ArcElement);

const { t, locale } = useI18n();

interface Settings {
  language: string;
  theme: string;
  auto_start_on_boot: boolean;
  screenshot_interval: number;
  screenshot_location: string;
  backup_frequency: string;
  backup_location: string;
}

interface Session {
  id: string;
  start_time: string;
  end_time: string | null;
  duration: number | null;
}

interface AppUsageStat {
  app_name: string;
  total_seconds: number;
  session_count: number;
  category: string;
}

interface UrlEntry {
  url: string;
  timestamp: string;
}

interface DashboardData {
  total_active_seconds: number;
  total_idle_seconds: number;
  session_seconds: number;
  app_stats: AppUsageStat[];
  recent_urls: UrlEntry[];
}

interface TimeLogEntry {
  id: string;
  app_name: string;
  window_title: string;
  start_time: string;
  end_time: string;
  duration: number;
  status: string;
}

interface UrlEntryFull {
  id: string;
  url: string;
  timestamp: string;
  activity_status: string;
}

interface ScreenshotEntry {
  id: string;
  file_path: string;
  captured_at: string;
}

// ─── Phase 8 Interfaces ───────────────────────────────────────────
interface AppConfig {
  mode: 'single_user' | 'multi_user';
  setup_done: boolean;
}

interface AuthUser {
  id: string;
  company_id: string;
  username: string;
  display_name: string;
  role: string;
  created_at: string;
}

interface LoginResult {
  token: string;
  user: AuthUser;
}

interface UserProductivityStat {
  user_id: string;
  display_name: string;
  username: string;
  total_active_seconds: number;
  session_count: number;
}

interface CreateUserPayload {
  username: string;
  display_name: string;
  password: string;
  role: string;
}

const settings = ref<Settings>({
  language: "en",
  theme: "system",
  auto_start_on_boot: false,
  screenshot_interval: 10,
  screenshot_location: "",
  backup_frequency: "never",
  backup_location: "",
});

const currentView = ref('dashboard');
const activeSession = ref<Session | null>(null);
const dashboardData = ref<DashboardData>({
  total_active_seconds: 0,
  total_idle_seconds: 0,
  session_seconds: 0,
  app_stats: [],
  recent_urls: [],
});
let refreshInterval: ReturnType<typeof setInterval> | null = null;
let taskbarInterval: ReturnType<typeof setInterval> | null = null;
const trackingStatus = ref<string>('running');
const defaultScreenshotDir = ref<string>('');

// ─── Phase 8 State ────────────────────────────────────────────────
const appConfig = ref<AppConfig>({ mode: 'single_user', setup_done: false });
const appScreen = ref<'loading' | 'wizard' | 'login' | 'app'>('loading');
const currentUser = ref<AuthUser | null>(null);
const sessionToken = ref<string>('');

// Wizard state
const wizardStep = ref(1);
const wizardMode = ref<'single_user' | 'multi_user'>('single_user');
const wizardCompanyName = ref('');
const wizardAdminUsername = ref('');
const wizardAdminDisplay = ref('');
const wizardAdminPassword = ref('');
const wizardConfirmPassword = ref('');
const wizardError = ref('');
const wizardLoading = ref(false);

// Login state
const loginUsername = ref('');
const loginPassword = ref('');
const loginError = ref('');
const loginLoading = ref(false);

// Admin state
const adminUsers = ref<AuthUser[]>([]);
const adminStats = ref<UserProductivityStat[]>([]);
const showCreateUser = ref(false);
const newUser = ref<CreateUserPayload>({ username: '', display_name: '', password: '', role: 'employee' });
const createUserError = ref('');
const createUserLoading = ref(false);

const isAdmin = computed(() => currentUser.value?.role === 'admin');
const isMultiUser = computed(() => appConfig.value.mode === 'multi_user');

// Settings mode change pending state
const pendingMode = ref<'single_user' | 'multi_user'>('single_user');

// Accumulated paused seconds — updated each time we enter/leave pause state
const pausedSeconds = ref(0);
let pauseStartedAt: number | null = null;

const filterType = ref('daily');
const customFromDate = ref(new Date().toISOString().split('T')[0]);
const customToDate = ref(new Date().toISOString().split('T')[0]);

const timeLogsList = ref<TimeLogEntry[]>([]);
const urlsList = ref<UrlEntryFull[]>([]);
const screenshotsList = ref<ScreenshotEntry[]>([]);

const timeLogsOffset = ref(0);
const urlsOffset = ref(0);
const screenshotsOffset = ref(0);
const pageSize = 50;

// ─── Helpers ──────────────────────────────────────────────────────
const formatTime = (seconds: number): string => {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
};

const updateCategory = async (appName: string, category: string) => {
  try {
    await invoke("cmd_update_app_category", { appName, category });
    const app = dashboardData.value.app_stats.find(a => a.app_name === appName);
    if (app) app.category = category;
  } catch (error) {
    console.error("Failed to update category:", error);
  }
};

const productivityChartData = computed(() => {
  let productive = 0;
  let unproductive = 0;
  let neutral = 0;

  dashboardData.value.app_stats.forEach(stat => {
    if (stat.category === 'productive') productive += stat.total_seconds;
    else if (stat.category === 'unproductive') unproductive += stat.total_seconds;
    else neutral += stat.total_seconds;
  });

  return {
    labels: ['Productive', 'Unproductive', 'Neutral'],
    datasets: [{
      backgroundColor: ['#10b981', '#ef4444', '#64748b'],
      data: [productive, unproductive, neutral],
      borderWidth: 0,
      hoverOffset: 4
    }]
  };
});
const productivityChartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: { position: 'bottom' as const, labels: { color: '#94a3b8' } }
  }
};

const formatTimestamp = (ts: string): string => {
  try {
    const d = new Date(ts);
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  } catch { return ts; }
};

const appIcon = (name: string): string => {
  const n = name.toLowerCase();
  if (n.includes('chrome') || n.includes('edge') || n.includes('firefox') || n.includes('brave') || n.includes('opera') || n.includes('safari')) return '🌐';
  if (n.includes('code') || n.includes('visual studio') || n.includes('vim') || n.includes('nvim')) return '💻';
  if (n.includes('terminal') || n.includes('cmd') || n.includes('powershell') || n.includes('windows terminal')) return '⬛';
  if (n.includes('slack') || n.includes('discord') || n.includes('teams') || n.includes('telegram')) return '💬';
  if (n.includes('explorer') || n.includes('finder')) return '📁';
  if (n.includes('spotify') || n.includes('music')) return '🎵';
  if (n.includes('outlook') || n.includes('mail') || n.includes('thunderbird')) return '📧';
  return '📄';
};

const percentage = (seconds: number): number => {
  const total = dashboardData.value.total_active_seconds;
  if (total <= 0) return 0;
  return Math.round((seconds / total) * 100);
};

// ─── Settings ─────────────────────────────────────────────────────
const loadSettings = async () => {
  try {
    const s = await invoke<Settings>("get_settings");
    settings.value = s;
    locale.value = s.language;
    applyTheme();
  } catch (error) {
    console.error("Failed to load settings:", error);
  }
};

const saveSettings = async () => {
  try {
    await invoke("update_settings", { settings: settings.value });
    locale.value = settings.value.language;
    applyTheme();
  } catch (error) {
    console.error("Failed to save settings:", error);
  }
};

const applyTheme = () => {
  const root = document.documentElement;
  if (settings.value.theme === "system") {
    const isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    root.classList.toggle("dark", isDark);
  } else {
    root.classList.toggle("dark", settings.value.theme === "dark");
  }
};

const selectScreenshotLocation = async () => {
  const selected = await open({ directory: true, multiple: false, title: t('message.screenshotLocation'), defaultPath: settings.value.screenshot_location || undefined });
  if (selected && typeof selected === 'string') settings.value.screenshot_location = selected;
};

const selectBackupLocation = async () => {
  const selected = await open({ directory: true, multiple: false, title: t('message.backupLocation'), defaultPath: settings.value.backup_location || undefined });
  if (selected && typeof selected === 'string') settings.value.backup_location = selected;
};

const exportData = async () => {
  const selected = await save({ filters: [{ name: 'Database', extensions: ['db'] }], title: t('message.exportData') });
  if (selected && typeof selected === 'string') {
    try {
      await invoke('cmd_export_db', { path: selected });
      alert(t('message.dataExportSuccess'));
    } catch (e) { console.error('Export failed:', e); }
  }
};

const importData = async () => {
  const selected = await open({ directory: false, multiple: false, filters: [{ name: 'Database', extensions: ['db'] }], title: t('message.importData') });
  if (selected && typeof selected === 'string') {
    try {
      await invoke('cmd_import_db', { path: selected });
      alert(t('message.dataImportSuccess'));
    } catch (e) { console.error('Import failed:', e); }
  }
};

// ─── Session Management ──────────────────────────────────────────
const startSession = async () => {
  try {
    const session = await invoke<Session>("cmd_start_session");
    activeSession.value = session;
  } catch (e) { console.error("Failed to start session:", e); }
};

const stopSession = async () => {
  if (!activeSession.value) return;
  try {
    await invoke<Session>("cmd_stop_session", { sessionId: activeSession.value.id });
    activeSession.value = null;
    await refreshDashboard();
  } catch (e) { console.error("Failed to stop session:", e); }
};

const loadActiveSession = async () => {
  try {
    const session = await invoke<Session | null>("cmd_get_active_session");
    activeSession.value = session;
  } catch (e) { console.error("Failed to load active session:", e); }
};

// ─── Dashboard Data ──────────────────────────────────────────────
const refreshDashboard = async () => {
  try {
    const data = await invoke<DashboardData>("cmd_get_dashboard_data");
    dashboardData.value = data;
  } catch (e) { console.error("Failed to refresh dashboard:", e); }
};

// ─── Tracking Control ────────────────────────────────────────────
const loadTrackingStatus = async () => {
  try {
    trackingStatus.value = await invoke<string>("cmd_get_tracking");
  } catch (e) { console.error("Failed to get tracking status:", e); }
};

const setTracking = async (status: string) => {
  try {
    await invoke("cmd_set_tracking", { status });
    // Accumulate pause duration
    if (status === 'paused') {
      pauseStartedAt = Date.now();
    } else if (status === 'running' && pauseStartedAt !== null) {
      pausedSeconds.value += Math.floor((Date.now() - pauseStartedAt) / 1000);
      pauseStartedAt = null;
    } else if (status === 'stopped') {
      pausedSeconds.value = 0;
      pauseStartedAt = null;
    }
    trackingStatus.value = status;
  } catch (e) { console.error("Failed to set tracking:", e); }
};

// ─── Filtered Data ───────────────────────────────────────────────
const getDateRange = () => {
  const to = new Date();
  const from = new Date();
  if (filterType.value === 'daily') {
    // Same day
  } else if (filterType.value === 'weekly') {
    from.setDate(from.getDate() - 7);
  } else if (filterType.value === 'monthly') {
    from.setMonth(from.getMonth() - 1);
  } else if (filterType.value === 'yearly') {
    from.setFullYear(from.getFullYear() - 1);
  } else if (filterType.value === 'custom') {
    return { from: customFromDate.value, to: customToDate.value };
  }
  return { 
    from: from.toISOString().split('T')[0], 
    to: to.toISOString().split('T')[0] 
  };
};

const loadFilteredData = async (append = false) => {
  const { from, to } = getDateRange();
  try {
    if (currentView.value === 'trackings') {
      if (!append) timeLogsOffset.value = 0;
      const data = await invoke<TimeLogEntry[]>('cmd_get_time_logs_range', { from, to, limit: pageSize, offset: timeLogsOffset.value });
      timeLogsList.value = append ? [...timeLogsList.value, ...data] : data;
    } else if (currentView.value === 'urls') {
      if (!append) urlsOffset.value = 0;
      const data = await invoke<UrlEntryFull[]>('cmd_get_urls_range', { from, to, limit: pageSize, offset: urlsOffset.value });
      urlsList.value = append ? [...urlsList.value, ...data] : data;
    } else if (currentView.value === 'screenshots') {
      if (!append) screenshotsOffset.value = 0;
      const data = await invoke<ScreenshotEntry[]>('cmd_get_screenshots_range', { from, to, limit: pageSize, offset: screenshotsOffset.value });
      screenshotsList.value = append ? [...screenshotsList.value, ...data] : data;
    }
  } catch (e) {
    console.error("Failed to load filtered data", e);
  }
};

const loadMoreTrackings = () => { timeLogsOffset.value += pageSize; loadFilteredData(true); };
const loadMoreUrls = () => { urlsOffset.value += pageSize; loadFilteredData(true); };
const loadMoreScreenshots = () => { screenshotsOffset.value += pageSize; loadFilteredData(true); };

// ─── Watchers ────────────────────────────────────────────────────
watch([currentView, filterType, customFromDate, customToDate], () => {
  if (['trackings', 'urls', 'screenshots'].includes(currentView.value)) {
    loadFilteredData(false);
  }
});

watch(settings, async (newVal, oldVal) => {
  saveSettings();
  if (oldVal && newVal.auto_start_on_boot !== oldVal.auto_start_on_boot) {
    try { await invoke("set_autostart", { enabled: newVal.auto_start_on_boot }); }
    catch (error) { console.error("Failed to set autostart:", error); }
  }
}, { deep: true });



// App init — runs once when appScreen transitions to 'app'
let appInitDone = false;
const initApp = async () => {
  if (appInitDone) return;
  appInitDone = true;
  await loadSettings();
  defaultScreenshotDir.value = await invoke("cmd_get_screenshot_dir");
  await loadActiveSession();
  await loadTrackingStatus();
  await refreshDashboard();
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", applyTheme);
  refreshInterval = setInterval(refreshDashboard, 5000) as unknown as number;
  taskbarInterval = setInterval(() => {
    const now = new Date();
    const timeStr = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const status = trackingStatus.value;
    if (activeSession.value && status === 'running') {
      const start = new Date(activeSession.value.start_time);
      const totalElapsed = Math.floor((now.getTime() - start.getTime()) / 1000);
      const netActive = Math.max(0, totalElapsed - pausedSeconds.value);
      document.title = `▶ ${formatTime(netActive)} | ${timeStr} — Time Guardian`;
    } else if (status === 'paused') {
      document.title = `⏸ Paused | ${timeStr} — Time Guardian`;
    } else {
      document.title = `⏹ Stopped | ${timeStr} — Time Guardian`;
    }
  }, 1000) as unknown as number;
  await listen<string>("tracking-status-changed", (event) => {
    const newStatus = event.payload;
    if (newStatus === 'paused' && trackingStatus.value !== 'paused') {
      pauseStartedAt = Date.now();
    } else if (newStatus === 'running' && trackingStatus.value === 'paused' && pauseStartedAt !== null) {
      pausedSeconds.value += Math.floor((Date.now() - pauseStartedAt) / 1000);
      pauseStartedAt = null;
    } else if (newStatus === 'stopped') {
      pausedSeconds.value = 0;
      pauseStartedAt = null;
    }
    trackingStatus.value = newStatus;
  });
};

onUnmounted(() => {
  if (refreshInterval) clearInterval(refreshInterval);
  if (taskbarInterval) clearInterval(taskbarInterval);
});

// ─── Phase 8 Logic ────────────────────────────────────────────────

const loadAppConfig = async () => {
  const cfg = await invoke<AppConfig>('cmd_get_app_config');
  appConfig.value = cfg;
  return cfg;
};

const saveAppConfig = async (cfg: AppConfig) => {
  await invoke('cmd_save_app_config', { cfg });
  appConfig.value = cfg;
};

const tryRestoreSession = async () => {
  const token = localStorage.getItem('tg_session_token');
  if (!token) return false;
  try {
    const user = await invoke<AuthUser>('cmd_validate_session', { token });
    currentUser.value = user;
    sessionToken.value = token;
    return true;
  } catch {
    localStorage.removeItem('tg_session_token');
    return false;
  }
};

const doLogin = async () => {
  loginError.value = '';
  loginLoading.value = true;
  try {
    const result = await invoke<LoginResult>('cmd_login', {
      payload: { username: loginUsername.value.trim(), password: loginPassword.value }
    });
    currentUser.value = result.user;
    sessionToken.value = result.token;
    localStorage.setItem('tg_session_token', result.token);
    appScreen.value = 'app';
  } catch (e: any) {
    loginError.value = e?.toString() ?? 'Login failed';
  } finally {
    loginLoading.value = false;
  }
};

const doLogout = async () => {
  if (sessionToken.value) {
    try { await invoke('cmd_logout', { token: sessionToken.value }); } catch {}
    localStorage.removeItem('tg_session_token');
  }
  currentUser.value = null;
  sessionToken.value = '';
  currentView.value = 'dashboard';
  appScreen.value = 'login';
};

const wizardValidateStep2 = () => {
  wizardError.value = '';
  if (!wizardCompanyName.value.trim()) { wizardError.value = 'Company name required'; return; }
  if (!wizardAdminUsername.value.trim()) { wizardError.value = 'Admin username required'; return; }
  if (!wizardAdminDisplay.value.trim()) { wizardError.value = 'Admin display name required'; return; }
  if (wizardAdminPassword.value.length < 6) { wizardError.value = 'Password must be 6+ characters'; return; }
  if (wizardAdminPassword.value !== wizardConfirmPassword.value) { wizardError.value = 'Passwords do not match'; return; }
  wizardStep.value = 3;
};

const wizardFinish = async () => {
  wizardError.value = '';
  wizardLoading.value = true;
  try {
    if (wizardMode.value === 'multi_user') {
      const result = await invoke<LoginResult>('cmd_register_company', {
        payload: {
          company_name: wizardCompanyName.value.trim(),
          admin_username: wizardAdminUsername.value.trim(),
          admin_display_name: wizardAdminDisplay.value.trim(),
          admin_password: wizardAdminPassword.value,
        }
      });
      currentUser.value = result.user;
      sessionToken.value = result.token;
      localStorage.setItem('tg_session_token', result.token);
    }
    const cfg: AppConfig = { mode: wizardMode.value, setup_done: true };
    await saveAppConfig(cfg);
    appScreen.value = wizardMode.value === 'single_user' ? 'app' : 'app';
  } catch (e: any) {
    wizardError.value = e?.toString() ?? 'Setup failed';
  } finally {
    wizardLoading.value = false;
  }
};

const loadAdminData = async () => {
  if (!isAdmin.value || !currentUser.value) return;
  try {
    const [users, stats] = await Promise.all([
      invoke<AuthUser[]>('cmd_get_company_users', { companyId: currentUser.value.company_id }),
      invoke<UserProductivityStat[]>('cmd_get_admin_stats', { companyId: currentUser.value.company_id }),
    ]);
    adminUsers.value = users;
    adminStats.value = stats;
  } catch (e) { console.error('Failed to load admin data', e); }
};

const doCreateUser = async () => {
  createUserError.value = '';
  createUserLoading.value = true;
  try {
    const created = await invoke<AuthUser>('cmd_create_user', {
      companyId: currentUser.value?.company_id,
      payload: newUser.value,
    });
    adminUsers.value.push(created);
    newUser.value = { username: '', display_name: '', password: '', role: 'employee' };
    showCreateUser.value = false;
  } catch (e: any) {
    createUserError.value = e?.toString() ?? 'Failed';
  } finally {
    createUserLoading.value = false;
  }
};

// Boot sequence
onMounted(async () => {
  // Apply stored theme BEFORE showing any screen (avoids flash)
  try {
    const s = await invoke<Settings>('get_settings');
    settings.value = s;
    locale.value = s.language;
    applyTheme();
  } catch {}

  const cfg = await loadAppConfig();
  pendingMode.value = cfg.mode;
  if (!cfg.setup_done) {
    appScreen.value = 'wizard';
  } else if (cfg.mode === 'multi_user') {
    const restored = await tryRestoreSession();
    appScreen.value = restored ? 'app' : 'login';
  } else {
    appScreen.value = 'app';
  }
});

watch(appScreen, (s) => {
  if (s === 'app') initApp();
});

watch(currentView, (v) => {
  if (v === 'admin') loadAdminData();
});

const doResetApp = async () => {
  if (!confirm(t('message.resetAppConfirm'))) return;
  try {
    await invoke('cmd_reset_app');
    localStorage.removeItem('tg_session_token');
    currentUser.value = null;
    sessionToken.value = '';
    appConfig.value = { mode: 'single_user', setup_done: false };
    wizardStep.value = 1;
    wizardMode.value = 'single_user';
    wizardCompanyName.value = '';
    wizardAdminUsername.value = '';
    wizardAdminDisplay.value = '';
    wizardAdminPassword.value = '';
    wizardConfirmPassword.value = '';
    wizardError.value = '';
    pendingMode.value = 'single_user';
    appScreen.value = 'wizard';
  } catch (e: any) {
    alert('Reset failed: ' + (e?.toString() ?? 'unknown error'));
  }
};

const doChangeMode = async () => {
  if (!appConfig.value) return;
  const newCfg: AppConfig = { ...appConfig.value, mode: pendingMode.value };
  // If switching to multi_user and not set up yet — run wizard
  if (pendingMode.value === 'multi_user') {
    await doResetApp();
    return;
  }
  // Single user: just save config, logout if needed
  await saveAppConfig(newCfg);
  if (currentUser.value) {
    await doLogout();
  }
};
</script>

<template>
  <!-- Loading -->
  <div v-if="appScreen === 'loading'" class="fullscreen-center">
    <div class="spinner"></div>
    <p class="loading-text">Loading Time Guardian...</p>
  </div>

  <!-- First-Run Wizard -->
  <div v-else-if="appScreen === 'wizard'" class="fullscreen-center wizard-bg">
    <div class="wizard-card">
      <div class="wizard-logo">
        <img src="/favicon.png" width="52" height="52" />
        <h1>{{ t('message.wizardWelcome') }}</h1>
        <p>{{ t('message.wizardSubtitle') }}</p>
      </div>

      <!-- Step 1: Mode -->
      <div v-if="wizardStep === 1" class="wizard-step">
        <h2>{{ t('message.wizardChooseMode') }}</h2>
        <div class="mode-cards">
          <div :class="['mode-card', { 'mode-card-active': wizardMode === 'single_user' }]" @click="wizardMode = 'single_user'">
            <span class="mode-icon">👤</span>
            <strong>{{ t('message.modeSingleUser') }}</strong>
            <p>{{ t('message.wizardSingleDesc') }}</p>
          </div>
          <div :class="['mode-card', { 'mode-card-active': wizardMode === 'multi_user' }]" @click="wizardMode = 'multi_user'">
            <span class="mode-icon">🏢</span>
            <strong>{{ t('message.modeMultiUser') }}</strong>
            <p>{{ t('message.wizardTeamDesc') }}</p>
          </div>
        </div>
        <button class="btn-wizard-next" @click="wizardStep = wizardMode === 'single_user' ? 3 : 2">{{ t('message.wizardContinue') }}</button>
      </div>

      <!-- Step 2: Company + Admin setup (multi_user only) -->
      <div v-if="wizardStep === 2" class="wizard-step">
        <h2>{{ t('message.wizardSetupCompany') }}</h2>
        <div class="wizard-form">
          <label>{{ t('message.wizardCompanyName') }}</label>
          <input type="text" v-model="wizardCompanyName" :placeholder="t('message.wizardCompanyName')" />
          <label>{{ t('message.wizardAdminUsername') }}</label>
          <input type="text" v-model="wizardAdminUsername" placeholder="admin" />
          <label>{{ t('message.wizardAdminDisplay') }}</label>
          <input type="text" v-model="wizardAdminDisplay" placeholder="John Doe" />
          <label>{{ t('message.wizardAdminPassword') }}</label>
          <input type="password" v-model="wizardAdminPassword" :placeholder="t('message.wizardAdminPassword')" />
          <label>{{ t('message.wizardConfirmPassword') }}</label>
          <input type="password" v-model="wizardConfirmPassword" :placeholder="t('message.wizardConfirmPassword')" />
        </div>
        <div v-if="wizardError" class="wizard-error">{{ wizardError }}</div>
        <div class="wizard-actions">
          <button class="btn-wizard-back" @click="wizardStep = 1; wizardError = ''">{{ t('message.wizardBack') }}</button>
          <button class="btn-wizard-next" :disabled="wizardLoading" @click="wizardValidateStep2">{{ t('message.wizardNext') }}</button>
        </div>
      </div>

      <!-- Step 3: Confirm -->
      <div v-if="wizardStep === 3" class="wizard-step">
        <h2>{{ t('message.wizardAllSet') }}</h2>
        <div class="confirm-summary">
          <div class="confirm-row"><span>{{ t('message.wizardMode') }}</span><strong>{{ wizardMode === 'single_user' ? t('message.modeSingleUser') : t('message.modeMultiUser') }}</strong></div>
          <div v-if="wizardMode === 'multi_user'" class="confirm-row"><span>{{ t('message.wizardCompany') }}</span><strong>{{ wizardCompanyName }}</strong></div>
          <div v-if="wizardMode === 'multi_user'" class="confirm-row"><span>{{ t('message.wizardAdmin') }}</span><strong>{{ wizardAdminUsername }}</strong></div>
        </div>
        <div v-if="wizardError" class="wizard-error">{{ wizardError }}</div>
        <div class="wizard-actions">
          <button class="btn-wizard-back" @click="wizardStep = wizardMode === 'single_user' ? 1 : 2; wizardError = ''">{{ t('message.wizardBack') }}</button>
          <button class="btn-wizard-next" :disabled="wizardLoading" @click="wizardFinish">{{ wizardLoading ? t('message.wizardLaunching') : t('message.wizardLaunch') }}</button>
        </div>
      </div>
    </div>
  </div>

  <!-- Login Screen (Multi-User mode) -->
  <div v-else-if="appScreen === 'login'" class="fullscreen-center login-bg">
    <div class="login-card">
      <div class="login-logo">
        <img src="/favicon.png" width="52" height="52" />
        <h1>{{ t('message.loginTitle') }}</h1>
        <p>{{ t('message.loginSubtitle') }}</p>
      </div>
      <div class="login-form">
        <label>{{ t('message.loginUsername') }}</label>
        <input type="text" v-model="loginUsername" :placeholder="t('message.loginUsername')" @keyup.enter="doLogin" />
        <label>{{ t('message.loginPassword') }}</label>
        <input type="password" v-model="loginPassword" placeholder="••••••••" @keyup.enter="doLogin" />
        <div v-if="loginError" class="login-error">{{ loginError }}</div>
        <button class="btn-login" :disabled="loginLoading" @click="doLogin">
          {{ loginLoading ? t('message.loginSigningIn') : t('message.loginSignIn') }}
        </button>
      </div>
    </div>
  </div>

  <!-- Main App -->
  <div v-else-if="appScreen === 'app'" class="app-layout">
    <aside class="sidebar">
      <div class="logo">
        <img src="/favicon.png" alt="Time Guardian" width="40" height="40" />
        <h2>Time Guardian</h2>
      </div>

      <nav>
        <button :class="{ active: currentView === 'dashboard' }" @click="currentView = 'dashboard'">
          📊 {{ t('message.dashboard') }}
        </button>
        <button :class="{ active: currentView === 'trackings' }" @click="currentView = 'trackings'">
          ⏱️ {{ t('message.trackings') }}
        </button>
        <button :class="{ active: currentView === 'urls' }" @click="currentView = 'urls'">
          🌐 {{ t('message.urls') }}
        </button>
        <button :class="{ active: currentView === 'productivity' }" @click="currentView = 'productivity'">
          📈 Productivity
        </button>
        <button :class="{ active: currentView === 'screenshots' }" @click="currentView = 'screenshots'">
          📸 {{ t('message.screenshots') }}
        </button>
        <!-- Admin Dashboard — only visible to admins in multi-user mode -->
        <button v-if="isMultiUser && isAdmin" :class="{ active: currentView === 'admin' }" @click="currentView = 'admin'">
          🛡️ Admin
        </button>
        <button :class="{ active: currentView === 'settings' }" @click="currentView = 'settings'">
          ⚙️ {{ t('message.settings') }}
        </button>
      </nav>

      <!-- Tracking Control -->
      <div class="tracking-control">
        <div class="tracking-status" :class="trackingStatus">
          <span v-if="trackingStatus === 'running'" class="status-dot running"></span>
          <span v-else-if="trackingStatus === 'paused'" class="status-dot paused"></span>
          <span v-else class="status-dot stopped"></span>
          <span class="status-label">
            {{ trackingStatus === 'running' ? t('message.trackingRunning') : trackingStatus === 'paused' ? t('message.trackingPaused') : t('message.trackingStopped') }}
          </span>
        </div>
        <div class="tracking-buttons">
          <button v-if="trackingStatus === 'running'" class="btn-tracking btn-pause" @click="setTracking('paused')">
            ⏸ {{ t('message.pauseTracking') }}
          </button>
          <button v-if="trackingStatus === 'running'" class="btn-tracking btn-stop-track" @click="setTracking('stopped')">
            ⏹ {{ t('message.stopTracking') }}
          </button>
          <button v-if="trackingStatus === 'paused'" class="btn-tracking btn-resume" @click="setTracking('running')">
            ▶ {{ t('message.resumeTracking') }}
          </button>
          <button v-if="trackingStatus === 'paused'" class="btn-tracking btn-stop-track" @click="setTracking('stopped')">
            ⏹ {{ t('message.stopTracking') }}
          </button>
          <button v-if="trackingStatus === 'stopped'" class="btn-tracking btn-start-track" @click="setTracking('running')">
            ▶ {{ t('message.startTracking') }}
          </button>
        </div>
      </div>

      <!-- Session Control in Sidebar -->
      <div class="session-control">
        <div v-if="activeSession" class="session-active">
          <span class="pulse-dot"></span>
          <span>{{ t('message.sessionActive') }}</span>
          <button class="btn-stop" @click="stopSession">⏹ {{ t('message.stopSession') }}</button>
        </div>
        <div v-else class="session-inactive">
          <button class="btn-start" @click="startSession">▶ {{ t('message.startSession') }}</button>
        </div>
      </div>
      
      <div style="flex: 1;"></div> <!-- spacer -->

      <!-- User badge + logout (multi-user only) -->
      <div v-if="isMultiUser && currentUser" class="user-badge">
        <span class="user-avatar">{{ currentUser.display_name.charAt(0).toUpperCase() }}</span>
        <div class="user-info">
          <span class="user-name">{{ currentUser.display_name }}</span>
          <span :class="['role-badge', currentUser.role === 'admin' ? 'role-admin' : 'role-emp']">{{ currentUser.role }}</span>
        </div>
        <button class="btn-logout" @click="doLogout" title="Sign out">
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"></path><polyline points="16 17 21 12 16 7"></polyline><line x1="21" y1="12" x2="9" y2="12"></line></svg>
        </button>
      </div>
    </aside>

    <main class="main-content">
      <!-- DASHBOARD VIEW -->
      <div v-if="currentView === 'dashboard'" class="view-dashboard">
        <header class="view-header">
          <h1>{{ t('message.todaySummary') }}</h1>
          <button class="btn-browse" @click="refreshDashboard" style="padding: 6px 12px; font-size:0.9rem;">
            🔄 Refresh
          </button>
        </header>

        <div class="summary-cards">
          <div class="card premium-card">
            <h3>{{ t('message.activeTime') }}</h3>
            <p class="big-stat">{{ formatTime(dashboardData.total_active_seconds) }}</p>
          </div>
          <div class="card premium-card">
            <h3>{{ t('message.idleTime') }}</h3>
            <p class="big-stat idle">{{ formatTime(dashboardData.total_idle_seconds) }}</p>
          </div>
          <div class="card premium-card">
            <h3>{{ t('message.sessionTime') }}</h3>
            <p class="big-stat session">{{ formatTime(dashboardData.session_seconds) }}</p>
          </div>
          <div class="card premium-card">
            <h3>{{ t('message.totalTime') }}</h3>
            <p class="big-stat total">{{ formatTime(dashboardData.total_active_seconds + dashboardData.total_idle_seconds) }}</p>
          </div>
        </div>

        <!-- App Usage Table -->
        <div class="section-block">
          <h2>{{ t('message.topApps') }}</h2>
          <div v-if="dashboardData.app_stats.length > 0" class="app-table">
            <div class="app-row header-row">
              <span>{{ t('message.appName') }}</span>
              <span>{{ t('message.timeSpent') }}</span>
              <span>{{ t('message.switches') }}</span>
              <span>%</span>
            </div>
            <div v-for="app in dashboardData.app_stats" :key="app.app_name" class="app-row">
              <span class="app-name">
                <span class="app-icon">{{ appIcon(app.app_name) }}</span>
                {{ app.app_name }}
              </span>
              <span class="app-time">{{ formatTime(app.total_seconds) }}</span>
              <span class="app-switches">{{ app.session_count }}</span>
              <span class="app-percent">
                <div class="bar-bg">
                  <div class="bar-fill" :style="{ width: percentage(app.total_seconds) + '%' }"></div>
                </div>
                {{ percentage(app.total_seconds) }}%
              </span>
            </div>
          </div>
          <div v-else class="empty-state">
            <p>{{ t('message.noData') }}</p>
          </div>
        </div>

        <!-- Recent URLs -->
        <div class="section-block">
          <h2>🌐 {{ t('message.recentUrls') }}</h2>
          <div v-if="dashboardData.recent_urls.length > 0" class="url-list">
            <div v-for="(entry, i) in dashboardData.recent_urls" :key="i" class="url-row">
              <span class="url-time">{{ formatTimestamp(entry.timestamp) }}</span>
              <span class="url-text">{{ entry.url }}</span>
            </div>
          </div>
          <div v-else class="empty-state">
            <p>{{ t('message.noUrls') }}</p>
          </div>
        </div>
      </div>

      <!-- TRACKINGS VIEW -->
      <div v-if="currentView === 'trackings'" class="view-trackings">
        <header class="view-header">
          <h1>{{ t('message.trackings') }}</h1>
          <div class="filter-controls">
            <button class="btn-browse" @click="loadFilteredData(false)" style="padding: 6px 12px; font-size:0.9rem;">
              🔄 Refresh
            </button>
            <select v-model="filterType">
              <option value="daily">{{ t('message.filterDaily') }}</option>
              <option value="weekly">{{ t('message.filterWeekly') }}</option>
              <option value="monthly">{{ t('message.filterMonthly') }}</option>
              <option value="yearly">{{ t('message.filterYearly') }}</option>
              <option value="custom">{{ t('message.filterCustom') }}</option>
            </select>
            <template v-if="filterType === 'custom'">
              <input type="date" v-model="customFromDate" />
              <span> - </span>
              <input type="date" v-model="customToDate" />
            </template>
          </div>
        </header>

        <div class="app-table">
          <div class="app-row header-row" style="grid-template-columns: 2fr 3fr 1fr 1fr 1fr;">
            <span>{{ t('message.appName') }}</span>
            <span>Title</span>
            <span>Start</span>
            <span>End</span>
            <span>{{ t('message.timeSpent') }}</span>
          </div>
          <div v-for="log in timeLogsList" :key="log.id" class="app-row" style="grid-template-columns: 2fr 3fr 1fr 1fr 1fr;">
            <span class="app-name">
              <span class="app-icon">{{ appIcon(log.app_name) }}</span>
              {{ log.app_name }}
            </span>
            <span class="url-text" :title="log.window_title">{{ log.window_title }}</span>
            <span class="url-time">{{ formatTimestamp(log.start_time) }}</span>
            <span class="url-time">{{ log.end_time ? formatTimestamp(log.end_time) : '-' }}</span>
            <span class="app-time">{{ formatTime(log.duration) }}</span>
          </div>
          <div v-if="timeLogsList.length === 0" class="empty-state">
            <p>{{ t('message.noData') }}</p>
          </div>
          <div v-else-if="timeLogsList.length % pageSize === 0" class="load-more-container">
            <button class="btn-start" @click="loadMoreTrackings">Load More</button>
          </div>
        </div>
      </div>

      <!-- URLS VIEW -->
      <div v-if="currentView === 'urls'" class="view-urls">
        <header class="view-header">
          <h1>{{ t('message.urls') }}</h1>
          <div class="filter-controls">
            <button class="btn-browse" @click="loadFilteredData(false)" style="padding: 6px 12px; font-size:0.9rem;">
              🔄 Refresh
            </button>
            <select v-model="filterType">
              <option value="daily">{{ t('message.filterDaily') }}</option>
              <option value="weekly">{{ t('message.filterWeekly') }}</option>
              <option value="monthly">{{ t('message.filterMonthly') }}</option>
              <option value="yearly">{{ t('message.filterYearly') }}</option>
              <option value="custom">{{ t('message.filterCustom') }}</option>
            </select>
            <template v-if="filterType === 'custom'">
              <input type="date" v-model="customFromDate" />
              <span> - </span>
              <input type="date" v-model="customToDate" />
            </template>
          </div>
        </header>

        <div class="url-list">
          <div v-for="entry in urlsList" :key="entry.id" class="url-row">
            <span class="url-time">{{ formatTimestamp(entry.timestamp) }}</span>
            <span class="url-text">{{ entry.url }}</span>
          </div>
          <div v-if="urlsList.length === 0" class="empty-state">
            <p>{{ t('message.noUrls') }}</p>
          </div>
          <div v-else-if="urlsList.length % pageSize === 0" class="load-more-container">
            <button class="btn-start" @click="loadMoreUrls">Load More</button>
          </div>
        </div>
      </div>

      <!-- SCREENSHOTS VIEW -->
      <div v-if="currentView === 'screenshots'" class="view-screenshots">
        <header class="view-header">
          <h1>{{ t('message.screenshots') }}</h1>
          <div class="filter-controls">
            <button class="btn-browse" @click="loadFilteredData(false)" style="padding: 6px 12px; font-size:0.9rem;">
              🔄 Refresh
            </button>
            <select v-model="filterType">
              <option value="daily">{{ t('message.filterDaily') }}</option>
              <option value="weekly">{{ t('message.filterWeekly') }}</option>
              <option value="monthly">{{ t('message.filterMonthly') }}</option>
              <option value="yearly">{{ t('message.filterYearly') }}</option>
              <option value="custom">{{ t('message.filterCustom') }}</option>
            </select>
            <template v-if="filterType === 'custom'">
              <input type="date" v-model="customFromDate" />
              <span> - </span>
              <input type="date" v-model="customToDate" />
            </template>
          </div>
        </header>

        <div class="screenshots-grid">
          <div v-for="shot in screenshotsList" :key="shot.id" class="screenshot-card">
            <img :src="convertFileSrc(shot.file_path)" alt="Screenshot" loading="lazy" />
            <div class="screenshot-info">
              <span>{{ formatTimestamp(shot.captured_at) }}</span>
            </div>
          </div>
          <div v-if="screenshotsList.length === 0" class="empty-state" style="grid-column: 1 / -1;">
            <p>{{ t('message.noData') }}</p>
          </div>
          <div v-else-if="screenshotsList.length % pageSize === 0" class="load-more-container" style="grid-column: 1 / -1;">
            <button class="btn-start" @click="loadMoreScreenshots">Load More</button>
          </div>
        </div>
      </div>

      <!-- PRODUCTIVITY VIEW -->
      <div v-if="currentView === 'productivity'" class="view-productivity">
        <header>
          <h1>Productivity</h1>
        </header>

        <div class="productivity-grid" style="display: grid; grid-template-columns: 1fr 2fr; gap: 24px; padding: 16px;">
          <div class="card chart-container" style="height: 300px;">
            <h3>Productivity Breakdown (Today)</h3>
            <Pie :data="productivityChartData" :options="productivityChartOptions" />
          </div>

          <div class="card">
            <h3>App Categories</h3>
            <div class="table-responsive">
              <table>
                <thead>
                  <tr>
                    <th>Application</th>
                    <th>Time Spent</th>
                    <th>Category</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="stat in dashboardData.app_stats" :key="stat.app_name">
                    <td>{{ stat.app_name }}</td>
                    <td>{{ formatTime(stat.total_seconds) }}</td>
                    <td>
                      <select v-model="stat.category" @change="updateCategory(stat.app_name, stat.category)">
                        <option value="productive">Productive</option>
                        <option value="neutral">Neutral</option>
                        <option value="unproductive">Unproductive</option>
                      </select>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>

      <!-- SETTINGS VIEW -->
      <!-- SETTINGS VIEW -->
      <div v-if="currentView === 'settings'" class="view-settings">
        <header>
          <h1>{{ t('message.settings') }}</h1>
        </header>
        <div class="settings-grid">
          <div class="card setting-card">
            <label>{{ t('message.language') }}</label>
            <select v-model="settings.language">
              <option value="en">English</option>
              <option value="bn">বাংলা</option>
            </select>
          </div>
          <div class="card setting-card">
            <label>{{ t('message.theme') }}</label>
            <select v-model="settings.theme">
              <option value="light">{{ t('message.light') }}</option>
              <option value="dark">{{ t('message.dark') }}</option>
              <option value="system">{{ t('message.system') }}</option>
            </select>
          </div>
          <div class="card setting-card">
            <label>{{ t('message.autoStart') }}</label>
            <input type="checkbox" v-model="settings.auto_start_on_boot" />
          </div>
          <div class="card setting-card">
            <label>{{ t('message.screenshotInterval') }}</label>
            <input type="number" v-model="settings.screenshot_interval" min="1" />
          </div>
          <div class="card setting-card">
            <label>{{ t('message.screenshotLocation') }}</label>
            <div class="input-with-button">
              <input type="text" v-model="settings.screenshot_location" :placeholder="defaultScreenshotDir" />
              <button class="btn-browse" @click="selectScreenshotLocation">📁</button>
            </div>
          </div>
          <div class="card setting-card">
            <label>{{ t('message.backupFrequency') }}</label>
            <select v-model="settings.backup_frequency">
              <option value="never">{{ t('message.never') }}</option>
              <option value="daily">{{ t('message.daily') }}</option>
              <option value="weekly">{{ t('message.weekly') }}</option>
            </select>
          </div>
          <div class="card setting-card">
            <label>{{ t('message.backupLocation') }}</label>
            <div class="input-with-button">
              <input type="text" v-model="settings.backup_location" placeholder="/backup/path" />
              <button class="btn-browse" @click="selectBackupLocation">📁</button>
            </div>
          </div>
          <div class="card setting-card">
            <label>{{ t('message.exportData') }}</label>
            <button class="btn-browse" style="width:100%; padding: 8px; margin-top: 8px;" @click="exportData">{{ t('message.exportData') }}</button>
          </div>
          <div class="card setting-card">
            <label>{{ t('message.importData') }}</label>
            <button class="btn-browse" style="width:100%; padding: 8px; margin-top: 8px;" @click="importData">{{ t('message.importData') }}</button>
          </div>

          <!-- App Mode -->
          <div class="card setting-card setting-card-wide">
            <label>{{ t('message.appMode') }}</label>
            <div class="mode-toggle-row">
              <span class="mode-current-badge" :class="appConfig?.mode === 'multi_user' ? 'mode-badge-multi' : 'mode-badge-single'">
                {{ appConfig?.mode === 'multi_user' ? t('message.modeMultiUser') : t('message.modeSingleUser') }}
              </span>
              <select v-model="pendingMode" class="mode-select">
                <option value="single_user">{{ t('message.modeSingleUser') }}</option>
                <option value="multi_user">{{ t('message.modeMultiUser') }}</option>
              </select>
              <button class="btn-change-mode" @click="doChangeMode" :disabled="pendingMode === appConfig?.mode">{{ t('message.changeMode') }}</button>
            </div>
          </div>

          <!-- Danger Zone -->
          <div class="card setting-card setting-card-wide danger-card">
            <label class="danger-label">⚠️ {{ t('message.dangerZone') }}</label>
            <p class="danger-desc">{{ t('message.resetAppConfirm') }}</p>
            <button class="btn-danger" @click="doResetApp">{{ t('message.resetApp') }}</button>
          </div>
        </div>
      </div>

      <!-- ADMIN DASHBOARD VIEW -->
      <div v-if="currentView === 'admin'" class="view-admin">
        <header class="view-header">
          <h1>🛡️ Admin Dashboard</h1>
          <button class="btn-browse" @click="loadAdminData" style="padding: 6px 12px; font-size:0.9rem;">
            🔄 Refresh
          </button>
        </header>

        <!-- Productivity Stats Table -->
        <div class="section-block">
          <h2>Today's Team Productivity</h2>
          <div class="app-table">
            <div class="app-row header-row" style="grid-template-columns: 2fr 1fr 1fr 1fr;">
              <span>Employee</span><span>Active Time</span><span>Sessions</span><span>Username</span>
            </div>
            <div v-for="stat in adminStats" :key="stat.user_id" class="app-row" style="grid-template-columns: 2fr 1fr 1fr 1fr;">
              <span class="app-name">👤 {{ stat.display_name }}</span>
              <span class="app-time">{{ formatTime(stat.total_active_seconds) }}</span>
              <span class="app-switches">{{ stat.session_count }}</span>
              <span class="app-time" style="font-size:0.85rem; color: var(--text-muted);">{{ stat.username }}</span>
            </div>
            <div v-if="adminStats.length === 0" class="empty-state"><p>No data today</p></div>
          </div>
        </div>

        <!-- User Management -->
        <div class="section-block">
          <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:16px;">
            <h2>Team Members</h2>
            <button class="btn-browse" @click="showCreateUser = !showCreateUser" style="padding: 8px 16px; font-size:0.9rem;">+ Add User</button>
          </div>

          <!-- Create User Form -->
          <div v-if="showCreateUser" class="card" style="margin-bottom: 20px; padding: 24px;">
            <h3 style="margin-bottom:16px;">New Team Member</h3>
            <div class="wizard-form">
              <label>Username</label>
              <input type="text" v-model="newUser.username" placeholder="jane_doe" />
              <label>Display Name</label>
              <input type="text" v-model="newUser.display_name" placeholder="Jane Doe" />
              <label>Password</label>
              <input type="password" v-model="newUser.password" placeholder="Temporary password" />
              <label>Role</label>
              <select v-model="newUser.role">
                <option value="employee">Employee</option>
                <option value="admin">Admin</option>
              </select>
            </div>
            <div v-if="createUserError" class="wizard-error" style="margin-top:12px;">{{ createUserError }}</div>
            <div style="display:flex; gap:12px; margin-top:16px;">
              <button class="btn-stop" style="flex:1;" @click="showCreateUser = false">Cancel</button>
              <button class="btn-start" style="flex:2;" :disabled="createUserLoading" @click="doCreateUser">
                {{ createUserLoading ? 'Creating...' : 'Create User' }}
              </button>
            </div>
          </div>

          <!-- Users Table -->
          <div class="app-table">
            <div class="app-row header-row" style="grid-template-columns: 2fr 1.5fr 1fr;">
              <span>Name</span><span>Username</span><span>Role</span>
            </div>
            <div v-for="user in adminUsers" :key="user.id" class="app-row" style="grid-template-columns: 2fr 1.5fr 1fr;">
              <span class="app-name">{{ user.display_name }}</span>
              <span class="url-text">{{ user.username }}</span>
              <span>
                <span :class="['role-badge', user.role === 'admin' ? 'role-admin' : 'role-emp']">{{ user.role }}</span>
              </span>
            </div>
            <div v-if="adminUsers.length === 0" class="empty-state"><p>No team members yet</p></div>
          </div>
        </div>
      </div>

    </main>
  </div>
</template>

<style>
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&display=swap');

:root {
  --bg-color: #f4f6f8;
  --sidebar-bg: #ffffff;
  --text-color: #1a1a2e;
  --text-muted: #64748b;
  --card-bg: #ffffff;
  --border-color: #e2e8f0;
  --accent: #4f46e5;
  --accent-hover: #4338ca;
  --success: #10b981;
  --danger: #ef4444;
  --warning: #f59e0b;
  --bar-bg: #e2e8f0;
}

:root.dark {
  --bg-color: #09090b;
  --sidebar-bg: #13141a;
  --text-color: #f8fafc;
  --text-muted: #94a3b8;
  --card-bg: #13141a;
  --border-color: #272a35;
  --accent: #6366f1;
  --accent-hover: #818cf8;
  --success: #10b981;
  --danger: #ef4444;
  --warning: #f59e0b;
  --bar-bg: #272a35;
}

* { box-sizing: border-box; margin: 0; }

body {
  margin: 0;
  background-color: var(--bg-color);
  color: var(--text-color);
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
  transition: background-color 0.3s, color 0.3s;
}

/* ─── Slim Modern Scrollbar ─────────────────────────────────────── */
* {
  scrollbar-width: thin;
  scrollbar-color: var(--border-color) transparent;
}
*::-webkit-scrollbar { width: 5px; height: 5px; }
*::-webkit-scrollbar-track { background: transparent; }
*::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 999px;
  transition: background 0.2s;
}
*::-webkit-scrollbar-thumb:hover { background: var(--text-muted); }
*::-webkit-scrollbar-corner { background: transparent; }

.app-layout { display: flex; height: 100vh; }

/* ─── Sidebar ──────────────────────────────────────────────────── */
.sidebar {
  width: 260px;
  background-color: var(--sidebar-bg);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
}

.logo {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 24px;
  font-weight: 700;
  color: var(--accent);
}
.logo h2 { font-size: 1.1rem; }
.logo img { border-radius: 8px; }

nav { display: flex; flex-direction: column; padding: 0 16px; gap: 4px; }

nav button {
  background: transparent;
  border: none;
  padding: 12px 16px;
  border-radius: 8px;
  text-align: left;
  font-size: 0.95rem;
  font-weight: 500;
  color: var(--text-color);
  cursor: pointer;
  transition: all 0.2s ease;
}
nav button:hover { background: var(--border-color); }
nav button.active { background: var(--accent); color: white; }

/* Session Control */
.session-control {
  margin: 0 16px 24px;
}
.session-active {
  display: flex;
  flex-direction: column;
  gap: 8px;
  align-items: center;
  text-align: center;
  color: var(--success);
  font-weight: 600;
  font-size: 0.9rem;
}
.session-inactive { text-align: center; }

.pulse-dot {
  width: 12px; height: 12px;
  background: var(--success);
  border-radius: 50%;
  animation: pulse 1.5s infinite;
  display: inline-block;
}
@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.5; transform: scale(1.3); }
}

.btn-start, .btn-stop {
  width: 100%;
  padding: 10px;
  border: none;
  border-radius: 8px;
  font-weight: 600;
  font-size: 0.9rem;
  cursor: pointer;
  transition: all 0.2s ease;
}
.btn-start { background: var(--success); color: white; }
.btn-start:hover { filter: brightness(1.1); }
.btn-stop { background: var(--danger); color: white; margin-top: 4px; }
.btn-stop:hover { filter: brightness(1.1); }

/* ─── Main Content ─────────────────────────────────────────────── */
.main-content { flex: 1; padding: 32px 40px; overflow-y: auto; }

header h1 { margin-top: 0; margin-bottom: 24px; font-size: 1.8rem; font-weight: 700; }

.card {
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 24px;
  box-shadow: 0 1px 3px rgba(0,0,0,0.04);
}

/* Summary Cards */
.summary-cards {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin-bottom: 32px;
}
.premium-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 20px;
  background: linear-gradient(135deg, var(--card-bg) 0%, rgba(99, 102, 241, 0.04) 100%);
}
.premium-card h3 {
  margin: 0 0 8px 0;
  font-size: 0.85rem;
  font-weight: 500;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.big-stat { font-size: 2rem; font-weight: 800; color: var(--accent); margin: 0; }
.big-stat.idle { color: var(--warning); }
.big-stat.session { color: var(--success); }
.big-stat.total { color: var(--text-color); }

/* Section Blocks */
.section-block { margin-bottom: 32px; }
.section-block h2 { font-size: 1.2rem; font-weight: 700; margin-bottom: 16px; }

/* App Table */
.app-table {
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  overflow: hidden;
}
.app-row {
  display: grid;
  grid-template-columns: 2fr 1fr 0.5fr 1.5fr;
  padding: 12px 20px;
  align-items: center;
  border-bottom: 1px solid var(--border-color);
  font-size: 0.9rem;
}
.app-row:last-child { border-bottom: none; }
.header-row {
  background: var(--bg-color);
  font-weight: 600;
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
}
.app-name { display: flex; align-items: center; gap: 8px; font-weight: 500; }
.app-icon { font-size: 1.2rem; }
.app-time { font-weight: 600; color: var(--accent); }
.app-switches { text-align: center; color: var(--text-muted); }
.app-percent { display: flex; align-items: center; gap: 8px; font-size: 0.85rem; color: var(--text-muted); }
.bar-bg { flex: 1; height: 6px; background: var(--bar-bg); border-radius: 3px; overflow: hidden; }
.bar-fill { height: 100%; background: var(--accent); border-radius: 3px; transition: width 0.5s ease; }

/* URL List */
.url-list {
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  overflow: hidden;
}
.url-row {
  display: flex;
  gap: 16px;
  padding: 10px 20px;
  border-bottom: 1px solid var(--border-color);
  font-size: 0.88rem;
}
.url-row:last-child { border-bottom: none; }
.url-time { color: var(--text-muted); font-size: 0.85rem; min-width: 80px; }
.url-text { color: var(--accent); font-weight: 500; word-break: break-all; }

.load-more-container {
  padding: 16px;
  text-align: center;
  border-top: 1px solid var(--border-color);
}
.load-more-container button { max-width: 200px; }

.empty-state {
  padding: 40px;
  text-align: center;
  background: var(--card-bg);
  border: 1px dashed var(--border-color);
  border-radius: 12px;
  color: var(--text-muted);
}

/* ─── Settings ─────────────────────────────────────────────────── */
.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 20px;
}
.setting-card { display: flex; flex-direction: column; gap: 12px; }
.setting-card label { font-weight: 600; font-size: 0.95rem; }

select, input[type="text"], input[type="password"], input[type="number"] {
  width: 100%;
  padding: 10px;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background: var(--bg-color);
  color: var(--text-color);
  font-size: 1rem;
  transition: all 0.2s ease;
}
select:focus, input[type="text"]:focus, input[type="password"]:focus, input[type="number"]:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
}
input[type="checkbox"] { width: 24px; height: 24px; accent-color: var(--accent); cursor: pointer; }

.input-with-button { display: flex; gap: 8px; }
.input-with-button input { flex: 1; }
.btn-browse {
  background: var(--bg-color);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 0 16px;
  cursor: pointer;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.2rem;
  color: var(--text-color);
}
.btn-browse:hover { background: var(--border-color); }

/* ─── Tracking Control ─────────────────────────────────────────── */
.tracking-control {
  margin: 24px 16px 16px;
  padding: 16px; background: var(--bg-color);
  border-radius: 12px;
}
.tracking-status {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
  font-size: 0.88rem;
  font-weight: 600;
}
.status-dot {
  width: 10px; height: 10px;
  border-radius: 50%;
  display: inline-block;
}
.status-dot.running { background: var(--success); animation: pulse 1.5s infinite; }
.status-dot.paused { background: var(--warning); }
.status-dot.stopped { background: var(--danger); }
.status-label { color: var(--text-muted); }

.tracking-buttons {
  display: flex;
  gap: 6px;
}
.btn-tracking {
  flex: 1;
  padding: 8px 0;
  border: none;
  border-radius: 8px;
  font-weight: 600;
  font-size: 0.82rem;
  cursor: pointer;
  transition: all 0.2s ease;
  text-align: center;
}
.btn-pause { background: var(--warning); color: #1a1a2e; }
.btn-pause:hover { filter: brightness(1.1); }
.btn-resume { background: var(--success); color: white; }
.btn-resume:hover { filter: brightness(1.1); }
.btn-stop-track { background: var(--danger); color: white; }
.btn-stop-track:hover { filter: brightness(1.1); }
.btn-start-track { background: var(--success); color: white; width: 100%; }
.btn-start-track:hover { filter: brightness(1.1); }

/* ─── Filters & New Views ──────────────────────────────────────── */
.view-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}
.view-header h1 { margin-bottom: 0; }
.filter-controls {
  display: flex;
  gap: 12px;
  align-items: center;
}
input[type="date"] {
  padding: 8px 12px;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background: var(--bg-color);
  color: var(--text-color);
  font-size: 0.9rem;
}

.screenshots-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 20px;
}
.screenshot-card {
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0,0,0,0.04);
}
.screenshot-card img {
  width: 100%;
  height: auto;
  aspect-ratio: 16 / 9;
  object-fit: cover;
  display: block;
}
.screenshot-info {
  padding: 12px;
  font-size: 0.85rem;
  color: var(--text-muted);
  text-align: center;
}

/* ─── Phase 8: Fullscreen Screens ──────────────────────────────── */
.fullscreen-center {
  position: fixed; inset: 0;
  display: flex; flex-direction: column;
  align-items: center; justify-content: center;
  background: var(--bg-color);
  z-index: 999;
}
.loading-text { color: var(--text-muted); margin-top: 16px; font-size: 0.95rem; }
.spinner {
  width: 40px; height: 40px;
  border: 3px solid var(--border-color);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }

/* ─── Wizard ────────────────────────────────────────────────────── */
.wizard-bg { background: linear-gradient(135deg, #0f1115 0%, #1a1040 100%); }
.wizard-card {
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 20px;
  padding: 48px 40px;
  width: 100%; max-width: 560px;
  box-shadow: 0 20px 60px rgba(0,0,0,0.4);
  animation: fadeIn 0.4s ease;
}
@keyframes fadeIn { from { opacity: 0; transform: translateY(20px); } to { opacity: 1; transform: translateY(0); } }

.wizard-logo { text-align: center; margin-bottom: 32px; }
.wizard-logo h1 { font-size: 1.6rem; font-weight: 800; margin: 12px 0 4px; }
.wizard-logo p { color: var(--text-muted); }
.wizard-logo img { border-radius: 12px; }

.wizard-step h2 { font-size: 1.2rem; font-weight: 700; margin-bottom: 20px; }

.mode-cards { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-bottom: 24px; }
.mode-card {
  border: 2px solid var(--border-color);
  border-radius: 12px; padding: 20px 16px;
  cursor: pointer; text-align: center;
  transition: all 0.2s ease;
}
.mode-card:hover { border-color: var(--accent); background: rgba(99,102,241,0.05); }
.mode-card-active { border-color: var(--accent) !important; background: rgba(99,102,241,0.1) !important; }
.mode-icon { font-size: 2rem; display: block; margin-bottom: 8px; }
.mode-card strong { font-size: 1rem; display: block; margin-bottom: 6px; }
.mode-card p { font-size: 0.82rem; color: var(--text-muted); margin: 0; line-height: 1.4; }

.wizard-form { display: flex; flex-direction: column; gap: 8px; margin-bottom: 20px; }
.wizard-form label { font-size: 0.88rem; font-weight: 600; color: var(--text-muted); margin-top: 4px; }

.wizard-error, .login-error {
  background: rgba(239,68,68,0.1); border: 1px solid var(--danger);
  color: var(--danger); border-radius: 8px;
  padding: 10px 14px; font-size: 0.88rem; margin-bottom: 12px;
}

.wizard-actions { display: flex; gap: 12px; margin-top: 8px; }
.btn-wizard-next {
  flex: 2; background: var(--accent); color: white;
  border: none; border-radius: 10px; padding: 12px 20px;
  font-weight: 700; font-size: 1rem; cursor: pointer;
  transition: all 0.2s ease;
}
.btn-wizard-next:hover:not(:disabled) { filter: brightness(1.12); transform: translateY(-1px); }
.btn-wizard-next:disabled { opacity: 0.6; cursor: not-allowed; }
.btn-wizard-back {
  flex: 1; background: var(--bg-color); color: var(--text-muted);
  border: 1px solid var(--border-color); border-radius: 10px;
  padding: 12px; font-weight: 600; cursor: pointer;
  transition: all 0.2s ease;
}
.btn-wizard-back:hover { background: var(--border-color); }

.confirm-summary {
  background: var(--bg-color); border-radius: 10px;
  padding: 16px 20px; margin-bottom: 20px;
}
.confirm-row {
  display: flex; justify-content: space-between;
  padding: 8px 0; border-bottom: 1px solid var(--border-color);
  font-size: 0.92rem;
}
.confirm-row:last-child { border-bottom: none; }
.confirm-row span { color: var(--text-muted); }

/* ─── Login ─────────────────────────────────────────────────────── */
.login-bg { background: linear-gradient(135deg, #0f1115 0%, #0c1240 100%); }
.login-card {
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 20px; padding: 48px 40px;
  width: 100%; max-width: 400px;
  box-shadow: 0 20px 60px rgba(0,0,0,0.4);
  animation: fadeIn 0.4s ease;
}
.login-logo { text-align: center; margin-bottom: 32px; }
.login-logo h1 { font-size: 1.5rem; font-weight: 800; margin: 12px 0 4px; }
.login-logo p { color: var(--text-muted); }
.login-logo img { border-radius: 12px; }
.login-form { display: flex; flex-direction: column; gap: 8px; }
.login-form label { font-size: 0.88rem; font-weight: 600; color: var(--text-muted); margin-top: 8px; }
.btn-login {
  width: 100%; margin-top: 16px;
  background: var(--accent); color: white;
  border: none; border-radius: 10px; padding: 13px;
  font-weight: 700; font-size: 1rem; cursor: pointer;
  transition: all 0.2s ease;
}
.btn-login:hover:not(:disabled) { filter: brightness(1.12); transform: translateY(-1px); }
.btn-login:disabled { opacity: 0.6; cursor: not-allowed; }

/* ─── User Badge (sidebar bottom) ───────────────────────────────── */
.user-badge {
  display: flex; align-items: center; gap: 10px;
  padding: 16px 24px;
  border-top: 1px solid var(--border-color);
  background: var(--sidebar-bg);
  margin-top: auto; /* Push to bottom if sidebar has flex-grow elements */
}
.user-avatar {
  width: 34px; height: 34px; border-radius: 50%;
  background: var(--accent); color: white;
  display: flex; align-items: center; justify-content: center;
  font-weight: 700; font-size: 0.95rem; flex-shrink: 0;
}
.user-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
.user-name { font-size: 0.88rem; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.btn-logout {
  background: none; border: 1px solid var(--border-color);
  border-radius: 6px; width: 30px; height: 30px;
  cursor: pointer; color: var(--text-muted);
  display: flex; align-items: center; justify-content: center;
  font-size: 1rem; transition: all 0.2s ease; flex-shrink: 0;
}
.btn-logout:hover { background: var(--danger); border-color: var(--danger); color: white; }

/* ─── Role Badge ─────────────────────────────────────────────────── */
.role-badge {
  display: inline-block; padding: 2px 8px; border-radius: 999px;
  font-size: 0.72rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.04em;
}
.role-admin { background: rgba(99,102,241,0.15); color: var(--accent); }
.role-emp { background: rgba(100,116,139,0.15); color: var(--text-muted); }

/* ─── Admin View ─────────────────────────────────────────────────── */
.view-admin { padding-bottom: 40px; }

/* ─── Settings Extra ─────────────────────────────────────────────── */
.setting-card-wide {
  grid-column: 1 / -1;
}
.mode-toggle-row {
  display: flex; align-items: center; gap: 12px; margin-top: 10px; flex-wrap: wrap;
}
.mode-current-badge {
  padding: 4px 12px; border-radius: 999px; font-size: 0.8rem; font-weight: 700;
}
.mode-badge-single { background: rgba(100,116,139,0.15); color: var(--text-muted); }
.mode-badge-multi { background: rgba(99,102,241,0.15); color: var(--accent); }
.mode-select { flex: 1; min-width: 160px; }
.btn-change-mode {
  background: var(--accent); color: white;
  border: none; border-radius: 8px; padding: 8px 16px;
  font-weight: 600; font-size: 0.9rem; cursor: pointer;
  transition: all 0.2s ease; white-space: nowrap;
}
.btn-change-mode:hover:not(:disabled) { filter: brightness(1.12); }
.btn-change-mode:disabled { opacity: 0.4; cursor: not-allowed; }

.danger-card { border-color: rgba(239,68,68,0.3) !important; }
.danger-label { color: var(--danger) !important; font-weight: 700; }
.danger-desc { font-size: 0.85rem; color: var(--text-muted); margin: 8px 0 14px; line-height: 1.5; }
.btn-danger {
  background: var(--danger); color: white;
  border: none; border-radius: 8px; padding: 10px 20px;
  font-weight: 700; cursor: pointer; font-size: 0.9rem;
  transition: all 0.2s ease;
}
.btn-danger:hover { filter: brightness(1.1); }
</style>