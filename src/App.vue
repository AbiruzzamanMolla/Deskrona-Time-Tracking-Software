<script setup lang="ts">
// @ts-nocheck
import { onMounted, onUnmounted, ref, watch, computed } from "vue";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { useI18n } from "vue-i18n";
import { Pie } from "vue-chartjs";
import { Chart as ChartJS, Title, Tooltip, Legend, ArcElement } from "chart.js";
import {
  loadApiConfig,
  saveApiConfig,
  getApiConfig,
  isOnline,
  getQueueStats,
  retryFailedJobs,
  clearCompletedJobs,
  clearAllJobs,
} from "./utils/apiService";
import { startSync, stopSync, restartSync, runSyncCycle } from "./utils/dataSync";
import {
  proxyLogin, proxyRegisterCompany, proxyValidateSession, proxyLogout,
  proxyGetCompanyUsers, proxyCreateUser,
  proxyGetAdminStats,
  proxyGetUserScreenshots, proxyGetUserTimeLogs, proxyGetUserActivity, proxyGetUserUrls, proxyGetUserInputStats,
  proxyGetDashboardData, proxyGetFilteredDashboardData,
  proxyGetTimeLogsRange, proxyGetUrlsRange, proxyGetScreenshotsRange,
  proxyGetAllAppCategories, proxyUpdateAppCategory,
  proxyGetCalendarMonth,
  proxyGetSettings, proxyUpdateSettings,
  proxyPomodoroStart, proxyPomodoroSkip, proxyPomodoroStop, proxyPomodoroStatus,
  proxySetAutostart,
  proxyExportDb, proxyImportDb,
  proxyGetAppConfig, proxySaveAppConfig,
  proxyResetApp,
  proxyStartSession, proxyStopSession, proxyGetActiveSession,
  proxySetTracking, proxyGetTracking,
} from "./utils/apiProxy";
import type { ApiConfigFile, ApiEndpointConfig, EndpointKey } from "./types/api";
import { ENDPOINT_GROUPS } from "./types/api";

ChartJS.register(Title, Tooltip, Legend, ArcElement);

const { t, locale } = useI18n();

interface Settings {
  language: string;
  theme: string;
  auto_start_on_boot: boolean;
  screenshot_interval: number;
  is_screenshot_enabled: boolean;
  screenshot_location: string;
  backup_frequency: string;
  backup_location: string;
  idle_threshold: number;
  idle_monitor_mouse: boolean;
  idle_monitor_keyboard: boolean;
  overlay_enabled?: boolean;
  overlay_always_on_top?: boolean;
  overlay_click_through?: boolean;
  overlay_position_x?: number;
  overlay_position_y?: number;
  pomodoro_focus_minutes?: number;
  pomodoro_short_break_minutes?: number;
  pomodoro_long_break_minutes?: number;
  pomodoro_sessions_before_long?: number;
  pomodoro_auto_start?: boolean;
  pomodoro_sound_enabled?: boolean;
  break_reminder_enabled?: boolean;
  break_mini_interval_minutes?: number;
  break_mini_duration_seconds?: number;
  break_long_duration_seconds?: number;
  break_mini_breaks_before_long?: number;
  break_postpone_limit?: number;
  break_postpone_duration_minutes?: number;
  break_pre_notification_seconds?: number;
  break_sound_volume?: number;
  break_ideas_enabled?: boolean;
  break_fullscreen?: boolean;
  break_mini_enabled?: boolean;
  break_long_enabled?: boolean;
  break_allow_force_exit?: boolean;
  break_bg_color?: string;
  task_tracking_enabled?: boolean;
  task_auto_detect_enabled?: boolean;
  task_show_in_overlay?: boolean;
  task_show_in_tray?: boolean;
  task_timesheet_week_start?: string;
  task_show_untracked?: boolean;
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
  keyboard_count: number;
  mouse_count: number;
  productivity_score: number;
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
  mode: "single_user" | "multi_user";
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
  keyboard_count: number;
  mouse_count: number;
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
  is_screenshot_enabled: true,
  screenshot_location: "",
  backup_frequency: "never",
  backup_location: "",
  idle_threshold: 5,
  idle_monitor_mouse: true,
  idle_monitor_keyboard: true,
  break_reminder_enabled: false,
  break_mini_interval_minutes: 20,
  break_mini_duration_seconds: 20,
  break_long_duration_seconds: 300,
  break_mini_breaks_before_long: 4,
  break_postpone_limit: 3,
  break_postpone_duration_minutes: 2,
  break_pre_notification_seconds: 10,
  break_sound_volume: 50,
  break_ideas_enabled: true,
  break_fullscreen: true,
  break_mini_enabled: true,
  break_long_enabled: true,
  break_allow_force_exit: true,
  break_bg_color: "#0f172a",
  task_tracking_enabled: true,
  task_auto_detect_enabled: true,
  task_show_in_overlay: false,
  task_show_in_tray: true,
  task_timesheet_week_start: "monday",
  task_show_untracked: true,
});

const apiConfig = ref<ApiConfigFile>({
  mode: 'offline',
  bearer_token: '',
  endpoints: {} as Record<EndpointKey, ApiEndpointConfig>,
});

const queueStats = ref({ total: 0, pending: 0, failed: 0, jobs: [] });
const lastSyncTime = ref('Never');
const apiConfigTab = ref('general');

const lastSyncTimeFormatted = computed(() => {
  if (lastSyncTime.value === 'Never') return 'Never';
  try {
    const d = new Date(lastSyncTime.value);
    return d.toLocaleTimeString();
  } catch { return lastSyncTime.value; }
});

const triggerManualSync = async () => {
  await runSyncCycle();
  lastSyncTime.value = new Date().toISOString();
};

const currentView = ref("dashboard");
const activeSession = ref<Session | null>(null);
const dashboardData = ref<DashboardData>({
  total_active_seconds: 0,
  total_idle_seconds: 0,
  session_seconds: 0,
  app_stats: [],
  recent_urls: [],
  keyboard_count: 0,
  mouse_count: 0,
  productivity_score: 0,
});
let refreshInterval: ReturnType<typeof setInterval> | null = null;
let taskbarInterval: ReturnType<typeof setInterval> | null = null;
const trackingStatus = ref<string>("running");
const defaultScreenshotDir = ref<string>("");
const privacyNoticeDismissed = ref(false);

// Overlay state managed by Rust background thread
const overlayEnabled = ref(true);
const overlayAlwaysOnTop = ref(true);
const overlayClickThrough = ref(false);
const overlayPosition = ref({ x: -1, y: 20 });
const overlayElapsed = ref(0);
let overlayInterval: any = null;

// Settings tab
const settingsTab = ref("general");

// Break Reminder state
interface BreakReminderState {
  enabled: boolean;
  state: string;
  is_long: boolean;
  countdown_secs: number;
  mini_completed: number;
  postpone_count: number;
  postpone_limit: number;
  current_idea: string | null;
}
const breakState = ref<BreakReminderState>({
  enabled: false,
  state: 'idle',
  is_long: false,
  countdown_secs: 0,
  mini_completed: 0,
  postpone_count: 0,
  postpone_limit: 3,
  current_idea: null
});
const breakCountdownFormatted = computed(() => {
  const s = breakState.value.countdown_secs;
  return Math.floor(s / 60) + ':' + String(s % 60).padStart(2, '0');
});
const syncBreakState = async () => {
  try {
    breakState.value = await invoke('cmd_break_status') as BreakReminderState;
  } catch {}
};
const breakPause = async (m: number) => {
  try {
    await invoke('cmd_break_pause', { durationMinutes: m });
    await syncBreakState();
  } catch (e) {
    console.error(e);
  }
};
const breakResume = async () => {
  try {
    await invoke('cmd_break_resume');
    await syncBreakState();
  } catch (e) {
    console.error(e);
  }
};
const breakReset = async () => {
  try {
    await invoke('cmd_break_reset');
    await syncBreakState();
  } catch (e) {
    console.error(e);
  }
};


// ─── Projects & Tasks Interfaces ──────────────────────────────────────────
interface Project {
  id: string;
  name: string;
  color: string;
  archived: boolean;
  created_at: string;
}
interface Task {
  id: string;
  project_id: string;
  name: string;
  status: string;
  created_at: string;
}
interface TimesheetCell {
  task_id: string;
  task_name: string;
  project_id: string;
  project_name: string;
  project_color: string;
  date: string;
  seconds: number;
}
interface DailyTaskEntry {
  task_id: string | null;
  task_name: string;
  project_name: string | null;
  total_seconds: number;
}

// ─── Projects & Tasks State ──────────────────────────────────────────
const projects = ref<Project[]>([]);
const tasks = ref<Task[]>([]);
const selectedProjectId = ref<string | null>(null);
const activeTaskId = ref<string | null>(null);
const activeTaskName = ref<string | null>(null);
const newProjectName = ref('');
const newProjectColor = ref('#3b82f6');
const newTaskName = ref('');
const editingProject = ref<Project | null>(null);
const editingTask = ref<Task | null>(null);
const timesheetData = ref<TimesheetCell[]>([]);
const dailyTaskSummary = ref<DailyTaskEntry[]>([]);
const timesheetDateRange = ref<{ from: string; to: string }>({ from: '', to: '' });
const showProjectForm = ref(false);
const showTaskForm = ref(false);

// Task rules state
interface TaskRule {
  id: string;
  task_id: string;
  app_name: string;
  window_pattern: string | null;
}
const taskRules = ref<TaskRule[]>([]);
const newTaskRuleAppName = ref('');
const newTaskRulePattern = ref('');
const showRulesForTaskId = ref<string | null>(null);

// Filters & Archiving settings
const showArchivedProjects = ref(false);
const taskStatusFilter = ref<'all' | 'active' | 'done'>('all');

const loadProjects = async () => {
  try {
    projects.value = await invoke('cmd_list_projects') as Project[];
  } catch (e) { console.error(e); }
};

const loadTasks = async (projectId?: string) => {
  try {
    tasks.value = await invoke('cmd_list_tasks', { projectId: projectId || null }) as Task[];
  } catch (e) { console.error(e); }
};

const createProject = async () => {
  if (!newProjectName.value.trim()) return;
  try {
    await invoke('cmd_create_project', { name: newProjectName.value.trim(), color: newProjectColor.value });
    newProjectName.value = '';
    newProjectColor.value = '#3b82f6';
    showProjectForm.value = false;
    await loadProjects();
  } catch (e) { console.error(e); }
};

const updateProject = async (id: string, name: string, color: string) => {
  try {
    await invoke('cmd_update_project', { id, name, color });
    await loadProjects();
    editingProject.value = null;
  } catch (e) { console.error(e); }
};

const archiveProject = async (id: string) => {
  try {
    await invoke('cmd_update_project', { id, archived: true });
    await loadProjects();
  } catch (e) { console.error(e); }
};

const unarchiveProject = async (id: string) => {
  try {
    await invoke('cmd_update_project', { id, archived: false });
    await loadProjects();
  } catch (e) { console.error(e); }
};

const deleteProject = async (id: string) => {
  if (!confirm("Are you sure you want to delete this project? This will delete all its tasks and cannot be undone.")) return;
  try {
    await invoke('cmd_delete_project', { id });
    await loadProjects();
  } catch (e) { console.error(e); }
};

const createTask = async (projectId: string) => {
  if (!newTaskName.value.trim()) return;
  try {
    await invoke('cmd_create_task', { projectId, name: newTaskName.value.trim() });
    newTaskName.value = '';
    showTaskForm.value = false;
    await loadTasks(projectId);
  } catch (e) { console.error(e); }
};

const updateTask = async (id: string, name: string, status: string) => {
  try {
    await invoke('cmd_update_task', { id, name, status });
    await loadTasks(selectedProjectId.value || undefined);
    editingTask.value = null;
  } catch (e) { console.error(e); }
};

const deleteTask = async (id: string) => {
  if (!confirm("Are you sure you want to delete this task? This cannot be undone.")) return;
  try {
    await invoke('cmd_delete_task', { id });
    await loadTasks(selectedProjectId.value || undefined);
  } catch (e) { console.error(e); }
};

const loadActiveTask = async () => {
  try {
    const active = await invoke('cmd_get_active_task') as Task | null;
    if (active) {
      activeTaskId.value = active.id;
      activeTaskName.value = active.name;
      selectedProjectId.value = active.project_id;
      await loadTasks(active.project_id);
    } else {
      activeTaskId.value = null;
      activeTaskName.value = null;
    }
  } catch (e) { console.error(e); }
};

const setActiveTask = async (taskId: string | null) => {
  try {
    await invoke('cmd_set_active_task', { taskId });
    activeTaskId.value = taskId;
    if (taskId) {
      // Find locally or reload
      const allTasks = await invoke('cmd_list_tasks', { projectId: null }) as Task[];
      const task = allTasks.find(t => t.id === taskId);
      activeTaskName.value = task?.name || null;
      
      // Auto-start general tracking if stopped/paused
      if (trackingStatus.value !== "running") {
        await setTracking("running");
      }
    } else {
      activeTaskName.value = null;
    }
    await loadDailyTaskSummary(todayStr.value);
  } catch (e) { console.error(e); }
};

const loadTaskRules = async (taskId: string) => {
  try {
    taskRules.value = await invoke('cmd_list_task_rules', { taskId }) as TaskRule[];
  } catch (e) { console.error(e); }
};

const createTaskRule = async (taskId: string) => {
  if (!newTaskRuleAppName.value.trim()) return;
  try {
    await invoke('cmd_create_task_rule', {
      taskId,
      appName: newTaskRuleAppName.value.trim(),
      windowPattern: newTaskRulePattern.value.trim() || null
    });
    newTaskRuleAppName.value = '';
    newTaskRulePattern.value = '';
    await loadTaskRules(taskId);
  } catch (e) { console.error(e); }
};

const deleteTaskRule = async (ruleId: string, taskId: string) => {
  try {
    await invoke('cmd_delete_task_rule', { id: ruleId });
    await loadTaskRules(taskId);
  } catch (e) { console.error(e); }
};

const loadTimesheetData = async (from: string, to: string) => {
  try {
    timesheetData.value = await invoke('cmd_get_timesheet_data', { from, to }) as TimesheetCell[];
  } catch (e) { console.error(e); }
};

const setTimesheetPreset = (preset: 'today' | 'week' | 'last_week' | 'month') => {
  const today = new Date();
  let fromDate = new Date();
  let toDate = new Date();

  if (preset === 'today') {
    fromDate = today;
  } else if (preset === 'week') {
    const day = today.getDay();
    const diff = today.getDate() - day + (day === 0 ? -6 : 1);
    fromDate = new Date(today.setDate(diff));
  } else if (preset === 'last_week') {
    const day = today.getDay();
    const diff = today.getDate() - day + (day === 0 ? -6 : 1) - 7;
    fromDate = new Date(today.setDate(diff));
    toDate = new Date(fromDate.getTime() + 6 * 24 * 60 * 60 * 1000);
  } else if (preset === 'month') {
    fromDate = new Date(today.getFullYear(), today.getMonth(), 1);
  }

  timesheetDateRange.value = {
    from: fromDate.toISOString().split('T')[0],
    to: toDate.toISOString().split('T')[0]
  };
  loadTimesheetData(timesheetDateRange.value.from, timesheetDateRange.value.to);
};

const exportTimesheetCSV = () => {
  if (timesheetData.value.length === 0) return;
  let csv = 'Project,Task,Date,Time Spent (Seconds),Time Spent (Formatted)\n';
  timesheetData.value.forEach(cell => {
    csv += `"${cell.project_name.replace(/"/g, '""')}","${cell.task_name.replace(/"/g, '""')}",${cell.date},${cell.seconds},"${formatTime(cell.seconds)}"\n`;
  });

  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.setAttribute('href', url);
  link.setAttribute('download', `timesheet_${timesheetDateRange.value.from}_to_${timesheetDateRange.value.to}.csv`);
  link.style.visibility = 'hidden';
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
};

const loadDailyTaskSummary = async (date: string) => {
  try {
    dailyTaskSummary.value = await invoke('cmd_get_daily_task_summary', { date }) as DailyTaskEntry[];
  } catch (e) { console.error(e); }
};

// Pomodoro state
const pomodoroPhase = ref("idle");
const pomodoroRemaining = ref(0);
const pomodoroCountToday = ref(0);
const wellnessWidgetExpanded = ref(false);

// Nav scroll arrows (mobile tab bar)
const navRef = ref<HTMLElement | null>(null);
const navCanScrollLeft = ref(false);
const navCanScrollRight = ref(false);

const updateNavScroll = () => {
  const el = navRef.value;
  if (!el) return;
  navCanScrollLeft.value = el.scrollLeft > 2;
  navCanScrollRight.value = el.scrollLeft + el.clientWidth < el.scrollWidth - 2;
};

const scrollNav = (dir: 'left' | 'right') => {
  const el = navRef.value;
  if (!el) return;
  el.scrollBy({ left: dir === 'left' ? -120 : 120, behavior: 'smooth' });
};

onMounted(() => {
  setTimeout(updateNavScroll, 300);
  window.addEventListener('resize', updateNavScroll);
});
onUnmounted(() => {
  window.removeEventListener('resize', updateNavScroll);
});

const pomodoroFormatted = computed(() => {
  const m = Math.floor(pomodoroRemaining.value / 60);
  const s = pomodoroRemaining.value % 60;
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
});

const syncPomodoro = async () => {
  try {
    const st = await proxyPomodoroStatus();
    pomodoroPhase.value = st.phase;
    pomodoroRemaining.value = st.remaining_secs;
    pomodoroCountToday.value = st.count_today;
  } catch {}
};

const startPomodoro = async () => {
  await proxyPomodoroStart();
  await syncPomodoro();
};

const skipPomodoro = async () => {
  await proxyPomodoroSkip();
  await syncPomodoro();
};

const stopPomodoro = async () => {
  await proxyPomodoroStop();
  await syncPomodoro();
};

const showOverlay = async () => {
  try {
    await invoke("show_overlay_window", {
      x: overlayPosition.value.x,
      y: overlayPosition.value.y,
      alwaysOnTop: overlayAlwaysOnTop.value,
      clickThrough: overlayClickThrough.value
    });
  } catch (e) {
    console.error("Failed to show overlay:", e);
  }
};

const hideOverlay = async () => {
  try {
    await invoke("hide_overlay_window");
  } catch (e) {
    console.error("Failed to hide overlay:", e);
  }
};

const testBreakOverlay = async () => {
  try {
    await invoke('cmd_break_test_preview');
  } catch (e) {
    console.error('Failed to trigger test overlay', e);
  }
};

const updateOverlayTimer = () => {
  // This is now primarily handled by Rust, but we keep the variable for consistency
  overlayElapsed.value++;
};

// Overlay time is updated from Rust side via window.updateOverlayTime
(window as any).updateOverlayTime = (time: string, status: string) => {
  // If we wanted to sync any local state with overlay time, we could do it here
};

watch(trackingStatus, async (newStatus) => {
  console.log("Watcher: tracking status changed to", newStatus);
  // Rust background thread will handle overlay visibility based on this status
});

// ─── Update Check State ─────────────────────────────────────
import packageJson from "../package.json";
const currentVersion = packageJson.version;

const latestVersion = ref<string>("");
const updateAvailable = ref(false);
const updateCheckLoading = ref(false);
const updateError = ref("");

const checkForUpdates = async () => {
  updateCheckLoading.value = true;
  updateError.value = "";
  updateAvailable.value = false;
  latestVersion.value = "";
  try {
    // Use tags endpoint - more reliable than releases
    const response = await fetch(
      "https://api.github.com/repos/AbiruzzamanMolla/Deskrona-Time-Tracking-Software/tags?per_page=1",
      {
        headers: { Accept: "application/vnd.github.v3+json" },
      }
    );

    if (!response.ok) {
      if (response.status === 403) {
        throw new Error("API rate limit. Try again later.");
      }
      throw new Error(`HTTP ${response.status}`);
    }

    const tagsData = await response.json();
    if (!Array.isArray(tagsData) || tagsData.length === 0) {
      updateError.value = "No versions found";
      return;
    }

    // Get first tag and remove 'v' prefix
    const latest = tagsData[0].name ? tagsData[0].name.replace(/^v/, "") : "";
    latestVersion.value = latest;

    if (!latest) {
      updateError.value = "No version tag found";
      return;
    }

    // Compare versions
    const currentParts = currentVersion.split(".").map((n: string) => parseInt(n) || 0);
    const latestParts = latest.split(".").map((n: string) => parseInt(n) || 0);

    let hasUpdate = false;
    for (let i = 0; i < Math.max(currentParts.length, latestParts.length); i++) {
      const c = currentParts[i] || 0;
      const l = latestParts[i] || 0;
      if (l > c) {
        hasUpdate = true;
        break;
      } else if (l < c) {
        break;
      }
    }
    updateAvailable.value = hasUpdate;
  } catch (e: any) {
    updateError.value = e.message || "Failed to check for updates";
  } finally {
    updateCheckLoading.value = false;
  }
};

// ─── Phase 8 State ────────────────────────────────────────────────────
const appConfig = ref<AppConfig>({ mode: "single_user", setup_done: false });
const appScreen = ref<"loading" | "wizard" | "login" | "app">("loading");
const currentUser = ref<AuthUser | null>(null);
const sessionToken = ref<string>("");

// Wizard state
const wizardStep = ref(1);
const wizardMode = ref<"single_user" | "multi_user">("single_user");
const wizardCompanyName = ref("");
const wizardAdminUsername = ref("");
const wizardAdminDisplay = ref("");
const wizardAdminPassword = ref("");
const wizardConfirmPassword = ref("");
const wizardError = ref("");
const wizardLoading = ref(false);

// Wizard API config
const wizardApiMode = ref<'offline' | 'online'>('offline');
const wizardApiServerUrl = ref('');
const wizardApiBearerToken = ref('');
const wizardApiQuickFilled = ref(false);

// Login state
const loginUsername = ref("");
const loginPassword = ref("");
const loginError = ref("");
const loginLoading = ref(false);

// Admin state
const adminUsers = ref<AuthUser[]>([]);
const adminStats = ref<UserProductivityStat[]>([]);
const showCreateUser = ref(false);
const newUser = ref<CreateUserPayload>({
  username: "",
  display_name: "",
  password: "",
  role: "employee",
});
const createUserError = ref("");
const createUserLoading = ref(false);

// Admin drill-down state
const adminTab = ref<
  "team" | "screenshots" | "timelogs" | "activity" | "urls" | "categories"
>("team");
const selectedUser = ref<AuthUser | null>(null);
const adminDrillDate = ref(new Date().toISOString().split("T")[0]);
interface AdminScreenshot {
  id: string;
  user_id: string;
  display_name: string;
  file_path: string;
  captured_at: string;
}
interface AdminTimeLog {
  id: string;
  user_id: string;
  display_name: string;
  app_name: string;
  window_title: string;
  start_time: string;
  end_time: string;
  duration: number;
  status: string;
}
interface AdminActivity {
  id: string;
  event_type: string;
  timestamp: string;
  activity_status: string;
}
const adminUserScreenshots = ref<AdminScreenshot[]>([]);
const adminUserTimeLogs = ref<AdminTimeLog[]>([]);
const adminUserActivity = ref<AdminActivity[]>([]);
const adminUserUrls = ref<UrlEntryFull[]>([]);
const adminDrillLoading = ref(false);

const adminTimeLogsOffset = ref(0);
const adminScreenshotsOffset = ref(0);
const adminActivityOffset = ref(0);
const adminUrlsOffset = ref(0);
interface AppCategoryEntry {
  app_name: string;
  category: string;
}
const adminCategories = ref<AppCategoryEntry[]>([]);
interface InputStats {
  keyboard_count: number;
  mouse_count: number;
  idle_start_count: number;
}
const adminInputStats = ref<InputStats>({
  keyboard_count: 0,
  mouse_count: 0,
  idle_start_count: 0,
});

const isAdmin = computed(() => currentUser.value?.role === "admin");
const isMultiUser = computed(() => appConfig.value.mode === "multi_user");

// Settings mode change pending state
const pendingMode = ref<"single_user" | "multi_user">("single_user");
const fullscreenScreenshot = ref<string | null>(null);
const openFullscreen = (path: string) => {
  fullscreenScreenshot.value = path;
};
const closeFullscreen = () => {
  fullscreenScreenshot.value = null;
};

// Accumulated paused seconds — updated each time we enter/leave pause state
const pausedSeconds = ref(0);
let pauseStartedAt: number | null = null;

// ─── Calendar State ───────────────────────────────────────────
interface CalendarDay {
  date: string;
  total_seconds: number;
  app_count: number;
  has_screenshots: boolean;
}
const calendarDays = ref<CalendarDay[]>([]);
const calendarYear = ref(new Date().getFullYear());
const calendarMonthNum = ref(new Date().getMonth()); // 0-based
const calendarLoading = ref(false);

const loadCalendarMonth = async () => {
  calendarLoading.value = true;
  const y = calendarYear.value;
  const m = calendarMonthNum.value;
  const from = `${y}-${String(m + 1).padStart(2, '0')}-01`;
  const lastDay = new Date(y, m + 1, 0).getDate();
  const to = `${y}-${String(m + 1).padStart(2, '0')}-${String(lastDay).padStart(2, '0')}`;
  try {
    calendarDays.value = await proxyGetCalendarMonth(from, to) as CalendarDay[];
  } catch (e) {
    console.error("Failed to load calendar:", e);
    calendarDays.value = [];
  } finally {
    calendarLoading.value = false;
  }
};

const calendarPrevMonth = () => {
  if (calendarMonthNum.value === 0) {
    calendarMonthNum.value = 11;
    calendarYear.value--;
  } else {
    calendarMonthNum.value--;
  }
};

const calendarNextMonth = () => {
  if (calendarMonthNum.value === 11) {
    calendarMonthNum.value = 0;
    calendarYear.value++;
  } else {
    calendarMonthNum.value++;
  }
};

const calendarGoToday = () => {
  const now = new Date();
  calendarYear.value = now.getFullYear();
  calendarMonthNum.value = now.getMonth();
};

const selectedCalendarDay = ref<string | null>(new Date().toISOString().split("T")[0]);

watch([calendarYear, calendarMonthNum], () => {
  if (currentView.value === 'calendar' || (currentView.value === 'productivity' && filterType.value === 'calendar')) {
    loadCalendarMonth();
  }
});

const calendarMonthName = computed(() => {
  const d = new Date(calendarYear.value, calendarMonthNum.value, 1);
  return d.toLocaleString('default', { month: 'long', year: 'numeric' });
});

const calendarDaysInMonth = computed(() => {
  const y = calendarYear.value;
  const m = calendarMonthNum.value;
  const first = new Date(y, m, 1);
  const last = new Date(y, m + 1, 0);
  const daysInMonth = last.getDate();
  const startDow = first.getDay(); // 0=Sun
  const emptyStart = startDow === 0 ? 6 : startDow - 1; // Mon-first
  const cells: ({ day: number; dateStr: string } | null)[] = [];
  for (let i = 0; i < emptyStart; i++) cells.push(null);
  for (let d = 1; d <= daysInMonth; d++) {
    const dateStr = `${y}-${String(m + 1).padStart(2, '0')}-${String(d).padStart(2, '0')}`;
    cells.push({ day: d, dateStr });
  }
  return cells;
});

const getCalendarDay = (dateStr: string): CalendarDay | undefined => {
  return calendarDays.value.find(d => d.date === dateStr);
};

const calendarDayColor = (seconds: number): string => {
  if (seconds <= 0) return 'var(--bg-color)';
  if (seconds < 3600) return 'rgba(16, 185, 129, 0.25)';
  if (seconds < 14400) return 'rgba(16, 185, 129, 0.45)';
  if (seconds < 28800) return 'rgba(16, 185, 129, 0.65)';
  return 'rgba(16, 185, 129, 0.85)';
};

const calendarDayTextColor = (seconds: number): string => {
  if (seconds >= 28800) return '#fff';
  return 'var(--text-color)';
};
const selectedCalendarDayData = computed(() => {
  if (!selectedCalendarDay.value) return null;
  return getCalendarDay(selectedCalendarDay.value);
});

const todayStr = computed(() => new Date().toISOString().split('T')[0]);

// ─── Calendar Day Detail ──────────────────────────────────────
const calendarDayApps = ref<AppUsageStat[]>([]);
const calendarDayLoadingDetail = ref(false);
const calendarDayInputStats = ref({ keyboard_count: 0, mouse_count: 0 });

const loadDayDetail = async (dateStr: string) => {
  calendarDayLoadingDetail.value = true;
  try {
    const [data, inputStats] = await Promise.all([
      proxyGetFilteredDashboardData(dateStr, dateStr) as Promise<DashboardData>,
      proxyGetUserInputStats('default_user' as any, dateStr, dateStr),
    ]);
    calendarDayApps.value = data.app_stats || [];
    calendarDayInputStats.value = inputStats as any;
  } catch {
    calendarDayApps.value = [];
  } finally {
    calendarDayLoadingDetail.value = false;
  }
};

const viewDayDetail = (dateStr: string) => {
  selectedCalendarDay.value = dateStr;
  loadFilteredData(false);
};
const filterType = ref("calendar");
const customFromDate = ref(new Date().toISOString().split("T")[0]);
const customToDate = ref(new Date().toISOString().split("T")[0]);

const activeSessionSeconds = computed(() => {
  if (!activeSession.value) return 0;
  const start = new Date(activeSession.value.start_time).getTime();
  const now = Date.now();
  return Math.floor((now - start) / 1000) - pausedSeconds.value;
});

