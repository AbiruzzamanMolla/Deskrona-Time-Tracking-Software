# 📝 Project Todo List: Time Guardian

## 🚀 Phase 1: MVP Setup (In Progress)
- [x] Setup Tauri + Rust + Vue 3 Scaffolding
- [x] Integrate SQLite (rusqlite)
- [x] Design & Initialize DB Schema (settings, users, sessions, time_logs, etc.)
- [x] Build basic background time tracking (Active window monitoring)
- [x] Implement default settings initialization (Language: en, Theme: system)
- [x] Setup Frontend i18n (vue-i18n)
- [x] Implement Light/Dark theme logic in Vue UI
- [x] Basic UI dashboard (daily summary view)

## ⚙️ Phase 1.5: Core Configurations & Settings
- [ ] Implement settings schema updates (auto_start, screenshot_location, backup config)
- [ ] Settings UI to configure application preferences
- [ ] Setup background auto-start on system boot logic

## 🔄 Phase 2: Enhanced Tracking
- [ ] Add idle detection (keyboard/mouse inactivity)
- [ ] Session tracking (manual start/stop work)

## 📸 Phase 3: Monitoring Features
- [ ] Screenshot capture (interval-based using settings.screenshot_interval)
- [ ] Custom screenshot save locations (using settings.screenshot_location)
- [ ] Store screenshot file paths in DB

## 💾 Phase 4: Backup & Data Integrity
- [ ] Local database backup system (cron/interval based)
- [ ] Customizable backup frequency (daily, weekly) and location
- [ ] Manual export/import of data

## 📊 Phase 5: Productivity & Reporting
- [ ] Productivity scoring logic
- [ ] Categorize apps (productive vs unproductive)
- [ ] Generate summary reports (charts and graphs in UI)

## 🔁 Phase 6: Synchronization & Polish
- [ ] Prepare sync-ready schema refinements
- [ ] Resolve conflict logic for future cloud sync
- [ ] Refine UI/UX for premium feel

## 🤖 Phase 7: AI Insights (Optional)
- [ ] AI-based time usage summary
- [ ] Local ML model integration for behavior patterns
