$ErrorActionPreference = "Stop"

$VERSION = "0.0.1"

Write-Host "Committing changes..."
git add .
git commit -m "chore: release v$VERSION"
git push origin main

Write-Host "Tagging..."
git tag -d "v$VERSION" 2>$null
git push origin :refs/tags/"v$VERSION" 2>$null
git tag -a "v$VERSION" -m "Release v$VERSION"
git push origin "v$VERSION"

Write-Host "Building 64-bit..."
npm run tauri build

Write-Host "Adding 32-bit target..."
rustup target add i686-pc-windows-msvc

Write-Host "Building 32-bit..."
npm run tauri build -- --target i686-pc-windows-msvc

Write-Host "Creating GitHub Release..."
gh release delete "v$VERSION" --yes 2>$null
gh release create "v$VERSION" `
  --title "Time Guardian v$VERSION" `
  --notes "Initial MVP Release with full tracking, dashboard, and screenshots." `
  "src-tauri/target/release/bundle/msi/time-guardian_$VERSION`_x64_en-US.msi" `
  "src-tauri/target/release/bundle/nsis/time-guardian_$VERSION`_x64-setup.exe" `
  "src-tauri/target/i686-pc-windows-msvc/release/bundle/msi/time-guardian_$VERSION`_x86_en-US.msi" `
  "src-tauri/target/i686-pc-windows-msvc/release/bundle/nsis/time-guardian_$VERSION`_x86-setup.exe"

Write-Host "Release Completed successfully!"