const activeSessionTimeFormatted = computed(() => {
  const totalSecs = activeSessionSeconds.value;
  const hours = Math.floor(totalSecs / 3600);
  const mins = Math.floor((totalSecs % 3600) / 60);
  const secs = totalSecs % 60;
  return `${hours}:${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
});

const timeLogsList = ref<TimeLogEntry[]>([]);
const urlsList = ref<UrlEntryFull[]>([]);
const expandedUrlRows = ref<Set<string>>(new Set());
const screenshotsList = ref<ScreenshotEntry[]>([]);
const userActivityList = ref<AdminActivity[]>([]);
const userInputStats = ref<InputStats>({
  keyboard_count: 0,
  mouse_count: 0,
  idle_start_count: 0,
});

const timeLogsOffset = ref(0);
const urlsOffset = ref(0);
const screenshotsOffset = ref(0);
const activityOffset = ref(0);
const pageSize = 50;

// ─── Helpers ──────────────────────────────────────────────────────
const formatTime = (seconds: number): string => {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
};

// Compact format for calendar cells: "1h30m" or "45m"
const formatTimeCompact = (seconds: number): string => {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return `${h}h${m > 0 ? m + 'm' : ''}`;
  if (m > 0) return `${m}m`;
  return '<1m';
};

const updateCategory = async (appName: string, category: string) => {
  try {
    await proxyUpdateAppCategory(appName, category);
    const app = dashboardData.value.app_stats.find((a) => a.app_name === appName);
    if (app) app.category = category;
  } catch (error) {
    console.error("Failed to update category:", error);
  }
};

const productivityChartData = computed(() => {
  let productive = 0;
  let unproductive = 0;
  let neutral = 0;

  dashboardData.value.app_stats.forEach((stat) => {
    if (stat.category === "productive") productive += stat.total_seconds;
    else if (stat.category === "unproductive") unproductive += stat.total_seconds;
    else neutral += stat.total_seconds;
  });

  return {
    labels: ["Productive", "Unproductive", "Neutral"],
    datasets: [
      {
        backgroundColor: ["#10b981", "#ef4444", "#64748b"],
        data: [productive, unproductive, neutral],
        borderWidth: 0,
        hoverOffset: 4,
      },
    ],
  };
});
const productivityChartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: { position: "bottom" as const, labels: { color: "#94a3b8" } },
  },
};

const formatTimestamp = (ts: string): string => {
  try {
    const d = new Date(ts);
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  } catch {
    return ts;
  }
};

const appIcon = (name: string): string => {
  const n = name.toLowerCase();
  if (
    n.includes("chrome") ||
    n.includes("edge") ||
    n.includes("firefox") ||
    n.includes("brave") ||
    n.includes("opera") ||
    n.includes("safari")
  )
    return "🌐";
  if (
    n.includes("code") ||
    n.includes("visual studio") ||
    n.includes("vim") ||
    n.includes("nvim")
  )
    return "💻";
  if (
    n.includes("terminal") ||
    n.includes("cmd") ||
    n.includes("powershell") ||
    n.includes("windows terminal")
  )
    return "⬛";
  if (
    n.includes("slack") ||
    n.includes("discord") ||
    n.includes("teams") ||
    n.includes("telegram")
  )
    return "💬";
  if (n.includes("explorer") || n.includes("finder")) return "📁";
  if (n.includes("spotify") || n.includes("music")) return "🎵";
  if (n.includes("outlook") || n.includes("mail") || n.includes("thunderbird"))
    return "📧";
  return "📄";
};

const percentage = (seconds: number): number => {
  const total = dashboardData.value.total_active_seconds;
  if (total <= 0) return 0;
  return Math.round((seconds / total) * 100);
};

// ─── Settings ─────────────────────────────────────────────────────
const loadSettings = async () => {
  try {
    const s = await proxyGetSettings() as Settings;
    settings.value = s;
    locale.value = s.language;
    applyTheme();
  } catch (error) {
    console.error("Failed to load settings:", error);
  }
};

const saveSettings = async () => {
  try {
    await proxyUpdateSettings(settings.value);
    locale.value = settings.value.language;
    applyTheme();
  } catch (error) {
    console.error("Failed to save settings:", error);
  }
};

const saveOverlaySettings = async () => {
  const s = await proxyGetSettings() as any;
  (s as any).overlay_enabled = !!overlayEnabled.value;
  (s as any).overlay_always_on_top = !!overlayAlwaysOnTop.value;
  (s as any).overlay_click_through = !!overlayClickThrough.value;
  (s as any).overlay_position_x = overlayPosition.value.x;
  (s as any).overlay_position_y = overlayPosition.value.y;
  await proxyUpdateSettings(s);
};

const loadApiConfigFromDisk = async () => {
  try {
    apiConfig.value = await loadApiConfig();
    // Safely backfill any missing endpoint configurations
    if (!apiConfig.value.endpoints) {
      apiConfig.value.endpoints = {} as any;
    }
    for (const group of ENDPOINT_GROUPS) {
      for (const ep of group.endpoints) {
        if (!apiConfig.value.endpoints[ep.key]) {
          (apiConfig.value.endpoints as any)[ep.key] = {
            enabled: false,
            method: ep.method as any,
            url: '',
            headers: { 'Content-Type': 'application/json' },
          };
        }
      }
    }
  } catch (e) {
    console.error("Failed to load API config:", e);
  }
};

const saveApiConfigToDisk = async () => {
  try {
    await saveApiConfig(apiConfig.value);
    restartSync();
  } catch (e) {
    console.error("Failed to save API config:", e);
  }
};

const apiGroupExpanded = ref<Record<string, boolean>>({});
for (const group of ENDPOINT_GROUPS) {
  apiGroupExpanded.value[group.key] = false;
}

const toggleApiGroup = (key: string) => {
  apiGroupExpanded.value[key] = !apiGroupExpanded.value[key];
};

const renameHeader = (key: EndpointKey, oldName: string, newName: string) => {
  if (!newName) return;
  const headers = apiConfig.value.endpoints[key]?.headers;
  if (!headers) return;
  const val = headers[oldName];
  delete headers[oldName];
  headers[newName] = val;
  saveApiConfigToDisk();
};

const updateHeader = (key: EndpointKey, name: string, value: string) => {
  const headers = apiConfig.value.endpoints[key]?.headers;
  if (!headers) return;
  headers[name] = value;
  saveApiConfigToDisk();
};

const removeHeader = (key: EndpointKey, name: string) => {
  const headers = apiConfig.value.endpoints[key]?.headers;
  if (!headers) return;
  delete headers[name];
  saveApiConfigToDisk();
};

const addHeader = (key: EndpointKey) => {
  const headers = apiConfig.value.endpoints[key]?.headers;
  if (!headers) return;
  headers[''] = '';
  // Force reactivity
  apiConfig.value = { ...apiConfig.value };
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
  const selected = await open({
    directory: true,
    multiple: false,
    title: t("message.screenshotLocation"),
    defaultPath: settings.value.screenshot_location || undefined,
  });
  if (selected && typeof selected === "string")
    settings.value.screenshot_location = selected;
};

const selectBackupLocation = async () => {
  const selected = await open({
    directory: true,
    multiple: false,
    title: t("message.backupLocation"),
    defaultPath: settings.value.backup_location || undefined,
  });
  if (selected && typeof selected === "string") settings.value.backup_location = selected;
};

const exportData = async () => {
  const selected = await save({
    filters: [{ name: "Backup Archive", extensions: ["zip"] }],
    title: t("message.exportData"),
  });
  if (selected && typeof selected === "string") {
    try {
      await proxyExportDb(selected);
      alert(t("message.dataExportSuccess"));
    } catch (e) {
      console.error("Export failed:", e);
    }
  }
};

const importData = async () => {
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [{ name: "Backup Archive", extensions: ["zip"] }],
    title: t("message.importData"),
  });
  if (selected && typeof selected === "string") {
    try {
      await proxyImportDb(selected);
      alert(t("message.dataImportSuccess"));
    } catch (e) {
      console.error("Import failed:", e);
    }
  }
};

// ─── Session Management ──────────────────────────────────────────
const startSession = async () => {
  try {
    const session = await proxyStartSession() as Session;
    activeSession.value = session;
  } catch (e) {
    console.error("Failed to start session:", e);
  }
};

const stopSession = async () => {
  if (!activeSession.value) return;
  try {
    await proxyStopSession(activeSession.value.id) as Session;
    activeSession.value = null;
    await refreshDashboard();
  } catch (e) {
    console.error("Failed to stop session:", e);
  }
};

const loadActiveSession = async () => {
  try {
    const session = await proxyGetActiveSession() as Session | null;
    activeSession.value = session;
  } catch (e) {
    console.error("Failed to load active session:", e);
  }
};

// ─── Dashboard Data ──────────────────────────────────────────────
const refreshDashboard = async () => {
  try {
    const isViewingToday = currentView.value === "dashboard" ||
      (currentView.value === "productivity" && filterType.value === "calendar" && selectedCalendarDay.value === todayStr.value);

    const data = await proxyGetDashboardData() as DashboardData;

    if (isViewingToday) {
      const localBackup = { ...dashboardData.value };
      // Never let timer go backward — local counter is authoritative
      if (data.total_active_seconds < localBackup.total_active_seconds) {
        data.total_active_seconds = localBackup.total_active_seconds;
      }
      dashboardData.value = data;
    }

    // Sync the local timer with backend data
    if (data.total_active_seconds !== undefined) {
      activeSessionSeconds.value = data.total_active_seconds;
    }
  } catch (e) {
    console.error("Failed to refresh dashboard:", e);
  }
};

const loadDashboardData = refreshDashboard;

const ensureNativeNotificationPermission = async () => {
  try {
    const granted = await isPermissionGranted();
    if (granted) return true;
    const permission = await requestPermission();
    return permission === "granted";
  } catch (e) {
    console.error("Notification permission check failed:", e);
    return false;
  }
};

const sendTrackingNativeNotification = async (status: string) => {
  const allowed = await ensureNativeNotificationPermission();
  if (!allowed) return;

  const body =
    status === "running"
      ? t("message.nativeTrackingStarted")
      : status === "paused"
      ? t("message.nativeTrackingPaused")
      : t("message.nativeTrackingStopped");

  try {
    await sendNotification({
      title: t("message.nativeTrackingTitle"),
      body,
    });
  } catch (e) {
    console.error("Native notification failed:", e);
  }
};

// ─── Tracking Control ────────────────────────────────────────────
const loadTrackingStatus = async () => {
  try {
    trackingStatus.value = await proxyGetTracking();
  } catch (e) {
    console.error("Failed to get tracking status:", e);
  }
};

const setTracking = async (status: string) => {
  try {
    await proxySetTracking(status);
    // Accumulate pause duration
    if (status === "paused") {
      pauseStartedAt = Date.now();
    } else if (status === "running" && pauseStartedAt !== null) {
      pausedSeconds.value += Math.floor((Date.now() - pauseStartedAt) / 1000);
      pauseStartedAt = null;
    } else if (status === "running" && activeTaskId.value) {
      // Re-activate active task in the database when starting tracking again
      await invoke('cmd_set_active_task', { taskId: activeTaskId.value });
    } else if (status === "stopped") {
      pausedSeconds.value = 0;
      pauseStartedAt = null;
      // Stop active task in backend DB is already handled by backend cmd_set_tracking.
      // We retain the local activeTaskId/activeTaskName reference for auto-resuming.
    }
    trackingStatus.value = status;
    await sendTrackingNativeNotification(status);
  } catch (e) {
    console.error("Failed to set tracking:", e);
  }
};

// ─── Filtered Data ───────────────────────────────────────────────
const getDateRange = () => {
  const to = new Date();
  const from = new Date();
  if (filterType.value === "calendar") {
    const day = selectedCalendarDay.value || new Date().toISOString().split("T")[0];
    return { from: day, to: day };
  } else if (filterType.value === "daily") {
    // Same day
  } else if (filterType.value === "weekly") {
    from.setDate(from.getDate() - 7);
  } else if (filterType.value === "monthly") {
    from.setMonth(from.getMonth() - 1);
  } else if (filterType.value === "yearly") {
    from.setFullYear(from.getFullYear() - 1);
  } else if (filterType.value === "custom") {
    return { from: customFromDate.value, to: customToDate.value };
  }
  return {
    from: from.toISOString().split("T")[0],
    to: to.toISOString().split("T")[0],
  };
};

const loadFilteredData = async (append = false) => {
  const { from, to } = getDateRange();
  try {
    if (currentView.value === "trackings") {
      if (!append) timeLogsOffset.value = 0;
      const data = await proxyGetTimeLogsRange(from, to, pageSize, timeLogsOffset.value) as TimeLogEntry[];
      timeLogsList.value = append ? [...timeLogsList.value, ...data] : data;
    } else if (currentView.value === "urls") {
      if (!append) urlsOffset.value = 0;
      const data = await proxyGetUrlsRange(from, to, pageSize, urlsOffset.value) as UrlEntryFull[];
      urlsList.value = append ? [...urlsList.value, ...data] : data;
    } else if (currentView.value === "screenshots") {
      if (!append) screenshotsOffset.value = 0;
      const data = await proxyGetScreenshotsRange(from, to, pageSize, screenshotsOffset.value) as ScreenshotEntry[];
      screenshotsList.value = append ? [...screenshotsList.value, ...data] : data;
    } else if (currentView.value === "activity") {
      const { from, to } = getDateRange();
      const userId = currentUser.value?.id || "default_user";
      if (!append) activityOffset.value = 0;

      const [acts, stats] = await Promise.all([
        proxyGetUserActivity(userId, from, to, pageSize, activityOffset.value) as Promise<AdminActivity[]>,
        proxyGetUserInputStats(userId, from, to) as Promise<InputStats>,
      ]);
      userActivityList.value = append ? [...userActivityList.value, ...acts] : acts;
      userInputStats.value = stats;
    } else if (currentView.value === "productivity") {
      const data = await proxyGetFilteredDashboardData(from, to) as DashboardData;
      dashboardData.value = data;
    }
  } catch (e) {
    console.error("Failed to load filtered data", e);
  }
};

const loadMoreTrackings = () => {
  timeLogsOffset.value += pageSize;
  loadFilteredData(true);
};
const loadMoreUrls = () => {
  urlsOffset.value += pageSize;
  loadFilteredData(true);
};

const isLikelyUrl = (value: string) => /^(https?:\/\/|www\.)/i.test(value) || /^[\w.-]+\.[a-z]{2,}(\/|$)/i.test(value);

const normalizeUrl = (value: string) => {
  if (!value) return "";
  return /^(https?:\/\/)/i.test(value) ? value : `https://${value}`;
};

const getHistoryTitle = (value: string) => {
  if (!value) return "";
  if (!isLikelyUrl(value)) return value;
  try {
    const u = new URL(normalizeUrl(value));
    if (u.pathname && u.pathname !== "/") {
      const last = u.pathname.split("/").filter(Boolean).pop() || "";
      const decoded = decodeURIComponent(last).replace(/[-_]+/g, " ").trim();
      if (decoded.length > 0) return decoded;
    }
    return u.hostname.replace(/^www\./, "");
  } catch {
    return value;
  }
};

const getHistoryUrl = (value: string) => (isLikelyUrl(value) ? normalizeUrl(value) : "");

const toggleUrlRow = (id: string) => {
  const next = new Set(expandedUrlRows.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  expandedUrlRows.value = next;
};
const loadMoreScreenshots = () => {
  screenshotsOffset.value += pageSize;
  loadFilteredData(true);
};
const loadMoreActivity = () => {
  activityOffset.value += pageSize;
  loadFilteredData(true);
};

// ─── Watchers ────────────────────────────────────────────────────
watch([currentView, filterType, customFromDate, customToDate], () => {
  if (["trackings", "urls", "screenshots", "activity", "productivity"].includes(currentView.value)) {
    loadFilteredData(false);
  }
  if (currentView.value === 'productivity' && filterType.value === 'calendar') {
    loadCalendarMonth();
  }
});

watch(
  settings,
  async (newVal, oldVal) => {
    saveSettings();
    if (oldVal && newVal.auto_start_on_boot !== oldVal.auto_start_on_boot) {
      try {
        await proxySetAutostart(newVal.auto_start_on_boot);
      } catch (error) {
        console.error("Failed to set autostart:", error);
      }
    }
  },
  { deep: true }
);

// App init — runs once when appScreen transitions to 'app'
let appInitDone = false;
const initApp = async () => {
  if (appInitDone) return;
  appInitDone = true;
  checkForUpdates();
  await loadSettings();
  defaultScreenshotDir.value = await invoke("cmd_get_screenshot_dir");
  
  // Wait for session to be fully restored/validated
  await loadActiveSession();
  await loadTrackingStatus();
  await syncPomodoro();
  await syncBreakState();
  await listen<string>("pomodoro-phase-changed", () => syncPomodoro());
  await listen<string>("break-started", async (event) => {
    await syncBreakState();
    const allowed = await ensureNativeNotificationPermission();
    if (allowed) {
      const idea = event.payload || "";
      await sendNotification({
        title: "Break Started!",
        body: idea ? `Idea: ${idea}` : "Time to take a break.",
      });
    }
  });
  await listen<string>("break-finished", () => syncBreakState());
  await listen<string>("break-postponed", () => syncBreakState());
  await listen<string>("break-paused", () => syncBreakState());
  await listen<string>("break-resumed", () => syncBreakState());
  await listen<string>("break-pre-notification", async (event) => {
    await syncBreakState();
    const allowed = await ensureNativeNotificationPermission();
    if (allowed) {
      const idea = event.payload || "";
      await sendNotification({
        title: "Break Starting Soon!",
        body: idea ? `Next break: ${idea}` : "A break is starting soon.",
      });
    }
  });
  
  // Force multiple refreshes to ensure sync
  // Try to get initial data multiple times in case of DB lock or slow start
  for (let i = 0; i < 5; i++) {
    console.log(`App init (attempt ${i+1}): refreshing dashboard...`);
    await refreshDashboard();
    if (activeSessionSeconds.value > 0) break;
    await new Promise(r => setTimeout(r, 500));
  }
  
  window
    .matchMedia("(prefers-color-scheme: dark)")
    .addEventListener("change", applyTheme);
  
  refreshInterval = (setInterval(async () => {
    await refreshDashboard();
    await syncBreakState();
  }, 5000) as unknown) as number;
  taskbarInterval = (setInterval(() => {
    if (breakState.value && breakState.value.state !== 'idle' && breakState.value.state !== 'paused' && breakState.value.countdown_secs > 0) {
      breakState.value.countdown_secs--;
    }
    const hours = Math.floor(activeSessionSeconds.value / 3600);
    const mins = Math.floor((activeSessionSeconds.value % 3600) / 60);
    const secs = activeSessionSeconds.value % 60;
    const timeStr = `${hours}:${String(mins).padStart(2, "0")}:${String(secs).padStart(
      2,
      "0"
    )}`;
    const status = trackingStatus.value;
    if (activeSession.value && status === "running") {
      const prod = dashboardData.value.productivity_score;
      document.title = `▶ ${timeStr} | ${prod}% Prod — Deskrona`;
    } else if (activeSession.value && status === "paused") {
      document.title = `⏸ Break Time | Deskrona`;
    } else {
      document.title = `Deskrona | Productivity Tracker`;
    }
  }, 1000) as unknown) as number;

  // Initial active task loading
  await loadActiveTask();
  await loadDailyTaskSummary(todayStr.value);

  await listen<string>("tracking-status-changed", (event) => {
    const newStatus = event.payload;
    if (newStatus === "paused" && trackingStatus.value !== "paused") {
      pauseStartedAt = Date.now();
    } else if (
      newStatus === "running" &&
      trackingStatus.value === "paused" &&
      pauseStartedAt !== null
    ) {
      pausedSeconds.value += Math.floor((Date.now() - pauseStartedAt) / 1000);
      pauseStartedAt = null;
    } else if (newStatus === "running" && activeTaskId.value) {
      invoke('cmd_set_active_task', { taskId: activeTaskId.value }).catch(console.error);
    } else if (newStatus === "stopped") {
      pausedSeconds.value = 0;
      pauseStartedAt = null;
    }
    trackingStatus.value = newStatus;
    // Auto-start pomodoro when tracking starts
    if (newStatus === "running" && pomodoroPhase.value === "idle" && settings.value?.pomodoro_auto_start) {
      startPomodoro();
    }
  });

  await listen<string | null>("active-task-changed", (event) => {
    // Only update local activeTaskId if the backend actively changed it to a task,
    // OR if it is set to null but we are NOT in stopped state.
    // If tracking is stopped, the backend fires null, but we want to retain activeTaskId locally.
    if (event.payload || trackingStatus.value !== "stopped") {
      activeTaskId.value = event.payload;
      if (event.payload) {
        invoke('cmd_list_tasks', { projectId: null }).then((allTasks: any) => {
          const task = (allTasks as Task[]).find(t => t.id === event.payload);
          activeTaskName.value = task?.name || null;
          if (task) {
            selectedProjectId.value = task.project_id;
            loadTasks(task.project_id);
          }
        });
      } else {
        activeTaskName.value = null;
      }
    }
    loadDailyTaskSummary(todayStr.value);
  });

  // Initialize timesheet date range to current week (Mon to Today)
  const today = new Date();
  const day = today.getDay();
  const diff = today.getDate() - day + (day === 0 ? -6 : 1); // adjust when day is sunday
  const monday = new Date(today.setDate(diff));
  timesheetDateRange.value = {
    from: monday.toISOString().split('T')[0],
    to: new Date().toISOString().split('T')[0]
  };
  await loadTimesheetData(timesheetDateRange.value.from, timesheetDateRange.value.to);
};

onUnmounted(() => {
  if (refreshInterval) clearInterval(refreshInterval);
  if (taskbarInterval) clearInterval(taskbarInterval);
});

// ─── Phase 8 Logic ────────────────────────────────────────────────

const loadAppConfig = async () => {
  const cfg = await proxyGetAppConfig() as AppConfig;
  appConfig.value = cfg;
  return cfg;
};

const saveAppConfig = async (cfg: AppConfig) => {
  await proxySaveAppConfig(cfg);
  appConfig.value = cfg;
};

const tryRestoreSession = async () => {
  const token = localStorage.getItem("tg_session_token");
  if (!token) return false;
  try {
    const user = await proxyValidateSession(token) as AuthUser;
    currentUser.value = user;
    sessionToken.value = token;
    return true;
  } catch {
    localStorage.removeItem("tg_session_token");
    return false;
  }
};

// @ts-ignore
async function doLogin() {
  loginError.value = "";
  loginLoading.value = true;
  try {
    const result = await proxyLogin({ username: loginUsername.value.trim(), password: loginPassword.value }) as LoginResult;
    currentUser.value = result.user;
    sessionToken.value = result.token;
    localStorage.setItem("tg_session_token", result.token);
    appScreen.value = "app";
  } catch (e: any) {
    loginError.value = e?.toString() ?? "Login failed";
  } finally {
    loginLoading.value = false;
  }
}

const doLogout = async () => {
  if (sessionToken.value) {
    try {
      await proxyLogout(sessionToken.value);
    } catch { }
    localStorage.removeItem("tg_session_token");
  }
  currentUser.value = null;
  sessionToken.value = "";
  currentView.value = "dashboard";
  appScreen.value = "login";
};

const wizardValidateStep2 = () => {
  wizardError.value = "";
  if (!wizardCompanyName.value.trim()) {
    wizardError.value = "Company name required";
    return;
  }
  if (!wizardAdminUsername.value.trim()) {
    wizardError.value = "Admin username required";
    return;
  }
  if (!wizardAdminDisplay.value.trim()) {
    wizardError.value = "Admin display name required";
    return;
  }
  if (wizardAdminPassword.value.length < 6) {
    wizardError.value = "Password must be 6+ characters";
    return;
  }
  if (wizardAdminPassword.value !== wizardConfirmPassword.value) {
    wizardError.value = "Passwords do not match";
    return;
  }
  wizardStep.value = 3;
};

const quickFillEndpoints = (serverUrl: string) => {
  if (!serverUrl || wizardApiQuickFilled.value) return;
  // Auto-fill common endpoint URLs based on server URL
  const s = serverUrl.replace(/\/+$/, '');
  const fill: Record<string, string> = {
    auth_register: `${s}/api/auth/register`,
    auth_login: `${s}/api/auth/login`,
    auth_validate: `${s}/api/auth/validate`,
    auth_logout: `${s}/api/auth/logout`,
    session_start: `${s}/api/sessions/start`,
    session_stop: `${s}/api/sessions/stop`,
    session_active: `${s}/api/sessions/active`,
    tracking_status: `${s}/api/tracking/status`,
    time_logs_sync: `${s}/api/time-logs/sync`,
    time_logs_get: `${s}/api/time-logs`,
    screenshots_upload: `${s}/api/screenshots/upload`,
    screenshots_get: `${s}/api/screenshots`,
    urls_sync: `${s}/api/urls/sync`,
    urls_get: `${s}/api/urls`,
    activity_sync: `${s}/api/activity/sync`,
    activity_get: `${s}/api/activity`,
    input_stats_get: `${s}/api/input-stats`,
    dashboard_today: `${s}/api/dashboard/today`,
    dashboard_range: `${s}/api/dashboard/range`,
    admin_users_list: `${s}/api/admin/users`,
    admin_users_create: `${s}/api/admin/users`,
    admin_stats: `${s}/api/admin/stats`,
    admin_user_screenshots: `${s}/api/admin/users/{userId}/screenshots`,
    admin_user_time_logs: `${s}/api/admin/users/{userId}/time-logs`,
    admin_user_activity: `${s}/api/admin/users/{userId}/activity`,
    admin_user_urls: `${s}/api/admin/users/{userId}/urls`,
    admin_user_input_stats: `${s}/api/admin/users/{userId}/input-stats`,
    app_categories_get: `${s}/api/categories`,
    app_categories_update: `${s}/api/categories`,
    config_get: `${s}/api/config`,
    config_save: `${s}/api/config`,
    settings_get: `${s}/api/settings`,
    settings_update: `${s}/api/settings`,
    backup_export: `${s}/api/backup/export`,
    backup_import: `${s}/api/backup/import`,
    update_check: `${s}/api/updates`,
    pomodoro_start: `${s}/api/pomodoro/start`,
    pomodoro_skip: `${s}/api/pomodoro/skip`,
    pomodoro_stop: `${s}/api/pomodoro/stop`,
    pomodoro_status: `${s}/api/pomodoro`,
    autostart_set: `${s}/api/autostart`,
    autostart_get: `${s}/api/autostart`,
    calendar_month: `${s}/api/calendar/month`,
    projects_sync: `${s}/api/projects/sync`,
    tasks_sync: `${s}/api/tasks/sync`,
    task_rules_sync: `${s}/api/task-rules/sync`,
    reset_app: `${s}/api/reset`,
  };
  for (const [key, url] of Object.entries(fill)) {
    if (apiConfig.value.endpoints[key as EndpointKey]) {
      apiConfig.value.endpoints[key as EndpointKey].url = url;
      apiConfig.value.endpoints[key as EndpointKey].enabled = true;
    }
  }
  wizardApiQuickFilled.value = true;
  saveApiConfigToDisk();
};

const wizardFinish = async () => {
  wizardError.value = "";
  wizardLoading.value = true;
  try {
    if (wizardMode.value === "multi_user") {
      const result = await proxyRegisterCompany({
        company_name: wizardCompanyName.value.trim(),
        admin_username: wizardAdminUsername.value.trim(),
        admin_display_name: wizardAdminDisplay.value.trim(),
        admin_password: wizardAdminPassword.value,
      }) as LoginResult;
      currentUser.value = result.user;
      sessionToken.value = result.token;
      localStorage.setItem("tg_session_token", result.token);
    }

    // Save API config set during wizard
    apiConfig.value.mode = wizardApiMode.value;
    apiConfig.value.bearer_token = wizardApiBearerToken.value;
    if (wizardApiServerUrl.value && !wizardApiQuickFilled.value) {
      quickFillEndpoints(wizardApiServerUrl.value);
    }
    await saveApiConfigToDisk();

    const cfg: AppConfig = { mode: wizardMode.value, setup_done: true };
    await saveAppConfig(cfg);
    appScreen.value = "app";
  } catch (e: any) {
    wizardError.value = e?.toString() ?? "Setup failed";
  } finally {
    wizardLoading.value = false;
  }
};

const loadAdminData = async () => {
  if (!isAdmin.value || !currentUser.value) return;
  try {
    const [users, stats, categories] = await Promise.all([
      proxyGetCompanyUsers(currentUser.value.company_id) as Promise<AuthUser[]>,
      proxyGetAdminStats(currentUser.value.company_id) as Promise<UserProductivityStat[]>,
      proxyGetAllAppCategories() as Promise<AppCategoryEntry[]>,
    ]);
    adminUsers.value = users;
    adminStats.value = stats;
    adminCategories.value = categories;
  } catch (e) {
    console.error("Failed to load admin data", e);
  }
};

const loadUserDetail = async (
  user: AuthUser,
  tab: "screenshots" | "timelogs" | "activity" | "urls" | "categories",
  append = false
) => {
  selectedUser.value = user;
  adminTab.value = tab;
  adminDrillLoading.value = true;
  const from = adminDrillDate.value;
  const to = adminDrillDate.value;
  try {
    if (tab === "screenshots") {
      if (!append) adminScreenshotsOffset.value = 0;
      const data = await proxyGetUserScreenshots(user.id, from, to, pageSize, adminScreenshotsOffset.value) as AdminScreenshot[];
      adminUserScreenshots.value = append
        ? [...adminUserScreenshots.value, ...data]
        : data;
    } else if (tab === "timelogs") {
      if (!append) adminTimeLogsOffset.value = 0;
      const data = await proxyGetUserTimeLogs(user.id, from, to, pageSize, adminTimeLogsOffset.value) as AdminTimeLog[];
      adminUserTimeLogs.value = append ? [...adminUserTimeLogs.value, ...data] : data;
    } else if (tab === "activity") {
      if (!append) adminActivityOffset.value = 0;
      const [acts, stats] = await Promise.all([
        proxyGetUserActivity(user.id, from, to, pageSize, adminActivityOffset.value) as Promise<AdminActivity[]>,
        proxyGetUserInputStats(user.id, from, to) as Promise<InputStats>,
      ]);
      adminUserActivity.value = append ? [...adminUserActivity.value, ...acts] : acts;
      adminInputStats.value = stats;
    } else if (tab === "urls") {
      if (!append) adminUrlsOffset.value = 0;
      const data = await proxyGetUserUrls(user.id, from, to, pageSize, adminUrlsOffset.value) as UrlEntryFull[];
      adminUserUrls.value = append ? [...adminUserUrls.value, ...data] : data;
    }
  } catch (e) {
    console.error("Failed to load user detail", e);
  } finally {
    adminDrillLoading.value = false;
  }
};

const loadMoreAdminScreenshots = () => {
  if (!selectedUser.value) return;
  adminScreenshotsOffset.value += pageSize;
  loadUserDetail(selectedUser.value, "screenshots", true);
};
const loadMoreAdminLogs = () => {
  if (!selectedUser.value) return;
  adminTimeLogsOffset.value += pageSize;
  loadUserDetail(selectedUser.value, "timelogs", true);
};
const loadMoreAdminActivity = () => {
  if (!selectedUser.value) return;
  adminActivityOffset.value += pageSize;
  loadUserDetail(selectedUser.value, "activity", true);
};
const loadMoreAdminUrls = () => {
  if (!selectedUser.value) return;
  adminUrlsOffset.value += pageSize;
  loadUserDetail(selectedUser.value, "urls", true);
};

const refreshAdminDrill = async () => {
  if (!selectedUser.value) return;
  if (adminTab.value === "team") return;
  await loadUserDetail(selectedUser.value, adminTab.value);
};

const doCreateUser = async () => {
  createUserError.value = "";
  createUserLoading.value = true;
  try {
    const created = await proxyCreateUser(currentUser.value?.company_id, newUser.value) as AuthUser;
    adminUsers.value.push(created);
    newUser.value = { username: "", display_name: "", password: "", role: "employee" };
    showCreateUser.value = false;
  } catch (e: any) {
    createUserError.value = e?.toString() ?? "Failed";
  } finally {
    createUserLoading.value = false;
  }
};

// Boot sequence
onMounted(async () => {
  // Apply stored theme BEFORE showing any screen (avoids flash)
  try {
    const s = await proxyGetSettings() as Settings;
    settings.value = s;
    locale.value = s.language;
    applyTheme();
  } catch { }

  // Load API config
  try {
    await loadApiConfigFromDisk();
  } catch { }

  // Listen for queue updates
  window.addEventListener('api-queue-update', (e: Event) => {
    queueStats.value = (e as CustomEvent).detail;
  });

  // Sync queue stats periodically
  setInterval(() => {
    queueStats.value = getQueueStats() as any;
  }, 5000);

  // Track last sync time
  setInterval(() => {
    try {
      const raw = localStorage.getItem('deskrona_sync_state');
      if (raw) {
        const state = JSON.parse(raw);
        if (state.lastSyncTime) lastSyncTime.value = state.lastSyncTime;
      }
    } catch {}
  }, 10000);

  // Start sync if online mode
  startSync();

  const cfg = await loadAppConfig();
  pendingMode.value = cfg.mode;
  // Periodic refresh
  setInterval(async () => {
    if (appScreen.value === 'app') {
      await refreshDashboard();
    }
  }, 10000); // 10s

  // Real-time second counter for dashboard matching
  setInterval(() => {
    if (appScreen.value === 'app' && trackingStatus.value === 'running') {
      const isViewingToday = currentView.value === "dashboard" ||
        (currentView.value === "productivity" && filterType.value === "calendar" && selectedCalendarDay.value === todayStr.value);
      if (isViewingToday) {
        dashboardData.value.total_active_seconds += 1;
      }
    }
  }, 1000);

  if (!cfg.setup_done) {
    appScreen.value = "wizard";
  } else if (cfg.mode === "multi_user") {
    const restored = await tryRestoreSession();
    appScreen.value = restored ? "app" : "login";
  } else {
    const restored = await tryRestoreSession(); // Even in single user, restore ID if needed
    appScreen.value = "app";
  }
});

watch(appScreen, (s) => {
  if (s === "app") initApp();
});

watch(currentView, (v) => {
  if (v === "admin") loadAdminData();
  else if (v === "calendar" || (v === "productivity" && filterType.value === "calendar")) loadCalendarMonth();
  else if (["trackings", "urls", "screenshots", "activity"].includes(v))
    loadFilteredData();
});

const doResetApp = async () => {
  if (!confirm(t("message.resetAppConfirm"))) return;
  try {
    await proxyResetApp();
    localStorage.removeItem("tg_session_token");
    currentUser.value = null;
    sessionToken.value = "";
    appConfig.value = { mode: "single_user", setup_done: false };
    wizardStep.value = 1;
    wizardMode.value = "single_user";
    wizardCompanyName.value = "";
    wizardAdminUsername.value = "";
    wizardAdminDisplay.value = "";
    wizardAdminPassword.value = "";
    wizardConfirmPassword.value = "";
    wizardError.value = "";
    pendingMode.value = "single_user";
    wizardApiMode.value = 'offline';
    wizardApiServerUrl.value = '';
    wizardApiBearerToken.value = '';
    wizardApiQuickFilled.value = false;
    appScreen.value = "wizard";
  } catch (e: any) {
    alert(t("message.resetFailed") + ": " + (e?.toString() ?? t("message.unknownError")));
  }
};

const doChangeMode = async () => {
  if (!appConfig.value) return;
  const newCfg: AppConfig = { ...appConfig.value, mode: pendingMode.value };
  // If switching to multi_user and not set up yet — run wizard
  if (pendingMode.value === "multi_user") {
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
    <p class="loading-text">{{ t("message.loadingApp") }}</p>
  </div>

  <!-- First-Run Wizard -->
  <div v-else-if="appScreen === 'wizard'" class="fullscreen-center wizard-bg">
    <div class="wizard-card">
      <div class="wizard-logo">
        <img src="/favicon.png" width="52" height="52" />
        <h1>{{ t("message.wizardWelcome") }}</h1>
        <p>{{ t("message.wizardSubtitle") }}</p>
      </div>

      <!-- Step 1: Mode -->
      <div v-if="wizardStep === 1" class="wizard-step">
        <h2>{{ t("message.wizardChooseMode") }}</h2>
        <div class="mode-cards">
          <div :class="['mode-card', { 'mode-card-active': wizardMode === 'single_user' }]"
            @click="wizardMode = 'single_user'">
            <span class="mode-icon">👤</span>
            <strong>{{ t("message.modeSingleUser") }}</strong>
            <p>{{ t("message.wizardSingleDesc") }}</p>
          </div>
          <div :class="['mode-card', { 'mode-card-active': wizardMode === 'multi_user' }]"
            @click="wizardMode = 'multi_user'">
            <span class="mode-icon">🏢</span>
            <strong>{{ t("message.modeMultiUser") }}</strong>
            <p>{{ t("message.wizardTeamDesc") }}</p>
          </div>
        </div>
        <button class="btn-wizard-next" @click="wizardStep = wizardMode === 'single_user' ? 3 : 2">
          {{ t("message.wizardContinue") }}
        </button>
      </div>

      <!-- Step 2: Company + Admin setup (multi_user only) -->
      <div v-if="wizardStep === 2" class="wizard-step">
        <h2>{{ t("message.wizardSetupCompany") }}</h2>
        <div class="wizard-form">
          <label>{{ t("message.wizardCompanyName") }}</label>
          <input type="text" v-model="wizardCompanyName" :placeholder="t('message.wizardCompanyName')" />
          <label>{{ t("message.wizardAdminUsername") }}</label>
          <input type="text" v-model="wizardAdminUsername" :placeholder="t('message.admin')" />
          <label>{{ t("message.wizardAdminDisplay") }}</label>
          <input type="text" v-model="wizardAdminDisplay" :placeholder="t('message.displayName')" />
          <label>{{ t("message.wizardAdminPassword") }}</label>
          <input type="password" v-model="wizardAdminPassword" :placeholder="t('message.wizardAdminPassword')" />
          <label>{{ t("message.wizardConfirmPassword") }}</label>
          <input type="password" v-model="wizardConfirmPassword" :placeholder="t('message.wizardConfirmPassword')" />
        </div>
        <div v-if="wizardError" class="wizard-error">{{ wizardError }}</div>
        <div class="wizard-actions">
          <button class="btn-wizard-back" @click="
            wizardStep = 1;
          wizardError = '';
          ">
            {{ t("message.wizardBack") }}
          </button>
          <button class="btn-wizard-next" :disabled="wizardLoading" @click="wizardValidateStep2">
            {{ t("message.wizardNext") }}
          </button>
        </div>
      </div>

      <!-- Step 3: API Config -->
      <div v-if="wizardStep === 3" class="wizard-step">
        <h2>{{ t("message.wizardApiConfig") }}</h2>
        <div class="wizard-form">
          <label>{{ t("message.wizardApiMode") }}</label>
          <div class="mode-toggle-switch" style="margin-bottom: 12px;">
            <button :class="['toggle-btn', { active: wizardApiMode === 'offline' }]" @click="wizardApiMode = 'offline'">
              <span class="toggle-icon">📴</span> {{ t("message.wizardApiModeOffline") }}
            </button>
            <button :class="['toggle-btn', { active: wizardApiMode === 'online' }]" @click="wizardApiMode = 'online'">
              <span class="toggle-icon">🌐</span> {{ t("message.wizardApiModeOnline") }}
            </button>
          </div>
          <template v-if="wizardApiMode === 'online'">
            <label>{{ t("message.wizardApiServerUrl") }}</label>
            <input type="text" v-model="wizardApiServerUrl" :placeholder="t('message.wizardApiServerUrlPlaceholder')" @input="wizardApiQuickFilled = false" />
            <label>{{ t("message.wizardApiBearerToken") }}</label>
            <input type="password" v-model="wizardApiBearerToken" placeholder="eyJhbGci..." />
            <button class="btn-wizard-back" style="margin-top: 4px; text-align: center;" @click="quickFillEndpoints(wizardApiServerUrl)" :disabled="!wizardApiServerUrl || wizardApiQuickFilled">
              {{ t("message.wizardApiQuickFill") }}
            </button>
            <small style="display: block; margin-top: 6px; color: var(--text-muted); font-size: 0.8rem;">{{ t("message.wizardApiNote") }}</small>
          </template>
        </div>
        <div v-if="wizardError" class="wizard-error">{{ wizardError }}</div>
        <div class="wizard-actions">
          <button class="btn-wizard-back" @click="
            wizardStep = wizardMode === 'single_user' ? 1 : 2;
          wizardError = '';
          ">
            {{ t("message.wizardBack") }}
          </button>
          <button class="btn-wizard-next" @click="wizardStep = 4">
            {{ t("message.wizardContinue") }}
          </button>
        </div>
      </div>

      <!-- Step 4: Confirm -->
      <div v-if="wizardStep === 4" class="wizard-step">
        <h2>{{ t("message.wizardAllSet") }}</h2>
        <div class="confirm-summary">
          <div class="confirm-row">
            <span>{{ t("message.wizardMode") }}</span><strong>{{
              wizardMode === "single_user"
                ? t("message.modeSingleUser")
                : t("message.modeMultiUser")
            }}</strong>
          </div>
          <div v-if="wizardMode === 'multi_user'" class="confirm-row">
            <span>{{ t("message.wizardCompany") }}</span><strong>{{ wizardCompanyName }}</strong>
          </div>
          <div v-if="wizardMode === 'multi_user'" class="confirm-row">
            <span>{{ t("message.wizardAdmin") }}</span><strong>{{ wizardAdminUsername }}</strong>
          </div>
          <div class="confirm-row">
            <span>{{ t("message.wizardApiInfo") }}</span><strong>{{
              wizardApiMode === "online"
                ? t("message.wizardApiModeOnline")
                : t("message.wizardApiModeOffline")
            }}</strong>
          </div>
          <div v-if="wizardApiMode === 'online' && wizardApiServerUrl" class="confirm-row">
            <span>{{ t("message.wizardApiServerUrl") }}</span><strong>{{ wizardApiServerUrl }}</strong>
          </div>
          <div v-if="wizardApiMode === 'online' && wizardApiBearerToken" class="confirm-row">
            <span>{{ t("message.wizardApiBearerToken") }}</span><strong>{{ t("message.wizardApiTokenSet") }}</strong>
          </div>
        </div>
        <div v-if="wizardError" class="wizard-error">{{ wizardError }}</div>
        <div class="wizard-actions">
          <button class="btn-wizard-back" @click="
            wizardStep = 3;
          wizardError = '';
          ">
            {{ t("message.wizardBack") }}
          </button>
          <button class="btn-wizard-next" :disabled="wizardLoading" @click="wizardFinish">
            {{ wizardLoading ? t("message.wizardLaunching") : t("message.wizardLaunch") }}
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- Login Screen -->
  <div v-else-if="appScreen === 'login'" class="fullscreen-center login-bg">
    <div class="login-card">
      <div class="login-logo">
        <img src="/favicon.png" width="52" height="52" />
        <h1>{{ t("message.loginTitle") }}</h1>
        <p>{{ t("message.loginSubtitle") }}</p>
      </div>
      <div class="login-form">
        <label>{{ t("message.loginUsername") }}</label>
        <input type="text" v-model="loginUsername" :placeholder="t('message.loginUsername')" @keyup.enter="doLogin" />
        <label>{{ t("message.loginPassword") }}</label>
        <input type="password" v-model="loginPassword" :placeholder="t('message.loginPassword')" @keyup.enter="doLogin" />
      </div>
      <div v-if="loginError" class="login-error">{{ loginError }}</div>
      <button class="btn-login" :disabled="loginLoading" @click="doLogin">
        {{ loginLoading ? t("message.loginSigningIn") : t("message.loginSignIn") }}
      </button>
    </div>
  </div>

  <div class="app-layout">
    <aside class="sidebar">
      <!-- Logo -->
      <div class="logo">
        <img src="/favicon.png" width="32" height="32" />
        <h2>Deskrona</h2>
      </div>

      <!-- Sidebar Body (Scrollable nav and controls) -->
      <div class="sidebar-body">
        <!-- Navigation -->
        <div class="nav-scroll-wrapper">
          <button v-if="navCanScrollLeft" class="nav-scroll-btn nav-scroll-left" @click="scrollNav('left')" aria-label="Scroll left">‹</button>
          <nav ref="navRef" @scroll="updateNavScroll">
            <button :class="{ active: currentView === 'dashboard' }" @click="currentView = 'dashboard'">
              📊 {{ t("message.dashboard") }}
            </button>
            <button :class="{ active: currentView === 'projects' }" @click="currentView = 'projects'; loadProjects();">
              📁 {{ t('message.projects') }}
            </button>
            <button :class="{ active: currentView === 'trackings' }" @click="currentView = 'trackings'">
              📋 {{ t("message.trackings") }}
            </button>
            <button :class="{ active: currentView === 'urls' }" @click="currentView = 'urls'">
              🌐 {{ t("message.urls") }}
            </button>
            <button :class="{ active: currentView === 'activity' }" @click="currentView = 'activity'">
              📈 {{ t("message.activity") }}
            </button>
            <button :class="{ active: currentView === 'screenshots' }" @click="currentView = 'screenshots'">
              📸 {{ t("message.screenshots") }}
            </button>
            <button :class="{ active: currentView === 'productivity' }" @click="currentView = 'productivity'">
              ⭐ {{ t("message.productivity") }}
            </button>
            <button :class="{ active: currentView === 'timesheet' }" @click="currentView = 'timesheet';">
              📅 {{ t('message.timesheet') }}
            </button>
            <button :class="{ active: currentView === 'settings' }" @click="currentView = 'settings'">
              ⚙️ {{ t("message.settings") }}
            </button>
            <button v-if="isMultiUser && currentUser?.role === 'admin'" :class="{ active: currentView === 'admin' }"
              @click="currentView = 'admin'">
              👑 {{ t("message.admin") }}
            </button>
          </nav>
          <button v-if="navCanScrollRight" class="nav-scroll-btn nav-scroll-right" @click="scrollNav('right')" aria-label="Scroll right">›</button>
        </div>

      </div>

      <!-- Combined Tracking & Active Task Control Card (pinned to bottom) -->
      <div class="tracking-control">
        <div v-if="activeTaskId" class="sidebar-active-task-info" style="margin-bottom: 10px; padding-bottom: 10px; border-bottom: 1px solid var(--border-color); display: flex; flex-direction: column; gap: 4px;">
          <div style="display: flex; align-items: center; justify-content: space-between; font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-muted);">
            <span>Active Task</span>
            <span v-if="trackingStatus === 'running'" class="active-pulse" style="width: 6px; height: 6px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 6px var(--accent); animation: pulse 1.5s infinite;"></span>
          </div>
          <div style="font-weight: 700; font-size: 0.85rem; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
            {{ activeTaskName || 'Loading task...' }}
          </div>
          <div style="font-size: 0.75rem; font-variant-numeric: tabular-nums; font-weight: 500; color: var(--text-muted);">
            {{ formatTime(dailyTaskSummary.find(d => d.task_id === activeTaskId)?.total_seconds || 0) }} today
          </div>
        </div>

        <div class="tracking-status" :class="trackingStatus">
          <span v-if="trackingStatus === 'running'" class="status-dot running"></span>
          <span v-else-if="trackingStatus === 'paused'" class="status-dot paused"></span>
          <span v-else class="status-dot stopped"></span>
          <span class="status-label">
            {{
              trackingStatus === "running"
                ? t("message.trackingRunning")
                : trackingStatus === "paused"
                  ? t("message.trackingPaused")
                  : t("message.trackingStopped")
            }}
          </span>
        </div>

        <div class="tracking-buttons">
          <button v-if="trackingStatus === 'running'" class="btn-tracking btn-pause" @click="setTracking('paused')">
            ⏸ {{ t("message.pauseTracking") }}
          </button>
          <button v-if="trackingStatus === 'running'" class="btn-tracking btn-stop-track"
            @click="setTracking('stopped')">
            ⏹ {{ t("message.stopTracking") }}
          </button>
          <button v-if="trackingStatus === 'paused'" class="btn-tracking btn-resume" @click="setTracking('running')">
            ▶ {{ t("message.resumeTracking") }}
          </button>
          <button v-if="trackingStatus === 'paused'" class="btn-tracking btn-stop-track"
            @click="setTracking('stopped')">
            ⏹ {{ t("message.stopTracking") }}
          </button>
          <button v-if="trackingStatus === 'stopped'" class="btn-tracking btn-start-track"
            @click="setTracking('running')">
            ▶ {{ t("message.startTracking") }}
          </button>
        </div>
      </div>

      <!-- API Queue Status -->
      <div v-if="isOnline() && (queueStats.pending > 0 || queueStats.failed > 0)" class="api-queue-indicator">
        <div class="queue-info">
          <span class="queue-icon">📡</span>
          <span v-if="queueStats.pending > 0" class="queue-pending">{{ queueStats.pending }} pending</span>
          <span v-if="queueStats.failed > 0" class="queue-failed" @click="retryFailedJobs" title="Click to retry">⚠ {{ queueStats.failed }} failed</span>
        </div>
      </div>

      <!-- User badge + logout (multi-user only) -->
      <div v-if="isMultiUser && currentUser" class="user-badge">
        <span class="user-avatar">{{
          currentUser.display_name.charAt(0).toUpperCase()
        }}</span>
        <div class="user-info">
          <span class="user-name">{{ currentUser.display_name }}</span>
          <span :class="[
            'role-badge',
            currentUser.role === 'admin' ? 'role-admin' : 'role-emp',
          ]">{{
              currentUser.role === "admin"
                ? t("message.adminRole")
                : t("message.employeeRole")
            }}</span>
        </div>
        <button class="btn-logout" @click="doLogout" :title="t('message.signOut')">
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"></path>
            <polyline points="16 17 21 12 16 7"></polyline>
            <line x1="21" y1="12" x2="9" y2="12"></line>
          </svg>
        </button>
      </div>
    </aside>

    <!-- Floating Wellness FAB -->
    <div class="floating-wellness-widget">
      <!-- Popover Panel -->
      <div v-if="wellnessWidgetExpanded" class="wellness-popover">
        <div class="popover-header">
          <span class="popover-title">🧘 Wellness</span>
          <button class="popover-close" @click="wellnessWidgetExpanded = false">×</button>
        </div>

        <!-- Focus Mode Section -->
        <div class="popover-section">
          <div class="popover-section-title">🍅 {{ t("message.pomodoro") || 'Focus Mode' }}</div>
          <div v-if="pomodoroPhase !== 'idle'" class="session-active" style="gap: 4px;">
            <span class="pulse-dot"></span>
            <span style="font-size: 0.85rem; font-weight: 700;">{{ pomodoroPhase === 'focus' ? t("message.pomodoroFocus") : t("message.pomodoroBreak") }}</span>
            <div class="pomodoro-timer" style="font-size: 1.4rem; font-weight: 800; color: var(--accent); margin: 4px 0;">{{ pomodoroFormatted }}</div>
            <div style="font-size: 0.8rem; color: var(--text-muted); margin-bottom: 6px;">🍅 {{ pomodoroCountToday }}</div>
            <button class="btn-stop" @click="stopPomodoro" style="width: 100%;">⏹ {{ t("message.pomodoroEnd") }}</button>
          </div>
          <div v-else>
            <button class="btn-start" @click="startPomodoro" style="width: 100%;">▶ {{ t("message.pomodoroStart") }}</button>
          </div>
        </div>

        <!-- Break Reminder Section -->
        <div v-if="settings.break_reminder_enabled" class="popover-section">
          <div class="popover-section-title">🧘 {{ t("message.breakReminder") }}</div>
          <div class="break-fab-status">
            <span v-if="breakState.state === 'counting'" class="fab-break-badge counting">⏱ {{ breakCountdownFormatted }}</span>
            <span v-else-if="breakState.state === 'on_break'" class="fab-break-badge on-break">🧘 {{ t("message.breakOnBreak") }}</span>
            <span v-else-if="breakState.state === 'pre_break'" class="fab-break-badge pre-break">⚠ {{ t("message.breakPreBreak") }}</span>
            <span v-else-if="breakState.state === 'paused'" class="fab-break-badge paused">⏸ {{ t("message.breakPaused") }}</span>
            <span v-else class="fab-break-badge idle">○ {{ t("message.breakIdle") }}</span>
          </div>
          <div style="font-size: 0.75rem; color: var(--text-muted); margin: 6px 0 8px; text-align: center;">{{ breakState.mini_completed }}/{{ settings.break_mini_breaks_before_long || 4 }} {{ t("message.miniBreaksDone") }}</div>
          <div style="display: flex; gap: 6px;">
            <button v-if="breakState.state === 'paused'" class="btn-tracking btn-resume" style="flex: 1; padding: 6px;" @click="breakResume">▶ {{ t("message.breakResume") }}</button>
            <button v-else-if="breakState.state !== 'idle'" class="btn-tracking btn-pause" style="flex: 1; padding: 6px;" @click="breakPause(30)">⏸ {{ t("message.breakPause") }}</button>
            <button v-if="breakState.state !== 'idle' && breakState.state !== 'paused'" class="btn-tracking btn-stop-track" style="flex: 1; padding: 6px;" @click="breakReset">🔄 {{ t("message.breakReset") }}</button>
          </div>
        </div>
      </div>

      <!-- FAB Button -->
      <button class="wellness-fab" @click="wellnessWidgetExpanded = !wellnessWidgetExpanded">
        <span class="fab-icon">
          <template v-if="pomodoroPhase !== 'idle'">🍅</template>
          <template v-else-if="breakState.state === 'on_break'">🧘</template>
          <template v-else>💆</template>
        </span>
        <span class="fab-text">
          <template v-if="pomodoroPhase !== 'idle'">{{ pomodoroFormatted }}</template>
          <template v-else-if="breakState.state === 'counting'">⏱ {{ breakCountdownFormatted }}</template>
          <template v-else-if="breakState.state === 'on_break'">{{ t("message.breakOnBreak") }}</template>
          <template v-else>Wellness</template>
        </span>
      </button>
    </div>

    <main class="main-content">
      <!-- DASHBOARD VIEW -->
      <div v-if="currentView === 'dashboard'" class="view-dashboard">
        <header class="view-header" style="margin-bottom: 15px;">
          <div style="display: flex; flex-direction: column; gap: 4px;">
            <h1>{{ t("message.todaySummary") }}</h1>
            <div v-if="activeTaskId" style="display: flex; align-items: center; gap: 6px; font-size: 0.85rem; color: var(--success); font-weight: 600;">
              <span class="active-pulse" style="width: 8px; height: 8px; border-radius: 50%; background: var(--success); display: inline-block;"></span>
              <span>Running: {{ activeTaskName }}</span>
            </div>
          </div>
          <button class="btn-browse" @click="refreshDashboard">
            🔄 {{ t("message.refresh") }}
          </button>
        </header>

        <div class="summary-cards">
          <div class="card premium-card summary-metric-card summary-active">
            <h3>{{ t("message.activeTime") }}</h3>
            <p class="big-stat">{{ formatTime(dashboardData.total_active_seconds) }}</p>
          </div>
          <div class="card premium-card summary-metric-card summary-idle">
            <h3>{{ t("message.idleTime") }}</h3>
            <p class="big-stat idle">
              {{ formatTime(dashboardData.total_idle_seconds) }}
            </p>
          </div>
          <div class="card premium-card summary-metric-card summary-total">
            <h3>{{ t("message.totalTime") }}</h3>
            <p class="big-stat total">
              {{
                formatTime(
                  dashboardData.total_active_seconds + dashboardData.total_idle_seconds
                )
              }}
            </p>
          </div>
          <div class="card premium-card summary-input-card summary-activity">
            <h3>⚡ {{ t("message.activity") }}</h3>
            <div class="summary-input-grid">
              <div class="summary-input-chip keyboard">
                <span class="chip-icon">⌨️</span>
                <div class="chip-text">
                  <small>{{ t("message.keyboard") }}</small>
                  <strong>{{ dashboardData.keyboard_count.toLocaleString() }}</strong>
                </div>
              </div>
              <div class="summary-input-chip mouse">
                <span class="chip-icon">🖱</span>
                <div class="chip-text">
                  <small>{{ t("message.mouse") }}</small>
                  <strong>{{ dashboardData.mouse_count.toLocaleString() }}</strong>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Privacy Notice -->
        <div v-if="trackingStatus !== 'stopped' && !privacyNoticeDismissed" class="privacy-notice">
          <div class="privacy-content">
            <span class="privacy-icon">🔒</span>
            <div class="privacy-text">
              <strong>{{ t("message.privacyNotice") }}</strong>
              <p>{{ t("message.privacyNoticeDesc") }}</p>
            </div>
            <button class="privacy-dismiss" :title="t('message.dismiss')" :aria-label="t('message.dismiss')"
              @click="privacyNoticeDismissed = true">
              ×
            </button>
          </div>
        </div>

        <!-- Update Notification -->
        <div v-if="updateAvailable" class="update-notification">
          <div class="update-content">
            <span class="update-icon">🔄</span>
            <div class="update-text">
              <strong>{{ t("message.updateAvailable") }}</strong>
              <p>{{ t("message.updateDesc", { version: latestVersion }) }}</p>
            </div>
            <a :href="'https://github.com/AbiruzzamanMolla/Deskrona-Time-Tracking-Software/releases/tag/v' +
              latestVersion
              " target="_blank" class="btn-update">
              {{ t("message.downloadUpdate") }}
            </a>
          </div>
        </div>

        <div v-else-if="!updateCheckLoading" class="update-check-bar">
          <span class="version-text">{{
            t("message.currentVersion", { version: currentVersion })
          }}</span>
          <button class="btn-check-update" @click="checkForUpdates">
            {{ t("message.checkUpdates") }}
          </button>
        </div>

        <div v-if="updateCheckLoading" class="update-loading">
          {{ t("message.checkingUpdates") }}
        </div>

        <div v-if="updateError" class="update-error">
          {{ updateError }}
        </div>

        <!-- App Usage Table -->
        <div class="section-block">
          <h2>{{ t("message.topApps") }}</h2>
          <div v-if="dashboardData.app_stats.length > 0" class="app-table"
            :class="{ 'with-category': !isMultiUser || isAdmin }">
            <div class="app-row header-row">
              <span class="col-app">{{ t("message.appName") }} & %</span>
              <span class="col-time">{{ t("message.timeSpent") }}</span>
              <span class="col-switches">{{ t("message.switches") }}</span>
              <span v-if="!isMultiUser || isAdmin" class="col-category">{{
                t("message.category")
              }}</span>
            </div>
            <div v-for="app in dashboardData.app_stats" :key="app.app_name" class="app-row">
              <div class="col-app-info">
                <div class="app-main-info">
                  <span class="app-name">
                    <span class="app-icon">{{ appIcon(app.app_name) }}</span>
                    {{ app.app_name }}
                  </span>
                </div>
                <div class="app-progress-container">
                  <div class="bar-bg">
                    <div class="bar-fill" :style="{ width: percentage(app.total_seconds) + '%' }"></div>
                  </div>
                  <span class="app-percent-text">{{ percentage(app.total_seconds) }}%</span>
                </div>
              </div>
              <span class="col-time-val">{{ formatTime(app.total_seconds) }}</span>
              <span class="col-switches-val">{{ app.session_count }}</span>
              <div v-if="!isMultiUser || isAdmin" class="col-category-val">
                <select v-model="app.category" @change="updateCategory(app.app_name, app.category)"
                  :class="['badge-select', app.category]" :disabled="isMultiUser && !isAdmin">
                  <option value="productive">{{ t("message.productive") }}</option>
                  <option value="neutral">{{ t("message.neutral") }}</option>
                  <option value="unproductive">{{ t("message.unproductive") }}</option>
                </select>
              </div>
            </div>
          </div>
          <div v-else class="empty-state">
            <p>{{ t("message.noData") }}</p>
          </div>
        </div>

        <!-- Recent URLs -->
        <div class="section-block">
          <h2>🌐 {{ t("message.recentUrls") }}</h2>
          <div v-if="dashboardData.recent_urls.length > 0" class="url-list">
            <div v-for="entry in dashboardData.recent_urls" :key="entry.timestamp" class="url-row">
              <span class="url-time">{{ formatTimestamp(entry.timestamp) }}</span>
              <span class="url-text" :title="entry.url">{{ entry.url }}</span>
            </div>
          </div>
          <div v-else class="empty-state">
            <p>{{ t("message.noUrls") }}</p>
          </div>
        </div>
      </div>

      <!-- TRACKINGS VIEW -->
      <div v-if="currentView === 'trackings'" class="view-trackings">
        <header class="view-header">
          <h1>{{ t("message.trackings") }}</h1>
          <div class="filter-controls">
            <button class="btn-browse" @click="loadFilteredData(false)">
              🔄 {{ t("message.refresh") }}
            </button>
            <select v-model="filterType">
              <option value="daily">{{ t("message.filterDaily") }}</option>
              <option value="weekly">{{ t("message.filterWeekly") }}</option>
              <option value="monthly">{{ t("message.filterMonthly") }}</option>
              <option value="yearly">{{ t("message.filterYearly") }}</option>
              <option value="custom">{{ t("message.filterCustom") }}</option>
            </select>
            <template v-if="filterType === 'custom'">
              <input type="date" v-model="customFromDate" />
              <span> - </span>
              <input type="date" v-model="customToDate" />
            </template>
          </div>
        </header>

        <div class="app-table">
          <div class="app-row header-row" style="grid-template-columns: 2fr 3fr 1fr 1fr 1fr">
            <span>{{ t("message.appName") }}</span>
            <span>{{ t("message.title") }}</span>
            <span>{{ t("message.start") }}</span>
            <span>{{ t("message.end") }}</span>
            <span>{{ t("message.timeSpent") }}</span>
          </div>
          <div v-for="log in timeLogsList" :key="log.id" class="app-row"
            style="grid-template-columns: 2fr 3fr 1fr 1fr 1fr">
            <span class="app-name">
              <span class="app-icon">{{ appIcon(log.app_name) }}</span>
              {{ log.app_name }}
            </span>
            <span class="url-text" :title="log.window_title">{{ log.window_title }}</span>
            <span class="url-time">{{ formatTimestamp(log.start_time) }}</span>
            <span class="url-time">{{
              log.end_time ? formatTimestamp(log.end_time) : "-"
            }}</span>
            <span class="app-time">{{ formatTime(log.duration) }}</span>
          </div>
          <div v-if="timeLogsList.length === 0" class="empty-state">
            <p>{{ t("message.noData") }}</p>
          </div>
          <div v-else-if="timeLogsList.length % pageSize === 0" class="load-more-container">
            <button class="btn-start" @click="loadMoreTrackings">
              {{ t("message.loadMore") }}
            </button>
          </div>
        </div>
      </div>

      <!-- URLS VIEW -->
      <div v-if="currentView === 'urls'" class="view-urls">
        <header class="view-header">
          <h1>{{ t("message.urls") }}</h1>
          <div class="filter-controls">
            <button class="btn-browse" @click="loadFilteredData(false)">
              🔄 {{ t("message.refresh") }}
            </button>
            <select v-model="filterType">
              <option value="daily">{{ t("message.filterDaily") }}</option>
              <option value="weekly">{{ t("message.filterWeekly") }}</option>
              <option value="monthly">{{ t("message.filterMonthly") }}</option>
              <option value="yearly">{{ t("message.filterYearly") }}</option>
              <option value="custom">{{ t("message.filterCustom") }}</option>
            </select>
            <template v-if="filterType === 'custom'">
              <input type="date" v-model="customFromDate" />
              <span> - </span>
              <input type="date" v-model="customToDate" />
            </template>
          </div>
        </header>

        <div class="url-list">
          <div v-for="entry in urlsList" :key="entry.id" class="url-row url-row-clickable"
            :class="{ 'url-row-expanded': expandedUrlRows.has(entry.id) }" @click="toggleUrlRow(entry.id)">
            <span class="url-time">{{ formatTimestamp(entry.timestamp) }}</span>
            <div class="url-entry-content">
              <span class="url-text" :title="entry.url">{{ getHistoryTitle(entry.url) }}</span>
              <a v-if="expandedUrlRows.has(entry.id) && getHistoryUrl(entry.url)" class="url-expanded-link"
                :href="getHistoryUrl(entry.url)" target="_blank" rel="noreferrer noopener"
                @click.stop>
                {{ getHistoryUrl(entry.url) }}
              </a>
            </div>
          </div>
          <div v-if="urlsList.length === 0" class="empty-state">
            <p>{{ t("message.noUrls") }}</p>
          </div>
          <div v-else-if="urlsList.length % pageSize === 0" class="load-more-container">
            <button class="btn-start" @click="loadMoreUrls">
              {{ t("message.loadMore") }}
            </button>
          </div>
        </div>
      </div>

      <!-- ACTIVITY VIEW -->
      <div v-if="currentView === 'activity'" class="view-activity">
        <header class="view-header">
          <h1>{{ t("message.detailedActivity") }}</h1>
          <div class="filter-controls">
            <button class="btn-browse" @click="loadFilteredData(false)">
              🔄 {{ t("message.refresh") }}
            </button>
            <select v-model="filterType">
              <option value="daily">{{ t("message.filterDaily") }}</option>
              <option value="weekly">{{ t("message.filterWeekly") }}</option>
              <option value="monthly">{{ t("message.filterMonthly") }}</option>
              <option value="custom">{{ t("message.filterCustom") }}</option>
            </select>
          </div>
        </header>

        <div class="stats-grid activity-stats-grid" style="
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
            gap: 12px;
            margin-bottom: 20px;
          ">
          <div class="card premium-card small-stat-card"
            style="border-left: 3px solid var(--accent); padding: 12px 16px">
            <h3 style="
                font-size: 0.7rem;
                text-transform: uppercase;
                letter-spacing: 0.05em;
                margin-bottom: 6px;
              ">
              ⌨️ {{ t("message.keyboard") }}
            </h3>
            <p style="
                font-size: 1.5rem;
                font-weight: 800;
                color: var(--accent);
                margin: 2px 0;
              ">
              {{ userInputStats.keyboard_count.toLocaleString() }}
            </p>
            <small style="color: var(--text-muted); font-size: 0.65rem">{{
              t("message.keysPressed")
            }}</small>
          </div>
          <div class="card premium-card small-stat-card"
            style="border-left: 3px solid var(--success); padding: 12px 16px">
            <h3 style="
                font-size: 0.7rem;
                text-transform: uppercase;
                letter-spacing: 0.05em;
                margin-bottom: 6px;
              ">
              🖱 {{ t("message.mouse") }}
            </h3>
            <p style="
                font-size: 1.5rem;
                font-weight: 800;
                color: var(--success);
                margin: 2px 0;
              ">
              {{ userInputStats.mouse_count.toLocaleString() }}
            </p>
            <small style="color: var(--text-muted); font-size: 0.65rem">{{
              t("message.movementsDetected")
            }}</small>
          </div>
          <div class="card premium-card small-stat-card"
            style="border-left: 3px solid var(--warning); padding: 12px 16px">
            <h3 style="
                font-size: 0.7rem;
                text-transform: uppercase;
                letter-spacing: 0.05em;
                margin-bottom: 6px;
              ">
              😴 {{ t("message.idlePeriods") }}
            </h3>
            <p style="
                font-size: 1.5rem;
                font-weight: 800;
                color: var(--warning);
                margin: 2px 0;
              ">
              {{ userInputStats.idle_start_count }}
            </p>
            <small style="color: var(--text-muted); font-size: 0.65rem">{{
              t("message.idleStartsLogged")
            }}</small>
          </div>
        </div>

        <div class="app-table">
          <div class="app-row header-row" style="grid-template-columns: 2fr 2fr 1fr 1fr">
            <span>{{ t("message.type") }}</span><span>{{ t("message.time") }}</span><span>{{ t("message.input")
              }}</span><span>{{ t("message.status") }}</span>
          </div>
          <div v-for="act in userActivityList" :key="act.id" class="app-row"
            style="grid-template-columns: 2fr 2fr 1fr 1fr">
            <span class="app-name" style="display: flex; align-items: center; gap: 6px">
              <span v-if="act.event_type === 'keyboard'">⌨️</span>
              <span v-else-if="act.event_type === 'mouse'">🖱</span>
              <span v-else-if="act.event_type === 'idle_start'">😴</span>
              <span v-else-if="act.event_type === 'idle_end'">▶️</span>
              <span v-else-if="act.event_type.startsWith('url:')">🌐</span>
              <span v-else>📌</span>
              {{
                act.event_type.startsWith("url:")
                  ? act.event_type.replace("url:", "")
                  : act.event_type
              }}
            </span>
            <span class="url-time">{{ formatTimestamp(act.timestamp) }}</span>
            <span style="font-size: 0.8rem; font-weight: 600" :style="{
              color:
                act.event_type === 'keyboard'
                  ? 'var(--accent)'
                  : act.event_type === 'mouse'
                    ? 'var(--success)'
                    : 'var(--text-muted)',
            }">
              {{
                act.event_type === "keyboard"
                  ? "KB"
                  : act.event_type === "mouse"
                    ? t("message.mouse")
                    : "—"
              }}
            </span>
            <span :style="{
              color:
                act.activity_status === 'idle' ? 'var(--warning)' : 'var(--success)',
              fontWeight: '600',
              fontSize: '0.8rem',
            }">
              {{ act.activity_status }}
            </span>
          </div>
          <div v-if="userActivityList.length === 0" class="empty-state">
            <p>{{ t("message.noActivity") }}</p>
          </div>
          <div v-else-if="userActivityList.length % pageSize === 0" class="load-more-container"
            style="margin-top: 16px">
            <button class="btn-start" @click="loadMoreActivity">
              {{ t("message.loadMore") }}
            </button>
          </div>
        </div>
      </div>

      <!-- SCREENSHOTS VIEW -->
      <div v-if="currentView === 'screenshots'" class="view-screenshots">
        <header class="view-header">
          <h1>{{ t("message.screenshots") }}</h1>
          <div class="filter-controls">
            <button class="btn-browse" @click="loadFilteredData(false)">
              🔄 {{ t("message.refresh") }}
            </button>
            <select v-model="filterType">
              <option value="daily">{{ t("message.filterDaily") }}</option>
              <option value="weekly">{{ t("message.filterWeekly") }}</option>
              <option value="monthly">{{ t("message.filterMonthly") }}</option>
              <option value="yearly">{{ t("message.filterYearly") }}</option>
              <option value="custom">{{ t("message.filterCustom") }}</option>
            </select>
            <template v-if="filterType === 'custom'">
              <input type="date" v-model="customFromDate" />
              <span> - </span>
              <input type="date" v-model="customToDate" />
            </template>
          </div>
        </header>

        <div class="screenshots-grid">
          <div v-for="shot in screenshotsList" :key="shot.id" class="screenshot-card"
            @click="openFullscreen(shot.file_path)" style="cursor: pointer">
            <img :src="convertFileSrc(shot.file_path)" alt="Screenshot" loading="lazy" />
            <div class="screenshot-info">
              <span>{{ formatTimestamp(shot.captured_at) }}</span>
            </div>
          </div>
          <div v-if="screenshotsList.length === 0" class="empty-state" style="grid-column: 1 / -1">
            <p>{{ t("message.noData") }}</p>
          </div>
          <div v-else-if="screenshotsList.length % pageSize === 0" class="load-more-container"
            style="grid-column: 1 / -1">
            <button class="btn-start" @click="loadMoreScreenshots">
              {{ t("message.loadMore") }}
            </button>
          </div>
        </div>
      </div>

      <!-- PRODUCTIVITY VIEW -->
      <div v-if="currentView === 'productivity'" class="view-productivity">
        <header class="view-header productivity-header-deck">
          <div class="header-titles">
            <h1 class="view-title">{{ t("message.productivity") }}</h1>
            <p class="view-subtitle" v-if="filterType === 'calendar' && selectedCalendarDay">
              {{ t("message.calendar") }} (Single Day) &mdash; <span class="selected-date-highlight">{{ selectedCalendarDay }}</span>
            </p>
            <p class="view-subtitle" v-else-if="filterType === 'custom'">
              Custom Range &mdash; <span class="selected-date-highlight">{{ customFromDate }} to {{ customToDate }}</span>
            </p>
            <p class="view-subtitle" v-else>
              Range Mode &mdash; <span class="selected-date-highlight">{{ t("message.filter" + filterType.charAt(0).toUpperCase() + filterType.slice(1)) }}</span>
            </p>
          </div>
          <div class="filter-controls">
            <button class="btn-browse" @click="loadFilteredData(false)">
              🔄 {{ t("message.refresh") }}
            </button>
            <select v-model="filterType" class="modern-select">
              <option value="calendar">📅 {{ t("message.calendar") }} (Single Day)</option>
              <option value="daily">{{ t("message.filterDaily") }}</option>
              <option value="weekly">{{ t("message.filterWeekly") }}</option>
              <option value="monthly">{{ t("message.filterMonthly") }}</option>
              <option value="yearly">{{ t("message.filterYearly") }}</option>
              <option value="custom">{{ t("message.filterCustom") }}</option>
            </select>
            <template v-if="filterType === 'custom'">
              <div class="date-picker-group">
                <input type="date" v-model="customFromDate" class="modern-date-input" />
                <span class="date-sep">-</span>
                <input type="date" v-model="customToDate" class="modern-date-input" />
              </div>
            </template>
          </div>
        </header>

        <div class="calendar-productivity-layout" :class="{ 'has-calendar': filterType === 'calendar' }">
          <!-- Calendar Sidebar Column -->
          <div v-if="filterType === 'calendar'" class="card calendar-sidebar-card">
            <div class="calendar-sidebar-header">
              <button class="btn-browse btn-small btn-today" @click="calendarGoToday">
                🎯 {{ t("message.calendarToday") }}
              </button>
              <div class="month-selector">
                <button class="btn-nav" @click="calendarPrevMonth">◀</button>
                <span class="current-month">{{ calendarMonthName }}</span>
                <button class="btn-nav" @click="calendarNextMonth">▶</button>
              </div>
            </div>

            <div v-if="calendarLoading" class="empty-state" style="border: none;">
              <p>{{ t("message.loading") }}</p>
            </div>
            <template v-else>
              <div class="calendar-weekdays-container">
                <span>{{ t("message.calendarMon") }}</span>
                <span>{{ t("message.calendarTue") }}</span>
                <span>{{ t("message.calendarWed") }}</span>
                <span>{{ t("message.calendarThu") }}</span>
                <span>{{ t("message.calendarFri") }}</span>
                <span>{{ t("message.calendarSat") }}</span>
                <span>{{ t("message.calendarSun") }}</span>
              </div>
              <div class="calendar-grid-container">
                <div v-for="(cell, idx) in calendarDaysInMonth" :key="idx"
                  :class="[
                    'calendar-cell',
                    { 'calendar-cell-empty': !cell },
                    { 'calendar-cell-today': cell && cell.dateStr === todayStr },
                    { 'calendar-cell-selected': cell && cell.dateStr === selectedCalendarDay },
                  ]"
                  :style="cell ? {
                    background: getCalendarDay(cell.dateStr) ? calendarDayColor(getCalendarDay(cell.dateStr)!.total_seconds) : 'var(--bg-color)',
                    color: getCalendarDay(cell.dateStr) ? calendarDayTextColor(getCalendarDay(cell.dateStr)!.total_seconds) : 'var(--text-muted)',
                  } : {}"
                  @click="cell && viewDayDetail(cell.dateStr)">
                  <template v-if="cell">
                    <span class="calendar-day-num">{{ cell.day }}</span>
                    <span v-if="getCalendarDay(cell.dateStr)" class="calendar-day-bar">
                      {{ formatTimeCompact(getCalendarDay(cell.dateStr)!.total_seconds) }}
                    </span>
                  </template>
                </div>
              </div>
            </template>
          </div>

          <!-- Productivity Content Column -->
          <div class="productivity-main-column">
            <div class="activity-summary-row">
              <div class="card activity-stat-card total-time-card">
                <div class="stat-icon-wrapper">
                  <div class="stat-icon">⏱</div>
                </div>
                <div class="stat-info">
                  <div class="stat-label">
                    {{ t("message.totalTime") || "Total Time" }}
                  </div>
                  <div class="stat-value">
                    {{ formatTime(dashboardData.total_active_seconds + dashboardData.total_idle_seconds) }}
                  </div>
                </div>
              </div>
              
              <div class="card activity-stat-card active-time-card">
                <div class="stat-icon-wrapper">
                  <div class="stat-icon">▶</div>
                </div>
                <div class="stat-info">
                  <div class="stat-label">
                    {{ t("message.activeTime") }}
                  </div>
                  <div class="stat-value">
                    {{ formatTime(dashboardData.total_active_seconds) }}
                  </div>
                </div>
              </div>

              <div class="card activity-stat-card idle-time-card">
                <div class="stat-icon-wrapper">
                  <div class="stat-icon">⏸</div>
                </div>
                <div class="stat-info">
                  <div class="stat-label">
                    {{ t("message.idleTime") }}
                  </div>
                  <div class="stat-value">
                    {{ formatTime(dashboardData.total_idle_seconds) }}
                  </div>
                </div>
              </div>

              <div class="card activity-stat-card keyboard-card">
                <div class="stat-icon-wrapper">
                  <div class="stat-icon">⌨️</div>
                </div>
                <div class="stat-info">
                  <div class="stat-label">
                    {{ t("message.keyboardActivity") }}
                  </div>
                  <div class="stat-value">
                    {{ dashboardData.keyboard_count }}
                    <span class="stat-unit">{{ t("message.events") }}</span>
                  </div>
                </div>
              </div>

              <div class="card activity-stat-card mouse-card">
                <div class="stat-icon-wrapper">
                  <div class="stat-icon">🖱️</div>
                </div>
                <div class="stat-info">
                  <div class="stat-label">
                    {{ t("message.mouseActivity") }}
                  </div>
                  <div class="stat-value">
                    {{ dashboardData.mouse_count }}
                    <span class="stat-unit">{{ t("message.events") }}</span>
                  </div>
                </div>
              </div>
            </div>

            <div class="productivity-grid single-column">
              <div class="card category-list-card">
                <h3 class="card-title">{{ t("message.appCategories") }}</h3>
                <div class="table-responsive">
                  <table class="modern-table">
                    <thead>
                      <tr>
                        <th>{{ t("message.appName") }}</th>
                        <th>{{ t("message.timeSpent") }}</th>
                        <th>{{ t("message.category") }}</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr v-for="stat in dashboardData.app_stats" :key="stat.app_name">
                        <td class="app-name-cell">{{ stat.app_name }}</td>
                        <td class="time-spent-cell">{{ formatTime(stat.total_seconds) }}</td>
                        <td>
                          <select v-model="stat.category" @change="updateCategory(stat.app_name, stat.category)"
                            :disabled="isMultiUser && !isAdmin" :class="['category-badge-select', stat.category]">
                            <option value="productive">{{ t("message.productive") }}</option>
                            <option value="neutral">{{ t("message.neutral") }}</option>
                            <option value="unproductive">
                              {{ t("message.unproductive") }}
                            </option>
                          </select>
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- SETTINGS VIEW -->
      <div v-if="currentView === 'settings'" class="view-settings">
        <header class="view-header">
          <h1>{{ t("message.settings") }}</h1>
          <div class="settings-tabs">
            <button :class="['tab-btn', { active: settingsTab === 'general' }]" @click="settingsTab = 'general'">⚙️ {{ t("message.general") }}</button>
            <button :class="['tab-btn', { active: settingsTab === 'monitoring' }]" @click="settingsTab = 'monitoring'">🔍 {{ t("message.monitoring") }}</button>
            <button :class="['tab-btn', { active: settingsTab === 'storage' }]" @click="settingsTab = 'storage'">📁 {{ t("message.storage") }}</button>
            <button :class="['tab-btn', { active: settingsTab === 'overlay' }]" @click="settingsTab = 'overlay'">🪟 {{ t("message.overlay") }}</button>
            <button :class="['tab-btn', { active: settingsTab === 'pomodoro' }]" @click="settingsTab = 'pomodoro'">🍅 {{ t("message.pomodoro") }}</button>
            <button :class="['tab-btn', { active: settingsTab === 'break' }]" @click="settingsTab = 'break'">🧘 {{ t("message.breakReminder") }}</button>
            <button :class="['tab-btn', { active: settingsTab === 'api' }]" @click="settingsTab = 'api'">📡 API Config</button>
            <button :class="['tab-btn', { active: settingsTab === 'about' }]" @click="settingsTab = 'about'">ℹ️ About</button>
          </div>
        </header>

        <div class="settings-content">
          <!-- Version + Update (shown on all tabs) -->
          <section class="settings-section">
            <div v-if="!updateCheckLoading" class="update-check-bar">
              <span class="version-text">{{ t("message.currentVersion", { version: currentVersion }) }}</span>
              <button class="btn-check-update" @click="checkForUpdates">{{ t("message.checkUpdates") }}</button>
            </div>
            <div v-if="updateCheckLoading" class="update-loading">{{ t("message.checkingUpdates") }}</div>
            <div v-if="updateError" class="update-error">{{ updateError }}</div>
          </section>

          <!-- General Tab -->
          <section v-if="settingsTab === 'general'" class="settings-section">
            <h2 class="section-title">⚙️ {{ t("message.general") }}</h2>
            <div class="settings-grid">
              <div class="card setting-card">
                <label>{{ t("message.language") }}</label>
                <select v-model="settings.language">
                  <option value="en">English</option>
                  <option value="bn">বাংলা</option>
                </select>
              </div>
              <div class="card setting-card">
                <label>{{ t("message.theme") }}</label>
                <select v-model="settings.theme">
                  <option value="light">{{ t("message.light") }}</option>
                  <option value="dark">{{ t("message.dark") }}</option>
                  <option value="system">{{ t("message.system") }}</option>
                </select>
              </div>
              <div class="card setting-card">
                <label>{{ t("message.autoStart") }}</label>
                <div class="checkbox-row">
                  <label class="switch">
                    <input type="checkbox" v-model="settings.auto_start_on_boot" />
                    <span class="slider"></span>
                  </label>
                  <span>{{ settings.auto_start_on_boot ? t("message.enabled") : t("message.disabled") }}</span>
                </div>
              </div>
              <!-- Mode Switch Card -->
              <div class="card setting-card setting-card-wide">
                <label>{{ t("message.systemMode") }}</label>
                <div class="mode-toggle-row">
                  <span :class="['mode-current-badge', isMultiUser ? 'mode-badge-multi' : 'mode-badge-single']">
                    {{ isMultiUser ? t("message.modeMultiUser") : t("message.modeSingleUser") }}
                  </span>
                  <select v-model="pendingMode" class="mode-select">
                    <option value="single_user">{{ t("message.modeSingleUser") }}</option>
                    <option value="multi_user">{{ t("message.modeMultiUser") }}</option>
                  </select>
                  <button class="btn-change-mode" @click="doChangeMode" :disabled="pendingMode === appConfig.mode">
                    {{ t("message.changeMode") }}
                  </button>
                </div>
                <small class="setting-desc" style="margin-top: 8px;">{{ t("message.modeChangeWarning") }}</small>
              </div>
              <!-- Reset App Card -->
              <div class="card danger-card setting-card-wide">
                <div class="danger-header">
                  <label class="danger-label">⚠️ {{ t("message.dangerZone") }}</label>
                  <p class="danger-desc">{{ t("message.resetAppDesc") }}</p>
                </div>
                <button class="btn-danger" @click="doResetApp">{{ t("message.resetApp") }}</button>
              </div>
            </div>
          </section>

          <!-- Monitoring Tab -->
          <section v-if="settingsTab === 'monitoring'" class="settings-section">
            <h2 class="section-title">🔍 {{ t("message.monitoring") }}
              <span v-if="appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin'" class="admin-lock-hint">🔒 {{ t("message.controlledByAdmin") }}</span>
            </h2>
            <div class="settings-grid">
              <div class="card setting-card" :class="{ 'card-disabled': appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin' }">
                <label>{{ t("message.screenshotInterval") }} <span v-if="appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin'" class="lock-icon">🔒</span></label>
                <input type="number" v-model="settings.screenshot_interval" min="1" :disabled="appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin'" />
              </div>
              <div class="card setting-card" :class="{ 'card-disabled': appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin' }">
                <label>{{ t("message.idleTimeout") }} <span v-if="appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin'" class="lock-icon">🔒</span></label>
                <small class="setting-desc">{{ t("message.idleThresholdDesc") }}</small>
                <input type="number" v-model="settings.idle_threshold" min="1" max="60" :disabled="appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin'" />
              </div>
              <div class="card setting-card" :class="{ 'card-disabled': appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin' }">
                <label>{{ t("message.screenshotStatus") }} <span v-if="appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin'" class="lock-icon">🔒</span></label>
                <div class="status-toggle">
                  <label class="switch">
                    <input type="checkbox" v-model="settings.is_screenshot_enabled" :disabled="appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin'" />
                    <span class="slider"></span>
                  </label>
                  <span :style="{ color: settings.is_screenshot_enabled ? 'var(--success)' : 'var(--danger)' }">{{ settings.is_screenshot_enabled ? t("message.active") : t("message.disabled") }}</span>
                </div>
              </div>
              <div class="card setting-card" :class="{ 'card-disabled': appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin' }">
                <label>{{ t("message.idleMonitorMouse") || "Monitor Mouse" }} <span v-if="appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin'" class="lock-icon">🔒</span></label>
                <div class="status-toggle">
                  <label class="switch">
                    <input type="checkbox" v-model="settings.idle_monitor_mouse" />
                    <span class="slider"></span>
                  </label>
                  <span>{{ settings.idle_monitor_mouse ? t("message.enabled") : t("message.disabled") }}</span>
                </div>
              </div>
              <div class="card setting-card" :class="{ 'card-disabled': appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin' }">
                <label>{{ t("message.idleMonitorKeyboard") || "Monitor Keyboard" }} <span v-if="appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin'" class="lock-icon">🔒</span></label>
                <div class="status-toggle">
                  <label class="switch">
                    <input type="checkbox" v-model="settings.idle_monitor_keyboard" />
                    <span class="slider"></span>
                  </label>
                  <span>{{ settings.idle_monitor_keyboard ? t("message.enabled") : t("message.disabled") }}</span>
                </div>
              </div>
            </div>
          </section>

          <!-- Storage Tab -->
          <section v-if="settingsTab === 'storage'" class="settings-section">
            <h2 class="section-title">📁 {{ t("message.storage") }}</h2>
            <div class="settings-grid">
              <div class="card setting-card" :class="{ 'card-disabled': appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin' }">
                <label>{{ t("message.screenshotLocation") }} <span v-if="appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin'" class="lock-icon">🔒</span></label>
                <div class="input-with-button">
                  <input type="text" v-model="settings.screenshot_location" :placeholder="defaultScreenshotDir" :disabled="appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin'" />
                  <button class="btn-browse" @click="selectScreenshotLocation" :disabled="appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin'">📁</button>
                </div>
              </div>
              <div class="card setting-card" :class="{ 'card-disabled': appConfig?.mode === 'multi_user' && currentUser?.role !== 'admin' }">
                <label>{{ t("message.backupFrequency") }}</label>
                <select v-model="settings.backup_frequency">
                  <option value="never">{{ t("message.never") || "Never" }}</option>
                  <option value="daily">{{ t("message.daily") || "Daily" }}</option>
                  <option value="weekly">{{ t("message.weekly") || "Weekly" }}</option>
                  <option value="monthly">{{ t("message.monthly") || "Monthly" }}</option>
                </select>
              </div>
              <div class="card setting-card">
                <label>{{ t("message.backupLocation") }}</label>
                <div class="input-with-button">
                  <input type="text" v-model="settings.backup_location" readonly @click="selectBackupLocation" />
                  <button class="btn-browse" @click="selectBackupLocation">📁</button>
                </div>
              </div>
            </div>
          </section>

          <!-- Overlay Tab -->
          <section v-if="settingsTab === 'overlay'" class="settings-section">
            <h2 class="section-title">🪟 {{ t("message.overlay") }}</h2>
            <div class="settings-grid">
              <div class="card setting-card">
                <label>{{ t("message.enableOverlay") }}</label>
                <div class="status-toggle">
                  <label class="switch">
                    <input type="checkbox" v-model="overlayEnabled" @change="saveOverlaySettings" />
                    <span class="slider"></span>
                  </label>
                  <span :style="{ color: overlayEnabled ? 'var(--success)' : 'var(--danger)' }">{{ overlayEnabled ? t("message.enabled") : t("message.disabled") }}</span>
                </div>
              </div>
              <div class="card setting-card" :class="{ 'card-disabled': !overlayEnabled }">
                <label>{{ t("message.alwaysOnTop") }}</label>
                <div class="status-toggle">
                  <label class="switch">
                    <input type="checkbox" v-model="overlayAlwaysOnTop" :disabled="!overlayEnabled" @change="saveOverlaySettings" />
                    <span class="slider"></span>
                  </label>
                  <span :style="{ color: overlayAlwaysOnTop ? 'var(--success)' : 'var(--text-muted)' }">{{ overlayAlwaysOnTop ? t("message.enabled") : t("message.disabled") }}</span>
                </div>
              </div>
              <div class="card setting-card" :class="{ 'card-disabled': !overlayEnabled }">
                <label>{{ t("message.clickThrough") }}</label>
                <div class="status-toggle">
                  <label class="switch">
                    <input type="checkbox" v-model="overlayClickThrough" :disabled="!overlayEnabled" @change="saveOverlaySettings" />
                    <span class="slider"></span>
                  </label>
                  <small class="setting-desc">{{ t("message.clickThroughDesc") }}</small>
                </div>
              </div>
            </div>
          </section>

          <!-- Pomodoro Tab -->
          <section v-if="settingsTab === 'pomodoro'" class="settings-section">
            <h2 class="section-title">🍅 {{ t("message.pomodoro") }}</h2>
            <div class="settings-grid">
              <div class="card setting-card">
                <label>{{ t("message.pomodoroFocusDuration") || "Focus Duration" }}</label>
                <input type="number" v-model.number="settings.pomodoro_focus_minutes" min="1" max="120" />
                <small class="setting-desc">{{ t("message.minutes") || "minutes" }}</small>
              </div>
              <div class="card setting-card">
                <label>{{ t("message.pomodoroBreakDuration") || "Break Duration" }}</label>
                <input type="number" v-model.number="settings.pomodoro_short_break_minutes" min="1" max="30" />
                <small class="setting-desc">{{ t("message.minutes") || "minutes" }}</small>
              </div>
              <div class="card setting-card">
                <label>{{ t("message.pomodoroLongBreak") || "Long Break" }}</label>
                <input type="number" v-model.number="settings.pomodoro_long_break_minutes" min="1" max="60" />
                <small class="setting-desc">{{ t("message.minutes") || "minutes" }}</small>
              </div>
              <div class="card setting-card">
                <label>{{ t("message.pomodoroBeforeLong") || "Pomodoros before long break" }}</label>
                <input type="number" v-model.number="settings.pomodoro_sessions_before_long" min="1" max="10" />
              </div>
              <div class="card setting-card">
                <label>{{ t("message.pomodoroAutoStart") || "Auto-start with tracking" }}</label>
                <div class="status-toggle">
                  <label class="switch">
                    <input type="checkbox" v-model="settings.pomodoro_auto_start" />
                    <span class="slider"></span>
                  </label>
                  <span :style="{ color: settings.pomodoro_auto_start ? 'var(--success)' : 'var(--text-muted)' }">{{ settings.pomodoro_auto_start ? t("message.enabled") : t("message.disabled") }}</span>
                </div>
              </div>
              <div class="card setting-card">
                <label>{{ t("message.pomodoroSound") || "Pomodoro Sound" }}</label>
                <div class="status-toggle">
                  <label class="switch">
                    <input type="checkbox" v-model="settings.pomodoro_sound_enabled" />
                    <span class="slider"></span>
                  </label>
                  <span :style="{ color: settings.pomodoro_sound_enabled ? 'var(--success)' : 'var(--text-muted)' }">{{ settings.pomodoro_sound_enabled ? t("message.enabled") : t("message.disabled") }}</span>
                </div>
              </div>
            </div>
          </section>

          <!-- Break Reminder Tab -->
          <section v-if="settingsTab === 'break'" class="settings-section">
            <h2 class="section-title">🧘 {{ t("message.breakReminder") }}</h2>

            <!-- Group 1: General Settings -->
            <div class="settings-group">
              <h3 class="settings-group-title">General Settings</h3>
              <div class="settings-grid">
                <div class="card setting-card">
                  <label>{{ t("message.enableBreakReminder") }}</label>
                  <div class="status-toggle">
                    <label class="switch">
                      <input type="checkbox" v-model="settings.break_reminder_enabled" />
                      <span class="slider"></span>
                    </label>
                    <span :style="{ color: settings.break_reminder_enabled ? 'var(--success)' : 'var(--danger)' }">{{ settings.break_reminder_enabled ? t("message.enabled") : t("message.disabled") }}</span>
                  </div>
                  <small class="setting-desc">{{ t("message.breakReminderDesc") }}</small>
                </div>

                <div class="card setting-card" :class="{ 'card-disabled': !settings.break_reminder_enabled }">
                  <label>{{ t("message.breakFullscreen") }}</label>
                  <div class="status-toggle">
                    <label class="switch">
                      <input type="checkbox" v-model="settings.break_fullscreen" :disabled="!settings.break_reminder_enabled" />
                      <span class="slider"></span>
                    </label>
                    <span :style="{ color: settings.break_fullscreen ? 'var(--success)' : 'var(--text-muted)' }">{{ settings.break_fullscreen ? t("message.enabled") : t("message.disabled") }}</span>
                  </div>
                  <small class="setting-desc">{{ t("message.breakFullscreenDesc") }}</small>
                </div>

                <div class="card setting-card" :class="{ 'card-disabled': !settings.break_reminder_enabled }">
                  <label>{{ t("message.breakIdeasEnabled") }}</label>
                  <div class="status-toggle">
                    <label class="switch">
                      <input type="checkbox" v-model="settings.break_ideas_enabled" :disabled="!settings.break_reminder_enabled" />
                      <span class="slider"></span>
                    </label>
                    <span :style="{ color: settings.break_ideas_enabled ? 'var(--success)' : 'var(--text-muted)' }">{{ settings.break_ideas_enabled ? t("message.enabled") : t("message.disabled") }}</span>
                  </div>
                </div>

                <div class="card setting-card" :class="{ 'card-disabled': !settings.break_reminder_enabled }">
                  <label>Allow Force Exit</label>
                  <div class="status-toggle">
                    <label class="switch">
                      <input type="checkbox" v-model="settings.break_allow_force_exit" :disabled="!settings.break_reminder_enabled" />
                      <span class="slider"></span>
                    </label>
                    <span :style="{ color: settings.break_allow_force_exit && settings.break_reminder_enabled ? 'var(--success)' : 'var(--danger)' }">
                      {{ settings.break_allow_force_exit && settings.break_reminder_enabled ? t("message.enabled") : t("message.disabled") }}
                    </span>
                  </div>
                  <small class="setting-desc">Show End Break button on break screen to force skip breaks.</small>
                </div>

                <div class="card setting-card" :class="{ 'card-disabled': !settings.break_reminder_enabled }">
                  <label>{{ t("message.breakSoundVolume") }} ({{ settings.break_sound_volume }}%)</label>
                  <input type="range" v-model.number="settings.break_sound_volume" min="0" max="100" style="width:100%;" :disabled="!settings.break_reminder_enabled" />
                </div>

                <div class="card setting-card" :class="{ 'card-disabled': !settings.break_reminder_enabled }">
                  <label>Background Color</label>
                  <div class="color-picker-wrapper" style="display: flex; gap: 12px; align-items: center;">
                    <input type="color" v-model="settings.break_bg_color" :disabled="!settings.break_reminder_enabled" style="border: 1px solid var(--border-color); border-radius: 6px; cursor: pointer; height: 36px; width: 60px; padding: 0;" />
                    <span style="font-family: monospace; font-size: 0.9rem;">{{ settings.break_bg_color }}</span>
                  </div>
                  <small class="setting-desc">Selected background color for the break overlay.</small>
                </div>

                <div class="card setting-card" :class="{ 'card-disabled': !settings.break_reminder_enabled }">
                  <label>{{ t("message.breakPreNotification") }}</label>
                  <input type="number" v-model.number="settings.break_pre_notification_seconds" min="1" :disabled="!settings.break_reminder_enabled" />
                  <small class="setting-desc">{{ t("message.seconds") || "seconds" }}</small>
                </div>
                
                <!-- Test Preview Button -->
                <div class="card setting-card" :class="{ 'card-disabled': !settings.break_reminder_enabled }">
                  <label>Preview Overlay</label>
                  <button class="btn btn-primary" @click="testBreakOverlay" :disabled="!settings.break_reminder_enabled" style="margin-top: 8px; width: 100%; padding: 8px 12px; font-weight: 600;">
                    👁️ Test Preview
                  </button>
                  <small class="setting-desc" style="margin-top: 6px; display: block;">Test the visual break reminder screen.</small>
                </div>
              </div>
            </div>

            <!-- Group 2: Mini Break Settings -->
            <div class="settings-group" :class="{ 'card-disabled': !settings.break_reminder_enabled }">
              <h3 class="settings-group-title">Mini Break Settings</h3>
              <div class="settings-grid">
                <div class="card setting-card" :class="{ 'card-disabled': !settings.break_reminder_enabled }">
                  <label>Enable Mini Breaks</label>
                  <div class="status-toggle">
                    <label class="switch">
                      <input type="checkbox" v-model="settings.break_mini_enabled" :disabled="!settings.break_reminder_enabled" />
                      <span class="slider"></span>
                    </label>
                    <span :style="{ color: settings.break_mini_enabled && settings.break_reminder_enabled ? 'var(--success)' : 'var(--danger)' }">
                      {{ settings.break_mini_enabled && settings.break_reminder_enabled ? t("message.enabled") : t("message.disabled") }}
                    </span>
                  </div>
                  <small class="setting-desc">Enable or disable short micro-breaks.</small>
                </div>

                <div class="card setting-card" :class="{ 'card-disabled': !settings.break_reminder_enabled || !settings.break_mini_enabled }">
                  <label>{{ t("message.miniBreakInterval") }}</label>
                  <input type="number" v-model.number="settings.break_mini_interval_minutes" min="1" :disabled="!settings.break_reminder_enabled || !settings.break_mini_enabled" />
                  <small class="setting-desc">{{ t("message.minutes") || "minutes" }}</small>
                </div>

                <div class="card setting-card" :class="{ 'card-disabled': !settings.break_reminder_enabled || !settings.break_mini_enabled }">
                  <label>{{ t("message.miniBreakDuration") }}</label>
                  <input type="number" v-model.number="settings.break_mini_duration_seconds" min="5" :disabled="!settings.break_reminder_enabled || !settings.break_mini_enabled" />
                  <small class="setting-desc">{{ t("message.seconds") || "seconds" }}</small>
                </div>

              </div>
            </div>

            <!-- Group 3: Long Break Settings -->
            <div class="settings-group" :class="{ 'card-disabled': !settings.break_reminder_enabled }">
              <h3 class="settings-group-title">Long Break Settings</h3>
              <div class="settings-grid">
                <div class="card setting-card" :class="{ 'card-disabled': !settings.break_reminder_enabled }">
                  <label>Enable Long Breaks</label>
                  <div class="status-toggle">
                    <label class="switch">
                      <input type="checkbox" v-model="settings.break_long_enabled" :disabled="!settings.break_reminder_enabled" />
                      <span class="slider"></span>
                    </label>
                    <span :style="{ color: settings.break_long_enabled && settings.break_reminder_enabled ? 'var(--success)' : 'var(--danger)' }">
                      {{ settings.break_long_enabled && settings.break_reminder_enabled ? t("message.enabled") : t("message.disabled") }}
                    </span>
                  </div>
                  <small class="setting-desc">Enable or disable longer rest periods.</small>
                </div>

                <div class="card setting-card" :class="{ 'card-disabled': !settings.break_reminder_enabled || !settings.break_long_enabled }">
                  <label>{{ t("message.longBreakDuration") }}</label>
                  <input type="number" v-model.number="settings.break_long_duration_seconds" min="10" :disabled="!settings.break_reminder_enabled || !settings.break_long_enabled" />
                  <small class="setting-desc">{{ t("message.seconds") || "seconds" }}</small>
                </div>

                <div class="card setting-card" :class="{ 'card-disabled': !settings.break_reminder_enabled || !settings.break_long_enabled }">
                  <label>{{ t("message.miniBreaksBeforeLong") }}</label>
                  <input type="number" v-model.number="settings.break_mini_breaks_before_long" min="1" :disabled="!settings.break_reminder_enabled || !settings.break_long_enabled" />
                </div>
              </div>
            </div>

            <!-- Group 4: Postpone Settings -->
            <div class="settings-group" :class="{ 'card-disabled': !settings.break_reminder_enabled }">
              <h3 class="settings-group-title">Postpone Settings</h3>
              <div class="settings-grid">
                <div class="card setting-card" :class="{ 'card-disabled': !settings.break_reminder_enabled }">
                  <label>{{ t("message.breakPostponeLimit") }}</label>
                  <input type="number" v-model.number="settings.break_postpone_limit" min="0" :disabled="!settings.break_reminder_enabled" />
                </div>

                <div class="card setting-card" :class="{ 'card-disabled': !settings.break_reminder_enabled }">
                  <label>{{ t("message.breakPostponeDuration") }}</label>
                  <input type="number" v-model.number="settings.break_postpone_duration_minutes" min="1" :disabled="!settings.break_reminder_enabled" />
                  <small class="setting-desc">{{ t("message.minutes") || "minutes" }}</small>
                </div>
              </div>
            </div>
          </section>

          <!-- About Tab -->
          <section v-if="settingsTab === 'about'" class="settings-section">
            <h2 class="section-title">ℹ️ About Deskrona</h2>
            <div class="about-content" style="max-width: 600px;">
              <p style="color: var(--text-muted); margin-bottom: 16px; line-height: 1.7;">
                Deskrona is a local-first, privacy-focused time tracking and productivity monitoring application.
                Built with <strong>Tauri</strong>, <strong>Rust</strong>, and <strong>Vue 3</strong>.
                All data stays on your machine — no cloud, no telemetry.
              </p>
              <p style="color: var(--text-muted); margin-bottom: 16px;">
                Version <strong>{{ currentVersion }}</strong> · MIT License
              </p>

              <h3 style="margin: 16px 0 8px;">🔧 Deskrona Tech Stack</h3>
              <div class="card" style="padding: 14px; font-size: 0.85rem; color: var(--text-muted); line-height: 1.8;">
                <div>Tauri 2 · Rust · Vue 3 · TypeScript · Vite</div>
                <div>SQLite (rusqlite) · Chart.js · vue-chartjs · vue-i18n</div>
                <div>device_query · active-win-pos-rs · screenshots-rs</div>
                <div>argon2 · uuid · chrono · serde · zip</div>
              </div>

              <h3 style="margin: 20px 0 8px;">👨‍💻 Developer</h3>
              <div class="card" style="padding: 16px;">
                <div style="display: flex; align-items: center; gap: 16px;">
                  <img src="https://avatars.githubusercontent.com/u/90689063?v=4" alt="Abiruzzaman" style="width: 56px; height: 56px; border-radius: 50%; border: 2px solid var(--accent);" />
                  <div>
                    <strong style="font-size: 1.1rem;">Abiruzzaman Molla</strong><br />
                    <span style="font-size: 0.85rem; color: var(--text-muted);">Full-stack Developer · Open Source Enthusiast</span>
                  </div>
                </div>
                <div style="margin-top: 14px; display: flex; gap: 10px; flex-wrap: wrap;">
                  <a class="btn btn-secondary" href="https://github.com/AbiruzzamanMolla" target="_blank" style="padding: 6px 14px; font-size: 0.85rem; text-decoration: none;">🐙 GitHub</a>
                  <a class="btn btn-secondary" href="https://az.is-a.dev/" target="_blank" style="padding: 6px 14px; font-size: 0.85rem; text-decoration: none;">🌐 Website</a>
                  <a class="btn btn-secondary" href="https://www.supportkori.com/abiruzzaman" target="_blank" style="padding: 6px 14px; font-size: 0.85rem; text-decoration: none;">❤️ Support</a>
                </div>
              </div>

              <h3 style="margin: 20px 0 8px;">📦 Open Source Packages by Abiruzzaman</h3>

              <h4 style="margin: 16px 0 6px; color: var(--text);">📦 Laravel Packages</h4>
              <div class="settings-grid" style="grid-template-columns: 1fr 1fr;">
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://packagist.org/packages/azmolla/laravelcart" target="_blank">
                  <strong>LaravelCart</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">Flexible shopping cart for Laravel</small>
                </a>
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://packagist.org/packages/azmolla/blade-try-catch" target="_blank">
                  <strong>Blade Try-Catch</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">Error handling inside Blade templates</small>
                </a>
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://packagist.org/packages/azmolla/laravel-mail-logger" target="_blank">
                  <strong>Laravel Mail Logger</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">Log outgoing emails for debugging</small>
                </a>
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://packagist.org/packages/azmolla/maintenance-mode" target="_blank">
                  <strong>Maintenance Mode</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">Database-controlled maintenance toggle</small>
                </a>
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://packagist.org/packages/azmolla/transaction-middleware" target="_blank">
                  <strong>Transaction Middleware</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">Auto DB transactions for requests</small>
                </a>
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://packagist.org/packages/azmolla/spell-money-multilang" target="_blank">
                  <strong>Spell Money Multilang</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">Numbers to money words, multi-language</small>
                </a>
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://packagist.org/packages/azmolla/fraud-checker-bd-courier" target="_blank">
                  <strong>Fraud Checker BD Courier</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">Fraud detection for BD courier orders</small>
                </a>
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://packagist.org/packages/azmolla/fraud-checker-bd-courier-laravel" target="_blank">
                  <strong>Fraud Checker BD Courier Laravel</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">Laravel fraud detection for BD courier</small>
                </a>
              </div>

              <h4 style="margin: 16px 0 6px; color: var(--text);">🧩 VS Code Extensions</h4>
              <div class="settings-grid" style="grid-template-columns: 1fr 1fr;">
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://marketplace.visualstudio.com/items?itemName=azmolla.laragon-terminal-config" target="_blank">
                  <strong>Laragon Terminal Config</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">Configure VS Code terminal for Laragon</small>
                </a>
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://marketplace.visualstudio.com/items?itemName=azmolla.prayer-timer-bangladesh" target="_blank">
                  <strong>Prayer Timer Bangladesh</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">Real-time prayer times for Bangladesh</small>
                </a>
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://marketplace.visualstudio.com/items?itemName=azmolla.warp-terminal-launcher" target="_blank">
                  <strong>Warp Terminal Launcher</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">Launch Warp terminal from VS Code</small>
                </a>
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://marketplace.visualstudio.com/items?itemName=azmolla.laravel-log-watcher" target="_blank">
                  <strong>Laravel Log Watcher</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">Watch & notify on Laravel log changes</small>
                </a>
              </div>

              <h4 style="margin: 16px 0 6px; color: var(--text);">📦 NPM Packages</h4>
              <div class="settings-grid" style="grid-template-columns: 1fr 1fr;">
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://www.npmjs.com/package/git-diff-copy" target="_blank">
                  <strong>Git Diff Copy</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">Copy git diff to new directory</small>
                </a>
              </div>

              <h4 style="margin: 16px 0 6px; color: var(--text);">🖥 Open Source Apps</h4>
              <div class="settings-grid" style="grid-template-columns: 1fr 1fr;">
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://github.com/AbiruzzamanMolla/tdc-pos" target="_blank">
                  <strong>TDC-POS</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">Offline-first Point of Sale desktop app</small>
                </a>
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://github.com/AbiruzzamanMolla/DailyPrayerTimeSoftware" target="_blank">
                  <strong>Daily Prayer Timer</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">Islamic prayer times with Adhan alerts</small>
                </a>
                <a class="card" style="padding: 12px; text-decoration: none; color: inherit;" href="https://github.com/AbiruzzamanMolla/Deskrona-Time-Tracking-Software" target="_blank">
                  <strong>Deskrona</strong>
                  <small style="display: block; color: var(--text-muted); margin-top: 4px;">This app — time tracking & productivity</small>
                </a>
              </div>
            </div>
          </section>

          <!-- API Config Tab -->
          <section v-if="settingsTab === 'api'" class="settings-section">
            <h2 class="section-title">📡 API Configuration</h2>
            <p style="color: var(--text-muted); margin-bottom: 16px; font-size: 0.85rem;">
              Configure endpoints for online mode. All data stays local when mode is Offline.
              Enable Online mode and fill in your server endpoint URLs to sync data remotely.
            </p>

            <div class="card setting-card" style="margin-bottom: 16px;">
              <label>Mode</label>
              <div class="mode-toggle-switch">
                <button :class="['toggle-btn', { active: apiConfig.mode === 'offline' }]" @click="apiConfig.mode = 'offline'; saveApiConfigToDisk()">
                  <span class="toggle-icon">📴</span> Offline
                </button>
                <button :class="['toggle-btn', { active: apiConfig.mode === 'online' }]" @click="apiConfig.mode = 'online'; saveApiConfigToDisk()">
                  <span class="toggle-icon">🌐</span> Online
                </button>
              </div>
              <small style="color: var(--text-muted); display: block; margin-top: 8px; line-height: 1.5;">
                <strong>Offline:</strong> All data stored locally only. No network requests. App function fully without internet.<br>
                <strong>Online:</strong> Sync data to your own server. Each endpoint URL and method is configurable below. Falls back to local storage if server unreachable.
              </small>
            </div>

            <div class="card setting-card" style="margin-bottom: 16px;">
              <label>Bearer Token</label>
              <input type="password" v-model="apiConfig.bearer_token" @change="saveApiConfigToDisk" placeholder="JWT token for authenticated requests" style="width: 100%; font-family: monospace;" />
              <small style="color: var(--text-muted);">Set manually if your server requires auth. Leave empty for open endpoints.</small>
            </div>

            <!-- Sync Controls -->
            <div class="card" style="padding: 12px; margin-bottom: 16px; background: var(--bg-secondary);">
              <div style="display: flex; align-items: center; gap: 12px; flex-wrap: wrap;">
                <span style="font-weight: 600;">🔄 Sync</span>
                <span v-if="apiConfig.mode === 'online'" style="color: var(--success); font-size: 0.85rem;">● Active</span>
                <span v-else style="color: var(--text-muted); font-size: 0.85rem;">● Disabled</span>
                <span style="font-size: 0.8rem; color: var(--text-muted);">Last sync: {{ lastSyncTimeFormatted }}</span>
                <button class="btn btn-secondary" @click="triggerManualSync" style="padding: 4px 12px; font-size: 0.8rem;" :disabled="apiConfig.mode === 'offline'">Sync Now</button>
              </div>
            </div>

            <!-- Queue Controls -->
            <div v-if="queueStats.total > 0" class="card" style="padding: 12px; margin-bottom: 16px; background: var(--bg-secondary);">
              <div style="display: flex; align-items: center; gap: 12px; flex-wrap: wrap;">
                <span style="font-weight: 600;">📡 Queue: {{ queueStats.total }} jobs</span>
                <span v-if="queueStats.pending > 0" style="color: var(--warning);">{{ queueStats.pending }} pending</span>
                <span v-if="queueStats.failed > 0" style="color: var(--danger);">{{ queueStats.failed }} failed</span>
                <button v-if="queueStats.failed > 0" class="btn btn-secondary" @click="retryFailedJobs" style="padding: 4px 12px; font-size: 0.8rem;">Retry Failed</button>
                <button class="btn btn-secondary" @click="clearCompletedJobs" style="padding: 4px 12px; font-size: 0.8rem;">Clear Completed</button>
                <button class="btn btn-secondary" @click="clearAllJobs" style="padding: 4px 12px; font-size: 0.8rem;">Clear All</button>
              </div>
            </div>

            <!-- Endpoint Groups -->
            <div v-for="group in ENDPOINT_GROUPS" :key="group.key" class="card" style="margin-bottom: 12px; padding: 0; overflow: hidden;">
              <div class="api-group-header" @click="toggleApiGroup(group.key)" style="display: flex; align-items: center; justify-content: space-between; padding: 12px 16px; cursor: pointer; background: var(--bg-secondary); border-bottom: 1px solid var(--border-color); user-select: none;">
                <span style="font-weight: 600;">{{ group.label }}</span>
                <span>{{ apiGroupExpanded[group.key] ? '▼' : '▶' }}</span>
              </div>
              <div v-if="apiGroupExpanded[group.key]" style="padding: 12px 16px;">
                <div v-for="ep in group.endpoints" :key="ep.key" class="api-endpoint-row" style="margin-bottom: 12px; padding: 12px; border: 1px solid var(--border-color); border-radius: 8px;">
                  <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 8px; flex-wrap: wrap;">
                    <input type="checkbox" v-model="apiConfig.endpoints[ep.key].enabled" @change="saveApiConfigToDisk" title="Enable/disable endpoint" />
                    <code style="font-size: 0.8rem; background: var(--bg-secondary); padding: 2px 6px; border-radius: 4px;">{{ ep.key }}</code>
                    <select v-model="apiConfig.endpoints[ep.key].method" @change="saveApiConfigToDisk" style="width: auto; padding: 2px 6px; font-size: 0.8rem;">
                      <option value="GET">GET</option>
                      <option value="POST">POST</option>
                      <option value="PUT">PUT</option>
                      <option value="DELETE">DELETE</option>
                    </select>
                  </div>
                  <input type="text" v-model="apiConfig.endpoints[ep.key].url" @change="saveApiConfigToDisk" :placeholder="'https://your-server.com' + (ep.key.includes('{userId}') ? '/api/.../{userId}' : '/api/...')" style="width: 100%; font-family: monospace; font-size: 0.85rem; margin-bottom: 8px;" />
                  
                  <!-- Headers -->
                  <div class="api-headers" style="margin-bottom: 6px;">
                    <div v-for="(v, k) in apiConfig.endpoints[ep.key].headers" :key="k" style="display: flex; gap: 6px; align-items: center; margin-bottom: 4px;">
                      <input type="text" :value="k" @change="(e) => renameHeader(ep.key, k, (e.target as HTMLInputElement).value)" placeholder="Header name" style="flex: 1; font-size: 0.8rem; font-family: monospace;" />
                      <input type="text" :value="v" @change="(e) => updateHeader(ep.key, k, (e.target as HTMLInputElement).value)" placeholder="Value" style="flex: 2; font-size: 0.8rem; font-family: monospace;" />
                      <button @click="removeHeader(ep.key, k)" style="padding: 2px 6px; font-size: 0.8rem; background: none; border: 1px solid var(--danger); color: var(--danger); border-radius: 4px; cursor: pointer;">✕</button>
                    </div>
                    <button @click="addHeader(ep.key)" style="padding: 2px 8px; font-size: 0.75rem; background: var(--bg-secondary); border: 1px dashed var(--border-color); border-radius: 4px; cursor: pointer;">+ Add Header</button>
                  </div>

                  <!-- Info card: request/response spec -->
                  <details style="font-size: 0.75rem; color: var(--text-muted); margin-top: 6px;">
                    <summary style="cursor: pointer; user-select: none;">📋 Request / Response spec</summary>
                    <div style="margin-top: 6px; padding: 8px; background: var(--bg-secondary); border-radius: 4px; font-family: monospace; white-space: pre-wrap; word-break: break-word;">
                      <strong>➡ Request:</strong> {{ ep.requestBody }}
                      <br />
                      <strong>⬅ Response:</strong> {{ ep.responseBody }}
                    </div>
                  </details>
                </div>
              </div>
            </div>
          </section>

        </div>
      </div>

      <!-- PROJECTS & TASKS VIEW -->
      <div v-if="currentView === 'projects'" class="view-projects">
        <header class="view-header" style="flex-wrap: wrap; gap: 12px; margin-bottom: 20px;">
          <div class="projects-header-left">
            <h1>📁 {{ t('message.projects') }}</h1>
            <span class="projects-count-badge">{{ projects.filter(p => !p.archived).length }} active</span>
          </div>
          <div style="display: flex; gap: 8px; align-items: center;">
            <button class="project-action-btn" style="padding: 9px 14px; font-size: 0.85rem; display: flex; align-items: center; gap: 4px;" @click="showArchivedProjects = !showArchivedProjects">
              📦 {{ showArchivedProjects ? 'Hide Archived' : 'Show Archived' }}
            </button>
            <button class="btn-new-project" @click="showProjectForm = !showProjectForm">
              {{ showProjectForm ? '✕ Cancel' : '+ New Project' }}
            </button>
          </div>
        </header>

        <!-- New Project Form -->
        <div v-if="showProjectForm" class="project-create-form">
          <div class="project-form-inner">
            <div class="form-color-preview" :style="{ background: newProjectColor }"></div>
            <input v-model="newProjectName" placeholder="Project name…" class="project-name-input" @keyup.enter="createProject" />
            <label class="color-picker-label" title="Pick color">
              <span>🎨</span>
              <input v-model="newProjectColor" type="color" style="position: absolute; opacity: 0; width: 0; height: 0;" />
            </label>
            <button class="btn-primary" @click="createProject" :disabled="!newProjectName.trim()">Create Project</button>
          </div>
        </div>

        <!-- Active Task Banner -->
        <div v-if="activeTaskId && trackingStatus === 'running'" class="active-task-banner">
          <span class="active-pulse"></span>
          <span class="active-task-label">Tracking:</span>
          <span class="active-task-name">{{ activeTaskName }}</span>
          <button class="btn-stop-task" @click="setTracking('stopped')">⏹ Stop</button>
        </div>

        <!-- Projects Grid -->
        <div class="projects-grid">
          <div v-for="project in projects.filter(p => showArchivedProjects ? true : !p.archived)" :key="project.id"
            class="project-card-premium"
            :class="{ 'project-card-expanded': selectedProjectId === project.id, 'project-archived': project.archived }"
            :style="project.archived ? 'opacity: 0.8; border-style: dashed;' : ''">

            <!-- Card Header -->
            <div class="project-card-header"
              @click="selectedProjectId = selectedProjectId === project.id ? null : project.id; if (selectedProjectId) loadTasks(selectedProjectId);">
              <div class="project-card-identity">
                <span class="project-color-bar" :style="{ background: project.color }"></span>
                <div class="project-card-info">
                  <h3 class="project-card-name" style="display: flex; align-items: center; gap: 6px;">
                    {{ project.name }}
                    <span v-if="project.archived" style="font-size: 0.65rem; background: var(--bg-secondary); padding: 2px 6px; border-radius: 4px; color: var(--text-muted);">Archived</span>
                  </h3>
                  <span class="project-card-meta">
                    {{ selectedProjectId === project.id ? tasks.length + ' task(s)' : 'Click to expand' }}
                  </span>
                </div>
              </div>
              <div class="project-card-actions">
                <button class="project-action-btn" @click.stop="editingProject = { ...project }" title="Edit">✏️</button>
                <button v-if="!project.archived" class="project-action-btn" @click.stop="archiveProject(project.id)" title="Archive">📦</button>
                <button v-else class="project-action-btn" @click.stop="unarchiveProject(project.id)" title="Restore">🔄</button>
                <button class="project-action-btn project-action-danger" @click.stop="deleteProject(project.id)" title="Delete">🗑️</button>
                <span class="project-chevron" :class="{ 'chevron-open': selectedProjectId === project.id }">›</span>
              </div>
            </div>

            <!-- Edit Project Inline -->
            <div v-if="editingProject?.id === project.id" class="project-edit-form">
              <input v-model="editingProject.name" class="project-name-input" placeholder="Project name" />
              <input v-model="editingProject.color" type="color" style="width: 36px; height: 32px; border: none; cursor: pointer; border-radius: 6px;" />
              <button class="btn-primary" style="font-size: 13px;" @click="updateProject(editingProject.id, editingProject.name, editingProject.color)">Save</button>
              <button class="btn-cancel" @click="editingProject = null">Cancel</button>
            </div>

            <!-- Tasks Expanded Panel -->
            <div v-if="selectedProjectId === project.id" class="project-tasks-panel">
              <!-- Task Filter controls -->
              <div class="task-filters-row" style="display: flex; gap: 8px; margin-bottom: 12px; border-bottom: 1px solid var(--border-color); padding-bottom: 10px;">
                <button :class="['project-action-btn', taskStatusFilter === 'all' ? 'active-filter-btn' : '']" style="padding: 4px 12px; font-size: 0.8rem;" @click="taskStatusFilter = 'all'">All</button>
                <button :class="['project-action-btn', taskStatusFilter === 'active' ? 'active-filter-btn' : '']" style="padding: 4px 12px; font-size: 0.8rem;" @click="taskStatusFilter = 'active'">Active</button>
                <button :class="['project-action-btn', taskStatusFilter === 'done' ? 'active-filter-btn' : '']" style="padding: 4px 12px; font-size: 0.8rem;" @click="taskStatusFilter = 'done'">Done</button>
              </div>

              <div class="task-list">
                <div v-if="tasks.length === 0" class="task-empty-state">No tasks yet — add one below</div>
                <div v-for="task in tasks.filter(t => taskStatusFilter === 'all' ? true : taskStatusFilter === 'active' ? t.status !== 'done' : t.status === 'done')" :key="task.id"
                  class="task-row"
                  :class="{ 'task-row-active': activeTaskId === task.id, 'task-row-done': task.status === 'done' }"
                  style="display: flex; flex-direction: column; gap: 6px; padding: 10px; border-radius: 8px; margin-bottom: 6px;">

                  <div style="display: flex; align-items: center; justify-content: space-between; width: 100%;">
                    <div style="display: flex; align-items: center; gap: 8px; flex: 1; min-width: 0;">
                      <button class="task-check-btn" @click="updateTask(task.id, task.name, task.status === 'done' ? 'active' : 'done')">
                        {{ task.status === 'done' ? '✅' : '⭕' }}
                      </button>
                      <template v-if="editingTask?.id !== task.id">
                        <span class="task-name" :class="{ 'task-name-done': task.status === 'done' }" style="font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                          {{ task.name }}
                        </span>
                        <span style="font-size: 0.75rem; color: var(--text-muted); font-variant-numeric: tabular-nums; font-weight: 500; margin-left: 6px; white-space: nowrap;">
                          ({{ formatTime(dailyTaskSummary.find(d => d.task_id === task.id)?.total_seconds || 0) }})
                        </span>
                      </template>
                      <template v-else>
                        <input v-model="editingTask.name" class="task-edit-input" style="flex: 1; padding: 4px 8px; border-radius: 4px;"
                          @keyup.enter="updateTask(editingTask.id, editingTask.name, editingTask.status)" />
                      </template>
                    </div>

                    <div class="task-row-actions" style="display: flex; gap: 6px; align-items: center;">
                      <template v-if="editingTask?.id !== task.id">
                        <button v-if="activeTaskId !== task.id || trackingStatus !== 'running'" class="task-track-btn" @click="setActiveTask(task.id)">▶ Track</button>
                        <button v-else class="task-stop-btn" @click="setTracking('stopped')">⏹ Stop</button>
                        <button class="project-action-btn" style="padding: 2px 6px; font-size: 0.8rem;" @click="showRulesForTaskId = showRulesForTaskId === task.id ? null : task.id; if (showRulesForTaskId) loadTaskRules(task.id);" title="Auto-Assignment Rules">⚙️ Rules</button>
                        <button class="project-action-btn" @click="editingTask = { ...task }" title="Edit">✏️</button>
                        <button class="project-action-btn project-action-danger" @click="deleteTask(task.id)" title="Delete">🗑️</button>
                      </template>
                      <template v-else>
                        <button class="btn-primary" style="font-size: 12px; padding: 4px 10px;"
                          @click="updateTask(editingTask.id, editingTask.name, editingTask.status)">Save</button>
                        <button class="btn-cancel" @click="editingTask = null">✕</button>
                      </template>
                    </div>
                  </div>

                  <!-- Task Auto-Assignment Rules Panel -->
                  <div v-if="showRulesForTaskId === task.id" class="task-rules-panel" style="margin-top: 6px; padding: 8px 12px; background: var(--bg-secondary); border-radius: 6px; border: 1px solid var(--border-color); font-size: 0.8rem;">
                    <div style="font-weight: 600; margin-bottom: 6px; color: var(--text);">🤖 Auto-assignment Rules</div>
                    <div v-if="taskRules.length === 0" style="color: var(--text-muted); font-size: 0.75rem; margin-bottom: 8px;">No auto-assignment rules for this task.</div>
                    <div v-else style="display: flex; flex-direction: column; gap: 4px; margin-bottom: 8px;">
                      <div v-for="rule in taskRules" :key="rule.id" style="display: flex; align-items: center; justify-content: space-between; background: var(--card-bg); padding: 4px 8px; border-radius: 4px; border: 1px solid var(--border-color);">
                        <span style="font-family: monospace;">App: <strong>{{ rule.app_name }}</strong> <span v-if="rule.window_pattern" style="color: var(--text-muted);"> (Title: "{{ rule.window_pattern }}")</span></span>
                        <button @click="deleteTaskRule(rule.id, task.id)" style="background: none; border: none; color: var(--danger); cursor: pointer; font-size: 0.8rem; padding: 2px;">✕</button>
                      </div>
                    </div>
                    <!-- Add Rule form -->
                    <div style="display: flex; gap: 6px; align-items: center; flex-wrap: wrap;">
                      <input v-model="newTaskRuleAppName" placeholder="App process (e.g. Code, chrome)" style="flex: 1; min-width: 140px; padding: 4px 8px; font-size: 0.75rem; border-radius: 4px; border: 1px solid var(--border-color); background: var(--card-bg); color: var(--text-color);" />
                      <input v-model="newTaskRulePattern" placeholder="Window title pattern (optional)" style="flex: 1; min-width: 140px; padding: 4px 8px; font-size: 0.75rem; border-radius: 4px; border: 1px solid var(--border-color); background: var(--card-bg); color: var(--text-color);" />
                      <button class="btn btn-primary" style="padding: 4px 10px; font-size: 0.75rem;" @click="createTaskRule(task.id)">+ Add Rule</button>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Add Task -->
              <div class="task-add-area">
                <template v-if="showTaskForm && selectedProjectId === project.id">
                  <input v-model="newTaskName" placeholder="New task name…" class="task-add-input"
                    @keyup.enter="createTask(project.id)" />
                  <button class="btn-primary" style="font-size: 13px; padding: 6px 14px;"
                    @click="createTask(project.id)" :disabled="!newTaskName.trim()">Add</button>
                  <button class="btn-cancel" @click="showTaskForm = false; newTaskName = ''">✕</button>
                </template>
                <button v-else class="task-add-btn"
                  @click="showTaskForm = true; selectedProjectId = project.id; loadTasks(project.id);">+ Add Task</button>
              </div>
            </div>
          </div>
        </div>

        <!-- Empty State -->
        <div v-if="projects.filter(p => showArchivedProjects ? true : !p.archived).length === 0 && !showProjectForm" class="projects-empty-state">
          <div class="empty-state-icon">📁</div>
          <h3>No projects yet</h3>
          <p>Create your first project to start tracking time against tasks.</p>
          <button class="btn-primary" @click="showProjectForm = true">+ Create Project</button>
        </div>
      </div>

      <!-- TIMESHEET VIEW -->
      <div v-if="currentView === 'timesheet'" class="view-timesheet">
        <header class="view-header" style="flex-wrap: wrap; gap: 12px;">
          <h1>📅 {{ t('message.timesheet') }}</h1>
          <div style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap;">
            <!-- Date range presets -->
            <button class="project-action-btn" style="padding: 6px 12px; font-size: 0.8rem;" @click="setTimesheetPreset('today')">Today</button>
            <button class="project-action-btn" style="padding: 6px 12px; font-size: 0.8rem;" @click="setTimesheetPreset('week')">This Week</button>
            <button class="project-action-btn" style="padding: 6px 12px; font-size: 0.8rem;" @click="setTimesheetPreset('last_week')">Last Week</button>
            <button class="project-action-btn" style="padding: 6px 12px; font-size: 0.8rem;" @click="setTimesheetPreset('month')">This Month</button>
            
            <input type="date" v-model="timesheetDateRange.from" @change="loadTimesheetData(timesheetDateRange.from, timesheetDateRange.to)" style="padding: 6px 10px; border-radius: 6px; border: 1px solid var(--border-color); background: var(--card-bg); color: var(--text-color); font-size: 0.8rem;" />
            <span style="color: var(--text-muted);">to</span>
            <input type="date" v-model="timesheetDateRange.to" @change="loadTimesheetData(timesheetDateRange.from, timesheetDateRange.to)" style="padding: 6px 10px; border-radius: 6px; border: 1px solid var(--border-color); background: var(--card-bg); color: var(--text-color); font-size: 0.8rem;" />
            
            <button v-if="timesheetData.length > 0" class="btn btn-primary" style="padding: 6px 12px; font-size: 0.85rem; border-radius: 8px;" @click="exportTimesheetCSV">
              📥 Export CSV
            </button>
          </div>
        </header>

        <div class="card" style="overflow-x: auto; margin-bottom: 16px;">
          <table v-if="timesheetData.length > 0" class="timesheet-table">
            <thead>
              <tr>
                <th style="text-align: left; padding: 10px 14px;">Project</th>
                <th style="text-align: left; padding: 10px 14px;">Task</th>
                <th style="text-align: left; padding: 10px 14px;">Date</th>
                <th style="text-align: right; padding: 10px 14px;">Total</th>
                <th style="text-align: right; padding: 10px 14px; width: 80px;">Action</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(cell, idx) in timesheetData" :key="idx">
                <td style="padding: 10px 14px;">
                  <span style="display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 8px;" :style="{ background: cell.project_color }"></span>
                  {{ cell.project_name }}
                </td>
                <td style="padding: 10px 14px;">{{ cell.task_name }}</td>
                <td style="padding: 10px 14px; color: var(--text-muted); font-size: 13px;">{{ cell.date }}</td>
                <td style="padding: 10px 14px; text-align: right; font-variant-numeric: tabular-nums; font-weight: 600;">{{ formatTime(cell.seconds) }}</td>
                <td style="padding: 6px 14px; text-align: right;">
                  <button v-if="activeTaskId !== cell.task_id || trackingStatus !== 'running'" class="task-track-btn" style="padding: 2px 6px; font-size: 0.75rem;" @click="setActiveTask(cell.task_id)">▶ Track</button>
                  <button v-else class="task-stop-btn" style="padding: 2px 6px; font-size: 0.75rem;" @click="setTracking('stopped')">⏹ Stop</button>
                </td>
              </tr>
            </tbody>
          </table>
          <div v-else style="text-align: center; padding: 40px; color: var(--text-muted);">No timesheet data for this period.</div>
        </div>

        <div class="card">
          <h3 style="margin: 0 0 14px 0; font-size: 14px; font-weight: 700;">Daily Breakdown</h3>
          <div v-for="entry in dailyTaskSummary" :key="entry.task_id || 'untracked'" style="display: flex; align-items: center; gap: 8px; padding: 7px 0; border-bottom: 1px solid var(--border-color);">
            <span style="flex: 1; font-size: 13px;" :style="{ color: entry.task_id ? 'var(--text-color)' : 'var(--text-muted)' }">{{ entry.task_name }}</span>
            <span v-if="entry.project_name" style="font-size: 11px; color: var(--text-muted);">({{ entry.project_name }})</span>
            <span style="font-variant-numeric: tabular-nums; font-size: 13px; font-weight: 600;">{{ formatTime(entry.total_seconds) }}</span>
            <div style="width: 80px; text-align: right;">
              <template v-if="entry.task_id">
                <button v-if="activeTaskId !== entry.task_id || trackingStatus !== 'running'" class="task-track-btn" style="padding: 2px 6px; font-size: 0.75rem;" @click="setActiveTask(entry.task_id)">▶ Track</button>
                <button v-else class="task-stop-btn" style="padding: 2px 6px; font-size: 0.75rem;" @click="setTracking('stopped')">⏹ Stop</button>
              </template>
            </div>
          </div>
          <div v-if="dailyTaskSummary.length === 0" style="text-align: center; padding: 20px; color: var(--text-muted); font-size: 13px;">No data for today</div>
        </div>
      </div>

      <!-- ADMIN DASHBOARD VIEW -->
      <div v-if="currentView === 'admin'" class="view-admin">
        <header class="view-header">
          <h1>🛡️ {{ t("message.adminDashboard") }}</h1>
          <div style="display: flex; gap: 8px; align-items: center">
            <input type="date" v-model="adminDrillDate" @change="refreshAdminDrill" style="
                padding: 6px 10px;
                border-radius: 6px;
                border: 1px solid var(--border-color);
                background: var(--card-bg);
                color: var(--text-color);
                font-size: 0.9rem;
              " />
            <button class="btn-browse" @click="loadAdminData">
              🔄 {{ t("message.refresh") }}
            </button>
          </div>
        </header>

        <!-- Admin Tabs -->
        <div class="admin-tabs">
          <button :class="['admin-tab', { 'admin-tab-active': adminTab === 'team' }]" @click="
            adminTab = 'team';
          selectedUser = null;
          ">
            👥 {{ t("message.team") }}
          </button>
          <button :class="['admin-tab', { 'admin-tab-active': adminTab === 'categories' }]" @click="
            adminTab = 'categories';
          selectedUser = null;
          ">
            🏷️ {{ t("message.categories") }}
          </button>
          <button v-if="selectedUser" :class="['admin-tab', { 'admin-tab-active': adminTab === 'screenshots' }]"
            @click="loadUserDetail(selectedUser, 'screenshots')">
            📸 {{ t("message.screenshots") }}
          </button>
          <button v-if="selectedUser" :class="['admin-tab', { 'admin-tab-active': adminTab === 'timelogs' }]"
            @click="loadUserDetail(selectedUser, 'timelogs')">
            ⏱ {{ t("message.trackings") }}
          </button>
          <button v-if="selectedUser" :class="['admin-tab', { 'admin-tab-active': adminTab === 'activity' }]"
            @click="loadUserDetail(selectedUser, 'activity')">
            ⌨️ {{ t("message.activity") }}
          </button>
          <button v-if="selectedUser" :class="['admin-tab', { 'admin-tab-active': adminTab === 'urls' }]"
            @click="loadUserDetail(selectedUser, 'urls')">
            🌐 {{ t("message.urls") }}
          </button>
          <span v-if="selectedUser" style="
              margin-left: 8px;
              font-size: 0.85rem;
              color: var(--text-muted);
              align-self: center;
            ">{{ t("message.viewing") }}
            <strong>{{ selectedUser.display_name }}</strong></span>
        </div>

        <!-- CATEGORIES TAB -->
        <div v-if="adminTab === 'categories'">
          <div class="section-block">
            <div style="
                display: flex;
                justify-content: space-between;
                align-items: center;
                margin-bottom: 16px;
              ">
              <h2>{{ t("message.manageAppCategories") }}</h2>
              <button class="btn-browse" @click="loadAdminData">
                🔄 {{ t("message.reloadList") }}
              </button>
            </div>
            <div class="app-table categories-manage-table">
              <div class="app-row header-row" style="grid-template-columns: 1fr 180px">
                <span>{{ t("message.applicationName") }}</span><span>{{ t("message.category") }}</span>
              </div>
              <div v-for="cat in adminCategories" :key="cat.app_name" class="app-row"
                style="grid-template-columns: 1fr 180px">
                <span class="app-name-cell">
                  <span class="app-icon">{{ appIcon(cat.app_name) }}</span>
                  {{ cat.app_name }}
                </span>
                <select v-model="cat.category" @change="updateCategory(cat.app_name, cat.category)" class="badge-select"
                  :class="cat.category">
                  <option value="productive">✅ {{ t("message.productive") }}</option>
                  <option value="neutral">⚪ {{ t("message.neutral") }}</option>
                  <option value="unproductive">❌ {{ t("message.unproductive") }}</option>
                </select>
              </div>
              <div v-if="adminCategories.length === 0" class="empty-state">
                <p>{{ t("message.noTrackedApps") }}</p>
              </div>
            </div>
          </div>
        </div>

        <!-- TEAM TAB -->
        <div v-if="adminTab === 'team'">
          <!-- Productivity Stats -->
          <div class="section-block">
            <h2>{{ t("message.todaysTeamProductivity") }}</h2>
            <div class="app-table">
              <div class="app-row header-row" style="grid-template-columns: 2fr 1fr 1fr 1fr 1fr 1.5fr">
                <span>{{ t("message.employee") }}</span><span>{{ t("message.activeTime") }}</span><span>{{
                  t("message.kbMouse") }}</span><span>{{ t("message.sessions") }}</span><span>{{ t("message.username")
                  }}</span><span>{{ t("message.actions") }}</span>
              </div>
              <div v-for="stat in adminStats" :key="stat.user_id" class="app-row"
                style="grid-template-columns: 2fr 1fr 1fr 1fr 1fr 1.5fr">
                <span class="app-name">👤 {{ stat.display_name }}</span>
                <span class="app-time">{{ formatTime(stat.total_active_seconds) }}</span>
                <span class="app-activity" style="font-size: 0.8rem">
                  <span :title="t('message.keyboardEvents')">⌨️ {{ stat.keyboard_count }}</span>
                  /
                  <span :title="t('message.mouseEvents')">🖱️ {{ stat.mouse_count }}</span>
                </span>
                <span class="app-switches">{{ stat.session_count }}</span>
                <span class="app-time" style="font-size: 0.85rem; color: var(--text-muted)">{{ stat.username }}</span>
                <span style="display: flex; gap: 4px">
                  <button class="btn-drill"
                    @click="loadUserDetail(adminUsers.find(u => u.id === stat.user_id)!, 'screenshots')">
                    📸
                  </button>
                  <button class="btn-drill"
                    @click="loadUserDetail(adminUsers.find(u => u.id === stat.user_id)!, 'timelogs')">
                    ⏱
                  </button>
                  <button class="btn-drill"
                    @click="loadUserDetail(adminUsers.find(u => u.id === stat.user_id)!, 'activity')">
                    ⌨️
                  </button>
                </span>
              </div>
              <div v-if="adminStats.length === 0" class="empty-state">
                <p>{{ t("message.noDataToday") }}</p>
              </div>
            </div>
          </div>

          <!-- User Management -->
          <div class="section-block">
            <div style="
                display: flex;
                justify-content: space-between;
                align-items: center;
                margin-bottom: 16px;
              ">
              <h2>{{ t("message.teamMembers") }}</h2>
              <button class="btn-browse" @click="showCreateUser = !showCreateUser"
                style="padding: 8px 16px; font-size: 0.9rem">
                + {{ t("message.addUser") }}
              </button>
            </div>

            <!-- Create User Form -->
            <div v-if="showCreateUser" class="card" style="margin-bottom: 20px; padding: 24px">
              <h3 style="margin-bottom: 16px">{{ t("message.newTeamMember") }}</h3>
              <div class="wizard-form">
                <label>{{ t("message.username") }}</label>
                <input type="text" v-model="newUser.username" placeholder="jane_doe" />
                <label>{{ t("message.displayName") }}</label>
                <input type="text" v-model="newUser.display_name" placeholder="Jane Doe" />
                <label>{{ t("message.loginPassword") }}</label>
                <input type="password" v-model="newUser.password" :placeholder="t('message.temporaryPassword')" />
                <label>{{ t("message.role") }}</label>
                <select v-model="newUser.role">
                  <option value="employee">{{ t("message.employeeRole") }}</option>
                  <option value="admin">{{ t("message.adminRole") }}</option>
                </select>
              </div>
              <div v-if="createUserError" class="wizard-error" style="margin-top: 12px">
                {{ createUserError }}
              </div>
              <div style="display: flex; gap: 12px; margin-top: 16px">
                <button class="btn-stop" style="flex: 1" @click="showCreateUser = false">
                  {{ t("message.cancel") }}
                </button>
                <button class="btn-start" style="flex: 2" :disabled="createUserLoading" @click="doCreateUser">
                  {{
                    createUserLoading ? t("message.creating") : t("message.createUser")
                  }}
                </button>
              </div>
            </div>

            <!-- Users Table -->
            <div class="app-table">
              <div class="app-row header-row" style="grid-template-columns: 2fr 1.5fr 1fr">
                <span>{{ t("message.name") }}</span><span>{{ t("message.username") }}</span><span>{{ t("message.role")
                  }}</span>
              </div>
              <div v-for="user in adminUsers" :key="user.id" class="app-row"
                style="grid-template-columns: 2fr 1.5fr 1fr">
                <span class="app-name">{{ user.display_name }}</span>
                <span class="url-text">{{ user.username }}</span>
                <span>
                  <span :class="[
                    'role-badge',
                    user.role === 'admin' ? 'role-admin' : 'role-emp',
                  ]">{{ user.role }}</span>
                </span>
              </div>
              <div v-if="adminUsers.length === 0" class="empty-state">
                <p>{{ t("message.noTeamMembers") }}</p>
              </div>
            </div>
          </div>
        </div>

        <!-- SCREENSHOTS TAB -->
        <div v-if="adminTab === 'screenshots' && selectedUser">
          <div class="section-block">
            <h2>📸 {{ t("message.screenshots") }} — {{ selectedUser.display_name }}</h2>
            <div v-if="adminDrillLoading" class="empty-state">
              <p>{{ t("message.loading") }}</p>
            </div>
            <div v-else class="screenshots-grid">
              <div v-for="shot in adminUserScreenshots" :key="shot.id" class="screenshot-card"
                @click="openFullscreen(shot.file_path)" style="cursor: pointer">
                <img :src="convertFileSrc(shot.file_path)" alt="Screenshot" loading="lazy" />
                <div class="screenshot-info">
                  <span>{{ formatTimestamp(shot.captured_at) }}</span>
                </div>
              </div>
              <div v-if="adminUserScreenshots.length === 0" class="empty-state" style="grid-column: 1/-1">
                <p>{{ t("message.noScreenshotsForDate") }}</p>
              </div>
              <div v-else-if="adminUserScreenshots.length % pageSize === 0" class="load-more-container"
                style="grid-column: 1/-1">
                <button class="btn-start" @click="loadMoreAdminScreenshots">
                  {{ t("message.loadMore") }}
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- TIME LOGS TAB -->
        <div v-if="adminTab === 'timelogs' && selectedUser">
          <div class="section-block">
            <h2>⏱ {{ t("message.timeLogs") }} — {{ selectedUser.display_name }}</h2>
            <div v-if="adminDrillLoading" class="empty-state">
              <p>{{ t("message.loading") }}</p>
            </div>
            <div v-else class="app-table">
              <div class="app-row header-row" style="grid-template-columns: 2fr 3fr 1fr 1fr 1fr">
                <span>{{ t("message.app") }}</span><span>{{ t("message.window") }}</span><span>{{ t("message.start")
                  }}</span><span>{{ t("message.end") }}</span><span>{{ t("message.duration") }}</span>
              </div>
              <div v-for="log in adminUserTimeLogs" :key="log.id" class="app-row"
                style="grid-template-columns: 2fr 3fr 1fr 1fr 1fr">
                <span class="app-name"><span class="app-icon">{{ appIcon(log.app_name) }}</span>{{ log.app_name
                  }}</span>
                <span class="url-text" :title="log.window_title">{{
                  log.window_title
                }}</span>
                <span class="url-time">{{ formatTimestamp(log.start_time) }}</span>
                <span class="url-time">{{
                  log.end_time ? formatTimestamp(log.end_time) : "-"
                }}</span>
                <span :class="['app-time', log.status === 'idle' ? 'idle' : '']">
                  {{ formatTime(log.duration) }}
                </span>
              </div>
              <div v-if="adminUserTimeLogs.length === 0" class="empty-state">
                <p>{{ t("message.noTimeLogsForDate") }}</p>
              </div>
              <div v-else-if="adminUserTimeLogs.length % pageSize === 0" class="load-more-container">
                <button class="btn-start" @click="loadMoreAdminLogs">
                  {{ t("message.loadMore") }}
                </button>
              </div>
            </div>
          </div>
        </div>

        <div v-if="adminTab === 'activity' && selectedUser">
          <div class="section-block">
            <h2>⌨️ {{ t("message.activity") }} — {{ selectedUser.display_name }}</h2>
            <small style="display: block; margin-bottom: 12px; color: var(--text-muted)">
              {{
                t("message.activityTrackingDesc", { threshold: settings.idle_threshold })
              }}
            </small>
            <div v-if="adminDrillLoading" class="empty-state">
              <p>{{ t("message.loading") }}</p>
            </div>
            <div v-else>
              <!-- Keyboard / Mouse / Idle cards -->
              <div style="
                  display: grid;
                  grid-template-columns: repeat(4, 1fr);
                  gap: 12px;
                  margin-bottom: 16px;
                ">
                <div class="card premium-card" style="border-left: 3px solid var(--accent); padding: 12px 16px">
                  <h3 style="
                      font-size: 0.7rem;
                      text-transform: uppercase;
                      letter-spacing: 0.05em;
                    ">
                    ⌨️ {{ t("message.keyboardEvents") }}
                  </h3>
                  <p style="
                      font-size: 1.5rem;
                      font-weight: 800;
                      color: var(--accent);
                      margin: 2px 0;
                    ">
                    {{ adminInputStats.keyboard_count.toLocaleString() }}
                  </p>
                  <small style="color: var(--text-muted); font-size: 0.7rem">{{
                    t("message.keysPressed")
                  }}</small>
                </div>
                <div class="card premium-card" style="border-left: 3px solid var(--success); padding: 12px 16px">
                  <h3 style="
                      font-size: 0.7rem;
                      text-transform: uppercase;
                      letter-spacing: 0.05em;
                    ">
                    🖱 {{ t("message.mouseEvents") }}
                  </h3>
                  <p style="
                      font-size: 1.5rem;
                      font-weight: 800;
                      color: var(--success);
                      margin: 2px 0;
                    ">
                    {{ adminInputStats.mouse_count.toLocaleString() }}
                  </p>
                  <small style="color: var(--text-muted); font-size: 0.7rem">{{
                    t("message.movementsDetected")
                  }}</small>
                </div>
                <div class="card premium-card" style="border-left: 3px solid var(--warning); padding: 12px 16px">
                  <h3 style="
                      font-size: 0.7rem;
                      text-transform: uppercase;
                      letter-spacing: 0.05em;
                    ">
                    😴 {{ t("message.idlePeriods") }}
                  </h3>
                  <p style="
                      font-size: 1.5rem;
                      font-weight: 800;
                      color: var(--warning);
                      margin: 2px 0;
                    ">
                    {{ adminInputStats.idle_start_count }}
                  </p>
                  <small style="color: var(--text-muted); font-size: 0.7rem">{{
                    t("message.idleStartsLogged")
                  }}</small>
                </div>
                <div class="card premium-card" style="border-left: 3px solid var(--text-muted); padding: 12px 16px">
                  <h3 style="
                      font-size: 0.7rem;
                      text-transform: uppercase;
                      letter-spacing: 0.05em;
                    ">
                    📋 {{ t("message.totalEvents") }}
                  </h3>
                  <p style="font-size: 1.5rem; font-weight: 800; margin: 2px 0">
                    {{ adminUserActivity.length.toLocaleString() }}
                  </p>
                  <small style="color: var(--text-muted); font-size: 0.7rem">{{
                    t("message.rawLogEntries")
                  }}</small>
                </div>
              </div>
              <!-- Raw event log -->
              <div class="app-table">
                <div class="app-row header-row" style="grid-template-columns: 2fr 2fr 1fr 1fr">
                  <span>{{ t("message.type") }}</span><span>{{ t("message.time") }}</span><span>{{ t("message.input")
                    }}</span><span>{{ t("message.status") }}</span>
                </div>
                <div v-for="act in adminUserActivity" :key="act.id" class="app-row"
                  style="grid-template-columns: 2fr 2fr 1fr 1fr">
                  <span class="app-name" style="display: flex; align-items: center; gap: 6px">
                    <span v-if="act.event_type === 'keyboard'">⌨️</span>
                    <span v-else-if="act.event_type === 'mouse'">🖱</span>
                    <span v-else-if="act.event_type === 'idle_start'">😴</span>
                    <span v-else-if="act.event_type === 'idle_end'">▶️</span>
                    <span v-else-if="act.event_type.startsWith('url:')">🌐</span>
                    <span v-else>📌</span>
                    {{
                      act.event_type.startsWith("url:")
                        ? act.event_type.replace("url:", "")
                        : act.event_type
                    }}
                  </span>
                  <span class="url-time">{{ formatTimestamp(act.timestamp) }}</span>
                  <span style="font-size: 0.8rem; font-weight: 600" :style="{
                    color:
                      act.event_type === 'keyboard'
                        ? 'var(--accent)'
                        : act.event_type === 'mouse'
                          ? 'var(--success)'
                          : 'var(--text-muted)',
                  }">
                    {{
                      act.event_type === "keyboard"
                        ? "KB"
                        : act.event_type === "mouse"
                          ? t("message.mouse")
                          : "—"
                    }}
                  </span>
                  <span :style="{
                    color:
                      act.activity_status === 'idle'
                        ? 'var(--warning)'
                        : 'var(--success)',
                    fontWeight: '600',
                    fontSize: '0.8rem',
                  }">
                    {{ act.activity_status }}
                  </span>
                </div>
                <div v-if="adminUserActivity.length === 0" class="empty-state">
                  <p>{{ t("message.noActivityForDate") }}</p>
                </div>
                <div v-else-if="adminUserActivity.length % pageSize === 0" class="load-more-container">
                  <button class="btn-start" @click="loadMoreAdminActivity">
                    {{ t("message.loadMore") }}
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- URLS TAB -->
        <div v-if="adminTab === 'urls' && selectedUser">
          <div class="section-block">
            <h2>🌐 {{ t("message.urls") }} — {{ selectedUser.display_name }}</h2>
            <div v-if="adminDrillLoading" class="empty-state">
              <p>{{ t("message.loading") }}</p>
            </div>
            <div v-else>
              <div class="url-list">
                <div v-for="entry in adminUserUrls" :key="entry.id" class="url-row">
                  <span class="url-time">{{ formatTimestamp(entry.timestamp) }}</span>
                  <span class="url-text">{{ entry.url }}</span>
                </div>
                <div v-if="adminUserUrls.length === 0" class="empty-state">
                  {{ t("message.noHistoryForDate") }}
                </div>
                <div v-else-if="adminUserUrls.length % pageSize === 0" class="load-more-container">
                  <button class="btn-start" @click="loadMoreAdminUrls">
                    {{ t("message.loadMore") }}
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>
  </div>

  <!-- Global Fullscreen Screenshot Lightbox -->
  <div v-if="fullscreenScreenshot" class="screenshot-lightbox" @click="closeFullscreen">
    <img :src="convertFileSrc(fullscreenScreenshot)" alt="Fullscreen Screenshot" />
    <button class="lightbox-close" @click.stop="closeFullscreen">×</button>
  </div>
