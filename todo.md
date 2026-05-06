# 📝 Project Todo List: Deskrona

## 🚀 Phase 1: MVP Setup (In Progress)

- [x] Setup Tauri + Rust + Vue 3 Scaffolding
- [x] Integrate SQLite (rusqlite)
- [x] Design & Initialize DB Schema (settings, users, sessions, time_logs, etc.)
- [x] Build basic background time tracking (Active window monitoring)
- [x] Build Time Tracking logic properly. store app name, app times.
- [x] Show Time Trackings in UI like (Dashboard, Top Apps, Total Time, etc.)
- [x] Make a session based tracking. start a session, stop session, active session, no session.
- [x] Show All trackings in UI.
- [x] Implement default settings initialization (Language: en, Theme: system)
- [x] Setup Frontend i18n (vue-i18n)
- [x] Implement Light/Dark theme logic in Vue UI
- [x] Basic UI dashboard (daily summary view)
- [x] Add Menu item and seperate pages to show All Trackings, with filter of daily, weekly, monthly, yearly and custom range.
- [x] Add Menu item and seperate pages to show All Browser Visited Urls, with filter of daily, weekly, monthly, yearly and custom range.
- [x] Add Menu item to show ScreenShots with date range.
- [x] Make a system for updating screenshot interval and change it in ui also. And save screenshots at the time of app starts in the location where app is started.
- [x] take screenshot when tracking is running and save in database and show in ui also. stop when tracking is stopped. take snapshot of all screens.
- [x] Make a system for updating browser visited urls and save in database and show in ui also.
- [x] Make a system for updating app usage and save in database and show in ui also.
- [x] Make a system for updating session and save in database and show in ui also.
- [x] add status based url tracking and app tracking based on settings. active, in-active, idle, pause etc.

## ⚙️ Phase 1.5: Core Configurations & Settings

- [x] Implement settings schema updates (auto_start, screenshot_location, backup config)
- [x] Settings UI to configure application preferences
- [x] Setup background auto-start on system boot logic

## 🔄 Phase 2: Enhanced Tracking

- [x] Add idle detection (keyboard/mouse inactivity)
- [x] Session tracking (manual start/stop work)
- [x] Show idle time in UI
- [x] Show session time in UI
- [x] Show total time in UI
- [x] Show time spent on each app in UI
- [x] Show app name, time, statistice etc in UI
- [x] Captucher visited urls also and show them in UI also.

## 📸 Phase 3: Monitoring Features

- [x] Screenshot capture (interval-based using settings.screenshot_interval)
- [x] Custom screenshot save locations (using settings.screenshot_location)
- [x] Store screenshot file paths in DB
- [x] Show Screenshots in UI Based on Time and Date Range

## 💾 Phase 4: Backup & Data Integrity

- [x] Local database backup system (cron/interval based)
- [x] Customizable backup frequency (daily, weekly) and location
- [x] Manual export/import of data

## 📊 Phase 5: Productivity & Reporting

- [x] Productivity scoring logic
- [x] Categorize apps (productive vs unproductive)
- [x] Generate summary reports (charts and graphs in UI)

## 🔁 Phase 6: Synchronization & Polish

- [x] Prepare sync-ready schema refinements (Add `deleted_at` for soft deletes)
- [x] Resolve conflict logic for future cloud sync
- [x] Fix Screenshot view in ui issue.
- [ ] Implement multi-platform permission handling (macOS Screen Recording/Accessibility)
- [x] Refine UI/UX for premium feel

## 🔁 Phase 6.5: Taskbar Operations

- [x] Add operations on taskbar icon (start, stop, pause)
- [x] Show active session time on taskbar
- [x] Show current time in taskbar
- [x] Update logics and UI to calcuatle pause as break time.

## 🤖 Phase 7: AI Insights (Optional)

- [ ] AI-based time usage summary
- [ ] Local ML model integration for behavior patterns

## 🏢 Phase 8: Enterprise & Multi-User Architecture (Dual Mode Support)

- [x] **Dual-Mode Initialization**: Implement a "First Run" wizard that allows users to choose between **Single User** (legacy local-only system) and **Multi-User** (enterprise/team system) modes. Store selection in a permanent config file.
- [x] **Hierarchical Identity System**: Implement multi-tenant schema for Multi-User mode where a `Company` is the root entity. Admins are registered as standard users with an elevated `admin` role.
- [x] **Local Authentication & Security**: Develop a local login system for Multi-User mode using password hashing (Argon2/PBKDF2) and secure session token persistence.
- [x] **Multi-Tenant Data Isolation**: Update the database engine to require `company_id` and `user_id` for Multi-User entries, while maintaining compatibility with legacy Single User data.
- [x] **Administrative Privileges & UI**: Create a role-restricted Admin Dashboard for Multi-User mode to manage company users and access aggregated productivity statistics.
- [x] **Role-Based Frontend Logic**: Implement UI state management that toggles management features for Admins vs Employees in Multi-User mode, while keeping the standard interface for Single User mode.

## 🔐 Phase 9: Security & Privacy

- [x] Add Privacy Notice/In-app notification about tracking active state
- [ ] Implement OS-level permission request flows for macOS/Linux
- [ ] Explore local database encryption for sensitive metadata (Optional)
