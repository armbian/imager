<p align="center">
  <a href="https://www.armbian.com">
    <img src="https://raw.githubusercontent.com/armbian/.github/master/profile/logosmall.png" alt="Armbian logo" width="200">
  </a>
</p>

<p align="center">
  <b>The official tool for flashing Armbian OS to your single-board computer</b>
</p>

<p align="center">
  <a href="https://github.com/armbian/imager/releases"><img src="https://img.shields.io/github/v/release/armbian/imager?style=for-the-badge&color=orange" alt="Release"></a>
  <a href="https://github.com/armbian/imager/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-GPLv2-blue?style=for-the-badge" alt="License"></a>
</p>

<p align="center">
  <img src="images/armbian-imager.png" alt="Armbian Imager" width="700">
</p>

## Features

- **307+ Boards** — Browse every Armbian-supported SBC, organized by manufacturer
- **Smart Filtering** — Filter by stable/nightly, desktop/server/minimal, kernel variant
- **Safe by Design** — System disks are automatically excluded
- **Verified Writes** — SHA256 read-back verification
- **Custom Images** — Use your own `.img`, `.img.xz`, `.img.gz`, `.img.bz2`, `.img.zst` files
- **Touch ID** — Biometric authentication on macOS
- **15 Languages** — Auto-detects system language
- **Light/Dark Mode** — Follows your system preference
- **Device Hot-Swap** — Automatically detects when devices are connected/disconnected
- **Log Upload** — One-click error log upload to paste.armbian.com with QR code
- **Tiny Footprint** — ~15MB app size vs 200MB+ for Electron alternatives

## Download

<p align="center">

| <img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/apple.svg" width="40"> | <img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/windows11.svg" width="40"> | <img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/linux.svg" width="40"> |
|:---:|:---:|:---:|
| **macOS** | **Windows** | **Linux** |
| [Intel & Apple Silicon](https://github.com/armbian/imager/releases) | [x64 & ARM64](https://github.com/armbian/imager/releases) | [x64 & ARM64](https://github.com/armbian/imager/releases) |
| `.dmg` | `.exe` / `.msi` | `.deb` |

</p>

## How It Works

1. **Select Manufacturer** — Choose from 70+ SBC manufacturers or load a custom image
2. **Select Board** — Pick your board with real photos from armbian.com
3. **Select Image** — Choose desktop/server, kernel variant, stable/nightly
4. **Flash** — Download, decompress, write, and verify automatically


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

English, Italian, German, French, Spanish, Portuguese, Dutch, Polish, Russian, Chinese, Japanese, Korean, Ukrainian, Turkish, Slovenian

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
npm run build            # Build frontend
npm run tauri:build      # Build distributable
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
│   ├── hooks/                    # React Hooks
│   ├── config/                   # Configuration
│   ├── locales/                  # i18n translations (15 languages)
│   ├── styles/                   # Modular CSS
│   ├── types/                    # TypeScript interfaces
│   └── assets/                   # Images, logos, OS icons
│
├── src-tauri/                    # Rust Backend
│   ├── src/
│   │   ├── commands/             # Tauri IPC handlers
│   │   ├── devices/              # Platform device detection
│   │   ├── flash/                # Platform flash implementation
│   │   ├── images/               # Image management and filtering
│   │   ├── download.rs           # HTTP streaming downloads
│   │   ├── decompress.rs         # XZ/GZ/BZ2/ZST decompression
│   │   └── paste/                # Log upload
│   └── icons/                    # App icons (all platforms)
│
├── scripts/                      # Build scripts
└── .github/workflows/            # CI/CD
```

</details>

## Data Sources

| Data | Source |
|------|--------|
| Board List | [github.armbian.com/all-images.json](https://github.armbian.com/all-images.json) |
| Board Photos | [cache.armbian.com](https://cache.armbian.com) |
| Checksums | Embedded in image metadata (SHA256) |
| Log Upload | [paste.armbian.com](https://paste.armbian.com) |


## Contributing

We welcome contributions!

- **Bug reports** — [Open an issue](https://github.com/armbian/imager/issues)
- **Feature requests** — Let's discuss it
- **Pull requests** — Code improvements are always welcome
- **Translations** — Add or improve in `src/locales/`
- **Documentation** — Help others get started

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

GPLv2 — Part of the [Armbian](https://www.armbian.com) ecosystem.

## Acknowledgments

- [Raspberry Pi Imager](https://github.com/raspberrypi/rpi-imager) — The inspiration for this project
- [Tauri](https://tauri.app/) — The framework that makes native apps accessible
- [Armbian Community](https://forum.armbian.com) — For years of amazing work on SBC support

---

<p align="center">
  <sub>Made with ❤️ by the Armbian community</sub>
</p>