</template>

<style>
@import url("https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&display=swap");

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

/* ─── Lightbox ─────────────────────────────────────── */
.screenshot-lightbox {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.85);
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(4px);
  cursor: zoom-out;
}

.screenshot-lightbox img {
  max-width: 90vw;
  max-height: 90vh;
  border-radius: 8px;
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.5);
  object-fit: contain;
}

.lightbox-close {
  position: absolute;
  top: 20px;
  right: 30px;
  background: none;
  border: none;
  color: white;
  font-size: 2.5rem;
  cursor: pointer;
  line-height: 1;
}

.lightbox-close:hover {
  color: #ccc;
}

* {
  box-sizing: border-box;
  margin: 0;
}

body {
  margin: 0;
  background-color: var(--bg-color);
  color: var(--text-color);
  font-family: "Inter", system-ui, -apple-system, sans-serif;
  transition: background-color 0.3s, color 0.3s;
}

/* ─── Slim Modern Scrollbar ─────────────────────────────────────── */
* {
  scrollbar-width: thin;
  scrollbar-color: var(--border-color) transparent;
}

*::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

*::-webkit-scrollbar-track {
  background: rgba(148, 163, 184, 0.12);
  border-radius: 999px;
}

*::-webkit-scrollbar-thumb {
  background: linear-gradient(180deg, var(--accent) 0%, #7c83ff 100%);
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.15);
  transition: filter 0.2s ease;
}

