<h2 align="center">
  <a href=#><img src="https://raw.githubusercontent.com/armbian/.github/master/profile/logosmall.png" alt="Armbian logo"></a>
  <br><br>
</h2>

# Armbian Imager

## Purpose of This Repository

Armbian Imager is the official cross-platform desktop tool for downloading, writing, and verifying Armbian OS images to SD cards, USB drives, and EDL-capable boards. This repository holds its complete source code: a React/TypeScript front end and a Rust/Tauri back end, together with the companion crates and packaging assets that produce the published Linux, macOS, and Windows builds.

## About

Armbian Imager checks the target disk before writing, validates the checksum, and verifies the image after the write, so a bad download or the wrong disk doesn't turn into a broken card. It talks to armbian.com for board metadata, image lists, and board photos, and it can also flash a local image file you supply yourself.

## Features

- Works with 300+ boards, with filtering and board metadata from armbian.com
- Disk safety checks, checksum validation, and post-write verification
- Native builds for Linux, Windows, and macOS, on x64 and ARM64
- QDL (Qualcomm Device Loader) flashing path for EDL-based boards such as the Arduino UNO Q
- First-boot autoconfiguration written directly into an image's ext4 rootfs via the in-repo `armbian-write-conf` crate
- Multi-language interface in 18 languages that follows your system language by default
- Built-in application updater with signed updates

## Download

