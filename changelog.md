# Changelog

## 0.1.4 — Project Tracking & Tray Menu Improvements

### 🆕 System Tray Submenu
- Grouped all break reminder options (Pause, Resume, Reset) into a nested "Break Cycle" submenu in the tray menu.

### 💄 Projects View & Task Resuming
- Auto-resume tracking the last active task when starting tracking.
- Auto-expand the active task's project in the Projects view list.
- Stop button on tasks/banners now pauses/stops general tracking instead of clearing active task reference.

### 🐛 Layout & About View Polish
- Reordered settings tabs (API Config now before About).
- Removed duplicate developer card in About tab.


## 0.1.3 — Modernized Buttons & Peace Dove Icon

### 🆕 Peace Dove SVG Icon
- Replaced the meditation emoji (`🧘`) with a beautifully crafted inline SVG representing the flying peace dove with olive branch and pink hearts.
- Designed directly in HTML and dynamically set in the break type JS toggle, avoiding any local file asset dependencies.

### 💄 Premium Modern Primary Button Style
- Custom modernized `.btn-primary` class using system theme accent colors (`var(--accent)`).
- Premium micro-animations for hover scaling, active state feedback, and responsive box shadow transitions.

### 🐛 Layout & Break Reminder Tuning
- Resized settings card columns to fit standard grid layouts without full-width span.
- Modified Rust backend logic to suppress overlays during the pre-break warnings and only trigger on the active break phase.
- Fixed notification warning timers to apply consistently to both short and long breaks.

## 0.1.2 — Stretchly Integration & UI Improvements

### 🆕 Stretchly Break Reminder System
- Full Stretchly-like background break reminder daemon tracking active/idle work
- Interactive fullscreen overlay window triggered across all connected monitors
- Custom animations, floating particles, chime sounds, and customizable break ideas
- Settings to customize mini break duration, interval, long break duration, pre-break warnings, and postpone limits
- Full list of break API endpoints registered under API Config

### 💄 Modern UI Refinement
- Converted all Settings checkboxes to premium modern toggle switches with sliding transitions and theme-aware colors

### 🐛 Fixes & Backend Stability
- Fixed timer freeze on break overlay due to idle tracking detection (timer now counts down properly)
- Solved sqlite database lockups under high-frequency writes by setting 5-second connection busy timeouts
- Fixed zero-active-seconds reporting between midnight and 6:00 AM by correctly using local timezone query casting (`date(start_time, 'localtime')`)

## 0.1.1 — Calendar Day Detail & Bug Fix

### 🆕 Day Detail Panel
- Click a calendar day to show: top apps (up to 10), keyboard/mouse counts
- Auto-loads filtered dashboard data + input stats for selected day

### 🐛 Fixes
- Calendar showing blank day cells: SQL subquery alias `day` not visible in SQLite — replaced with `date(t.start_time)`
- Blank app screen on startup: `filterType` ref accidentally deleted during calendar code merge — restored

## 0.1.0 — Calendar View & Month Overview

### 🆕 Calendar View
- New Calendar view in sidebar with month grid visualization
- Color-coded days based on total active time (green intensity scale)
- Month navigation: prev, next, and "Today" quick jump
- Day detail panel shows: total active time, app count, screenshot availability
- Empty spacer cells for proper Mon-Sun grid alignment
- Loading state handling for async data fetching

### 🗄️ Backend
- `get_calendar_month()` SQL: per-day rollup of total seconds, distinct app count, screenshot existence
- `cmd_get_calendar_month` Tauri command registered in invoke handler
- `CalendarDayEntry` struct for serialized day data

### 🔗 API Layer
- `calendar_month` endpoint key added to endpoint config type
- `proxyGetCalendarMonth()` in apiProxy with online/fallback pattern
- Quick-fill URL for `/api/calendar/month`
- Endpoint documented in ENDPOINT_GROUPS spec

### 🌐 i18n
- 16 new English + Bengali keys: calendar, day names, detail labels, hints

## 0.0.9 — Online Mode & Configurable API Endpoints

### 🆕 Online Mode
- Added full online/offline mode toggle in Settings → API Config
- All 46 operations can be routed through remote API endpoints
- Endpoints fully customizable: URL, method, headers per operation
- Bearer token configurable in settings

### 📡 Configurable Endpoint System
- 46 individual endpoint configurations across 18 groups
- Each endpoint: enable toggle, HTTP method, URL, editable headers
- Request/response spec documented inline in settings UI
- Endpoints stored in standalone `api-config.json` (survives reset)

