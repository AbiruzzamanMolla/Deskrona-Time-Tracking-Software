$ErrorActionPreference = "Stop"

$VERSION = "0.0.4"

Write-Host "Committing changes..."
git add .
git commit -m "chore: release v$VERSION"
git push origin main

Write-Host "Tagging..."
git tag -a "v$VERSION" -m "Release v$VERSION" -f
git push origin "v$VERSION" -f

Write-Host "Building 64-bit..."
cmd /c npm run tauri build

Write-Host "Adding 32-bit target..."
rustup target add i686-pc-windows-msvc

Write-Host "Building 32-bit..."
cmd /c npm run tauri build -- -t i686-pc-windows-msvc

Write-Host "Creating GitHub Release..."
try { gh.exe release delete "v$VERSION" --yes 2>$null } catch {}
gh.exe release create "v$VERSION" `
  --title "Time Guardian v$VERSION" `
  --notes-file "CHANGELOG.md" `
  (Get-Item "src-tauri/target/release/bundle/msi/*.msi").FullName `
  (Get-Item "src-tauri/target/release/bundle/nsis/*.exe").FullName `
  (Get-Item "src-tauri/target/i686-pc-windows-msvc/release/bundle/msi/*.msi").FullName `
  (Get-Item "src-tauri/target/i686-pc-windows-msvc/release/bundle/nsis/*.exe").FullName

Write-Host "Release Completed successfully!"
