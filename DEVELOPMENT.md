# Development Guide

Complete guide for setting up, building, and contributing to Armbian Imager.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Prerequisites](#prerequisites)
3. [Step-by-Step Setup](#step-by-step-setup)
4. [Development Workflow](#development-workflow)
5. [Building for Distribution](#building-for-distribution)
6. [Project Structure](#project-structure)
7. [Tech Stack](#tech-stack)
8. [Data Sources](#data-sources)
9. [Troubleshooting](#troubleshooting)

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
| Rust | 1.77 | [rustup.rs](https://rustup.rs) |
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
node --version    # ≥ 20.19.0
rustc --version   # ≥ 1.77
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
| `npm run dev` | Frontend only |
| `npm run tauri:dev` | Full app with hot reload |
| `npm run build` | Build frontend |
| `npm run tauri:build` | Build distributable |
| `npm run lint` | Run ESLint |
| `npm run clean` | Clean artifacts |

### Daily Workflow

1. `npm run tauri:dev` - Start dev server
2. Edit [`src/`](src/) or [`src-tauri/src/`](src-tauri/src/) - Auto reload
3. Test changes
4. `npm run tauri:build` - Build when ready

---

## Building for Distribution

### Single Platform

```bash
./scripts/build-macos.sh      # macOS (Intel + ARM)
./scripts/build-linux.sh      # Linux (x64 + ARM)
npm run tauri:build          # Windows
```

### All Platforms

```bash
./scripts/build-all.sh
```

### Build Options

```bash
./scripts/build-macos.sh --clean  # Clean build
./scripts/build-macos.sh --dev    # Debug symbols
./scripts/build-macos.sh --clean --dev  # Both
```

### Output

- `src-tauri/target/release/bundle/` - Installers
- `src-tauri/target/release/armbian-imager` - Binary

---

## Project Structure

```
armbian-imager/
├── src/                          # React Frontend
│   ├── components/               # UI Components
│   │   ├── flash/                # Flash progress
│   │   ├── layout/               # Header, HomePage
│   │   ├── modals/               # Board, Image, Device, Manufacturer
│   │   └── shared/               # Reusable components
│   ├── hooks/                    # React Hooks
│   ├── config/                   # Badges, manufacturers, OS info
│   ├── locales/                  # i18n (15 languages)
│   ├── styles/                   # Modular CSS
│   ├── types/                    # TypeScript interfaces
│   ├── utils/                    # Utilities
│   └── assets/                   # Images, logos, icons
│
├── src-tauri/                    # Rust Backend
│   ├── src/
│   │   ├── commands/             # Tauri IPC handlers
│   │   ├── config/               # Configuration
│   │   ├── devices/              # Device detection
│   │   ├── flash/                # Platform flash implementations
│   │   ├── images/               # Image management
│   │   ├── logging/              # Session logging
│   │   ├── paste/                # Log upload
│   │   ├── utils/                # Utilities
│   │   ├── download.rs           # HTTP downloads
│   │   └── decompress.rs         # XZ, GZ, ZSTD
│   ├── icons/                    # App icons
│   └── target/                   # Compiled binaries (gitignored)
│
├── scripts/                      # Build and setup
│   ├── build-*.sh                # Platform build scripts
│   └── setup/                    # Dependency installation
│       ├── install.sh
│       ├── install-linux.sh
│       ├── install-macos.sh
│       └── install-windows.ps1
│
└── .github/workflows/            # CI/CD
```

---

## Tech Stack

### Frontend

| Technology | Purpose |
|------------|---------|
| React 19 | UI Framework |
| TypeScript | Type Safety |
| Vite | Build Tool & Dev Server |
| i18next | i18n (15 languages) |
| Lucide | Icons |

### Backend

| Technology | Purpose |
|------------|---------|
| Rust | Systems Programming |
| Tauri 2 | Desktop Framework |
| Tokio | Async Runtime |
| Serde | Serialization |
| Reqwest | HTTP Client |

### Why Tauri over Electron?

| Metric | Tauri | Electron |
|--------|-------|----------|
| Size | ~15 MB | 150-200 MB |
| RAM | ~50 MB | 200-400 MB |
| Startup | < 1s | 2-5s |
| Native | ✅ System webview | ❌ Bundled Chromium |

---

## Data Sources

| Data | Source |
|------|--------|
| Board List & Images | [github.armbian.com/armbian-images.json](https://github.armbian.com/armbian-images.json) |
| Board Photos | [cache.armbian.com/images/272/](https://cache.armbian.com/images/) |
| Vendor Logos | [cache.armbian.com/images/vendors/150/](https://cache.armbian.com/images/vendors/150/) |
| MOTD Tips | [github.com/armbian/os](https://raw.githubusercontent.com/armbian/os/main/motd.json) |
| Log Upload | [paste.armbian.com](https://paste.armbian.com) |

---

## Troubleshooting

### Common Issues

| Issue | Solution |
|-------|----------|
| `cargo metadata failed` | Run `bash scripts/setup/install.sh` or install [Rust](https://rustup.rs) |
| `glib-2.0 not found` (Linux) | Run `sudo bash scripts/setup/install-linux.sh` |
| Xcode tools missing (macOS) | Run `xcode-select --install` |
| VS Build Tools missing (Windows) | Run `scripts/setup/install-windows.ps1` as Administrator |
| Node modules failing | Ensure Node.js ≥ 20.19.0, then `npm install` |

### Getting Help

1. Check [`scripts/setup/README.md`](scripts/setup/README.md)
2. Search [GitHub Issues](https://github.com/armbian/imager/issues)
3. Create issue with:
   - OS and version
   - `node --version`, `rustc --version`, `npm --version`
   - Full error and stack trace
   - Steps to reproduce

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md).

### Tips

- Keep commits small and atomic
- Test on multiple platforms for platform-specific changes
- Follow ESLint and Rustfmt
- Update translations for user-facing text
- Add tests for new features

### PR Process

1. Fork repository
2. `git checkout -b feature/amazing-feature`
3. `git commit -m 'Add amazing feature'`
4. `git push origin feature/amazing-feature`
5. Open Pull Request

---

## Acknowledgments

- [Raspberry Pi Imager](https://github.com/raspberrypi/rpi-imager) — Inspiration
- [Tauri](https://tauri.app/) — Framework
- [i18next](https://www.i18next.com/) — i18n
- [Lucide](https://lucide.dev/) — Icons
- [Armbian Community](https://forum.armbian.com) — SBC support
