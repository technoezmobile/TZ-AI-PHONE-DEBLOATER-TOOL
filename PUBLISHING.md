# Publishing & Release Guide (Owner Operations)

This document provides exact, copy-ready instructions for publishing the **TZ AI DEBLOATER TOOL** repository to GitHub, compiling missing macOS and Linux binaries via native GitHub Actions runners, generating full checksums, creating the official `v1.1.0` release tag, attaching release artifacts, and deploying the static website.

---

## 1. Local Git State & Release Candidate Commit

The repository has been initialized on branch `main` and all 163 audited files (GUI, Rust core, CLI, icons, UAD catalog, tests, documentation, site) have been committed:

- **Release-Candidate Commit**: `c19e513050ece77d0170b6f6c0527b41814b10c7` (Short: `c19e513`)
- **Commit Message**: `feat(release): cut v1.1.0 release candidate`

To verify the working tree:
```bash
git status
git log -n 1 --oneline
```

---

## 2. Linking GitHub Remote & Pushing Main

Because macOS and Linux compilation require native toolchains (`hdiutil` for macOS DMGs, `libwebkit2gtk-4.1-dev` and `patchelf` for Linux AppImage/DEB) which do not exist on the Windows workstation, push the repository to GitHub so the manual build workflow can compile the native packages on GitHub-hosted runners.

