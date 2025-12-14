<p align="center">
  <a href="https://www.armbian.com">
    <img src="https://raw.githubusercontent.com/armbian/.github/master/profile/logosmall.png" alt="Armbian logo" width="144">
  </a>
</p>

<h1 align="center">Armbian Imager</h1>

<p align="center">
  <strong>The official tool for flashing Armbian OS to your single-board computer</strong>
</p>

<p align="center">
  <a href="https://github.com/armbian/armbian-imager/releases"><img src="https://img.shields.io/github/v/release/armbian/armbian-imager?style=flat-square" alt="Release"></a>
  <a href="https://github.com/armbian/armbian-imager/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-GPLv2-blue?style=flat-square" alt="License"></a>
  <a href="https://www.armbian.com"><img src="https://img.shields.io/badge/armbian-supported-orange?style=flat-square" alt="Armbian"></a>
</p>

<p align="center">
  <a href="#why-armbian-imager">Why?</a> •
  <a href="#features">Features</a> •
  <a href="#download">Download</a> •
  <a href="#how-it-works">How it works</a> •
  <a href="#development">Development</a>
</p>

---

## Why Armbian Imager?

Getting started with single-board computers shouldn't be complicated. Yet, for years, the process of flashing an OS image involved:

- **Hunting for the right image** across multiple download pages
- **Manually verifying checksums** to ensure file integrity
- **Using generic tools** that don't understand SBC specifics
- **Risk of bricking your main drive** with poorly designed software

**Armbian Imager changes everything.**

We built this tool because the Armbian community deserves a first-class experience. With 307+ supported boards from 70+ manufacturers, finding and flashing the right image should be effortless—and now it is.

### The Vision

