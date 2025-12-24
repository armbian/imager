## Development

### Prerequisites

- **Node.js 20+** — [nodejs.org](https://nodejs.org)
- **Rust 1.77+** — [rustup.rs](https://rustup.rs)
- **Platform tools** — Xcode (macOS), Visual Studio Build Tools (Windows), build-essential (Linux)

### Quick Start

```bash
git clone https://github.com/armbian/imager.git armbian-imager
cd armbian-imager
npm install
npm run tauri:dev
```

### Scripts

```bash
npm run dev              # Frontend only (Vite)
npm run tauri:dev        # Full app with hot reload
npm run build            # Build frontend for production
npm run tauri:build      # Build distributable
npm run tauri:build:dev  # Build with debug symbols
npm run lint             # ESLint
npm run clean            # Clean all build artifacts
```

### Build Scripts

```bash
./scripts/build-macos.sh [--clean] [--dev]   # macOS ARM64 + x64
./scripts/build-linux.sh [--clean] [--dev]   # Linux x64 + ARM64
./scripts/build-all.sh   [--clean] [--dev]   # All platforms
```

## Tech Stack

| Layer | Technology | Why |
|-------|------------|-----|
| **UI** | React 19 + TypeScript | Type-safe, component-based UI |
| **Bundler** | Vite | Lightning-fast HMR and builds |
| **Framework** | Tauri 2 | Native performance, tiny bundle |
| **Backend** | Rust | Memory-safe, blazing fast I/O |
| **Async** | Tokio | Efficient concurrent operations |
| **i18n** | i18next | 15 language translations |

### Why Tauri over Electron?

| Metric | Armbian Imager (Tauri) | Typical Electron App |
|--------|------------------------|---------------------|
| App Size | ~15 MB | 150-200 MB |
| RAM Usage | ~50 MB | 200-400 MB |
| Startup | < 1 second | 2-5 seconds |
| Native Feel | ✅ Uses system webview | ❌ Bundles Chromium |

## Project Structure

<details>
<summary>Click to expand</summary>

```
armbian-imager/
├── src/                          # React Frontend
│   ├── components/               # UI Components
│   │   ├── flash/                # Flash progress components
│   │   ├── layout/               # Header, HomePage
│   │   ├── modals/               # Board, Image, Device, Manufacturer modals
│   │   └── shared/               # Reusable components (UpdateModal, ErrorDisplay, etc.)
│   ├── hooks/                    # React Hooks (Tauri IPC, async data)
│   ├── config/                   # Badges, manufacturers, OS info
│   ├── locales/                  # i18n translations (15 languages)
│   ├── styles/                   # Modular CSS
│   ├── types/                    # TypeScript interfaces
│   ├── utils/                    # Utility functions
│   └── assets/                   # Images, logos, OS icons
│
├── src-tauri/                    # Rust Backend
│   ├── src/
│   │   ├── commands/             # Tauri IPC handlers
│   │   ├── config/               # Application configuration and constants
│   │   ├── devices/              # Platform device detection
│   │   ├── flash/                # Platform flash (macOS, Linux, Windows)
│   │   ├── images/               # Image management and filtering
│   │   ├── logging/              # Session logging
│   │   ├── paste/                # Log upload to paste.armbian.com
│   │   ├── utils/                # Shared utility functions
│   │   ├── download.rs           # HTTP streaming downloads
│   │   └── decompress.rs         # Decompression (XZ, GZ, ZSTD)
│   └── icons/                    # App icons (all platforms)
│
├── scripts/                      # Build scripts
└── .github/workflows/            # CI/CD
```

</details>

## Data Sources

| Data | Source |
|------|--------|
| Board List & Images | [github.armbian.com/armbian-images.json](https://github.armbian.com/armbian-images.json) |
| Board Photos | [cache.armbian.com/images/272/{slug}.png](https://cache.armbian.com/images/) |
| Vendor Logos | [cache.armbian.com/images/vendors/150/{vendor}.png](https://cache.armbian.com/images/vendors/150/) |
| MOTD Tips | [raw.githubusercontent.com/armbian/os/main/motd.json](https://raw.githubusercontent.com/armbian/os/main/motd.json) |
| Log Upload | [paste.armbian.com](https://paste.armbian.com) |

## Acknowledgments

- [Raspberry Pi Imager](https://github.com/raspberrypi/rpi-imager) — The inspiration for this project
- [Tauri](https://tauri.app/) — The framework that makes native apps accessible
- [i18next](https://www.i18next.com/) — Internationalization framework
- [Lucide](https://lucide.dev/) — Beautiful icons
- [Armbian Community](https://forum.armbian.com) — For years of amazing work on SBC support