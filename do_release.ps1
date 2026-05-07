$ErrorActionPreference = "Stop"

# 1. Detect Version from package.json
if (!(Test-Path "package.json")) {
    Write-Error "package.json not found in current directory."
}
$package = Get-Content "package.json" -Raw | ConvertFrom-Json
$VERSION = $package.version
Write-Host "Detected Version: $VERSION" -ForegroundColor Cyan

# 2. Validate version consistency
Write-Host "Validating version consistency..." -ForegroundColor Yellow

# Check tauri.conf.json
if (Test-Path "src-tauri/tauri.conf.json") {
    $tauriConf = Get-Content "src-tauri/tauri.conf.json" -Raw | ConvertFrom-Json
    if ($tauriConf.version -ne $VERSION) {
        Write-Error "Version mismatch: package.json ($VERSION) vs src-tauri/tauri.conf.json ($($tauriConf.version))"
    }
}

# Check Cargo.toml
if (Test-Path "src-tauri/Cargo.toml") {
    $cargoToml = Get-Content "src-tauri/Cargo.toml" -Raw
    if ($cargoToml -notmatch "version = `"$VERSION`"") {
        Write-Error "Version mismatch: package.json ($VERSION) not found in src-tauri/Cargo.toml"
    }
}

Write-Host "Version validation passed." -ForegroundColor Green

# 3. Git Operations
Write-Host "Committing changes..." -ForegroundColor Yellow
git add .
# Check if there are any changes to commit
$status = git status --porcelain
if ($status) {
    git commit -m "chore: release v$VERSION"
    git push origin main
} else {
    Write-Host "No changes to commit." -ForegroundColor Gray
}

# 4. Handle Tags
Write-Host "Tagging..." -ForegroundColor Yellow
# Delete local tag if exists
if (git tag -l "v$VERSION") {
    Write-Host "Deleting existing local tag v$VERSION" -ForegroundColor Gray
    git tag -d "v$VERSION"
}
# Delete remote tag if exists
Write-Host "Attempting to delete remote tag v$VERSION (if exists)..." -ForegroundColor Gray
git push origin ":refs/tags/v$VERSION" 2>$null

git tag -a "v$VERSION" -m "Release v$VERSION"
git push origin "v$VERSION"

# 5. Build 64-bit
Write-Host "Building 64-bit..." -ForegroundColor Yellow
cmd /c npm run tauri build

# 6. Build 32-bit
Write-Host "Adding 32-bit target..." -ForegroundColor Yellow
rustup target add i686-pc-windows-msvc
Write-Host "Building 32-bit..." -ForegroundColor Yellow
cmd /c npm run tauri build -- --target i686-pc-windows-msvc

# 7. GitHub Release
Write-Host "Creating GitHub Release..." -ForegroundColor Yellow
# Delete existing release if it exists
try { 
    gh release delete "v$VERSION" --yes 2>$null 
} catch {
    Write-Host "No existing release to delete." -ForegroundColor Gray
}

# Locate assets using globs
$assets = @()
$assets += Get-ChildItem "src-tauri/target/release/bundle/msi/*.msi" | Where-Object { $_.Name -like "*$VERSION*" }
$assets += Get-ChildItem "src-tauri/target/release/bundle/nsis/*.exe" | Where-Object { $_.Name -like "*$VERSION*" }
$assets += Get-ChildItem "src-tauri/target/i686-pc-windows-msvc/release/bundle/msi/*.msi" | Where-Object { $_.Name -like "*$VERSION*" }
$assets += Get-ChildItem "src-tauri/target/i686-pc-windows-msvc/release/bundle/nsis/*.exe" | Where-Object { $_.Name -like "*$VERSION*" }

if ($assets.Count -eq 0) {
    Write-Error "No build assets found for version $VERSION."
}

$assetPaths = $assets.FullName
Write-Host "Uploading assets: $($assets.Name -join ', ')" -ForegroundColor Gray

gh release create "v$VERSION" `
  --title "Deskrona v$VERSION" `
  --notes-file "changelog.md" `
  $assetPaths

Write-Host "Release v$VERSION completed successfully!" -ForegroundColor Green