*::-webkit-scrollbar-thumb:hover {
  filter: brightness(1.08);
}

*::-webkit-scrollbar-corner {
  background: transparent;
}

.app-layout {
  display: flex;
  height: 100vh;
  overflow: hidden;
}

/* ─── Sidebar ──────────────────────────────────────────────────── */
.sidebar {
  width: 260px;
  background-color: var(--sidebar-bg);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

.sidebar-body {
  flex: 1;
  overflow-y: auto;
  padding-bottom: 24px;
}

.sidebar-body::-webkit-scrollbar {
  width: 5px;
}

.sidebar-body::-webkit-scrollbar-track {
  background: transparent;
}

.sidebar-body::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 3px;
}

.sidebar-body::-webkit-scrollbar-thumb:hover {
  background: var(--text-muted);
}

.logo {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 24px;
  font-weight: 700;
  color: var(--accent);
}

.logo h2 {
  font-size: 1.1rem;
}

.logo img {
  border-radius: 8px;
}

nav {
  display: flex;
  flex-direction: column;
  padding: 0 16px;
  gap: 4px;
}

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

nav button:hover {
  background: var(--border-color);
}

nav button.active {
  background: var(--accent);
  color: white;
}

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

.session-inactive {
  text-align: center;
}

.pulse-dot {
  width: 12px;
  height: 12px;
  background: var(--success);
  border-radius: 50%;
  animation: pulse 1.5s infinite;
  display: inline-block;
}

