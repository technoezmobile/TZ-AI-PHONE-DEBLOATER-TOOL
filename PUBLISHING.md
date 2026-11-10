# Publishing & Release Guide (Owner Operations)

This document provides exact, copy-ready instructions for publishing the **TZ AI DEBLOATER TOOL** repository to GitHub, creating the official `v1.1.0` release tag, attaching release artifacts, and deploying the static website.

---

## 1. Initializing Git & Committing Audited Source

From the project root (`E:\AI DEBLOATER`):

```bash
# 1. Verify working tree is clean and unneeded binaries are ignored
git status

# 2. Add all audited source files, tests, documentation, and static site files
git add .

# 3. Commit the public release candidate
git commit -m "feat(release): cut v1.1.0 first public release

- Add standalone tz-debloater CLI with 11 commands and POSIX exit codes
- Add cross-platform ADB discovery and OS keyring integrations
- Add cross-process advisory locks with stale lock recovery
- Implement 3-dimensional package classification and default candidates view
- Add 4 distinct AI assistant shortcuts and preflight validation
- Complete multi-platform CI/CD and comprehensive docs"
```

---

## 2. Linking GitHub Remote & Pushing Main

If you haven't linked a GitHub remote yet:

### Option A: Using GitHub CLI (`gh`)
```bash
# Authenticate if needed
gh auth login

# Create public repository under your organization or personal account
gh repo create tz-labs/tz-ai-debloater-tool --public --source=. --remote=origin --push
```

### Option B: Using Standard Git
```bash
# Add your repository URL
git remote add origin https://github.com/tz-labs/tz-ai-debloater-tool.git

# Set main branch and push
git branch -M main
git push -u origin main
```

---

## 3. Creating & Pushing the Official Release Tag

```bash
# Create annotated tag for v1.1.0
git tag -a v1.1.0 -m "Release v1.1.0: First public GitHub release"

# Push tag to GitHub (triggers .github/workflows/release.yml)
git push origin v1.1.0
```

---

## 4. Uploading Release Binaries to GitHub Releases

The release bundles are pre-compiled and placed in `release_artifacts/github-release/v1.1.0/`:

| File Name | Description |
| :--- | :--- |
| **`TZ-AI-Debloater-Tool-v1.1.0-Setup.exe`** | Windows NSIS desktop installer |
| **`TZ-AI-Debloater-Tool-v1.1.0-x64.msi`** | Windows Enterprise WiX MSI installer |
| **`TZ-AI-Debloater-Tool-v1.1.0-portable.exe`** | Windows zero-install standalone executable |
| **`tz-debloater-v1.1.0-windows-x64.zip`** | Standalone Windows x64 terminal CLI |
| **`SHA256SUMS.txt`** | Authoritative SHA-256 checksums |

### Upload via GitHub CLI (`gh`)
```bash
gh release create v1.1.0 \
  release_artifacts/github-release/v1.1.0/* \
  --title "TZ AI DEBLOATER TOOL v1.1.0" \
  --notes-file CHANGELOG.md \
  --latest
```

### Upload via GitHub Web Interface
1. Go to `https://github.com/tz-labs/tz-ai-debloater-tool/releases`.
2. Click **Draft a new release**.
3. Select tag **`v1.1.0`**.
4. Set Title: **`TZ AI DEBLOATER TOOL v1.1.0`**.
5. Copy the `v1.1.0` section from `CHANGELOG.md` into the release notes.
6. Drag and drop all 5 files from `release_artifacts/github-release/v1.1.0/` into the upload box.
7. Click **Publish release**.

---

## 5. Deploying the Website Preview (`site/`)

The `site/` folder contains a standalone, zero-dependency static documentation and download site verified against all 20 web checklist standards.

### Option A: GitHub Pages (Recommended)
1. In your GitHub repository, navigate to **Settings > Pages**.
2. Under **Build and deployment > Source**, select **Deploy from a branch**.
3. Create an orphan `gh-pages` branch containing the contents of `site/`:
   ```bash
   # Push the site directory to gh-pages branch
   git subtree push --prefix site origin gh-pages
   ```
4. On GitHub, set branch to `gh-pages` and folder to `/ (root)`. Click **Save**.
5. Your site is live at `https://tz-labs.github.io/tz-ai-debloater-tool/` (or your custom domain `https://tzlabs.in`).

### Option B: Cloudflare Pages / Vercel
1. Connect the GitHub repository to Cloudflare Pages or Vercel.
2. Set **Root directory** to `site`.
3. Set **Build command** to none (pure static HTML).
4. Set **Output directory** to `.` (current directory).
5. Deploy.

---

## 6. Post-Release Verification Checklist

- [ ] Repository is set to Public on GitHub.
- [ ] Release tag `v1.1.0` is visible on GitHub Releases with all 5 binary assets attached.
- [ ] SHA-256 checksums match `SHA256SUMS.txt`.
- [ ] GitHub Actions CI workflow runs cleanly on all platforms.
- [ ] Documentation links in `README.md` navigate properly.
- [ ] Website preview loads cleanly with 0 broken links and correct meta tags.

