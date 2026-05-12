# 🛡️ Deskrona

**Local-first, privacy-focused time tracking & productivity monitoring.**  
Built with **Tauri**, **Rust**, and **Vue 3**. All data stays on your machine.

[![GitHub Release](https://img.shields.io/github/v/release/AbiruzzamanMolla/Deskrona-Time-Tracking-Software)](https://github.com/AbiruzzamanMolla/Deskrona-Time-Tracking-Software/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%2064%2F32-blue)](https://github.com/AbiruzzamanMolla/Deskrona-Time-Tracking-Software/releases)
[![License](https://img.shields.io/github/license/AbiruzzamanMolla/Deskrona-Time-Tracking-Software)](LICENSE)

⬇️ [**Download Latest**](https://github.com/AbiruzzamanMolla/Deskrona-Time-Tracking-Software/releases) · [❤️ **Support Me**](https://www.supportkori.com/abiruzzaman)

---

## ✨ Features

### 🔒 Local-First Privacy
All data stored in SQLite on your machine. No cloud sync, no data leaks. Zero external network requests for tracking data.

### ⏱ Real-Time Activity Tracking
Automatically detects active windows, application names, and browser URLs. Tracks keyboard and mouse input for accurate active/idle time analysis.

### 📸 Screenshot Monitoring
Configurable interval screenshots with multi-monitor support. Full-screen preview with click-to-view. Disable anytime from settings.

### 📊 Dashboard & Reports
Daily, weekly, monthly, yearly views with dynamic date filtering. Stats include active time, idle time, keyboard count, mouse count, and productivity score.

### 👥 Multi-User Mode
Company/team mode with admin and employee roles. Admins can manage users, view team productivity, and control settings centrally.

### 🖥 Floating Overlay
Compact draggable timer overlay with pause/resume button. Works in click-through mode — click past the timer to interact with apps underneath. Only visible when tracking is active or paused. Disable anytime from settings.

### 🍅 Pomodoro Timer
Built-in pomodoro focus session system. Auto-starts with tracking. Countdown timer in sidebar and overlay. Auto-switches between focus (25min), short break (5min), and long break (15min after 4 sessions). Fully configurable durations. Sound notifications on phase change.

### 🌙 Themes & i18n
Dark and light themes. English and Bengali localization. System tray integration with live timer tooltip.

### 💾 Backup & Export
Automated database backups with configurable frequency (daily/weekly/monthly). Manual export and import support for data portability.

## 🧱 Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | Vue 3, TypeScript, Vite |
| Backend | Rust, Tauri 2 |
| Database | SQLite (rusqlite) |
| Charts | Chart.js, vue-chartjs |
| i18n | vue-i18n |
| Tracking | active-win-pos-rs, device_query |

## 📦 Download

[GitHub Releases](https://github.com/AbiruzzamanMolla/Deskrona-Time-Tracking-Software/releases) — Windows 64-bit & 32-bit MSI/EXE installers.

## 🛠️ Build from Source

```bash
git clone https://github.com/AbiruzzamanMolla/Deskrona-Time-Tracking-Software.git
cd Deskrona-Time-Tracking-Software
npm install
npm run tauri dev     # dev mode
npm run tauri build   # production build
```

## 📄 License

MIT
