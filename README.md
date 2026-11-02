<h2 align="center">
  <a href=#><img src="https://raw.githubusercontent.com/armbian/.github/master/profile/logosmall.png" alt="Armbian logo"></a>
  <br><br>
</h2>

# Armbian Imager

## Purpose of This Repository

Armbian Imager is the official cross-platform desktop tool for downloading and flashing Armbian OS images to SD cards and USB drives. It handles target-disk safety checks, checksum validation, and post-write verification so that a bad download or the wrong disk doesn't turn into a broken card.

## About

Armbian Imager works with 300+ boards, with filtering and board metadata sourced from armbian.com. Native builds are produced for Linux, Windows, and macOS, on x64 and ARM64. The UI follows your system language by default and ships with built-in application updates.

### Features

- Works with 300+ boards, with filtering and board metadata from armbian.com
- Disk safety checks, checksum validation, and post-write verification
- Native builds for Linux, Windows, and macOS, on x64 and ARM64
- Multi-language interface that follows your system language by default
- Built-in application updates
- Small binary with few runtime dependencies

### Testimonials

> "What a fantastic tool for getting people started with a non Raspberry PI"
> *Interfacing Linux*, hardware and software guides for Linux creatives ([source](https://www.youtube.com/watch?v=RAxQebKsnuc))

> "A proper multi-platform desktop app that actually works, which is rarer than you'd think."
> *Bruno Verachten*, Senior Developer Relations Engineer ([source](https://www.linkedin.com/pulse/adding-risc-v-support-armbian-imager-tale-qemu-tauri-deja-verachten-86fxe))

> "The Upcoming Armbian Imager Tool is a Godsend for Non-Raspberry Pi SBC Owners"
> *Sourav Rudra*, It's FOSS ([source](https://itsfoss.com/news/armbian-imager-quietly-debuts/))

> "According to Armbian, this results in less RAM and storage usage and a faster experience."
> *Jordan Gloor*, HowtoGeek.com ([source](https://www.howtogeek.com/armbians-raspberry-pi-imager-alternative-is-here/))

> "It's super easy to write an operating system... I'm always happy when an Armbian version comes out because you've got more stability and much more compatibility."
> *leepspvideo*, Simple Linux install for 300+ Arm devices ([source](https://www.youtube.com/watch?v=vUvGD2GSALI))

## Download

Prebuilt binaries are available for every supported platform.

| <a href="https://github.com/armbian/imager/releases"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/apple.svg" width="24"><br><strong>macOS</strong></a> | <a href="https://github.com/armbian/imager/releases"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/windows11.svg" width="24"><br><strong>Windows</strong></a> | <a href="https://github.com/armbian/imager/releases"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/linux.svg" width="24"><br><strong>Linux</strong></a> |
|:---:|:---:|:---:|
| Intel & Apple Silicon | x64 & ARM64 (code-signed) | x64 & ARM64 |
| <code>.dmg</code> / <code>.app.zip</code> | <code>.exe</code> / <code>.msi</code> | <code>.deb</code> / <code>.AppImage</code> |

## How It Works

1. **Pick a manufacturer.** Choose one of the supported SBC vendors, or load your own image file.
2. **Pick a board.** Boards show real photos and metadata from armbian.com.
3. **Pick an image.** Desktop or server, a kernel branch, and a stable, nightly, or rolling release build.
4. **Flash.** The app downloads, decompresses, writes, and verifies for you.

## Customization

- Theme: light, dark, or follow the system setting
- Developer mode: turn on detailed logging and open the log viewer
- Language: 20 languages, auto-detected from your system

## Platform Support

| Platform | Architecture | Notes |
|----------|-------------|-------|
| macOS | Intel x64 | Full support |
| macOS | Apple Silicon | Native ARM64 build, Touch ID support |
| Windows | x64 | Requires Administrator privileges |
| Windows | ARM64 | Native ARM64 build, requires Administrator privileges |
| Linux | x64 | Uses lsblk for detection and UDisks2/polkit for elevated device access |
| Linux | ARM64 | Native ARM64 build |

### Supported Languages

English, Italian, German, French, Spanish, Portuguese, Portuguese (Brazil), Dutch, Polish, Russian, Chinese, Japanese, Korean, Ukrainian, Turkish, Slovenian, Swedish, Croatian, Greek, Serbian.

The locale files live under [`src/locales/`](src/locales/) with `en.json` as the source of truth. Translation parity is enforced in CI by [`scripts/locales/check-parity.mjs`](scripts/locales/check-parity.mjs), and a scheduled job runs [`scripts/locales/sync-locales.js`](scripts/locales/sync-locales.js) to propagate new keys.

## Tech Stack

Armbian Imager is a [Tauri 2](https://tauri.app/) desktop application.

- **Frontend:** TypeScript + React 19, built with Vite 8, styled with CSS, linted with ESLint 9. Internationalization via `i18next` / `react-i18next`.
- **Backend:** Rust (edition 2021, MSRV 1.85), using the Tauri 2 runtime with the `shell`, `dialog`, `updater`, `process`, `store`, and `mcp-bridge` plugins. HTTP via `reqwest` (rustls), async via `tokio`, decompression via `lzma-rust2`, `xz2`, `bzip2`, `flate2`, and `zstd`. QDL (Qualcomm Device Loader) flashing via the `qdl` crate for EDL-based boards.
- **In-repo crate:** [`crates/armbian-write-conf/`](crates/armbian-write-conf/) — an MIT-licensed library that writes a small first-boot config file into the ext4 rootfs of a RAW disk image in userspace, then validates it. It uses `gptman`, `mbrman`, and `ext4-view`, and vendors a fork named `armbian-ext4fs`.
- **Setup scripts:** platform installers in [`scripts/setup/`](scripts/setup/) — Bash for Linux/macOS, PowerShell for Windows.

## Repository Layout

```
.
├── src/                     # React 19 + TypeScript frontend
│   ├── components/          # UI (autoconfig, dev, flash, layout, modals, settings, shared)
│   ├── hooks/               # Custom React hooks (incl. Tauri IPC wrappers)
│   ├── config/              # Static config: i18n, boards, filters, flags, themes
│   ├── contexts/            # React contexts (Motion, Theme, Update)
│   ├── locales/             # 20 translation JSON files
│   ├── styles/              # CSS
│   ├── assets/              # Images, logos, settings artwork
│   ├── utils/               # Shared TypeScript utilities
│   └── types/               # Shared type definitions
├── src-tauri/               # Rust backend (Tauri app)
│   ├── src/
│   │   ├── commands/        # Tauri IPC command handlers
│   │   ├── devices/         # Per-OS device detection (linux, macos, windows)
│   │   ├── flash/           # Per-OS flash writers + verification
│   │   ├── qdl/             # EDL/QDL flashing path
│   │   ├── dev_scenarios/   # Debug-only simulator (not in release builds)
│   │   ├── images/          # Image metadata and filters
│   │   ├── paste/           # Log upload to paste.armbian.com
│   │   ├── utils/           # Shared Rust utilities
│   │   └── main.rs
│   ├── capabilities/        # Tauri capability definitions
│   ├── icons/               # App icons (desktop, Android, iOS)
│   └── tauri.conf.json      # Tauri configuration
├── crates/
│   └── armbian-write-conf/  # In-repo crate: ext4 first-boot config writer
├── scripts/
│   ├── locales/             # Locale parity + AI translation sync
│   └── setup/               # Linux/macOS/Windows dev-env installers
├── public/                  # Static web assets
├── .github/
│   ├── ISSUE_TEMPLATE/
│   └── workflows/           # GitHub Actions workflows
├── package.json
├── vite.config.ts
├── eslint.config.js
└── index.html
```

## Continuous Integration

CI runs on GitHub Actions and covers linting, type checks, locale parity, cross-platform build tests on pull requests, release builds for Linux (x64/ARM64), macOS (x64/Apple Silicon), and Windows (x64/ARM64), and various repository-maintenance tasks.

For a per-workflow status overview, see the Armbian CI dashboard for this repository: **<https://actions.armbian.com/?repo=imager>**.

## Why We Sign Our Code

Downloading software shouldn't take a leap of faith. Every Windows release is cryptographically signed, so you can confirm the binary is exactly what we built and hasn't been tampered with on the way to you.

This is possible thanks to [SignPath Foundation](https://signpath.org?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager), which gives free code signing certificates to open source projects, and [SignPath.io](https://signpath.io?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager) for the signing infrastructure.

## Development

Setup, build instructions, and project layout live in [DEVELOPMENT.md](DEVELOPMENT.md). Contribution guidelines are in [CONTRIBUTING.md](CONTRIBUTING.md), and community expectations in [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

### Quick start

```bash
git clone https://github.com/armbian/imager.git && cd imager
bash scripts/setup/install.sh
npm install
npm run tauri:dev
```

Requirements: Node.js ≥ 20.19.0, Rust ≥ 1.85, and the platform-specific system dependencies documented in [DEVELOPMENT.md](DEVELOPMENT.md).

## License

Armbian Imager is distributed under the GNU General Public License, either version 2 or (at your option) any later version. Version 2 is the floor, so the terms stay compatible with the [Armbian build framework](https://github.com/armbian/build).

SPDX-License-Identifier: `GPL-2.0-or-later`

The full text is in [LICENSE](LICENSE). A few bundled components keep their own terms: the `armbian-write-conf` crate and its vendored `armbian-ext4fs` fork are `MIT`, and the language flag graphics from [Twemoji](https://github.com/jdecked/twemoji) are `CC-BY-4.0`. Per-file details are in [`src-tauri/packaging/copyright`](src-tauri/packaging/copyright).

## Related Links

- Armbian website: <https://www.armbian.com>
- Armbian documentation: <https://docs.armbian.com>
- Armbian build framework: <https://github.com/armbian/build>
- Forum: <https://forum.armbian.com>

---

<p align="center">
  <sub>Made with ❤️ by the Armbian community</sub>
</p>
