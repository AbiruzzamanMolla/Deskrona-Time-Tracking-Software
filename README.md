# 🛡️ Deskrona

**Local-first time tracking & productivity monitoring — with optional remote sync.**  
Built with **Tauri 2**, **Rust**, and **Vue 3**. All data stays on your machine by default.  
Turn on **Online Mode** to sync to your own server.

[![GitHub Release](https://img.shields.io/github/v/release/AbiruzzamanMolla/Deskrona-Time-Tracking-Software)](https://github.com/AbiruzzamanMolla/Deskrona-Time-Tracking-Software/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%2064%2F32-blue)](https://github.com/AbiruzzamanMolla/Deskrona-Time-Tracking-Software/releases)
[![License](https://img.shields.io/github/license/AbiruzzamanMolla/Deskrona-Time-Tracking-Software)](LICENSE)

⬇️ [**Download Latest**](https://github.com/AbiruzzamanMolla/Deskrona-Time-Tracking-Software/releases) · [❤️ **Support Me**](https://www.supportkori.com/abiruzzaman)

---

## ✨ Features

### 🔒 Local-First Privacy
SQLite database on your machine. Zero network requests unless you enable Online Mode. No cloud, no telemetry, no accounts required.

### 📡 Configurable Online Mode
Toggle Online Mode in Settings → API Config. Every operation routes through **46 customizable API endpoints** — URL, HTTP method, headers, and bearer token are all configurable. Point it at any server.

```
┌─ Settings → API Config ──────────────────────────────────┐
│  Mode: [Offline] / [Online]                               │
│  Bearer Token: [••••••••••••••••••••]                     │
│                                                           │
│  ┌─ Auth (4) ───── [▼] ────────────────────────────────┐ │
│  │  ☑ auth_register  POST  [https://my-server.com/...] │ │
│  │  ☑ auth_login     POST  [https://my-server.com/...] │ │
│  │  ☑ auth_validate  GET   [https://my-server.com/...] │ │
│  │  ☑ auth_logout    POST  [https://my-server.com/...] │ │
│  └──────────────────────────────────────────────────────┘ │
│  ┌─ Sessions (3) ── [▼] ───────────────────────────────┐ │
│  │  ...                                                  │ │
│  └──────────────────────────────────────────────────────┘ │
│  (18 groups, 46 endpoints total)                          │
└───────────────────────────────────────────────────────────┘
```

Each endpoint entry includes the request body spec and response body spec — so you know exactly what your server needs to handle.

### 🔄 Auto Sync & Job Queue
When Online Mode is active, local data syncs to your server every 30 seconds:
- Time logs, URLs, activity events, session state, tracking status
- Background queue with **exponential backoff retry** (5 attempts)
- Queue status shown in sidebar (pending/failed count)
- Retry failed jobs from Settings → API Config
- Queue persists across app restarts (localStorage)

### 🔌 Smart Proxy Layer
Every operation routes through a proxy. Online mode tries the API first, falls back to local invoke. Offline mode calls local invoke directly. Works for:

| Category | Operations |
|----------|-----------|
| Auth | login, register, validate, logout |
| Tracking | start/stop session, set/get status |
| Data | time logs, URLs, screenshots, activity, input stats |
| Dashboard | today summary, filtered range |
| Admin | list/create users, stats, drill-down (screenshots, logs, activity, URLs, input stats) |
| Categories | get all, update |
| Settings | get, update |
| App Config | get, save |
| Backup | export, import |
| Pomodoro | start, skip, stop, status |
| Autostart | set, get |
| Reset | factory reset |

### ⏱ Real-Time Activity Tracking
Detects active windows, application names, and browser URLs. Tracks keyboard/mouse input for accurate active/idle time analysis.

### 📸 Screenshot Monitoring
Configurable interval screenshots with multi-monitor support. Full-screen preview. Disable anytime.

### 📊 Dashboard & Reports
Daily, weekly, monthly, yearly views with date filtering. Active time, idle time, keyboard/mouse count, productivity score, app breakdown, recent URLs.

### 👥 Multi-User Mode
Company/team mode with admin and employee roles. Admins manage users, view team productivity, control settings centrally.

### 🖥 Floating Overlay
Draggable timer overlay with pause/resume. Click-through mode — interact with apps underneath. Only visible when tracking is active/paused.

### 🍅 Pomodoro Timer
Built-in focus session timer. Auto-starts with tracking. Auto-switches: focus → short break → long break. Configurable durations. Sound notifications.

### 🌙 Themes & i18n
Dark/light/system themes. English and Bengali localization. System tray integration with live timer tooltip.

### 💾 Backup & Restore
Auto-backup configurable (daily/weekly/monthly). Manual export/import as ZIP. Includes: `tracker.db`, `screenshots/`, `api-config.json`.

---

## 🧱 Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | Vue 3, TypeScript, Vite, Chart.js |
| Backend | Rust, Tauri 2 |
| Database | SQLite (rusqlite) |
| i18n | vue-i18n |
| Tracking | active-win-pos-rs, device_query |
| API Config | JSON file in app data dir |
| Job Queue | localStorage + fetch API |

---

## 🔌 Setting Up Your Own Server

1. Open **Settings** → **API Config**
2. Switch mode to **Online**
3. Set your **Bearer Token** (if your server requires auth)
4. Expand each group and fill in your server URLs
5. Each endpoint shows the **request body spec** and **response body spec**
6. Enable the endpoints you need
7. Click **Sync Now** to test

The app always works locally — Online Mode only adds sync on top.

---

## 📦 Download

[GitHub Releases](https://github.com/AbiruzzamanMolla/Deskrona-Time-Tracking-Software/releases) — Windows 64-bit & 32-bit MSI/EXE installers.

---

## 🛠️ Build from Source

```bash
git clone https://github.com/AbiruzzamanMolla/Deskrona-Time-Tracking-Software.git
cd Deskrona-Time-Tracking-Software
npm install
npm run tauri dev     # dev mode
npm run tauri build   # production build
```

---

## 📄 License

MIT
