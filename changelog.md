# Changelog

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

## 0.0.8 — Initial Release
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
