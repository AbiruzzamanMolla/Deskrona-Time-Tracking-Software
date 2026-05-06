# 📜 Changelog

All notable changes to **Time Guardian** will be documented in this file.

## [0.0.4-draft] - 2026-05-06

### ✨ Added
- **Enterprise Multi-User Architecture**: Completed Phase 8 roadmap, enabling the application to support team environments.
- **Dual-Mode Support**: Introduced a "First Run" wizard to select between **Single User** (local) and **Multi-User** (enterprise) modes.
- **Hierarchical Identity System**: Implemented a multi-tenant schema with Company, Admin, and Employee roles.
- **Local Authentication**: Added a secure local login system with password hashing and session persistence.
- **Admin Dashboard — Team Productivity**: Admin can now view every team member's active time and session count in a single table.
- **Admin Dashboard — Per-User Drill-Down**: Admins can inspect any team member's **Screenshots**, **Time Logs**, and **Activity** records by clicking drill-down action buttons (📸 ⏱ ⌨️) directly from the productivity table.
- **Admin Dashboard — Date Picker**: All drill-down views are scoped to a selected date (defaults to today), with a date picker in the admin header.
- **Admin Dashboard — Tab Navigation**: Tabbed UI (Team / Screenshots / Time Logs / Activity) with breadcrumb showing which user is being viewed.
- **Activity Monitoring — Keyboard & Mouse Separately**: Background tracker now logs `keyboard` and `mouse` events as distinct activity_events rows, enabling separate analysis.
- **Idle Timeout Setting**: New "Idle Timeout (minutes)" field in Settings — configures when the tracker marks a period as idle if no keyboard or mouse input is detected. Default: 5 minutes.
- **Dynamic Idle Threshold**: The idle threshold is reloaded from the database every 60 seconds by the background tracking thread, so changes take effect without a restart.
- **Data Isolation**: Updated the database engine to enforce strict data separation between users and companies.
- **Admin User Management**: Admins can add new team members with username, display name, password, and role assignment.

### 🐛 Fixed
- **Screenshot Rendering**: Fixed long-standing issue with locally stored screenshots not displaying in the UI.
- **Session Security**: Resolved a bug where administrative views remained accessible to employees after logout/login cycles.
- **Logout Stability**: Fixed a ReferenceError in the logout sequence that prevented the application from returning to the login screen.
- **TypeScript Strict Mode**: Eliminated TS2367 type overlap error in admin drill-down tab guard.

## [0.0.3] - 2026-05-05

### ✨ Added
- **Pagination**: Added pagination (Load More) functionality to All Trackings, Browser History, and Screenshots views to optimize database queries.
- **Enhanced Tracking List**: The "All Trackings" view now displays both start and end times for better session clarity.
- **System Tray Menu**: Added a taskbar icon with right-click menu operations to quickly Start, Pause, or Stop tracking.
- **Dynamic Taskbar Title**: The window title now dynamically updates to show active session time and current clock time, visible in the OS taskbar.
- **Break Time Tracking**: Pausing the tracker now automatically records the paused duration as "Break Time" in the time logs.
- **Productivity Scoring**: Added logic to categorize and score daily time blocks.
- **App Categorization**: Users can now assign apps into 'Productive', 'Unproductive', or 'Neutral' categories.
- **Visual Reports**: Built a new "Productivity" view featuring dynamic Chart.js donut graphs to visualize work efficiency.

### 🐛 Fixed
- **Screenshot Visibility**: Resolved an issue where locally stored screenshots were not correctly rendering in the UI.

## [0.0.2] - 2026-05-05

### ✨ Added
- **Backup & Data Integrity**: Local database backup system with customizable interval (daily, weekly) and location.
- **Manual Data Export/Import**: Added user-friendly settings UI to manually export and import `.db` SQLite files.
- Completed Phase 4 roadmap goals.

## [0.0.1] - 2026-05-05

### ✨ Added
- Initial project scaffolding with Tauri, Rust, and Vue 3.
- SQLite integration for local-first data storage.
- **Advanced Monitoring**: Multi-monitor screenshot capture at user-defined intervals.
- **Activity Reporting**: Specialized UI pages for viewing All Trackings, Browser History, and Screenshots.
- **Dynamic Filtering**: Robust date-range filters (Daily, Weekly, Monthly, Yearly, Custom) across all reporting views.
- **Idle Detection**: Automatic detection and logging of keyboard/mouse inactivity.
- **Browser Tracking**: Intelligent extraction of visited URLs from browser window titles.
- **Status-Aware Tracking**: Support for Start, Stop, and Pause states with database-level status logging.
- **Release Automation**: Integrated `do_release.ps1` script for building and pushing 32/64-bit installers to GitHub.
- Multi-language support (English and Bengali) using `vue-i18n`.
- Light/Dark/System theme switching with database persistence.
- Comprehensive `project.md` and `todo.md` for roadmap tracking.
- MIT License and updated README.

### 🔧 Fixed
- Resolved `vue-tsc` build errors caused by unused imports in `App.vue`.
- Fixed Tauri v2 capability configuration errors (Corrected `dialog:default` and removed invalid `fs` permissions).
- Improved database migration logic for adding status columns to existing tables.
