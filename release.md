# Release Process Automation

This guide outlines the steps required to build, tag, and publish new versions of Deskrona. It covers building the `.exe` and `.msi` for 64-bit platforms (and standard cross-compilation for 32-bit if configured), as well as pushing to GitHub Releases using the GitHub CLI (`gh`).

## 1. Prerequisites

- **Rust Toolchain:** Ensure `rustup` is installed.
- **Node.js:** Ensure `npm` is available.
- **Tauri CLI:** Installed via `npm install -g @tauri-apps/cli`.
- **GitHub CLI (`gh`):** Installed and authenticated (`gh auth login`).
- **Wix Toolset (for MSI):** Make sure WiX is installed to generate `.msi` files (Tauri uses this under the hood).

## 2. Pre-Release Checklist

Before building, ensure you have updated the necessary files:

- [ ] Update `version` in `package.json`.
- [ ] Update `version` in `src-tauri/tauri.conf.json`.
- [ ] Update `version` in `src-tauri/Cargo.toml`.
- [ ] Update `README.md` if new features were added.
- [ ] Add a new section to `CHANGELOG.md` detailing the new version's features, fixes, and changes.

## 3. Build Commands

### Clean the workspace

First, make sure the workspace is clean and dependencies are up to date:

```bash
npm install
cd src-tauri
cargo clean
cd ..
```

### Build 64-bit Windows (Default)

This builds the standard 64-bit `.exe` and `.msi` installers:

```bash
npm run tauri build
```

_(The output artifacts will be placed in `src-tauri/target/release/bundle/msi` and `src-tauri/target/release/bundle/nsis`.)_

### Build 32-bit Windows

To build for 32-bit Windows, you need the correct Rust target installed:

```bash
rustup target add i686-pc-windows-msvc
npm run tauri build -- --target i686-pc-windows-msvc
```

## 4. Tagging and Releasing via GitHub CLI

Once the builds are complete, follow these steps to push the release to GitHub:

### Step A: Commit and Push Code

```bash
git add .
git commit -m "chore: release v0.0.1"
git push origin main
```

### Step B: Create a Git Tag

```bash
git tag -a v0.0.1 -m "Release v0.0.1"
git push origin v0.0.1
```

### Step C: Create the GitHub Release

Use the `gh release create` command to generate the release and upload the built binaries simultaneously.

**Command Example:**

```bash
gh release create v0.0.1 \
  --title "Deskrona v0.0.1" \
  --notes-file CHANGELOG.md \
  "src-tauri/target/release/bundle/msi/deskrona_0.0.1_x64_en-US.msi" \
  "src-tauri/target/release/bundle/nsis/deskrona_0.0.1_x64-setup.exe"
```

## Automating with a Script

You can wrap the above into a PowerShell script (`release.ps1`):

```powershell
$VERSION = "0.0.1"

Write-Host "Building 64-bit..."
npm run tauri build

Write-Host "Committing code..."
git add .
git commit -m "chore: release v$VERSION"
git push origin main

Write-Host "Tagging..."
git tag -a "v$VERSION" -m "Release v$VERSION"
git push origin "v$VERSION"

Write-Host "Creating GitHub Release..."
gh release create "v$VERSION" `
  --title "Deskrona v$VERSION" `
  --notes "Initial Release" `
  "src-tauri/target/release/bundle/msi/deskrona_$VERSION`_x64_en-US.msi" `
  "src-tauri/target/release/bundle/nsis/deskrona_$VERSION`_x64-setup.exe"

Write-Host "Release Completed successfully!"
```
