<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useI18n } from "vue-i18n";

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
  const selected = await open({
    directory: true,
    multiple: false,
    title: t('message.screenshotLocation'),
    defaultPath: settings.value.screenshot_location || undefined,
  });
  if (selected && typeof selected === 'string') {
    settings.value.screenshot_location = selected;
  }
};

const selectBackupLocation = async () => {
  const selected = await open({
    directory: true,
    multiple: false,
    title: t('message.backupLocation'),
    defaultPath: settings.value.backup_location || undefined,
  });
  if (selected && typeof selected === 'string') {
    settings.value.backup_location = selected;
  }
};

watch(settings, () => {
  saveSettings();
}, { deep: true });

onMounted(() => {
  loadSettings();
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", applyTheme);
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
        <button 
          :class="{ active: currentView === 'dashboard' }" 
          @click="currentView = 'dashboard'"
        >
          📊 {{ t('message.dashboard') }}
        </button>
        <button 
          :class="{ active: currentView === 'settings' }" 
          @click="currentView = 'settings'"
        >
          ⚙️ {{ t('message.settings') }}
        </button>
      </nav>
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
            <p class="big-stat">0h 0m</p>
          </div>
          <div class="card premium-card">
            <h3>{{ t('message.idleTime') }}</h3>
            <p class="big-stat">0h 0m</p>
          </div>
        </div>

        <div class="top-apps">
          <h2>{{ t('message.topApps') }}</h2>
          <div class="empty-state">
            <p>Tracking has just started. Data will appear here soon.</p>
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
              <input type="text" v-model="settings.screenshot_location" placeholder="/default/path" />
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
        </div>
      </div>
    </main>
  </div>
</template>

<style>
:root {
  --bg-color: #f4f6f8;
  --sidebar-bg: #ffffff;
  --text-color: #1a1a2e;
  --card-bg: #ffffff;
  --border-color: #e2e8f0;
  --accent: #4f46e5;
  --accent-hover: #4338ca;
}

:root.dark {
  --bg-color: #0f1115;
  --sidebar-bg: #16181d;
  --text-color: #f1f5f9;
  --card-bg: #1e2128;
  --border-color: #2a2e37;
  --accent: #6366f1;
  --accent-hover: #818cf8;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  background-color: var(--bg-color);
  color: var(--text-color);
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
  transition: all 0.3s ease;
}

.app-layout {
  display: flex;
  height: 100vh;
}

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

.logo img {
  border-radius: 8px;
}

nav {
  display: flex;
  flex-direction: column;
  padding: 0 16px;
  gap: 8px;
}

nav button {
  background: transparent;
  border: none;
  padding: 12px 16px;
  border-radius: 8px;
  text-align: left;
  font-size: 1rem;
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

.main-content {
  flex: 1;
  padding: 40px;
  overflow-y: auto;
}

header h1 {
  margin-top: 0;
  font-size: 2rem;
  font-weight: 700;
}

.card {
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 24px;
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.05);
}

.summary-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 20px;
  margin-bottom: 40px;
}

.premium-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, var(--card-bg) 0%, rgba(99, 102, 241, 0.05) 100%);
  border: 1px solid var(--border-color);
}

.premium-card h3 {
  margin: 0 0 10px 0;
  font-size: 1rem;
  opacity: 0.8;
}

.big-stat {
  font-size: 2.5rem;
  font-weight: 800;
  color: var(--accent);
  margin: 0;
}

.top-apps {
  margin-top: 20px;
}

.empty-state {
  padding: 40px;
  text-align: center;
  background: var(--card-bg);
  border: 1px dashed var(--border-color);
  border-radius: 12px;
  color: var(--text-color);
  opacity: 0.7;
}

.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 20px;
}

.setting-card {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.setting-card label {
  font-weight: 600;
  font-size: 0.95rem;
}

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
  padding: 0 16px;
  cursor: pointer;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.2rem;
}

.btn-browse:hover {
  background: var(--border-color);
}
</style>