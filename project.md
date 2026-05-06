# 🧠 Project Instruction: Build a Local-First Cross-Platform Time Tracking & Monitoring App

## 🎯 Goal

Rebuild a desktop application similar to WorkComposer with a **local-first architecture** that works on:

- Linux
- macOS
- Windows

The app must function **fully offline**, storing all data locally, with a **future-ready sync system** for cloud integration.

---

## 🧱 Core Architecture

### 1. Desktop Application (Main App)

- Framework: **Tauri**
- Backend (core logic): **Rust**
- Frontend (UI): **Vue 3**

### 2. Local Database

- Database: **SQLite**
- ORM/Driver:
  - `sqlx` (recommended) or `rusqlite`

### 3. Background Service

- Language: **Rust**
- Runs independently or within Tauri backend
- Handles:
  - Activity tracking
  - Idle detection
  - App usage logging
  - Screenshot capture

---

## 📦 Features (MVP → Advanced)

### ✅ MVP (Phase 1)

- Track active window/app
- Track time spent per app
- Store logs in SQLite
- Basic UI dashboard (daily summary)

### 🔄 Phase 2

- Idle detection (keyboard/mouse inactivity)
- Session tracking (start/stop work)
- Background auto-start on system boot

### 📸 Phase 3

- Screenshot capture (interval-based)
- Store images locally (`/data/screenshots/`)
- Save file paths in DB (DO NOT store blobs)

### 📊 Phase 4

- Productivity scoring (rule-based)
- Categorize apps (productive/unproductive)
- Generate summaries

### 🤖 Phase 5 (Optional AI)

- AI-based insights:
  - Time usage summary
  - Behavior patterns
- Use:
  - Local ML model OR
  - External API (future)

---

## 🗄️ Database Design (SQLite)

### Tables:

#### users

- id (UUID)
- name
- created_at
- updated_at

#### sessions

- id (UUID)
- user_id
- start_time
- end_time
- created_at
- updated_at
- synced_at (nullable)

#### time_logs

- id (UUID)
- user_id
- app_name
- window_title
- start_time
- end_time
- duration
- created_at
- updated_at
- synced_at

#### app_usage

- id (UUID)
- user_id
- app_name
- total_time
- date
- created_at
- updated_at
- synced_at

#### screenshots

- id (UUID)
- user_id
- file_path
- captured_at
- created_at
- synced_at

#### activity_events

- id (UUID)
- type (mouse, keyboard, idle)
- timestamp
- created_at
- synced_at

---

## ⚙️ System-Level Capabilities

### Required:

- Active window tracking
- App usage detection
- Idle detection
- Screenshot capture

### Rust Libraries:

- `sysinfo` → system/app info
- `device_query` → keyboard/mouse activity
- `screenshots` → capture screen

### OS-Specific Notes:

#### macOS:

- Requires:
  - Screen Recording permission
  - Accessibility permission

#### Linux:

- X11: works fine
- Wayland: restricted (limited screenshot & tracking)

#### Windows:

- Use WinAPI bindings

---

## 🔁 Future Sync System (IMPORTANT)

Design database with sync in mind:

### Add to ALL tables:

- `id` → UUID (NOT auto-increment)
- `created_at`
- `updated_at`
- `deleted_at` (optional soft delete)
- `synced_at` (nullable)

### Sync Strategy:

- Push unsynced records → server
- Pull updates → local DB
- Resolve conflicts using timestamps

---

## 📁 Project Structure

/app /src (frontend UI) /src-tauri /core tracking.rs idle.rs screenshot.rs /db schema.sql queries.rs main.rs /data app.db /screenshots

---

## 🔐 Security & Privacy

- Always inform users about tracking
- Request OS-level permissions properly
- Avoid storing sensitive data in plain text
- Optionally encrypt:
  - User data
  - Screenshot metadata

---

## 🚀 Development Plan

### Step 1:

- Setup Tauri + Rust + UI
- Integrate SQLite
- Build basic time tracking

### Step 2:

- Add background tracking service
- Store logs continuously

### Step 3:

- Build dashboard UI

### Step 4:

- Add idle detection

### Step 5:

- Add screenshots

### Step 6:

- Prepare sync-ready schema

---

## ❌ Avoid These Mistakes

- Do NOT store screenshots in DB (use file paths)
- Do NOT use auto-increment IDs (use UUID)
- Do NOT tightly couple UI with tracking logic
- Do NOT start with AI — add later

---

## ✅ Final Output Expectations

- Cross-platform desktop app
- Fully offline functionality
- Efficient local database
- Modular architecture
- Ready for future cloud sync

---

## 🧠 Summary

Build a **lightweight, privacy-focused, offline-first desktop tracker** using:

- Tauri + Rust + Vue
- SQLite for storage
- Background tracking service
- Future-ready sync design

Focus on **core tracking first**, then scale features gradually.