### 🔄 Job Queue
- All outgoing API requests go through queue system
- Persistent queue in localStorage (survives app restart)
- Auto-retry with exponential backoff (5 attempts)
- Queue status indicator in sidebar
- Manual retry failed / clear completed in settings

### 🔁 Background Data Sync
- Auto-syncs local data to remote server every 30s when online
- Syncs: time logs, URLs, activity events, session state, tracking status
- Tracks already-synced IDs — no duplicates
- Manual "Sync Now" button in API Config tab
- Last sync time display

### 🔌 API Proxy Layer
- All read/write operations route through proxy
- Online mode: tries API endpoint first, falls back to local invoke
- Offline mode: calls local Tauri invoke as before
- Auth, admin drill-down, dashboard, data viewing all proxied
- Settings, pomodoro, autostart, backup, sessions, tracking all proxied

### 💾 Backup Includes API Config
- Manual export/import ZIP includes `api-config.json`
- Auto-backup copies `api-config.json` alongside DB
- Config survives restore and factory reset scenarios

### 🖥️ First-Run Setup Improvements
- New API Config step in setup wizard (choose offline/online, enter server URL + token)
- Auto-fill all 43 endpoint URLs from a single server URL during wizard
- Login screen now renders properly (was missing template — multi-user mode non-functional before)
- Mode switch card added in Settings → General (single/multi-user toggle)
- Missing i18n keys added (idleMonitorMouse, idleMonitorKeyboard)
- Mode toggle redesigned as pill buttons with icons instead of radio inputs
- API Config mode now has explanation text describing offline vs online behavior

### 📝 Documentation
- Comprehensive `API.md` with all 43 endpoint specs: method, request body, response body, headers, query params, form fields
- Quick-fill default path table for server developers

### 🔧 Other
- New endpoint groups: Pomodoro (4), Autostart (2), Reset (1)
- Total 20 new files/3 modified since v0.0.8
- Rust + Vue/TypeScript — full stack update

## [0.0.8] - 2026-05-12
- Local-first time tracking
- Activity monitoring (keyboard, mouse, idle)
- Screenshot capture with configurable interval
- Browser URL tracking
- Dashboard with charts and productivity score
- Multi-user mode with admin/employee roles
- Pomodoro timer with auto-switching phases
- Floating overlay with click-through mode
- Backup & restore (ZIP export/import)
- Bengali + English i18n
- Dark/light/system themes
- Tauri 2 tray integration

### ✨ Added

- **Productivity Tab**: Added Total Time, Active Time, and Idle Time stat cards with date filter (daily/weekly/monthly/custom).
- **Overlay Pomodoro**: Timer shows remaining time in focus mode. "TODAY" label changes to "REMAINING" during focus, "BREAK" during break.

### 🐛 Fixed

- **Pomodoro Hang**: Fixed Mutex deadlock in background thread that caused app to freeze when pomodoro phase completed.
- **Refresh Buttons**: Standardized design across all tabs to match select box height and font size.

## [0.0.7] - 2026-05-12

### ✨ Added

- **Automatic Update Check**: App now automatically checks for updates on startup.
- **Pomodoro Timer**: Built-in pomodoro focus session system with auto-transition between focus, short break, and long break phases. Fully configurable durations in settings. Auto-starts with tracking. Countdown displayed in sidebar and overlay.

### 🐛 Fixed

- **Terminal Flash**: Suppressed console window in release builds (`windows_subsystem = "windows"`).
- **Click-through overlay**: Buttons and drag strip now remain interactive even in click-through mode.

### 🚀 Release

- Bumped version to 0.0.7.
- Fixed version discrepancy in UI.

## [0.0.6] - 2026-05-07

### ✨ Added

- **Taskbar Timer**: System tray tooltip now shows real-time tracking time (e.g., "Deskrona - Running: 01:23:45").
- **Overlay Window**: Optional compact floating timer that shows when tracking is active or paused. Features:
  - Draggable position anywhere on screen (default: top-right)
  - Single button: Pause (⏸) when running, Resume (▶) when paused
  - Only shows when tracking is active/paused, hidden when stopped
  - Icon buttons with hover titles
- **Overlay Settings**: New settings section in Settings view:
  - Enable/Disable overlay
  - Always on Top toggle
  - Click-through toggle (mouse clicks pass through to apps below)