.pomodoro-timer {
  font-size: 1.4rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  color: var(--accent);
  margin: 4px 0;
}

.pomodoro-count {
  font-size: 0.85rem;
  color: var(--text-muted);
  margin-bottom: 4px;
}

@keyframes pulse {

  0%,
  100% {
    opacity: 1;
    transform: scale(1);
  }

  50% {
    opacity: 0.5;
    transform: scale(1.3);
  }
}

.btn-start,
.btn-stop {
  width: 100%;
  padding: 10px;
  border: none;
  border-radius: 8px;
  font-weight: 600;
  font-size: 0.9rem;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-start {
  background: var(--success);
  color: white;
}

.btn-start:hover {
  filter: brightness(1.1);
}

.btn-stop {
  background: var(--danger);
  color: white;
  margin-top: 4px;
}

.btn-stop:hover {
  filter: brightness(1.1);
}

/* Floating Wellness Widget (FAB) */
.floating-wellness-widget {
  position: fixed;
  bottom: 24px;
  right: 24px;
  z-index: 1000;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 12px;
  font-family: inherit;
}

.wellness-fab {
  background: var(--accent);
  color: white;
  border: none;
  padding: 12px 20px;
  border-radius: 50px;
  font-weight: 700;
  font-size: 0.9rem;
  cursor: pointer;
  box-shadow: 0 4px 15px rgba(99, 102, 241, 0.4);
  display: flex;
  align-items: center;
  gap: 8px;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.wellness-fab:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(99, 102, 241, 0.55);
}

.fab-icon {
  font-size: 1.1rem;
}

.fab-text {
  font-variant-numeric: tabular-nums;
}

/* Popover Panel */
.wellness-popover {
  width: 320px;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 16px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.12);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: slideUp 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  padding: 18px !important;
}

