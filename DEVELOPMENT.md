# Development Guide

Complete guide for setting up, building, and contributing to Armbian Imager.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Prerequisites](#prerequisites)
3. [Step-by-Step Setup](#step-by-step-setup)
4. [Development Workflow](#development-workflow)
5. [Building for Distribution](#building-for-distribution)
6. [Project Structure](#project-structure)
7. [Architecture Deep Dive](#architecture-deep-dive)
8. [Tech Stack](#tech-stack)
9. [Data Sources](#data-sources)
10. [Quality Checks](#quality-checks)
11. [Troubleshooting](#troubleshooting)

---

## Quick Start

```bash
git clone https://github.com/armbian/imager.git && cd imager
bash scripts/setup/install.sh
npm install
npm run tauri:dev
```

---

## Prerequisites

| Requirement | Minimum | Link |
|-------------|---------|------|
| Node.js | 20.19.0 | [nodejs.org](https://nodejs.org) |
| Rust | 1.77.2 | [rustup.rs](https://rustup.rs) |
| npm | 10+ | Included with Node.js |

### Platform-Specific

**Linux:** `libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev`

**macOS:** Xcode Command Line Tools

**Windows:** Visual Studio Build Tools 2022 + WebView2 Runtime

---

## Step-by-Step Setup

### 1. Clone Repository

```bash
git clone https://github.com/armbian/imager.git
cd imager
```

### 2. Install System Dependencies

**Automated (Recommended):**
```bash
bash scripts/setup/install.sh
```

### 3. Verify Prerequisites

```bash
node --version    # >= 20.19.0
rustc --version   # >= 1.77.2
```

### 4. Install & Run

```bash
npm install
npm run tauri:dev
```

---

## Development Workflow

### Available Scripts

| Command | Description |
|---------|-------------|
| `npm run dev` | Frontend only (Vite dev server) |
| `npm run tauri:dev` | Full app with hot reload (frontend + Rust) |
| `npm run build` | Production frontend build |
| `npm run build:dev` | Development frontend build |
| `npm run tauri:build` | Production distributable |
| `npm run tauri:build:dev` | Debug build with symbols |
| `npm run lint` | Run ESLint |
| `npm run clean` | Clean all artifacts (node_modules, dist, target) |

### Daily Workflow

1. `npm run tauri:dev` - Start dev server
2. Edit [`src/`](src/) or [`src-tauri/src/`](src-tauri/src/) - Auto reload
3. Test changes
4. Run quality checks before committing (see [Quality Checks](#quality-checks))

---

## Building for Distribution

### Single Platform

```bash
./scripts/build/build-macos.sh      # macOS universal (ARM64 + x64)
./scripts/build/build-linux.sh      # Linux (x64 + ARM64 via Docker)
npm run tauri:build                  # Current platform
```

### All Platforms

```bash
./scripts/build/build-all.sh
```

### Build Options

```bash
./scripts/build/build-macos.sh --clean        # Clean build
./scripts/build/build-macos.sh --dev          # Debug symbols
./scripts/build/build-linux.sh --x64          # Linux x64 only
./scripts/build/build-linux.sh --arm64        # Linux ARM64 only
./scripts/build/build-all.sh --macos --linux  # Specific platforms
```

### Output

| Platform | Format | Location |
|----------|--------|----------|
| macOS | .dmg, .app | `src-tauri/target/{arch}/release/bundle/` |
| Linux | .deb, .AppImage | `src-tauri/target/{arch}/release/bundle/` |
| Windows | .msi, .nsis | `src-tauri/target/{arch}/release/bundle/` |

---

## Project Structure

```
armbian-imager/
├── src/                              # React 19 Frontend
│   ├── App.tsx                       # Main app + selection state machine
│   ├── main.tsx                      # Entry point (i18n, theme, mount)
│   ├── i18n.ts                       # i18n setup with dynamic locale loading
│   │
│   ├── components/
│   │   ├── flash/                    # Flash progress UI
│   │   │   ├── FlashProgress.tsx     # Presentation (uses useFlashOperation hook)
│   │   │   ├── FlashActions.tsx      # Cancel/retry/done buttons
│   │   │   └── FlashStageIcon.tsx    # Stage icons + i18n keys
│   │   ├── modals/                   # 4-step selection wizard
│   │   │   ├── Modal.tsx             # Base modal (animations, exit handling)
│   │   │   ├── ManufacturerModal.tsx # Step 1: Vendor selection
│   │   │   ├── BoardModal.tsx        # Step 2: Board selection (image grid)
│   │   │   ├── ImageModal.tsx        # Step 3: OS image selection (filters)
│   │   │   ├── DeviceModal.tsx       # Step 4: Device selection (polling)
│   │   │   └── ArmbianBoardModal.tsx # Auto-detect when running on Armbian
│   │   ├── settings/                 # 5-tab settings modal
│   │   │   ├── SettingsModal.tsx     # Container with sidebar navigation
│   │   │   ├── AppearanceSection.tsx # Theme + language
│   │   │   ├── PreferencesSection.tsx# MOTD, skip verify, board detection
│   │   │   ├── StorageSection.tsx    # Cache management
│   │   │   ├── DeveloperSection.tsx  # Dev mode + logs viewer
│   │   │   ├── AboutSection.tsx      # Version, credits, links
│   │   │   ├── CacheManagerModal.tsx # Cached images browser with delete
│   │   │   └── LogsModal.tsx         # Log viewer + paste.armbian.com upload
│   │   ├── layout/
│   │   │   ├── Header.tsx            # App header with step indicators
│   │   │   └── HomePage.tsx          # Main selection buttons / flash view
│   │   └── shared/                   # Reusable UI components
│   │       ├── BoardBadges.tsx       # Support level badges
│   │       ├── ConfirmationDialog.tsx# Data loss / unstable image warnings
│   │       ├── ErrorDisplay.tsx      # Error with retry + log upload
│   │       ├── MarqueeText.tsx       # Scrolling overflow text
│   │       ├── MotdTip.tsx           # Rotating tips from Armbian API
│   │       ├── SearchBox.tsx         # Filter input for modals
│   │       ├── SkeletonCard.tsx      # Placeholder loaders
│   │       ├── Toast.tsx             # Success/error notifications
│   │       ├── UpdateModal.tsx       # App update dialog
│   │       └── ChangelogModal.tsx    # Release notes display
│   │
│   ├── hooks/                        # Custom React Hooks
│   │   ├── useTauri.ts              # 26+ Tauri IPC command wrappers
│   │   ├── useAsyncData.ts          # Generic async fetch (race-condition safe)
│   │   ├── useFlashOperation.ts     # Full flash lifecycle orchestration
│   │   ├── useVendorLogos.ts        # Logo preloading + manufacturer grouping
│   │   ├── useSettings.ts           # Tauri Store get/set (20+ settings)
│   │   ├── useSettingsGroup.ts      # Batch parallel settings loader
│   │   ├── useSkeletonLoading.ts    # Min-duration skeleton display
│   │   ├── useModalExitAnimation.ts # Exit animation with double-trigger guard
│   │   ├── useDeviceMonitor.ts      # Device connection polling
│   │   └── useToasts.tsx            # Global toast notification context
│   │
│   ├── contexts/
│   │   └── ThemeContext.tsx          # Light/dark/auto with system preference
│   │
│   ├── config/
│   │   ├── constants.ts             # Polling, timing, cache, UI, settings keys
│   │   ├── badges.ts                # Desktop env + kernel branch badge colors
│   │   ├── os-info.ts               # OS logos, app logos, release mappings
│   │   ├── deviceColors.ts          # Color scheme per device type
│   │   └── i18n.ts                  # 18 supported languages + metadata
│   │
│   ├── styles/                       # CSS with design tokens
│   │   ├── theme.css                # Custom properties (colors, spacing, radius)
│   │   ├── base.css                 # Reset, scrollbar, spinner, states
│   │   ├── layout.css               # Page layout and containers
│   │   ├── components.css           # Buttons, badges, cards, inputs
│   │   ├── modal.css                # Modal animations and settings UI
│   │   ├── flash.css                # Progress bar, stage icons, errors
│   │   └── responsive.css           # Breakpoints (600-1400px)
│   │
│   ├── types/index.ts               # BoardInfo, ImageInfo, BlockDevice, etc.
│   ├── utils/index.ts               # formatFileSize, parseArmbianFilename, etc.
│   ├── utils/deviceUtils.ts         # isDeviceConnected, getDeviceType
│   ├── locales/                     # 18 language JSON files
│   └── assets/                      # Logos (Armbian, OS distros)
│
├── src-tauri/                        # Rust Backend (Tauri 2)
│   ├── src/
│   │   ├── main.rs                  # App setup, plugin init, command registration
│   │   ├── download.rs              # HTTP streaming + SHA256 + mirror logging
│   │   ├── decompress.rs            # XZ (multi-threaded), GZ, BZ2, ZST
│   │   ├── cache.rs                 # LRU cache with configurable size limits
│   │   │
│   │   ├── commands/                # 54 Tauri IPC commands
│   │   │   ├── board_queries.rs     # get_boards, get_images_for_board, get_block_devices
│   │   │   ├── operations.rs        # download_image, flash_image, delete, cleanup
│   │   │   ├── progress.rs          # get_download/flash_progress, cancel_operation
│   │   │   ├── custom_image.rs      # select, decompress, detect board from filename
│   │   │   ├── scraping.rs          # get_board_image_url
│   │   │   ├── settings.rs          # 25+ get/set commands (theme, cache, etc.)
│   │   │   ├── system.rs            # open_url, locale, frontend logging, armbian detect
│   │   │   ├── update.rs            # get_github_release, is_app_in_applications
│   │   │   └── state.rs             # AppState (cached JSON, download/flash state)
│   │   │
│   │   ├── devices/                 # Platform-specific device detection
│   │   │   ├── types.rs             # BlockDevice struct, normalize_bus_type, detect_sd
│   │   │   ├── linux.rs             # lsblk JSON + sysfs read-only check
│   │   │   ├── macos.rs             # DiskArbitration framework (~50ms, APFS filtering)
│   │   │   └── windows.rs           # Win32 IOCTL (PhysicalDrive0-31)
│   │   │
│   │   ├── flash/                   # Platform-specific flash operations
│   │   │   ├── verify.rs            # Shared byte-by-byte verification
│   │   │   ├── linux/
│   │   │   │   ├── writer.rs        # UDisks2 device open + direct I/O
│   │   │   │   └── privileges.rs    # polkit authorization
│   │   │   ├── macos/
│   │   │   │   ├── writer.rs        # authopen + /dev/rdisk raw writes
│   │   │   │   ├── authorization.rs # Security.framework + Touch ID
│   │   │   │   └── bindings.rs      # FFI bindings
│   │   │   └── windows.rs           # Win32 volume lock + DeviceIoControl
│   │   │
│   │   ├── images/                  # API data parsing
│   │   │   ├── models.rs            # BoardInfo, ImageInfo structs
│   │   │   └── filters.rs           # Board/image extraction and filtering
│   │   │
│   │   ├── logging/mod.rs           # Structured logging (file + console + colors)
│   │   ├── paste/upload.rs          # Log upload to paste.armbian.com
│   │   ├── config/mod.rs            # All constants (URLs, buffers, timeouts, etc.)
│   │   └── utils/
│   │       ├── format.rs            # parse_armbian_filename, normalize_slug, format_size
│   │       ├── path.rs              # validate_cache_path, get_cache_dir
│   │       ├── progress.rs          # ProgressTracker with throttled logging
│   │       └── system.rs            # CPU count, recommended threads
│   │
│   ├── Cargo.toml                   # Rust dependencies
│   ├── tauri.conf.json              # App config (window, bundle, updater)
│   └── icons/                       # App icons (all platforms)
│
├── scripts/
│   ├── build/
│   │   ├── build-all.sh             # Multi-platform build orchestrator
│   │   ├── build-macos.sh           # macOS universal binary (ARM64 + x64)
│   │   └── build-linux.sh           # Linux via Docker (x64 + ARM64)
│   ├── setup/
│   │   ├── install.sh               # Cross-platform installer (auto-detects OS)
│   │   ├── install-linux.sh         # Linux deps (Ubuntu/Debian/Fedora/Arch)
│   │   ├── install-macos.sh         # macOS deps (Homebrew + Rust)
│   │   └── install-windows.ps1      # Windows deps (PowerShell)
│   └── locales/
│       └── sync-locales.js          # AI translation sync (OpenAI API)
│
├── .github/workflows/
│   ├── maintenance-pr-check.yml     # PR validation (lint, type-check, build, security)
│   ├── maintenance-build.yml        # Manual multi-platform builds
│   ├── maintenance-release.yml      # Release builds with signing + notarization
│   └── ...                          # Label sync, locale sync, cleanup
│
├── eslint.config.js                 # ESLint flat config (strict TS rules)
├── tsconfig.json                    # TypeScript root config
├── vite.config.ts                   # Vite build config
└── package.json                     # Node deps & scripts
```

---

## Architecture Deep Dive

### Selection Flow & State Machine

The app uses a linear 4-step wizard: **Manufacturer -> Board -> Image -> Device**

State is managed in `App.tsx` with cascade invalidation — changing a selection at step N resets all downstream selections (N+1, N+2, etc.) via `resetSelectionsFrom()`.

### Frontend -> Backend Communication

54 Tauri IPC commands connect the React frontend to the Rust backend:

```
React Component
  -> Hook (useTauri.ts)
    -> invoke('command_name', { params })
      -> Rust #[tauri::command] handler
        -> Platform-specific logic
          -> Progress via atomic state polling
            -> React UI update
```

Progress is tracked via **polling** (not events): the frontend polls `getDownloadProgress()` / `getFlashProgress()` every 250ms, reading atomic state from the Rust backend.

### Key Hook Architecture

| Hook | Purpose |
|------|---------|
| `useFlashOperation` | Orchestrates entire flash lifecycle (auth -> download -> decompress -> flash -> verify) with device monitoring, failure tracking, and cleanup |
| `useAsyncData` / `useAsyncDataWhen` | Race-condition-safe async data fetching with loading/error states |
| `useSkeletonLoading` | Prevents UI flickering with minimum 300ms skeleton display |
| `useVendorLogos` | Preloads vendor logos, groups failures under "other", sorts by tier |
| `useSettings` | 20+ getter/setter functions for Tauri Store plugin |
| `useModalExitAnimation` | 200ms exit animation with double-trigger prevention |

### Device Detection by Platform

| Platform | Method | Latency | System Disk Detection |
|----------|--------|---------|----------------------|
| Linux | `lsblk` JSON + sysfs | ~100-200ms | `findmnt` + `lsblk PKNAME` |
| macOS | DiskArbitration (native FFI) | ~50ms | `diskutil info /` (cached via `OnceLock`) |
| Windows | Win32 IOCTL (PhysicalDrive0-31) | ~200ms | Drive letter "C:" mapping |

### Flash Operations by Platform

| Platform | Privilege Model | Write Method | Verify Strategy |
|----------|----------------|--------------|-----------------|
| Linux | polkit (transparent) | UDisks2 file descriptor / direct I/O | `posix_fadvise` cache invalidation |
| macOS | Security.framework + Touch ID | `authopen` -> `/dev/rdisk*` (raw, sector-aligned) | BufReader for sector alignment |
| Windows | Administrator required | `CreateFileW` + `FILE_FLAG_WRITE_THROUGH` | Reopen with `FILE_FLAG_NO_BUFFERING` |

All platforms: quick erase (64MB zeros) before flashing, `fsync` after write, shared byte-by-byte verification logic.

### Download & Decompression

1. **Cache check** - Return cached image immediately if available (LRU, default 20GB)
2. **Download** - HTTP streaming to `.downloading` temp file with progress tracking
3. **Mirror logging** - Logs final URL after redirect from `dl.armbian.com` (debug mode)
4. **SHA256 verification** - Compare compressed file hash; special `[SHA_UNAVAILABLE]` handling lets user continue without SHA
5. **Decompression** - XZ (multi-threaded via lzma-rust2 with liblzma fallback), GZ, BZ2, ZST
6. **Failure tracking** - Auto-deletes cached image after 3 consecutive flash failures

### CSS Design Token System

`theme.css` defines a complete design token system:

- **Semantic colors**: `--color-success`, `--color-warning`, `--color-error`, `--color-info` (+ dark variants, backgrounds)
- **Spacing scale**: `--space-xs` (4px) through `--space-4xl` (48px)
- **Radius scale**: `--radius-sm` (4px) through `--radius-full` (50%)
- **Shadows**: 3-level system (sm, md, lg) with light/dark variants
- **Theme switching**: CSS classes `.theme-light` / `.theme-dark` + `prefers-color-scheme` auto

---

## Tech Stack

### Frontend

| Technology | Version | Purpose |
|------------|---------|---------|
| React | 19.2 | UI Framework |
| TypeScript | 5.9 | Type Safety (strict mode) |
| Vite | 7.2 | Build Tool & Dev Server |
| i18next | 25.7 | Internationalization (18 languages) |
| Lucide React | 0.560 | Icon Library |
| Tauri API | 2.9 | IPC Communication |

### Backend

| Technology | Version | Purpose |
|------------|---------|---------|
| Rust | 2021 edition | Systems Programming |
| Tauri | 2.x | Desktop Framework |
| Tokio | 1.x | Async Runtime |
| Reqwest | 0.12 | HTTP Client (rustls-tls) |
| lzma-rust2 | 0.15 | Multi-threaded XZ decompression |
| SHA2 | 0.10 | SHA256 verification |

### Tauri Plugins

| Plugin | Purpose |
|--------|---------|
| `tauri-plugin-store` | Persistent settings (JSON) |
| `tauri-plugin-shell` | Open URLs in browser |
| `tauri-plugin-dialog` | File picker dialogs |
| `tauri-plugin-updater` | Auto-updates from GitHub Releases |
| `tauri-plugin-process` | App restart/exit |

### Why Tauri over Electron?

| Metric | Tauri | Electron |
|--------|-------|----------|
| Bundle Size | ~15 MB | 150-200 MB |
| RAM Usage | ~50 MB | 200-400 MB |
| Startup | < 1s | 2-5s |
| Webview | System native | Bundled Chromium |

---

## Data Sources

| Data | Source |
|------|--------|
| Board List & Images | [github.armbian.com/armbian-images.json](https://github.armbian.com/armbian-images.json) |
| Board Photos | [cache.armbian.com/images/272/{slug}.png](https://cache.armbian.com/images/) |
| Vendor Logos | [cache.armbian.com/images/vendors/150/{vendor}.png](https://cache.armbian.com/images/vendors/150/) |
| MOTD Tips | [github.com/armbian/os/main/motd.json](https://raw.githubusercontent.com/armbian/os/main/motd.json) |
| Log Upload | [paste.armbian.com](https://paste.armbian.com) |
| App Updates | [GitHub Releases](https://github.com/armbian/imager/releases/latest/download/latest.json) |

---

## Quality Checks

Run **all checks** before committing:

### Frontend

```bash
npm run lint           # ESLint (strict: no-explicit-any, eqeqeq, prefer-const)
npx tsc --noEmit       # TypeScript type checking
```

### Backend

```bash
cd src-tauri
cargo fmt              # Code formatting (must have zero diff)
cargo clippy --all-targets --all-features -- -D warnings  # Linter (zero warnings)
```

### CI/CD Pipeline

PRs are validated automatically via GitHub Actions:
1. Frontend lint + type check
2. Rust fmt + clippy
3. Build test on all 3 platforms (Linux, macOS, Windows)
4. Security audit (`npm audit` + `cargo audit`)

---

## Troubleshooting

### Common Issues

| Issue | Solution |
|-------|----------|
| `cargo metadata failed` | Run `bash scripts/setup/install.sh` or install [Rust](https://rustup.rs) |
| `glib-2.0 not found` (Linux) | Run `sudo bash scripts/setup/install-linux.sh` |
| Xcode tools missing (macOS) | Run `xcode-select --install` |
| VS Build Tools missing (Windows) | Run `scripts/setup/install-windows.ps1` as Administrator |
| Node modules failing | Ensure Node.js >= 20.19.0, then `rm -rf node_modules && npm install` |
| Version mismatch error | Sync version across `package.json`, `Cargo.toml`, `tauri.conf.json` |

### Getting Help

1. Search [GitHub Issues](https://github.com/armbian/imager/issues)
2. Check [Armbian Forum](https://forum.armbian.com)
3. Create issue with: OS version, `node --version`, `rustc --version`, full error log

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md).

### Tips

- Keep commits small and atomic
- Test on multiple platforms for platform-specific changes
- Run all quality checks before pushing
- Update translations for user-facing text (all 18 locale files)
- Follow existing patterns (hooks, Tauri commands, CSS variables)

### PR Process

1. Fork repository
2. `git checkout -b feature/amazing-feature`
3. Implement + run quality checks
4. `git push origin feature/amazing-feature`
5. Open Pull Request (CI builds automatically)

---

## Acknowledgments

- [Raspberry Pi Imager](https://github.com/raspberrypi/rpi-imager) — Inspiration
- [Tauri](https://tauri.app/) — Framework
- [i18next](https://www.i18next.com/) — Internationalization
- [Lucide](https://lucide.dev/) — Icons
- [Armbian Community](https://forum.armbian.com) — SBC support
