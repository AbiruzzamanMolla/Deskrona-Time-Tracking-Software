# 🛡️ Time Guardian

**Time Guardian** is a local-first, privacy-focused, cross-platform time tracking and productivity monitoring application. Built with **Tauri**, **Rust**, and **Vue 3**, it ensures all your data stays on your machine while providing a premium, high-performance experience.

## 🚀 Key Features

- **Local-First Architecture**: Your data never leaves your computer unless you explicitly choose to sync it.
- **Cross-Platform**: Seamlessly works on Windows, macOS, and Linux.
- **Background Tracking**: Automatically monitors active applications and window titles to track your work accurately.
- **Privacy-Centric**: Fully offline functionality by default.
- **Multilingual Support**: Available in multiple languages (English and Bengali currently supported).
- **Dark/Light Mode**: Full theme support to match your system preferences.

## 🧱 Tech Stack

- **Frontend**: Vue 3, TypeScript, Vite
- **Backend**: Rust, Tauri
- **Database**: SQLite (via `rusqlite`)
- **i18n**: `vue-i18n`

## 🛠️ Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (latest LTS)
- [Rust](https://www.rust-lang.org/)
- [Tauri CLI](https://tauri.app/v1/guides/getting-started/prerequisites)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/your-repo/worktracker.git
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Run in development mode:
   ```bash
   npm run tauri dev
   ```

## 📝 Project Structure

- `src/`: Frontend Vue 3 application logic.
- `src-tauri/`: Rust backend, database management, and tracking logic.
- `project.md`: Detailed project architecture and roadmap.
- `todo.md`: Active task tracking.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