Prebuilt binaries are published on the [releases page](https://github.com/armbian/imager/releases).

| <a href="https://github.com/armbian/imager/releases"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/apple.svg" width="24"><br><strong>macOS</strong></a> | <a href="https://github.com/armbian/imager/releases"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/windows11.svg" width="24"><br><strong>Windows</strong></a> | <a href="https://github.com/armbian/imager/releases"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/linux.svg" width="24"><br><strong>Linux</strong></a> |
|:---:|:---:|:---:|
| Intel & Apple Silicon | x64 & ARM64 (code-signed) | x64 & ARM64 |
| `.dmg` | `.exe` / `.msi` | `.deb` / `.AppImage` |

## How It Works

1. **Pick a manufacturer.** Choose one of the supported SBC vendors, or load your own image file.
2. **Pick a board.** Boards show real photos and metadata from armbian.com.
3. **Pick an image.** Desktop or server, a kernel branch, and a stable, nightly, or rolling release build.
4. **Flash.** The app downloads, decompresses, writes, and verifies for you.

## Platform Support

| Platform | Architecture | Notes |
|----------|-------------|-------|
| macOS    | Intel x64        | Full support |
| macOS    | Apple Silicon    | Native ARM64 build |
| Windows  | x64              | Requires Administrator privileges |
| Windows  | ARM64            | Native ARM64 build, requires Administrator privileges |
| Linux    | x64              | Uses UDisks2/polkit for elevated device access |
| Linux    | ARM64            | Native ARM64 build |

### Supported Languages

English, Italian, German, French, Spanish, Portuguese, Portuguese (Brazil), Dutch, Polish, Russian, Chinese, Japanese, Korean, Ukrainian, Turkish, Slovenian, Swedish, Croatian, Greek, Serbian.

Translation files live in [`src/locales/`](src/locales/) and are kept aligned with `en.json` as the source of truth.

## Repository Layout

```
imager/
├── src/                          React 19 + TypeScript front end (Vite)
│   ├── components/               Flash UI, layout, modals, settings, shared widgets
│   ├── hooks/                    React hooks (Tauri IPC, flash lifecycle, settings…)
│   ├── contexts/                 Theme, motion, update contexts
│   ├── config/                   Constants, badges, i18n, OS info, QDL boards
│   ├── locales/                  Translations (20 language files)
│   ├── styles/                   CSS with design tokens
│   └── utils/, types/, assets/
├── src-tauri/                    Rust back end (Tauri 2)
│   ├── src/
│   │   ├── commands/             Tauri IPC command handlers
│   │   ├── devices/              Per-OS block device detection (linux/macos/windows)
│   │   ├── flash/                Per-OS writers + post-write verification
│   │   ├── qdl/                  Qualcomm EDL board support
│   │   ├── images/, paste/, config/, logging/, utils/
│   │   └── main.rs
│   ├── icons/                    App icons (desktop, iOS, Android)
│   ├── capabilities/             Tauri capability manifests
│   ├── packaging/copyright       Per-component license info
│   └── tauri.conf.json
├── crates/
│   └── armbian-write-conf/       In-repo Rust crate: inject first-boot config
│                                 into an image's ext4 rootfs, then validate
├── scripts/
│   ├── locales/sync-locales.js   AI-assisted locale sync helper
│   └── setup/                    Dev-environment bootstrap scripts
│                                 (install.sh, install-linux.sh,
│                                  install-macos.sh, install-windows.ps1)
├── public/                       Static assets served by Vite
└── .github/                      Issue templates, labels, workflows
```

## Tech Stack

- **Front end:** TypeScript, React 19, Vite 8, i18next, lucide-react, Tauri JS APIs
- **Back end:** Rust (edition 2021, MSRV 1.85.0), Tauri 2, Tokio, reqwest (rustls), serde
- **Decompression:** `lzma-rust2`, `xz2`, `bzip2`, `flate2`, `zstd`
- **Platform integrations:** `udisks2` + `zbus` on Linux; `security-framework` + `core-foundation` on macOS; `windows-sys` on Windows
- **EDL / QDL:** `qdl` (via `qdlrs`) + `nusb` for Qualcomm board flashing
- **Image writing:** in-repo `armbian-write-conf` crate (MIT) on top of `ext4-view`, `gptman`, `mbrman`
- **Linting:** ESLint 9 + typescript-eslint for the front end; `cargo fmt` + `cargo clippy` for Rust
- **Packaging formats:** `.deb` / `.AppImage` (Linux), `.dmg` (macOS), `.exe` / `.msi` via NSIS (Windows)

## Getting Started

Quick start:

```bash
git clone https://github.com/armbian/imager.git && cd imager
bash scripts/setup/install.sh
npm install
npm run tauri:dev
```

Requirements: Node.js ≥ 20.19.0, Rust ≥ 1.85.0 (stable toolchain), npm 10+. Platform-specific system dependencies (WebKitGTK on Linux, Xcode CLT on macOS, VS Build Tools + WebView2 on Windows) are installed by the scripts under [`scripts/setup/`](scripts/setup/).

Common npm scripts (from [`package.json`](package.json)):

| Command | Description |
|---------|-------------|
| `npm run dev`            | Vite dev server (front end only) |
| `npm run tauri:dev`      | Full app with hot reload (front end + Rust) |
| `npm run build`          | Production front-end build (`tsc -b && vite build`) |
| `npm run tauri:build`    | Production distributable |
| `npm run tauri:build:dev`| Debug build with symbols |
| `npm run lint`           | Run ESLint |
| `npm run clean`          | Remove `node_modules`, `dist`, and `src-tauri/target` |

Full setup, build, and architecture documentation lives in [DEVELOPMENT.md](DEVELOPMENT.md).

## Continuous Integration

Builds, lint/type checks, release automation, locale sync, and repository housekeeping all run on GitHub Actions. For an always-up-to-date overview of this repository's workflows and their status, see the Armbian CI dashboard:

<https://actions.armbian.com/?repo=imager>

## Why We Sign Our Code

Downloading software shouldn't take a leap of faith. Every Windows release is cryptographically signed, so you can confirm the binary is exactly what we built and hasn't been tampered with on the way to you.

This is possible thanks to [SignPath Foundation](https://signpath.org?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager), which gives free code-signing certificates to open-source projects, and [SignPath.io](https://signpath.io?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager) for the signing infrastructure.

## Contributing

Bug reports, feature ideas, translations, and pull requests are all welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow, branch and commit conventions, and the lint/format checks CI expects. All contributors are expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

Armbian Imager is distributed under the GNU General Public License, version 2 or (at your option) any later version. Version 2 is the floor, so the terms stay compatible with the [Armbian build framework](https://github.com/armbian/build).

`SPDX-License-Identifier: GPL-2.0-or-later`

The full text is in [LICENSE](LICENSE). A few bundled components keep their own terms: the `armbian-write-conf` crate is `MIT`, and the language-flag graphics from [Twemoji](https://github.com/jdecked/twemoji) are `CC-BY-4.0`. Per-file details are in [`src-tauri/packaging/copyright`](src-tauri/packaging/copyright).

---

<p align="center">
  <sub>Made with ❤️ by the Armbian community</sub>
</p>