### 🐛 Fixed

- **Taskbar Timer**: Fixed tray tooltip showing "00:00:00" - now properly shows elapsed tracking time.

## [0.0.5] - 2026-05-06

### ✨ Added

- **Tray Minimization**: App now hides to system tray when closing window instead of fully exiting.
- **Tray Menu Status**: Right-click tray menu displays current tracking status and elapsed session time (e.g., "Status: running (01:23:45)").
- **Sync-Ready Schema**: Added `deleted_at` columns to key tables (sessions, time_logs, app_usage, screenshots, activity_events) for future soft delete support.
- **Sync Metadata Table**: New table to track sync operations and enable conflict resolution for future cloud sync.
- **Premium UI Polish**: Added smooth transitions and refined visual elements throughout the application.
- **Privacy Notice**: In-app privacy notification that informs users when tracking is active and what data is being collected.
- **Update Notification**: Settings now shows update availability by checking GitHub releases, with download link for new versions.

### 🐛 Fixed

- **Single User Activity Tab**: Fixed activity tab showing empty in single user mode by using correct default user ID.

## [0.0.4] - 2026-05-06

### ✨ Added

- **Enterprise Multi-User Architecture**: Completed Phase 8 roadmap, enabling the application to support team environments.
- **Dual-Mode Support**: Introduced a "First Run" wizard to select between Single User (local) and Multi-User (enterprise) modes.
- **Hierarchical Identity System**: Implemented a multi-tenant schema with Company, Admin, and Employee roles.
- **Local Authentication**: Added a secure local login system with password hashing and session persistence.
- **Admin Dashboard**: Created a dedicated, role-restricted dashboard for admins to manage team members and view productivity statistics.
- **Real-Time Productivity Dashboard**: 1-second refresh cycle for immediate feedback on active windows and input metrics.
- **Enhanced Activity Tracking**: Individual counters for **Keyboard Hits** and **Mouse Movements** surfaced in user and admin views.
- **High-Fidelity Browser Tracking**: New Windows-native URL extraction leveraging UI Automation for Chrome, Edge, and Brave.
- **Enterprise Browser History**: Real-time URL capturing with immediate database insertion and administrative auditing.
- **Restricted Admin Controls**: In Multi-User mode, only admins can manage global application categories and tracking settings.
- **Improved Taskbar Status**: Enhanced taskbar title showing active time, current status, and real-time productivity score.
- **Standardized Local Timezone**: Migration of all tracking logic to `chrono::Local` to ensure consistent daily logs and reporting.
- **Data Isolation**: Updated the database engine to enforce strict data separation between users and companies.
- **Settings Expansion**: Added global tracking configuration (activity intervals, idle timeouts) persisted at the tenant level.
- **UI Tooltips**: Added hoverable tooltips for long URLs in the history and dashboard views.

### 🐛 Fixed

- **Rust Compilation**: Resolved `DateTime<Local> - DateTime<Utc>` subtraction errors in the tracking thread.
- **Browser History Display**: Fixed a rendering bug where history entries were not displaying due to missing unique keys.
- **Screenshot Inconsistency**: Standardized screenshot timestamps to local time for correct alignment in administrative views.
- **Screenshot Rendering**: Fixed long-standing issue with locally stored screenshots not displaying in the UI.
- **Duplicate History Logs**: Prevented redundant URL entries while ensuring real-time capture of browser activity.
- **Admin Data Isolation**: Reinforced multi-tenant boundaries to ensure admins only see data for the selected user.
- **Session Security**: Resolved a bug where administrative views remained accessible to employees after logout/login cycles.
- **Logout Stability**: Fixed a ReferenceError in the logout sequence that prevented the application from returning to the login screen.

## [0.0.3] - 2026-05-05

### ✨ Added

- **Software Mode Selection**: Choose between Personal and Enterprise modes on startup.
- **Improved Taskbar Integration**: Real-time timer and status in the taskbar title.
- **Automated Release Pipeline**: Support for 32-bit and 64-bit Windows installers.

## [0.0.1] - 2026-05-05

### ✨ Added

- Initial project scaffolding with Tauri, Rust, and Vue 3.
- SQLite integration for local-first data storage.
- **Advanced Monitoring**: Multi-monitor screenshot capture at user-defined intervals.
- **Privacy Controls**: Exclude specific URLs and applications from tracking.
- **Statistics Dashboard**: Daily, weekly, and monthly productivity breakdowns.
