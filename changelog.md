# 📜 Changelog

All notable changes to **Deskrona** will be documented in this file.

## [0.0.8] - 2026-05-12

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
