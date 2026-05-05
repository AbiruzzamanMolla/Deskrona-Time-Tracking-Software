# 📜 Changelog

All notable changes to **Time Guardian** will be documented in this file.

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
