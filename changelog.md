# 📜 Changelog

All notable changes to **Time Guardian** will be documented in this file.

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