@keyframes slideUp {
  from {
    opacity: 0;
    transform: translateY(15px) scale(0.95);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

/* Popover Inner Styles */
.popover-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 14px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--border-color);
}

.popover-title {
  font-size: 1rem;
  font-weight: 700;
  color: var(--text-color);
}

.popover-close {
  background: none;
  border: none;
  font-size: 1.4rem;
  color: var(--text-muted);
  cursor: pointer;
  padding: 0 4px;
  line-height: 1;
  transition: color 0.15s ease;
}

.popover-close:hover {
  color: var(--danger);
}

.popover-section {
  padding: 12px 0;
  border-bottom: 1px solid var(--border-color);
}

.popover-section:last-child {
  border-bottom: none;
  padding-bottom: 0;
}

.popover-section-title {
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  margin-bottom: 10px;
}

.break-fab-status {
  text-align: center;
  margin-bottom: 4px;
}

.fab-break-badge {
  display: inline-block;
  font-size: 0.8rem;
  font-weight: 700;
  padding: 4px 12px;
  border-radius: 20px;
}

.fab-break-badge.counting {
  background: rgba(16, 185, 129, 0.12);
  color: var(--success);
}

.fab-break-badge.on-break {
  background: rgba(245, 158, 11, 0.12);
  color: #f59e0b;
}