### Option A: Create Repository via GitHub Web UI
1. Navigate to [github.com/new](https://github.com/new).
2. Name the repository: `tz-ai-debloater-tool` (under `tz-labs` or your personal user account).
3. Set visibility to **Private** (if staging before release) or **Public**. Do **not** initialize with a README, .gitignore, or license (these already exist locally).
4. Run in `E:\AI DEBLOATER`:
   ```bash
   git remote add origin https://github.com/tz-labs/tz-ai-debloater-tool.git
   git branch -M main
   git push -u origin main
   ```

### Option B: Create Repository via GitHub CLI (`gh`)
```bash
gh auth login
gh repo create tz-labs/tz-ai-debloater-tool --public --source=. --remote=origin --push
```

---

## 3. Compiling macOS & Linux Artifacts via GitHub Actions

A dedicated build workflow [`.github/workflows/build-artifacts.yml`](./.github/workflows/build-artifacts.yml) is provided. It compiles all 4 target platforms independently of any release tag and stores downloadable CI artifacts.

### Matrix Runners:
- **Windows x64**: `windows-latest`
- **macOS Apple Silicon**: `macos-latest` (Target: `aarch64-apple-darwin`)
- **macOS Intel**: `macos-13` (Target: `x86_64-apple-darwin`)
- **Linux x64**: `ubuntu-22.04` (Packages: AppImage, `.deb`, and CLI tarball)

### Triggering the Workflow:
1. On GitHub, navigate to **Actions** > **Build CI Artifacts (Manual)**.
2. Click **Run workflow**.
3. Select branch: `main`, build profile: `release`, and click **Run workflow**.
4. Once completed (approx. 8–12 minutes), scroll down to the **Artifacts** section on the workflow run page:
   - `tz-debloater-windows-x64`
   - `tz-debloater-macos-aarch64`
   - `tz-debloater-macos-x64`
   - `tz-debloater-linux-x64`

### Collecting Artifacts into `release_artifacts/`:
1. Download each artifact zip from the run summary.
2. Extract the files into `E:\AI DEBLOATER\release_artifacts\github-release\v1.1.0\`:
   - `TZ-AI-Debloater-Tool-v1.1.0-aarch64.dmg`
   - `tz-debloater-v1.1.0-macos-aarch64.tar.gz`
   - `TZ-AI-Debloater-Tool-v1.1.0-x64.dmg`
   - `tz-debloater-v1.1.0-macos-x64.tar.gz`
   - `TZ-AI-Debloater-Tool-v1.1.0.AppImage`
   - `tz-ai-debloater_1.1.0_amd64.deb`
   - `tz-debloater-v1.1.0-linux-x64.tar.gz`
3. Regenerate the authoritative checksum file:
   ```powershell
   cd "E:\AI DEBLOATER\release_artifacts\github-release\v1.1.0"
   Get-ChildItem -File -Exclude "SHA256SUMS.txt" | ForEach-Object {
       $hash = (Get-FileHash -Path $_.FullName -Algorithm SHA256).Hash.ToLower()
       "$hash  $($_.Name)"
   } | Set-Content -Encoding utf8 SHA256SUMS.txt
   ```

---

## 4. Multi-Platform Smoke Testing Checklist

### Windows 10/11 x64 (Already Verified Locally)
- [x] NSIS Setup installs cleanly, creates Start/Desktop shortcuts, uninstalls with zero leftovers.
- [x] WiX MSI installs silently via `msiexec /i ... /quiet /qn`.
- [x] Portable EXE launches standalone without administrative rights.
- [x] Standalone CLI `tz-debloater doctor` returns exit code `0`.
- [x] Tested with real Xiaomi Poco X3 Pro (MIUI Global) and mock ADB suite (81/81 Rust tests pass).

### macOS (Apple Silicon & Intel)
- [ ] Open DMG, drag `TZ AI DEBLOATER TOOL.app` into `/Applications`.
- [ ] Clear quarantine attribute if Gatekeeper blocks launch:
  ```bash
  xattr -cr "/Applications/TZ AI DEBLOATER TOOL.app"
  ```
- [ ] Launch application; verify that window renders with fluid styling and menu items are responsive.
- [ ] Test standalone CLI:
  ```bash
  tar -xzvf tz-debloater-v1.1.0-macos-*.tar.gz
  ./tz-debloater doctor
  ```

### Linux (Ubuntu 22.04+ / Debian 12+)
- [ ] Make AppImage executable and launch:
  ```bash
  chmod +x TZ-AI-Debloater-Tool-v1.1.0.AppImage
  ./TZ-AI-Debloater-Tool-v1.1.0.AppImage
  ```
- [ ] Install Debian package:
  ```bash
  sudo dpkg -i tz-ai-debloater_1.1.0_amd64.deb
  sudo apt-get install -f
  ```
- [ ] Test CLI binary:
  ```bash
  tar -xzvf tz-debloater-v1.1.0-linux-x64.tar.gz
  ./tz-debloater doctor
  ```

---

## 5. Creating & Pushing the Official Release Tag

Once all artifacts are collected and verified:

```bash
# Ensure local repo has latest commits
git add .
git commit -m "docs: finalize v1.1.0 multi-platform release artifacts and verification matrix"
git push origin main

# Create annotated release tag
git tag -a v1.1.0 -m "Release v1.1.0: First public GitHub release"

# Push tag to GitHub (triggers .github/workflows/release.yml)
git push origin v1.1.0
```

---

## 6. Uploading Assets to GitHub Releases

### Option A: Using GitHub Web Interface
1. Go to `https://github.com/tz-labs/tz-ai-debloater-tool/releases`.
2. Click **Draft a new release**.
3. Choose existing tag **`v1.1.0`**.
4. Set Title: **`TZ AI DEBLOATER TOOL v1.1.0`**.
5. Copy release notes from `CHANGELOG.md` into description.
6. Drag and drop all 12 files from `release_artifacts/github-release/v1.1.0/` into the upload zone:
   - `TZ-AI-Debloater-Tool-v1.1.0-Setup.exe`
   - `TZ-AI-Debloater-Tool-v1.1.0-x64.msi`
   - `TZ-AI-Debloater-Tool-v1.1.0-portable.exe`
   - `tz-debloater-v1.1.0-windows-x64.zip`
   - `TZ-AI-Debloater-Tool-v1.1.0-aarch64.dmg`
   - `tz-debloater-v1.1.0-macos-aarch64.tar.gz`
   - `TZ-AI-Debloater-Tool-v1.1.0-x64.dmg`
   - `tz-debloater-v1.1.0-macos-x64.tar.gz`
   - `TZ-AI-Debloater-Tool-v1.1.0.AppImage`
   - `tz-ai-debloater_1.1.0_amd64.deb`
   - `tz-debloater-v1.1.0-linux-x64.tar.gz`
   - `SHA256SUMS.txt`
7. Click **Publish release**.

### Option B: Using GitHub CLI (`gh`)
```bash
gh release create v1.1.0 \
  release_artifacts/github-release/v1.1.0/* \
  --title "TZ AI DEBLOATER TOOL v1.1.0" \
  --notes-file CHANGELOG.md \
  --latest
```

---

## 7. Deploying the Static Documentation & Download Site (`site/`)

The `site/` directory contains the production-ready landing page, responsive download buttons, OS detection, interactive CLI demo, and complete documentation.

### Option A: GitHub Pages via Subtree (Zero Build Required)
```bash
# Push site directory to gh-pages branch
git subtree push --prefix site origin gh-pages
```
In your GitHub repo:
1. Navigate to **Settings > Pages**.
2. Select Source: **Deploy from a branch**.
3. Choose Branch: `gh-pages` / `/ (root)`.
4. Click **Save**.

### Option B: Cloudflare Pages
1. Connect repo to Cloudflare Pages.
2. Root directory: `site`.
3. Build command: None (pure static).
4. Output directory: `.`.
5. Deploy.
