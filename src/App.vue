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



onMounted(async () => {
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
      const activeStr = formatTime(netActive);
      document.title = `▶ ${activeStr} | ${timeStr} — Time Guardian`;
    } else if (status === 'paused') {
      document.title = `⏸ Paused | ${timeStr} — Time Guardian`;
    } else {
      document.title = `⏹ Stopped | ${timeStr} — Time Guardian`;
    }
  }, 1000) as unknown as number;

  await listen<string>("tracking-status-changed", (event) => {
    const newStatus = event.payload;
    // Sync pause accumulator for tray icon actions
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
});

onUnmounted(() => {
  if (refreshInterval) clearInterval(refreshInterval);
  if (taskbarInterval) clearInterval(taskbarInterval);
});
</script>

<template>
  <div class="app-layout">
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
    </aside>

    <main class="main-content">
      <!-- DASHBOARD VIEW -->
      <div v-if="currentView === 'dashboard'" class="view-dashboard">
        <header>
          <h1>{{ t('message.todaySummary') }}</h1>
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
  --bg-color: #0f1115;
  --sidebar-bg: #16181d;
  --text-color: #f1f5f9;
  --text-muted: #94a3b8;
  --card-bg: #1e2128;
  --border-color: #2a2e37;
  --accent: #6366f1;
  --accent-hover: #818cf8;
  --success: #34d399;
  --danger: #f87171;
  --warning: #fbbf24;
  --bar-bg: #2a2e37;
}

* { box-sizing: border-box; margin: 0; }

body {
  margin: 0;
  background-color: var(--bg-color);
  color: var(--text-color);
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
  transition: background-color 0.3s, color 0.3s;
}

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
  margin-top: auto;
  padding: 16px;
  border-top: 1px solid var(--border-color);
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

select, input[type="text"], input[type="number"] {
  width: 100%;
  padding: 10px;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background: var(--bg-color);
  color: var(--text-color);
  font-size: 1rem;
  transition: all 0.2s ease;
}
select:focus, input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
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
}
.btn-browse:hover { background: var(--border-color); }

/* ─── Tracking Control ─────────────────────────────────────────── */
.tracking-control {
  padding: 16px;
  border-top: 1px solid var(--border-color);
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
</style>