.fab-break-badge.pre-break {
  background: rgba(239, 68, 68, 0.12);
  color: var(--danger);
}

.fab-break-badge.paused {
  background: rgba(148, 163, 184, 0.12);
  color: var(--text-muted);
}

.fab-break-badge.idle {
  background: rgba(148, 163, 184, 0.08);
  color: var(--text-muted);
}


/* ─── Main Content ─────────────────────────────────────────────── */
.main-content {
  flex: 1;
  padding: 32px 40px;
  overflow: auto;
  min-width: 0;
  overscroll-behavior: contain;
}

header h1 {
  margin-top: 0;
  margin-bottom: 24px;
  font-size: 1.8rem;
  font-weight: 700;
}

.card {
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 24px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
}

/* Summary Cards */
.summary-cards {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  grid-template-areas:
    "active idle total"
    "activity activity activity";
  gap: 16px;
  margin-bottom: 24px;
  width: 100%;
}

.premium-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 16px;
  background: linear-gradient(135deg, var(--card-bg) 0%, rgba(99, 102, 241, 0.04) 100%);
  border-radius: 12px;
  border: 1px solid var(--border-color);
  transition: all 0.2s ease;
  overflow: hidden;
}

.premium-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.06);
}

.premium-card h3 {
  margin: 0 0 6px 0;
  font-size: 0.72rem;
  font-weight: 700;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.summary-metric-card {
  min-height: 92px;
}

.summary-active {
  grid-area: active;
}

.summary-idle {
  grid-area: idle;
}

.summary-total {
  grid-area: total;
}

.summary-input-card {
  grid-area: activity;
  align-items: stretch;
  padding: 14px;
}

.summary-input-card h3 {
  margin-bottom: 10px;
}

.summary-input-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.summary-input-chip {
  display: flex;
  align-items: center;
  gap: 10px;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  padding: 10px 12px;
  background: rgba(255, 255, 255, 0.02);
}

.summary-input-chip.keyboard {
  border-left: 3px solid var(--accent);
}

.summary-input-chip.mouse {
  border-left: 3px solid var(--success);
}

.chip-icon {
  font-size: 1rem;
  line-height: 1;
}

.chip-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.chip-text small {
  color: var(--text-muted);
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  font-weight: 700;
  line-height: 1.05;
}

.chip-text strong {
  font-size: 1.2rem;
  line-height: 1.1;
  color: var(--text-color);
  word-break: break-word;
}

.big-stat {
  font-size: 1.6rem;
  font-weight: 800;
  color: var(--accent);
  margin: 0;
  line-height: 1.2;
}

.big-stat.idle {
  color: var(--warning);
}

.big-stat.session {
  color: var(--success);
}

.big-stat.total {
  color: var(--text-color);
}

/* Productivity grid default */
.activity-summary-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 24px;
  padding: 0 16px 24px 16px;
}

.productivity-grid {
  display: grid;
  grid-template-columns: 1fr 2fr;
  gap: 24px;
  padding: 16px;
}

@media (max-width: 980px) {
  .sidebar {
    width: 220px;
    min-width: 220px;
  }

  .main-content {
    padding: 20px 16px;
  }

  .view-header h1 {
    font-size: 1.02rem;
  }

  .summary-cards {
    min-width: 620px;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-template-areas:
      "active idle"
      "total total"
      "activity activity";
  }

  .summary-metric-card {
    min-height: 84px;
    padding: 12px;
  }

  .premium-card h3 {
    font-size: 0.62rem;
    letter-spacing: 0.03em;
  }

  .big-stat {
    font-size: 1.28rem;
  }

  .summary-input-card {
    padding: 10px;
  }

  .summary-input-chip {
    gap: 8px;
    padding: 8px 9px;
  }

  .chip-icon {
    font-size: 0.84rem;
  }

  .chip-text small {
    font-size: 0.56rem;
  }

  .chip-text strong {
    font-size: 0.74rem;
  }
}

@media (max-width: 720px) {
  .sidebar {
    width: 192px;
    min-width: 192px;
  }

  .logo {
    padding: 16px;
  }

  nav button {
    font-size: 0.86rem;
    padding: 10px 12px;
  }

  .summary-cards {
    min-width: 560px;
  }

  .privacy-notice {
    padding: 10px 12px;
  }

  .privacy-text strong {
    font-size: 0.76rem;
  }

  .privacy-text p {
    font-size: 0.66rem;
    line-height: 1.35;
  }
}

@media (max-width: 900px) {
  .app-layout {
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .sidebar {
    order: 2;
    width: 100%;
    min-width: 0;
    max-height: none;
    height: auto;
    overflow: visible;
    border-right: none;
    border-top: 1px solid var(--border-color);
    flex-direction: column;
    flex-shrink: 0;
  }

  .logo {
    display: none !important;
  }

  .sidebar-body {
    flex: 0;
    overflow: visible;
    padding-bottom: 0;
  }

  nav {
    flex-direction: row;
    padding: 0;
    gap: 0;
    overflow-x: auto;
    overflow-y: hidden;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: none;
  }

  nav::-webkit-scrollbar {
    display: none;
  }

  nav button {
    flex: 0 0 auto;
    padding: 10px 14px;
    font-size: 0.78rem;
    border-radius: 0;
    text-align: center;
    white-space: nowrap;
    border-bottom: 3px solid transparent;
    transition: all 0.15s ease;
  }

  nav button.active {
    border-radius: 0;
    background: rgba(99, 102, 241, 0.1);
    border-bottom: 3px solid var(--accent);
    color: var(--accent);
  }

  nav button:hover {
    background: rgba(99, 102, 241, 0.05);
    border-radius: 0;
  }

  .tracking-control {
    order: -1;
    border-top: none;
    border-bottom: 1px solid var(--border-color);
    padding: 8px 12px;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .tracking-status {
    margin-bottom: 0;
    font-size: 0.8rem;
    white-space: nowrap;
  }

  .tracking-buttons {
    gap: 4px;
  }

  .btn-tracking {
    padding: 6px 10px !important;
    font-size: 0.75rem;
  }

  .api-queue-indicator {
    display: none;
  }

  .user-badge {
    display: none;
  }

  .main-content {
    order: 1;
    width: 100%;
    min-width: 0;
    padding: 14px 12px 18px;
    overflow: auto;
    flex: 1;
  }

  .summary-cards {
    min-width: 0;
    grid-template-columns: 1fr;
    grid-template-areas:
      "active"
      "idle"
      "total"
      "activity";
  }

  .summary-input-grid {
    grid-template-columns: 1fr;
  }

  .big-stat {
    font-size: 1.06rem;
  }

  .chip-text strong {
    font-size: 1rem;
  }

  .btn-browse {
    font-size: 0.82rem !important;
    padding: 6px 10px !important;
  }

  .view-productivity .activity-summary-row {
    display: grid !important;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)) !important;
    gap: 12px !important;
    padding: 0 0 12px 0 !important;
  }

  .view-productivity .activity-stat-card {
    width: 100% !important;
    min-width: 0 !important;
    padding: 12px !important;
    gap: 10px !important;
  }

  .view-productivity .activity-stat-card .stat-icon {
    width: 36px !important;
    height: 36px !important;
    font-size: 1rem !important;
    border-radius: 10px !important;
  }

  .view-productivity .activity-stat-card div[style*="font-size: 1.5rem"] {
    font-size: 1.15rem !important;
    line-height: 1.2 !important;
    word-break: break-word !important;
  }

  .view-productivity .productivity-grid {
    display: grid !important;
    grid-template-columns: 1fr !important;
    gap: 12px !important;
    padding: 0 !important;
  }

  .view-productivity .chart-container {
    height: 260px !important;
    padding: 12px !important;
  }

  .view-productivity .card {
    padding: 12px !important;
    min-width: 0 !important;
  }

  .view-productivity .table-responsive {
    overflow-x: auto !important;
    -webkit-overflow-scrolling: touch;
  }

  .view-productivity .table-responsive table {
    min-width: 520px;
  }

  .view-productivity th,
  .view-productivity td {
    padding: 10px 12px;
    font-size: 0.82rem;
    white-space: nowrap;
  }

  .view-productivity .category-badge-select {
    font-size: 0.7rem;
    padding: 4px 8px;
    min-width: 94px;
  }

  .floating-wellness-widget {
    bottom: 80px;
  }

  .nav-scroll-wrapper {
    position: relative;
    display: flex;
    align-items: stretch;
  }

  .nav-scroll-btn {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 28px;
    z-index: 5;
    background: none;
    border: none;
    font-size: 1.3rem;
    font-weight: 700;
    color: var(--accent);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    transition: opacity 0.15s ease;
  }

  .nav-scroll-left {
    left: 0;
    background: linear-gradient(to right, var(--sidebar-bg) 60%, transparent);
    padding-right: 4px;
  }

  .nav-scroll-right {
    right: 0;
    background: linear-gradient(to left, var(--sidebar-bg) 60%, transparent);
    padding-left: 4px;
  }

  .nav-scroll-btn:hover {
    color: var(--text-color);
  }
}

/* Section Blocks */
.section-block {
  margin-bottom: 24px;
}

.section-block h2 {
  font-size: 1.1rem;
  font-weight: 700;
  margin-bottom: 12px;
}

/* App Table */
.app-table {
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  overflow: hidden;
}

.app-row {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr;
  padding: 12px 20px;
  align-items: center;
  border-bottom: 1px solid var(--border-color);
  font-size: 0.9rem;
}

.app-table.with-category .app-row {
  grid-template-columns: 3fr 1fr 1fr 1.5fr;
}

.app-row:last-child {
  border-bottom: none;
}

.header-row {
  background: var(--bg-color);
  font-weight: 600;
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
}

.col-app-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.app-main-info {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.app-name {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  color: var(--text-color);
}

.app-icon {
  font-size: 1.2rem;
}

.app-progress-container {
  display: flex;
  align-items: center;
  gap: 10px;
}

.bar-bg {
  flex: 1;
  height: 4px;
  background: var(--bar-bg);
  border-radius: 2px;
  overflow: hidden;
}

.bar-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 2px;
  transition: width 0.5s ease;
}

.app-percent-text {
  font-size: 0.75rem;
  color: var(--text-muted);
  min-width: 30px;
  text-align: right;
}

.col-time-val {
  font-weight: 600;
  color: var(--accent);
  text-align: center;
}

.col-switches-val {
  text-align: center;
  color: var(--text-muted);
  font-family: monospace;
}

.col-category-val {
  display: flex;
  justify-content: flex-end;
}

/* Category Badge Select */
.category-badge-select {
  appearance: none;
  border: none;
  border-radius: 20px;
  padding: 4px 12px;
  font-size: 0.75rem;
  font-weight: 700;
  cursor: pointer;
  text-align: center;
  width: auto;
  min-width: 110px;
  transition: all 0.2s ease;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='white' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  padding-right: 24px;
  color: white;
}

.category-badge-select.productive {
  background-color: var(--success);
}

.category-badge-select.neutral {
  background-color: var(--text-muted);
}

.category-badge-select.unproductive {
  background-color: var(--danger);
}

.category-badge-select:hover {
  opacity: 0.9;
  transform: scale(1.02);
}

.category-badge-select:focus {
  outline: none;
  box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.2);
}

.category-badge-select option {
  background-color: var(--card-bg);
  color: var(--text-color);
}

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

.url-row-clickable {
  cursor: pointer;
}

.url-row-expanded {
  background: rgba(92, 110, 255, 0.08);
}

.url-row:last-child {
  border-bottom: none;
}

.url-time {
  color: var(--text-muted);
  font-size: 0.85rem;
  min-width: 80px;
}

.url-text {
  color: var(--accent);
  font-weight: 500;
  word-break: break-all;
}

.url-entry-content {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.url-expanded-link {
  font-size: 0.82rem;
  color: var(--text-muted);
  text-decoration: underline;
  word-break: break-all;
}

.load-more-container {
  padding: 16px;
  text-align: center;
  border-top: 1px solid var(--border-color);
}

.load-more-container button {
  max-width: 200px;
}

.empty-state {
  padding: 40px;
  text-align: center;
  background: var(--card-bg);
  border: 1px dashed var(--border-color);
  border-radius: 12px;
  color: var(--text-muted);
}

/* ─── Settings ─────────────────────────────────────────────────── */
/* ─── Settings ─────────────────────────────────────────────────── */
.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}

.setting-card {
  display: flex;
  flex-direction: column;
  gap: 10px;
  transition: transform 0.2s ease, box-shadow 0.2s ease;
  background: var(--card-bg);
}

.setting-card:hover:not(.card-disabled) {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
  border-color: var(--accent);
}

.setting-card label {
  font-weight: 700;
  font-size: 0.9rem;
  color: var(--text-color);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

select,
input[type="text"],
input[type="password"],
input[type="number"] {
  width: 100%;
  padding: 10px;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background: var(--bg-color);
  color: var(--text-color);
  font-size: 1rem;
  transition: all 0.2s ease;
}

select:focus,
input[type="text"]:focus,
input[type="password"]:focus,
input[type="number"]:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
}

/* The switch - the box around the slider */
.switch {
  position: relative;
  display: inline-block;
  width: 44px;
  height: 24px;
  flex-shrink: 0;
}

/* Hide default HTML checkbox */
.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

/* The slider */
.slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--border-color);
  transition: .3s ease;
  border-radius: 24px;
}

.slider:before {
  position: absolute;
  content: "";
  height: 18px;
  width: 18px;
  left: 3px;
  bottom: 3px;
  background-color: #fff;
  transition: .3s ease;
  border-radius: 50%;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}

input:checked + .slider {
  background-color: var(--accent);
}

input:focus + .slider {
  box-shadow: 0 0 1px var(--accent);
}

input:checked + .slider:before {
  transform: translateX(20px);
}

/* Disabled state styling */
input:disabled + .slider {
  opacity: 0.5;
  cursor: not-allowed;
}

input[type="checkbox"] {
  width: 24px;
  height: 24px;
  accent-color: var(--accent);
  cursor: pointer;
}

.input-with-button {
  display: flex;
  gap: 8px;
}

.input-with-button input {
  flex: 1;
}

.btn-browse {
  background: var(--bg-color);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 8px 14px;
  cursor: pointer;
  transition: all 0.2s ease;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  font-size: 0.9rem;
  color: var(--text-color);
  height: 36px;
  white-space: nowrap;
}

.btn-browse:hover {
  background: var(--border-color);
}

/* ─── Tracking Control ─────────────────────────────────────────── */
.tracking-control {
  margin: 0;
  padding: 14px 16px;
  background: var(--sidebar-bg);
  border-top: 1px solid var(--border-color);
  flex-shrink: 0;
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
  width: 10px;
  height: 10px;
  border-radius: 50%;
  display: inline-block;
}

.status-dot.running {
  background: var(--success);
  animation: pulse 1.5s infinite;
}

.status-dot.paused {
  background: var(--warning);
}

.status-dot.stopped {
  background: var(--danger);
}

.status-label {
  color: var(--text-muted);
}

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

.btn-pause {
  background: var(--warning);
  color: #1a1a2e;
}

.btn-pause:hover {
  filter: brightness(1.1);
}

.btn-resume {
  background: var(--success);
  color: white;
}

.btn-resume:hover {
  filter: brightness(1.1);
}

.btn-stop-track {
  background: var(--danger);
  color: white;
}

.btn-stop-track:hover {
  filter: brightness(1.1);
}

.btn-start-track {
  background: var(--success);
  color: white;
  width: 100%;
}

.btn-start-track:hover {
  filter: brightness(1.1);
}

/* ─── Filters & New Views ──────────────────────────────────────── */
.view-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
  flex-wrap: wrap;
  gap: 12px;
}

.view-header h1 {
  margin: 0;
  font-size: 1.5rem;
}

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
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
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
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: var(--bg-color);
  z-index: 999;
}

.loading-text {
  color: var(--text-muted);
  margin-top: 16px;
  font-size: 0.95rem;
}

.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid var(--border-color);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