Inspired by the simplicity of [Raspberry Pi Imager](https://github.com/raspberrypi/rpi-imager), we wanted to bring that same polished experience to the broader SBC ecosystem. But we didn't just copy—we innovated:

- **Native performance** with Rust and Tauri (not Electron's 200MB+ overhead)
- **Touch ID support** on macOS for seamless authentication
- **Real board photos** scraped directly from armbian.com
- **Smart filtering** by kernel type, desktop environment, and release channel

## Features

| Feature | Description |
|---------|-------------|
| **307+ Boards** | Browse every Armbian-supported SBC, organized by manufacturer |
| **Smart Filtering** | Filter by stable/nightly, desktop/server/minimal, kernel variant, apps |
| **Safe by Design** | System disks are automatically excluded—no accidents |
| **Verified Writes** | SHA256 read-back verification ensures your flash is perfect |
| **Custom Images** | Use your own `.img`, `.img.xz`, `.img.gz`, `.img.bz2`, `.img.zst` files |
| **Touch ID** | Authenticate with biometrics on macOS |
| **15 Languages** | Auto-detects system language (EN, IT, DE, FR, ES, PT, NL, PL, RU, ZH, JA, KO, UK, TR, SL) |
| **Light/Dark Mode** | Follows your system preference |
| **Device Hot-Swap** | Automatically detects when devices are connected/disconnected |
| **Log Upload** | One-click error log upload to paste.armbian.com with QR code |
| **Tiny Footprint** | ~15MB app size vs 200MB+ for Electron alternatives |

## Download

<table>
<tr>
<td align="center"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/apple.svg" width="48"><br><b>macOS</b></td>
<td align="center"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/windows.svg" width="48"><br><b>Windows</b></td>
<td align="center"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/linux.svg" width="48"><br><b>Linux</b></td>
</tr>
<tr>
<td align="center"><a href="https://github.com/armbian/armbian-imager/releases">Intel & Apple Silicon<br><code>.dmg</code></a></td>
<td align="center"><a href="https://github.com/armbian/armbian-imager/releases">x64 & ARM64<br><code>.msi</code> / <code>.exe</code></a></td>
<td align="center"><a href="https://github.com/armbian/armbian-imager/releases">x64 & ARM64<br><code>.deb</code></a></td>
</tr>
</table>

## How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   1. SELECT          2. SELECT         3. SELECT      4. FLASH  │
│   MANUFACTURER       BOARD             IMAGE          & VERIFY  │
│                                                                 │
│   ┌─────────┐       ┌─────────┐       ┌─────────┐    ┌───────┐ │
│   │ Orange  │  →    │ Pi 5    │  →    │ Bookworm│ →  │  ██   │ │
│   │ Pi      │       │         │       │ Desktop │    │ ████  │ │
│   │ Khadas  │       │ Pi 4    │       │ Minimal │    │ ████  │ │
│   │ Radxa   │       │ Zero 3  │       │ Nightly │    │ 100%  │ │
│   └─────────┘       └─────────┘       └─────────┘    └───────┘ │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

1. **Select Manufacturer** — Choose from 70+ SBC manufacturers or load a custom image
2. **Select Board** — Pick your board with real photos from armbian.com
3. **Select Image** — Choose desktop/server, kernel variant, stable/nightly
4. **Flash** — Download, decompress, write, and verify automatically

## Tech Stack

Built with modern technologies for optimal performance:

| Layer | Technology | Why |
|-------|------------|-----|
| **UI** | React 19 + TypeScript | Type-safe, component-based UI |
| **Bundler** | Vite 7 | Lightning-fast HMR and builds |
| **Framework** | Tauri 2 | Native performance, tiny bundle |
| **Backend** | Rust | Memory-safe, blazing fast I/O |
| **Async** | Tokio | Efficient concurrent operations |
| **i18n** | i18next + react-i18next | 15 language translations |
| **Icons** | Lucide React | Modern, consistent icon set |

### Why Tauri over Electron?

| Metric | Armbian Imager (Tauri) | Typical Electron App |
|--------|------------------------|---------------------|
| App Size | ~15 MB | 150-200 MB |
| RAM Usage | ~50 MB | 200-400 MB |
| Startup | < 1 second | 2-5 seconds |
| Native Feel | ✅ Uses system webview | ❌ Bundles Chromium |

## Platform Support

| Platform | Architecture | Status | Notes |
|----------|-------------|--------|-------|
| macOS | Intel x64 | ✅ | Full support |
| macOS | Apple Silicon | ✅ | Native ARM64 + Touch ID |
| Windows | x64 | ✅ | Admin elevation via UAC |
| Windows | ARM64 | ✅ | Native ARM64 build |
| Linux | x64 | ✅ | UDisks2 + pkexec for privileges |
| Linux | ARM64 | ✅ | Native ARM64 build |

### Supported Languages

The app automatically detects your system language:

| Language | Code | Language | Code |
|----------|------|----------|------|
| English | `en` | Russian | `ru` |
| Italian | `it` | Chinese (Simplified) | `zh` |
| German | `de` | Japanese | `ja` |
| French | `fr` | Korean | `ko` |
| Spanish | `es` | Ukrainian | `uk` |
| Portuguese | `pt` | Turkish | `tr` |
| Dutch | `nl` | Polish | `pl` |
| Slovenian | `sl` | | |

## Development

### Prerequisites

- **Node.js 20+** (LTS recommended)
- **Rust 1.77+** (install via [rustup](https://rustup.rs))
- **Platform tools**: Xcode (macOS), Visual Studio Build Tools (Windows), or build-essential (Linux)

### Quick Start

```bash
# Clone
git clone https://github.com/armbian/armbian-imager.git
cd armbian-imager

# Install dependencies
npm install

# Run in development mode
npm run tauri:dev

# Build for production
npm run tauri:build
```

### Available Scripts

```bash
# Development
npm run dev              # Frontend only (Vite)
npm run tauri:dev        # Full app with hot reload

# Production
npm run build            # Build frontend
npm run tauri:build      # Build distributable

# Utilities
npm run lint             # ESLint
npm run clean            # Clean all build artifacts
```

### Build Scripts

```bash
# Platform-specific builds (output in releases/)
./scripts/build-macos.sh [--clean] [--dev]   # macOS ARM64 + x64
./scripts/build-linux.sh [--clean] [--dev]   # Linux x64 + ARM64
./scripts/build-all.sh   [--clean] [--dev]   # All platforms
```

<details>
<summary><b>Project Structure</b></summary>

```
armbian-imager/
├── src/                          # React Frontend
│   ├── components/               # UI Components
│   │   ├── Header.tsx            # Progress steps header
│   │   ├── HomePage.tsx          # Main wizard interface
│   │   ├── ManufacturerModal.tsx # Manufacturer selection
│   │   ├── BoardModal.tsx        # Board selection
│   │   ├── ImageModal.tsx        # Image selection with filters
│   │   ├── DeviceModal.tsx       # Device selection with confirmation
│   │   ├── FlashProgress/        # Flash operation UI
│   │   └── shared/               # Shared components (ErrorDisplay)
│   ├── hooks/                    # React Hooks (useTauri, useAsyncData, useDeviceMonitor)
│   ├── config/                   # Configuration (manufacturers, badges, OS info)
│   ├── locales/                  # i18n translations (15 languages)
│   ├── styles/                   # Modular CSS
│   ├── types/                    # TypeScript interfaces
│   ├── assets/                   # Images, logos, OS icons
│   ├── i18n.ts                   # i18n initialization
│   └── App.tsx                   # Root component with state management
│
├── src-tauri/                    # Rust Backend
│   ├── src/
│   │   ├── commands/             # Tauri IPC handlers
│   │   ├── devices/              # Platform device detection (macOS, Linux, Windows)
│   │   ├── flash/                # Platform flash implementation
│   │   │   ├── macos/            # macOS writer + Touch ID auth
│   │   │   ├── linux/            # Linux writer + pkexec/UDisks2
│   │   │   └── windows.rs        # Windows writer
│   │   ├── images/               # Image management and filtering
│   │   ├── download.rs           # HTTP streaming downloads
│   │   ├── decompress.rs         # XZ/GZ/BZ2/ZST decompression
│   │   ├── paste/                # Log upload to paste.armbian.com
│   │   └── utils/                # Utilities
│   ├── icons/                    # App icons (all platforms)
│   └── tauri.conf.json           # Tauri configuration
│
├── scripts/                      # Build scripts (macOS, Linux, all platforms)
└── .github/workflows/            # CI/CD (multi-platform builds)
```

</details>

## Data Sources

| Data | Source |
|------|--------|
| Board List | [github.armbian.com/all-images.json](https://github.armbian.com/all-images.json) |
| Board Photos | [cache.armbian.com/images/{size}/{slug}.png](https://cache.armbian.com) |
| Checksums | Embedded in image metadata (SHA256) |
| Log Upload | [paste.armbian.com](https://paste.armbian.com) |

## Contributing

We welcome contributions! Whether it's:

- 🐛 **Bug reports** — Found an issue? [Open a ticket](https://github.com/armbian/armbian-imager/issues)
- 💡 **Feature requests** — Have an idea? Let's discuss it
- 🔧 **Pull requests** — Code improvements are always welcome
- 🌍 **Translations** — Add or improve translations in `src/locales/`
- 📖 **Documentation** — Help others get started

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is part of the [Armbian](https://www.armbian.com) ecosystem and is licensed under the GPLv2.

## Acknowledgments

- **[Raspberry Pi Imager](https://github.com/raspberrypi/rpi-imager)** — The inspiration for this project
- **[Tauri](https://tauri.app/)** — The framework that makes native apps accessible
- **[Armbian Community](https://forum.armbian.com)** — For years of amazing work on SBC support

---

<p align="center">
  <sub>Made with ❤️ by the Armbian community</sub>
</p>