/* ─── Wizard ────────────────────────────────────────────────────── */
.wizard-bg {
  background: linear-gradient(135deg, #0f1115 0%, #1a1040 100%);
}

.wizard-card {
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 20px;
  padding: 48px 40px;
  width: 100%;
  max-width: 560px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
  animation: fadeIn 0.4s ease;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(20px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.wizard-logo {
  text-align: center;
  margin-bottom: 32px;
}

.wizard-logo h1 {
  font-size: 1.6rem;
  font-weight: 800;
  margin: 12px 0 4px;
}

.wizard-logo p {
  color: var(--text-muted);
}

.wizard-logo img {
  border-radius: 12px;
}

.wizard-step h2 {
  font-size: 1.2rem;
  font-weight: 700;
  margin-bottom: 20px;
}

.mode-cards {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-bottom: 24px;
}

.mode-card {
  border: 2px solid var(--border-color);
  border-radius: 12px;
  padding: 20px 16px;
  cursor: pointer;
  text-align: center;
  transition: all 0.2s ease;
}

.mode-card:hover {
  border-color: var(--accent);
  background: rgba(99, 102, 241, 0.05);
}

.mode-card-active {
  border-color: var(--accent) !important;
  background: rgba(99, 102, 241, 0.1) !important;
}

.mode-icon {
  font-size: 2rem;
  display: block;
  margin-bottom: 8px;
}

.mode-card strong {
  font-size: 1rem;
  display: block;
  margin-bottom: 6px;
}

.mode-card p {
  font-size: 0.82rem;
  color: var(--text-muted);
  margin: 0;
  line-height: 1.4;
}

.wizard-form {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 20px;
}

.wizard-form label {
  font-size: 0.88rem;
  font-weight: 600;
  color: var(--text-muted);
  margin-top: 4px;
}

.wizard-error,
.login-error {
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid var(--danger);
  color: var(--danger);
  border-radius: 8px;
  padding: 10px 14px;
  font-size: 0.88rem;
  margin-bottom: 12px;
}

.wizard-actions {
  display: flex;
  gap: 12px;
  margin-top: 8px;
}

.btn-wizard-next {
  flex: 2;
  background: var(--accent);
  color: white;
  border: none;
  border-radius: 10px;
  padding: 12px 20px;
  font-weight: 700;
  font-size: 1rem;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-wizard-next:hover:not(:disabled) {
  filter: brightness(1.12);
  transform: translateY(-1px);
}

.btn-wizard-next:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-wizard-back {
  flex: 1;
  background: var(--bg-color);
  color: var(--text-muted);
  border: 1px solid var(--border-color);
  border-radius: 10px;
  padding: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-wizard-back:hover {
  background: var(--border-color);
}

.confirm-summary {
  background: var(--bg-color);
  border-radius: 10px;
  padding: 16px 20px;
  margin-bottom: 20px;
}

.confirm-row {
  display: flex;
  justify-content: space-between;
  padding: 8px 0;
  border-bottom: 1px solid var(--border-color);
  font-size: 0.92rem;
}

.confirm-row:last-child {
  border-bottom: none;
}

.confirm-row span {
  color: var(--text-muted);
}

/* ─── Login ─────────────────────────────────────────────────────── */
.login-bg {
  background: linear-gradient(135deg, #0f1115 0%, #0c1240 100%);
}

.login-card {
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 20px;
  padding: 48px 40px;
  width: 100%;
  max-width: 400px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
  animation: fadeIn 0.4s ease;
}

.login-logo {
  text-align: center;
  margin-bottom: 32px;
}

.login-logo h1 {
  font-size: 1.5rem;
  font-weight: 800;
  margin: 12px 0 4px;
}

.login-logo p {
  color: var(--text-muted);
}

.login-logo img {
  border-radius: 12px;
}

.login-form {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.login-form label {
  font-size: 0.88rem;
  font-weight: 600;
  color: var(--text-muted);
  margin-top: 8px;
}

.btn-login {
  width: 100%;
  margin-top: 16px;
  background: var(--accent);
  color: white;
  border: none;
  border-radius: 10px;
  padding: 13px;
  font-weight: 700;
  font-size: 1rem;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-login:hover:not(:disabled) {
  filter: brightness(1.12);
  transform: translateY(-1px);
}

.btn-login:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* ─── User Badge (sidebar bottom) ───────────────────────────────── */
.user-badge {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 16px 24px;
  border-top: 1px solid var(--border-color);
  background: var(--sidebar-bg);
  margin-top: auto;
  /* Push to bottom if sidebar has flex-grow elements */
}

.user-avatar {
  width: 34px;
  height: 34px;
  border-radius: 50%;
  background: var(--accent);
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 0.95rem;
  flex-shrink: 0;
}

.user-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.user-name {
  font-size: 0.88rem;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.btn-logout {
  background: none;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  width: 30px;
  height: 30px;
  cursor: pointer;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1rem;
  transition: all 0.2s ease;
  flex-shrink: 0;
}

.btn-logout:hover {
  background: var(--danger);
  border-color: var(--danger);
  color: white;
}

/* ─── Role Badge ─────────────────────────────────────────────────── */
.role-badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 0.72rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.role-admin {
  background: rgba(99, 102, 241, 0.15);
  color: var(--accent);
}

.role-emp {
  background: rgba(100, 116, 139, 0.15);
  color: var(--text-muted);
}

/* ─── Admin View ─────────────────────────────────────────────────── */
.view-admin {
  padding-bottom: 40px;
}

/* ─── Settings Extra ─────────────────────────────────────────────── */
.setting-card-wide {
  grid-column: 1 / -1;
}

.mode-toggle-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 10px;
  flex-wrap: wrap;
}

.mode-current-badge {
  padding: 4px 12px;
  border-radius: 999px;
  font-size: 0.8rem;
  font-weight: 700;
}

.mode-badge-single {
  background: rgba(100, 116, 139, 0.15);
  color: var(--text-muted);
}

.mode-badge-multi {
  background: rgba(99, 102, 241, 0.15);
  color: var(--accent);
}

.mode-select {
  flex: 1;
  min-width: 160px;
}

.btn-change-mode {
  background: var(--accent);
  color: white;
  border: none;
  border-radius: 8px;
  padding: 8px 16px;
  font-weight: 600;
  font-size: 0.9rem;
  cursor: pointer;
  transition: all 0.2s ease;
  white-space: nowrap;
}

.btn-change-mode:hover:not(:disabled) {
  filter: brightness(1.12);
}

.btn-change-mode:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.danger-card {
  border-color: rgba(239, 68, 68, 0.3) !important;
}

.danger-label {
  color: var(--danger) !important;
  font-weight: 700;
}

.danger-desc {
  font-size: 0.85rem;
  color: var(--text-muted);
  margin: 8px 0 14px;
  line-height: 1.5;
}

.btn-danger {
  background: var(--danger);
  color: white;
  border: none;
  border-radius: 8px;
  padding: 10px 20px;
  font-weight: 700;
  cursor: pointer;
  font-size: 0.9rem;
  transition: all 0.2s ease;
}

.btn-danger:hover {
  filter: brightness(1.1);
}

/* ─── Settings Reorganization ────────────────────────────────────── */
.settings-tabs {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}
.tab-btn {
  background: var(--bg-color);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 6px 12px;
  font-size: 0.82rem;
  cursor: pointer;
  color: var(--text-muted);
  transition: all 0.2s;
  white-space: nowrap;
}
.tab-btn:hover {
  border-color: var(--accent);
  color: var(--text);
}
.tab-btn.active {
  background: var(--accent);
  color: #fff;
  border-color: var(--accent);
}

.settings-content {
  display: flex;
  flex-direction: column;
  gap: 32px;
  padding: 0 4px;
}

.settings-section {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.settings-group {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 16px;
}

.settings-group-title {
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--text-color);
  opacity: 0.85;
  border-bottom: 1px solid var(--border-color);
  padding-bottom: 6px;
  margin-top: 8px;
  margin-bottom: 4px;
}

/* Categories & Badges */
.badge-select {
  appearance: none;
  border: none;
  border-radius: 8px;
  padding: 6px 12px;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
  width: 100%;
  max-width: 160px;
  text-align: left;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' fill='currentColor' viewBox='0 0 16 16'%3E%3Cpath d='M7.247 11.14 2.451 5.658C1.885 5.013 2.345 4 3.204 4h9.592a1 1 0 0 1 .753 1.659l-4.796 5.48a1 1 0 0 1-1.506 0z'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 10px center;
  border: 1px solid rgba(0, 0, 0, 0.05);
}

.badge-select:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

.badge-select.productive {
  background-color: rgba(16, 185, 129, 0.1);
  color: #059669;
  border-color: rgba(16, 185, 129, 0.2);
}

.badge-select.productive:hover:not(:disabled) {
  background-color: rgba(16, 185, 129, 0.15);
}

.badge-select.neutral {
  background-color: rgba(100, 116, 139, 0.1);
  color: #475569;
  border-color: rgba(100, 116, 139, 0.2);
}

.badge-select.neutral:hover:not(:disabled) {
  background-color: rgba(100, 116, 139, 0.15);
}

.badge-select.unproductive {
  background-color: rgba(239, 68, 68, 0.1);
  color: #dc2626;
  border-color: rgba(239, 68, 68, 0.2);
}

.badge-select.unproductive:hover:not(:disabled) {
  background-color: rgba(239, 68, 68, 0.15);
}

.categories-manage-table .app-row {
  align-items: center;
  padding: 12px 24px;
  border-bottom: 1px solid var(--border-color);
}

.categories-manage-table .app-row:last-child {
  border-bottom: none;
}

.app-name-cell {
  font-weight: 600;
  color: var(--text-color);
  font-size: 0.95rem;
  display: flex;
  align-items: center;
  gap: 12px;
}

.app-name-cell .app-icon {
  font-size: 1.2rem;
  background: var(--bg-color);
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
}

/* Productivity Table */
.table-responsive {
  width: 100%;
  overflow-x: auto;
  border-radius: 12px;
  border: 1px solid var(--border-color);
  background: var(--card-bg);
}

table {
  width: 100%;
  border-collapse: collapse;
  text-align: left;
}

th,
td {
  padding: 14px 20px;
  border-bottom: 1px solid var(--border-color);
  font-size: 0.9rem;
}

th {
  background: var(--bg-color);
  color: var(--text-muted);
  font-weight: 600;
  text-transform: uppercase;
  font-size: 0.75rem;
  letter-spacing: 0.05em;
}

tr:last-child td {
  border-bottom: none;
}

tr:hover td {
  background: rgba(99, 102, 241, 0.02);
}

.section-title {
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--text-color);
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 4px;
}

.admin-lock-hint {
  font-size: 0.75rem;
  background: rgba(99, 102, 241, 0.1);
  color: var(--accent);
  padding: 4px 10px;
  border-radius: 6px;
  font-weight: 600;
}

.setting-desc {
  font-size: 0.82rem;
  color: var(--text-muted);
  margin-bottom: 8px;
  display: block;
  line-height: 1.4;
}

.card-disabled {
  opacity: 0.7;
  cursor: not-allowed;
  border-style: dashed;
}

.lock-icon {
  font-size: 0.85rem;
  margin-left: 4px;
}

.checkbox-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 10px;
}

.status-toggle {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 8px;
  font-weight: 600;
}

.btn-action-outline {
  width: 100%;
  background: transparent;
  border: 1.5px solid var(--border-color);
  color: var(--text-color);
  padding: 10px;
  border-radius: 8px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
  margin-top: 4px;
}

.btn-action-outline:hover {
  background: var(--border-color);
  border-color: var(--text-muted);
}

.system-mode-container {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 20px;
  flex-wrap: wrap;
}

.mode-info {
  flex: 1;
}

.mode-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.danger-header {
  flex: 1;
}

.danger-card {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 20px;
  flex-wrap: wrap;
  padding: 24px;
}

.danger-card .danger-label {
  margin-bottom: 4px;
}

.danger-card .btn-danger {
  min-width: 160px;
}

/* Polishing Locked States */
.card-disabled label {
  color: var(--text-muted);
}

.lock-icon {
  opacity: 0.6;
  filter: grayscale(1);
}

.admin-lock-hint {
  animation: fadeIn 0.3s ease;
}

/* Section Header Polishing */
.section-title {
  border-left: 4px solid var(--accent);
  padding-left: 12px;
  height: 24px;
  line-height: 24px;
}

/* Better Select Styling */
select {
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' fill='%2364748b' viewBox='0 0 16 16'%3E%3Cpath d='M7.247 11.14 2.451 5.658C1.885 5.013 2.345 4 3.204 4h9.592a1 1 0 0 1 .753 1.659l-4.796 5.48a1 1 0 0 1-1.506 0z'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 12px center;
  padding-right: 36px !important;
}

/* ─── Admin Tabs ─────────────────────────────────────────────────── */
.admin-tabs {
  display: flex;
  gap: 4px;
  padding: 0 24px 16px;
  border-bottom: 1px solid var(--border-color);
  margin-bottom: 8px;
  align-items: center;
  flex-wrap: wrap;
}

.admin-tab {
  background: transparent;
  border: 1px solid var(--border-color);
  padding: 7px 16px;
  border-radius: 999px;
  font-size: 0.88rem;
  font-weight: 500;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.18s ease;
}

.admin-tab:hover {
  background: var(--border-color);
  color: var(--text-color);
}

.admin-tab-active {
  background: var(--accent) !important;
  color: white !important;
  border-color: var(--accent) !important;
}

/* ─── Drill Action Buttons ───────────────────────────────────────── */
.btn-drill {
  background: var(--border-color);
  border: none;
  border-radius: 6px;
  padding: 4px 8px;
  font-size: 1rem;
  cursor: pointer;
  transition: all 0.15s ease;
  color: var(--text-color);
}

.btn-drill:hover {
  background: var(--accent);
  color: white;
  transform: scale(1.08);
}

/* ─── Privacy Notice ────────────────────────────────── */
.privacy-notice {
  background: var(--accent);
  color: white;
  padding: 12px 20px;
  margin: 0 0 16px 0;
  border-radius: 8px;
  animation: slideDown 0.3s ease;
}

.privacy-content {
  display: flex;
  align-items: center;
  gap: 12px;
  position: relative;
  padding-right: 28px;
}

.privacy-icon {
  font-size: 1.2rem;
}

.privacy-text {
  flex: 1;
}

.privacy-text strong {
  display: block;
  font-size: 0.9rem;
  font-weight: 600;
}

.privacy-text p {
  margin: 4px 0 0 0;
  font-size: 0.8rem;
  opacity: 0.9;
}

.privacy-dismiss {
  position: absolute;
  top: 2px;
  right: 2px;
  width: 22px;
  height: 22px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.18);
  border: 1px solid rgba(255, 255, 255, 0.35);
  color: white;
  padding: 0;
  border-radius: 999px;
  font-size: 14px;
  line-height: 1;
  cursor: pointer;
  transition: all 0.15s ease;
}

.privacy-dismiss:hover {
  background: rgba(255, 255, 255, 0.3);
}

@keyframes slideDown {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* ─── Update Notification ─────────────────────────── */
.update-notification {
  background: var(--success);
  color: white;
  padding: 12px 20px;
  margin: 0 0 16px 0;
  border-radius: 8px;
  animation: slideDown 0.3s ease;
}

.update-content {
  display: flex;
  align-items: center;
  gap: 12px;
}

.update-icon {
  font-size: 1.2rem;
}

.update-text {
  flex: 1;
}

.update-text strong {
  display: block;
  font-size: 0.9rem;
  font-weight: 600;
}

.update-text p {
  margin: 4px 0 0 0;
  font-size: 0.8rem;
  opacity: 0.9;
}

.btn-update {
  background: rgba(255, 255, 255, 0.2);
  border: 1px solid rgba(255, 255, 255, 0.3);
  color: white;
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 0.8rem;
  cursor: pointer;
  text-decoration: none;
  transition: all 0.15s ease;
}

.btn-update:hover {
  background: rgba(255, 255, 255, 0.3);
}

.update-check {
  padding: 8px 20px;
  margin: 0 0 16px 0;
  font-size: 0.85rem;
  color: var(--text-muted);
}

.version-text {
  font-weight: 600;
  color: var(--text-color);
}

.btn-check-update {
  background: var(--border-color);
  border: none;
  padding: 4px 12px;
  border-radius: 6px;
  font-size: 0.8rem;
  cursor: pointer;
  margin-left: 8px;
  transition: all 0.15s ease;
}

.btn-check-update:hover {
  background: var(--accent);
  color: white;
}

.update-loading {
  padding: 8px 20px;
  margin: 0 0 16px 0;
  font-size: 0.85rem;
  color: var(--text-muted);
}

.update-error {
  padding: 8px 20px;
  margin: 0 0 16px 0;
  font-size: 0.85rem;
  color: var(--danger);
}

.update-check-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 8px 20px;
  margin: 0 0 16px 0;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  font-size: 0.85rem;
  gap: 12px;
}

/* ─── API Queue Indicator ─────────────────────────────────────────── */
.api-queue-indicator {
  padding: 8px 12px;
  margin: 4px 0;
  background: var(--bg-secondary);
  border-radius: 6px;
  font-size: 0.8rem;
}
.queue-info {
  display: flex;
  align-items: center;
  gap: 8px;
}
.queue-pending {
  color: var(--warning);
  font-weight: 600;
}
.queue-failed {
  color: var(--danger);
  font-weight: 600;
  cursor: pointer;
  text-decoration: underline;
}

/* ─── API Endpoint Rows ───────────────────────────────────────────── */
.api-endpoint-row input[type="text"],
.api-endpoint-row select {
  border: 1px solid var(--border-color);
  background: var(--card-bg);
  color: var(--text-color);
  border-radius: 4px;
  padding: 6px 8px;
}
.api-endpoint-row input[type="text"]:focus,
.api-endpoint-row select:focus {
  outline: none;
  border-color: var(--accent);
}
.api-group-header:hover {
  background: var(--card-bg) !important;
}
.api-headers input {
  border: 1px solid var(--border-color);
  background: var(--card-bg);
  color: var(--text-color);
  border-radius: 4px;
  padding: 4px 6px;
}

/* ─── Mode Toggle Switch ────────────────────────────────────────── */
.mode-toggle-switch {
  display: flex;
  gap: 0;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  overflow: hidden;
  background: var(--bg-color);
  width: fit-content;
}

.toggle-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 20px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-size: 0.9rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
  white-space: nowrap;
}

.toggle-btn:hover {
  background: rgba(99, 102, 241, 0.05);
  color: var(--text-color);
}

.toggle-btn.active {
  background: var(--accent);
  color: #fff;
  box-shadow: 0 2px 8px rgba(99, 102, 241, 0.3);
}

.toggle-btn.active .toggle-icon {
  filter: none;
}

.toggle-btn:first-child {
  border-radius: 9px 0 0 9px;
}

.toggle-btn:last-child {
  border-radius: 0 9px 9px 0;
}

.toggle-icon {
  font-size: 1rem;
  line-height: 1;
}

/* ─── Calendar View ──────────────────────────────────────────── */
.view-calendar {
  max-width: 1000px;
}

.calendar-layout {
  display: grid;
  grid-template-columns: 1fr 280px;
  gap: 20px;
  align-items: start;
}

.calendar-card {
  padding: 20px;
}

.calendar-weekdays {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 4px;
  margin-bottom: 8px;
  text-align: center;
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
}

.calendar-grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 4px;
}

.calendar-cell {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 8px 4px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s ease;
  min-height: 64px;
  border: 2px solid transparent;
  position: relative;
}

.calendar-cell:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  border-color: var(--accent);
}

.calendar-cell-today {
  border-color: var(--accent) !important;
  font-weight: 700;
}

.calendar-cell-selected {
  border-color: var(--warning) !important;
  box-shadow: 0 0 0 2px rgba(245, 158, 11, 0.3);
}

.calendar-day-num {
  font-size: 0.9rem;
  font-weight: 700;
  line-height: 1;
}

.calendar-day-bar {
  font-size: 0.6rem;
  line-height: 1;
  opacity: 0.8;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.calendar-detail-card {
  padding: 20px;
  position: sticky;
  top: 20px;
}

.calendar-detail-card h3 {
  font-size: 1rem;
  font-weight: 700;
  margin-bottom: 16px;
  text-align: center;
}

.calendar-detail-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 200px;
  color: var(--text-muted);
  text-align: center;
}

.calendar-detail-stats {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.detail-stat {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid var(--border-color);
}

.detail-stat:last-child {
  border-bottom: none;
}

.detail-stat-label {
  font-size: 0.85rem;
  color: var(--text-muted);
  font-weight: 600;
}

.detail-stat-value {
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--text-color);
}

.calendar-day-apps {
  border-top: 1px solid var(--border-color);
  margin-top: 12px;
  padding-top: 4px;
}

.detail-app-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 0;
  font-size: 0.85rem;
  border-bottom: 1px solid var(--border-color);
}

.detail-app-row:last-child {
  border-bottom: none;
}

.detail-app-time {
  font-weight: 600;
  color: var(--accent);
  font-variant-numeric: tabular-nums;
}

@media (max-width: 900px) {
  .calendar-layout {
    grid-template-columns: 1fr;
  }
  .calendar-detail-card {
    position: static;
  }
  .calendar-cell {
    min-height: 48px;
    padding: 4px 2px;
  }
  .calendar-day-num {
    font-size: 0.8rem;
  }
  .calendar-day-bar {
    font-size: 0.5rem;
  }
}

/* Merged Calendar & Productivity Layout - Premium Redesign */
.productivity-header-deck {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 16px;
  margin-bottom: 24px;
}
.productivity-header-deck .view-title {
  margin: 0;
  font-size: 1.8rem;
  font-weight: 800;
  letter-spacing: -0.025em;
  color: var(--text-color);
}
.productivity-header-deck .view-subtitle {
  margin: 4px 0 0 0;
  font-size: 0.95rem;
  color: var(--text-muted);
}
.productivity-header-deck .selected-date-highlight {
  color: var(--accent);
  font-weight: 700;
  background: rgba(99, 102, 241, 0.08);
  padding: 2px 8px;
  border-radius: 6px;
  font-variant-numeric: tabular-nums;
}
.filter-controls {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}
.modern-select {
  background-color: var(--card-bg);
  border: 1px solid var(--border-color);
  color: var(--text-color);
  padding: 8px 16px;
  border-radius: 8px;
  font-size: 0.9rem;
  font-weight: 600;
  cursor: pointer;
  outline: none;
  transition: all 0.2s;
}
.modern-select:hover {
  border-color: var(--accent);
}
.date-picker-group {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  padding: 4px 12px;
  border-radius: 8px;
}
.modern-date-input {
  background: transparent;
  border: none;
  color: var(--text-color);
  font-size: 0.85rem;
  font-weight: 600;
  outline: none;
}
.date-sep {
  color: var(--text-muted);
  font-size: 0.8rem;
}

.calendar-productivity-layout {
  display: flex;
  gap: 24px;
  align-items: start;
}
.calendar-productivity-layout.has-calendar {
  display: grid;
  grid-template-columns: 380px 1fr;
}

.calendar-sidebar-card {
  padding: 18px;
  border-radius: 18px;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  position: sticky;
  top: 24px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.06);
  overflow: hidden;
}

.calendar-sidebar-header {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 16px;
}
.calendar-sidebar-header .btn-today {
  width: 100%;
  padding: 9px;
  border-radius: 10px;
  font-weight: 600;
  font-size: 0.88rem;
  background: linear-gradient(135deg, rgba(99,102,241,0.08), rgba(99,102,241,0.04));
  border: 1px solid rgba(99,102,241,0.2);
  color: var(--accent);
  transition: all 0.2s;
}
.calendar-sidebar-header .btn-today:hover {
  background: linear-gradient(135deg, rgba(99,102,241,0.15), rgba(99,102,241,0.08));
  border-color: var(--accent);
}
.month-selector {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--bg-color);
  border-radius: 10px;
  padding: 4px 6px;
  border: 1px solid var(--border-color);
}
.month-selector .btn-nav {
  background: transparent;
  border: none;
  color: var(--text-color);
  cursor: pointer;
  padding: 5px 10px;
  font-size: 0.85rem;
  border-radius: 7px;
  transition: all 0.15s;
  line-height: 1;
}
.month-selector .btn-nav:hover {
  background: var(--border-color);
}
.month-selector .current-month {
  font-weight: 700;
  font-size: 0.92rem;
  color: var(--text-color);
  letter-spacing: -0.01em;
}

.calendar-weekdays-container {
  display: grid !important;
  grid-template-columns: repeat(7, 1fr) !important;
  text-align: center;
  font-size: 0.68rem;
  font-weight: 700;
  color: var(--text-muted);
  margin-bottom: 6px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.calendar-grid-container {
  display: grid !important;
  grid-template-columns: repeat(7, 1fr) !important;
  gap: 3px !important;
  width: 100% !important;
}
.calendar-sidebar-card .calendar-cell {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  aspect-ratio: 1;
  border-radius: 8px;
  padding: 2px 1px;
  cursor: pointer;
  border: 2px solid transparent;
  transition: all 0.18s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden;
  min-width: 0;
}
.calendar-sidebar-card .calendar-cell:not(.calendar-cell-empty):hover {
  background: rgba(99, 102, 241, 0.08) !important;
  border-color: rgba(99, 102, 241, 0.2) !important;
  transform: scale(1.05);
  z-index: 1;
}
.calendar-sidebar-card .calendar-cell-empty {
  cursor: default;
  background: transparent !important;
  border: none;
}
.calendar-sidebar-card .calendar-day-num {
  font-size: 0.78rem;
  font-weight: 700;
  line-height: 1.1;
}
.calendar-sidebar-card .calendar-day-bar {
  font-size: 0.5rem;
  font-weight: 600;
  opacity: 0.85;
  margin-top: 1px;
  line-height: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: clip;
  max-width: 100%;
  text-align: center;
}
.calendar-sidebar-card .calendar-cell-today {
  border: 2px solid var(--accent) !important;
  background: rgba(99, 102, 241, 0.06) !important;
  font-weight: 800;
}
.calendar-sidebar-card .calendar-cell-today .calendar-day-num {
  color: var(--accent);
}
.calendar-sidebar-card .calendar-cell-selected {
  background: var(--accent) !important;
  color: #ffffff !important;
  border-color: var(--accent) !important;
  box-shadow: 0 3px 10px rgba(99, 102, 241, 0.35);
  transform: scale(1.06);
  z-index: 2;
}
.calendar-sidebar-card .calendar-cell-selected .calendar-day-num {
  color: #ffffff !important;
}
.calendar-sidebar-card .calendar-cell-selected .calendar-day-bar {
  color: rgba(255, 255, 255, 0.9) !important;
}

.productivity-main-column {
  display: flex;
  flex-direction: column;
  gap: 24px;
  flex: 1;
}

/* Premium Metrics Cards */
.activity-stat-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 20px;
  border-radius: 16px;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
  overflow: hidden;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.02);
}
.activity-stat-card::before {
  content: "";
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 4px;
  transition: all 0.2s;
}
.activity-stat-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.06);
}

.total-time-card::before { background-color: var(--accent); }
.active-time-card::before { background-color: var(--success); }
.idle-time-card::before { background-color: var(--danger); }
.keyboard-card::before { background-color: #6366f1; }
.mouse-card::before { background-color: #a855f7; }

.stat-icon-wrapper {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border-radius: 12px;
  flex-shrink: 0;
}
.total-time-card .stat-icon-wrapper { background: rgba(99, 102, 241, 0.1); color: var(--accent); }
.active-time-card .stat-icon-wrapper { background: rgba(16, 185, 129, 0.1); color: var(--success); }
.idle-time-card .stat-icon-wrapper { background: rgba(239, 68, 68, 0.1); color: var(--danger); }
.keyboard-card .stat-icon-wrapper { background: rgba(99, 102, 241, 0.1); color: #6366f1; }
.mouse-card .stat-icon-wrapper { background: rgba(168, 85, 247, 0.1); color: #a855f7; }

.stat-icon {
  font-size: 1.3rem;
  font-weight: 700;
}
.stat-info {
  display: flex;
  flex-direction: column;
}
.stat-label {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.03em;
}
.stat-value {
  font-size: 1.4rem;
  font-weight: 700;
  color: var(--text-color);
  margin-top: 2px;
}
.stat-unit {
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--text-muted);
  margin-left: 2px;
}

/* Modernized Charts/Tables Grid */
.productivity-grid {
  display: grid;
  grid-template-columns: 1fr 1.6fr;
  gap: 24px;
}
.productivity-grid.single-column {
  grid-template-columns: 1fr !important;
}
.productivity-grid .card {
  padding: 24px;
  border-radius: 16px;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.02);
}
.card-title {
  margin: 0 0 20px 0;
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--text-color);
}
.chart-container {
  display: flex;
  flex-direction: column;
}
.chart-wrapper {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 250px;
}

/* Modern categories list card */
.modern-table {
  width: 100%;
  border-collapse: collapse;
  text-align: left;
}
.modern-table th {
  padding: 12px 16px;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  border-bottom: 2px solid var(--border-color);
  font-weight: 700;
}
.modern-table td {
  padding: 14px 16px;
  font-size: 0.9rem;
  border-bottom: 1px solid var(--border-color);
  color: var(--text-color);
}
.modern-table tr:last-child td {
  border-bottom: none;
}
.modern-table tbody tr {
  transition: background-color 0.15s;
}
.modern-table tbody tr:hover {
  background-color: rgba(99, 102, 241, 0.02);
}
.app-name-cell {
  font-weight: 600;
}
.time-spent-cell {
  font-variant-numeric: tabular-nums;
  color: var(--text-muted);
}

@media (max-width: 1180px) {
  .productivity-grid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 1050px) {
  .calendar-productivity-layout.has-calendar {
    grid-template-columns: 1fr;
  }
  .calendar-sidebar-card {
    position: static !important;
    top: 0 !important;
    width: 100% !important;
  }
}

/* Custom Premium Primary Button Styling (Bootstrap-like) */
.btn-primary {
  background: var(--accent) !important;
  color: white !important;
  border: none !important;
  border-radius: 8px !important;
  padding: 8px 16px !important;
  font-family: inherit !important;
  font-weight: 600 !important;
  cursor: pointer !important;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1) !important;
  display: inline-flex !important;
  align-items: center !important;
  justify-content: center !important;
  gap: 6px !important;
  outline: none !important;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.08) !important;
}

.btn-primary:hover:not(:disabled) {
  background: var(--accent-hover) !important;
  transform: translateY(-1px) !important;
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.25) !important;
}

.btn-primary:active:not(:disabled) {
  transform: translateY(0) !important;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.08) !important;
}

.btn-primary:disabled {
  opacity: 0.55 !important;
  cursor: not-allowed !important;
  transform: none !important;
  box-shadow: none !important;
}

/* ─── CSS Variable Aliases (new feature compat) ────────────────────────── */
:root {
  --border: var(--border-color);
  --bg-secondary: var(--bg-color);
  --text: var(--text-color);
  --primary: var(--accent);
}

.active-filter-btn {
  background: var(--accent) !important;
  color: white !important;
  border-color: var(--accent) !important;
}

/* ─── Utility Button Classes ────────────────────────────────────── */
.btn-icon {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 14px;
  padding: 4px 6px;
  border-radius: 6px;
  line-height: 1;
  transition: background 0.15s;
  color: var(--text-color);
}
.btn-icon:hover { background: var(--border-color); }

.btn-text {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 13px;
  padding: 4px 8px;
  border-radius: 6px;
  color: var(--text-muted);
  font-family: inherit;
  transition: color 0.15s, background 0.15s;
}
.btn-text:hover { color: var(--accent); background: rgba(99,102,241,0.06); }

.btn-cancel {
  background: none;
  border: 1px solid var(--border-color);
  cursor: pointer;
  font-size: 13px;
  padding: 5px 10px;
  border-radius: 7px;
  color: var(--text-muted);
  font-family: inherit;
  transition: all 0.15s;
}
.btn-cancel:hover { border-color: var(--danger); color: var(--danger); }

/* ─── Projects View ──────────────────────────────────────────────── */
.view-projects { padding: 0; }

.projects-header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.projects-count-badge {
  display: inline-flex;
  align-items: center;
  background: rgba(99,102,241,0.1);
  color: var(--accent);
  font-size: 12px;
  font-weight: 600;
  padding: 3px 10px;
  border-radius: 20px;
}

.btn-new-project {
  background: var(--accent);
  color: white;
  border: none;
  border-radius: 10px;
  padding: 9px 18px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  font-family: inherit;
  transition: all 0.2s cubic-bezier(0.4,0,0.2,1);
  box-shadow: 0 2px 8px rgba(99,102,241,0.25);
}
.btn-new-project:hover {
  background: var(--accent-hover);
  transform: translateY(-1px);
  box-shadow: 0 4px 14px rgba(99,102,241,0.35);
}

.project-create-form {
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 14px;
  padding: 16px 20px;
  margin-bottom: 20px;
  box-shadow: 0 4px 20px rgba(0,0,0,0.06);
  animation: slideDown 0.2s ease;
}
@keyframes slideDown {
  from { opacity: 0; transform: translateY(-8px); }
  to   { opacity: 1; transform: translateY(0); }
}
.project-form-inner { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
.form-color-preview { width: 14px; height: 14px; border-radius: 50%; flex-shrink: 0; transition: background 0.2s; box-shadow: 0 0 0 2px rgba(0,0,0,0.08); }
.project-name-input {
  flex: 1; min-width: 180px; padding: 9px 14px;
  border-radius: 8px; border: 1px solid var(--border-color);
  background: var(--bg-color); color: var(--text-color);
  font-size: 14px; font-family: inherit; outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}
.project-name-input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px rgba(99,102,241,0.12); }
.color-picker-label { cursor: pointer; font-size: 18px; padding: 4px 8px; border-radius: 8px; transition: background 0.15s; position: relative; }
.color-picker-label:hover { background: var(--border-color); }

/* Active Task Banner */
.active-task-banner {
  display: flex; align-items: center; gap: 12px;
  background: linear-gradient(135deg, rgba(34,197,94,0.08), rgba(16,185,129,0.06));
  border: 1px solid rgba(34,197,94,0.25); border-left: 4px solid #22c55e;
  border-radius: 12px; padding: 11px 16px; margin-bottom: 20px;
  animation: fadeIn 0.3s ease;
}
@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
.active-pulse {
  width: 10px; height: 10px; border-radius: 50%; background: #22c55e;
  animation: pulse 1.5s infinite; flex-shrink: 0;
}
.active-task-label { font-weight: 700; color: #22c55e; font-size: 13px; }
.active-task-name { font-weight: 600; color: var(--text-color); font-size: 14px; flex: 1; }
.btn-stop-task {
  background: rgba(239,68,68,0.1); color: #ef4444;
  border: 1px solid rgba(239,68,68,0.25); border-radius: 8px;
  padding: 5px 12px; font-size: 12px; font-weight: 600;
  cursor: pointer; font-family: inherit; transition: all 0.15s;
}
.btn-stop-task:hover { background: rgba(239,68,68,0.2); }

/* Projects Grid */
.projects-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 16px;
}

/* Premium Project Card */
.project-card-premium {
  background: var(--card-bg); border: 1px solid var(--border-color);
  border-radius: 14px; overflow: hidden;
  transition: all 0.2s cubic-bezier(0.4,0,0.2,1);
  box-shadow: 0 1px 4px rgba(0,0,0,0.04);
}
.project-card-premium:hover { border-color: var(--accent); box-shadow: 0 4px 16px rgba(99,102,241,0.12); }
.project-card-expanded { border-color: var(--accent); box-shadow: 0 4px 20px rgba(99,102,241,0.1) !important; }

.project-card-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 16px 20px; cursor: pointer; user-select: none; gap: 12px;
}
.project-card-identity { display: flex; align-items: center; gap: 12px; min-width: 0; flex: 1; }
.project-color-bar { width: 4px; height: 36px; border-radius: 4px; flex-shrink: 0; transition: width 0.2s; }
.project-card-premium:hover .project-color-bar, .project-card-expanded .project-color-bar { width: 5px; }
.project-card-info { min-width: 0; flex: 1; }
.project-card-name { margin: 0 0 2px; font-size: 16px; font-weight: 700; color: var(--text-color); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.project-card-meta { font-size: 12px; color: var(--text-muted); }
.project-card-actions { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
.project-action-btn {
  background: var(--bg-secondary); border: 1px solid var(--border-color); cursor: pointer; font-size: 13px;
  padding: 6px 10px; border-radius: 8px; line-height: 1;
  transition: all 0.15s; color: var(--text-color);
  display: inline-flex; align-items: center; justify-content: center; gap: 4px;
}
.project-action-btn:hover { background: var(--border-color); border-color: var(--text-muted); transform: translateY(-1px); }
.project-action-danger:hover { background: rgba(239,68,68,0.1) !important; border-color: rgba(239,68,68,0.4) !important; color: #ef4444; }
.project-chevron { font-size: 20px; color: var(--text-muted); transition: transform 0.25s cubic-bezier(0.4,0,0.2,1); display: inline-block; line-height: 1; margin-left: 4px; }
.chevron-open { transform: rotate(90deg); }

/* Edit form */
.project-edit-form {
  display: flex; align-items: center; gap: 8px; padding: 10px 16px;
  background: var(--bg-color); border-top: 1px solid var(--border-color); flex-wrap: wrap;
}

/* Tasks Panel */
.project-tasks-panel { border-top: 1px solid var(--border-color); padding: 16px 20px; background: rgba(99,102,241,0.01); }
.task-list { padding: 4px 0; }
.task-empty-state { padding: 14px 16px; text-align: center; color: var(--text-muted); font-size: 13px; font-style: italic; }
.task-row {
  display: flex; align-items: center; gap: 8px; padding: 12px 16px;
  transition: all 0.12s; border-left: 3px solid transparent;
  border-radius: 8px; border: 1px solid var(--border-color);
  background: var(--card-bg); margin-bottom: 8px;
}
.task-row:hover { background: rgba(99,102,241,0.02); border-color: var(--accent); }
.task-row-active { background: rgba(34,197,94,0.04) !important; border-color: rgba(34,197,94,0.3) !important; border-left-color: #22c55e !important; }
.task-row-done { opacity: 0.55; background: var(--bg-secondary) !important; }
.task-check-btn { background: none; border: none; cursor: pointer; font-size: 16px; padding: 0; line-height: 1; flex-shrink: 0; transition: transform 0.15s; }
.task-check-btn:hover { transform: scale(1.15); }
.task-name { flex: 1; font-size: 14px; color: var(--text-color); font-weight: 600; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.task-name-done { text-decoration: line-through; color: var(--text-muted); }
.task-row-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
.task-track-btn {
  background: rgba(34,197,94,0.1); color: #22c55e; border: 1px solid rgba(34,197,94,0.25);
  border-radius: 6px; padding: 4px 10px; font-size: 12px; font-weight: 600;
  cursor: pointer; font-family: inherit; transition: all 0.15s;
}
.task-track-btn:hover { background: rgba(34,197,94,0.2); transform: translateY(-1px); }
.task-stop-btn {
  background: rgba(239,68,68,0.1); color: #ef4444; border: 1px solid rgba(239,68,68,0.25);
  border-radius: 6px; padding: 4px 10px; font-size: 12px; font-weight: 600;
  cursor: pointer; font-family: inherit; transition: all 0.15s;
}
.task-stop-btn:hover { background: rgba(239,68,68,0.2); transform: translateY(-1px); }
.task-edit-input {
  flex: 1; padding: 6px 12px; border-radius: 6px;
  border: 1px solid var(--border-color); background: var(--bg-color);
  color: var(--text-color); font-size: 13px; font-family: inherit; outline: none;
}
.task-edit-input:focus { border-color: var(--accent); box-shadow: 0 0 0 2px rgba(99,102,241,0.12); }

/* Task Add Area */
.task-add-area {
  padding: 8px 0 0 0; border-top: 1px solid var(--border-color);
  display: flex; align-items: center; gap: 8px;
  background: transparent;
}
.task-add-input {
  flex: 1; padding: 8px 12px; border-radius: 8px;
  border: 1px solid var(--border-color); background: var(--bg-color);
  color: var(--text-color); font-size: 13px; font-family: inherit; outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}
.task-add-input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px rgba(99,102,241,0.12); }
.task-add-btn {
  background: var(--card-bg); border: 1px dashed var(--border-color); color: var(--accent);
  font-size: 13px; font-weight: 600; cursor: pointer; font-family: inherit;
  padding: 8px 16px; border-radius: 8px; width: 100%; transition: all 0.15s; text-align: center;
}
.task-add-btn:hover { background: rgba(99,102,241,0.04); border-color: var(--accent); }

/* Task Auto-Assignment Rules Panel */
.task-rules-panel {
  width: 100%;
  animation: slideDown 0.15s ease-out;
}

/* Empty State */
.projects-empty-state {
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  text-align: center; padding: 80px 24px; color: var(--text-muted); gap: 12px;
}
.empty-state-icon { font-size: 48px; margin-bottom: 4px; opacity: 0.6; }
.projects-empty-state h3 { font-size: 20px; font-weight: 700; color: var(--text-color); margin: 0; }
.projects-empty-state p { font-size: 14px; max-width: 360px; line-height: 1.6; margin: 0 0 8px; }

/* ─── Timesheet Styles ──────────────────────────────────────────── */
.timesheet-table { width: 100%; border-collapse: collapse; }
.timesheet-table th {
  border-bottom: 2px solid var(--border-color); font-size: 12px;
  text-transform: uppercase; letter-spacing: 0.5px; color: var(--text-muted); font-weight: 600;
}
.timesheet-table td { border-bottom: 1px solid var(--border-color); }
.timesheet-table tr:last-child td { border-bottom: none; }
.timesheet-table tr:hover td { background: var(--bg-color); }
</style>
