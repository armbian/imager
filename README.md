<h2 align="center">
  <a href=#><img src="https://raw.githubusercontent.com/armbian/.github/master/profile/logosmall.png" alt="Armbian logo"></a>
  <br><br>
</h2>

# Armbian Imager

## Purpose of This Repository

Armbian Imager is the official cross-platform desktop tool for downloading and flashing Armbian OS images to SD cards, USB drives, and EDL-mode boards. It takes care of target-disk safety checks, checksum validation, and post-write verification so a bad download or the wrong disk doesn't turn into a broken card.

## Features

- Works with 300+ boards, with filtering and board metadata from armbian.com
- Disk safety checks, checksum validation, and post-write verification
- First-boot autoconfig profiles (user, SSH keys, network, locale) injected directly into the image's ext4 rootfs
- QDL (Qualcomm Device Loader) support for EDL-based boards such as Arduino UNO Q
- Native builds for Linux, Windows, and macOS, on x64 and ARM64
- 20-language interface that follows your system language by default
- Built-in application updates

## Download

Prebuilt binaries for every supported platform are available on the [GitHub Releases page](https://github.com/armbian/imager/releases).

| <a href="https://github.com/armbian/imager/releases"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/apple.svg" width="24"><br><strong>macOS</strong></a> | <a href="https://github.com/armbian/imager/releases"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/windows11.svg" width="24"><br><strong>Windows</strong></a> | <a href="https://github.com/armbian/imager/releases"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/linux.svg" width="24"><br><strong>Linux</strong></a> |
|:---:|:---:|:---:|
| Intel & Apple Silicon | x64 & ARM64 (code-signed) | x64 & ARM64 |
| `.dmg` / `.app.zip` | `.exe` / `.msi` | `.deb` / `.AppImage` |

## How It Works

1. **Pick a manufacturer.** Choose a supported SBC vendor, or load your own image file.
2. **Pick a board.** Boards show real photos and metadata from armbian.com.
3. **Pick an image.** Desktop or server, a kernel branch, and a stable, nightly, or rolling build.
4. **Flash.** The app downloads, decompresses, writes, and verifies for you.

## Platform Support

| Platform | Architecture | Notes |
|----------|-------------|-------|
| macOS | Intel x64 | Full support |
| macOS | Apple Silicon | Native ARM64 build |
| Windows | x64 | Requires Administrator privileges, code-signed |
| Windows | ARM64 | Native ARM64 build, requires Administrator privileges |
| Linux | x64 | Uses `lsblk` for detection and UDisks2/polkit for elevated device access |
| Linux | ARM64 | Native ARM64 build |

### Supported Languages

English, Italian, German, French, Spanish, Portuguese, Portuguese (Brazil), Dutch, Polish, Russian, Chinese, Japanese, Korean, Ukrainian, Turkish, Slovenian, Swedish, Croatian, Greek, Serbian.

Translations live under [`src/locales/`](src/locales/); `en.json` is the source of truth and the other files are kept key-compatible with it (checked by `scripts/locales/check-parity.mjs`).

## Built With

- **Frontend:** React 19 + TypeScript, built with Vite 8, internationalised with `i18next` / `react-i18next`
- **Desktop shell:** [Tauri 2](https://tauri.app/) with the `dialog`, `shell`, `updater`, `process`, `store`, and `mcp-bridge` plugins
- **Backend:** Rust 2021 (MSRV 1.90), using `tokio`, `reqwest` (rustls), `serde`, `sha2`, and multi-threaded decompression (`lzma-rust2`, `xz2`, `bzip2`, `flate2`, `zstd`)
- **Platform integrations:** `udisks2` + `zbus` on Linux, Core Foundation on macOS, `windows-sys` on Windows
- **EDL / QDL flashing:** the `qdl` crate (via `nusb`) for Qualcomm Emergency Download mode
- **In-repo crate:** [`crates/armbian-write-conf`](crates/armbian-write-conf/) writes a small autoconfig file into an ext4 rootfs of a RAW disk image, in userspace (MIT-licensed)
- **Tooling:** ESLint 9 + `typescript-eslint`, `tsc` type-checking, `cargo fmt`, `cargo clippy`

## Repository Layout

```
.
├── src/                     # React 19 frontend (TypeScript)
│   ├── components/          # UI: layout, flash, settings, modals, shared, dev, autoconfig
│   ├── hooks/               # Custom React hooks (Tauri IPC, flash lifecycle, settings, …)
│   ├── config/              # Static config (i18n, image filters, board data, hero art, …)
│   ├── contexts/            # Theme, motion, update contexts
│   ├── locales/             # 20 translation JSON files
│   ├── styles/              # CSS modules
│   ├── utils/               # Helpers (color, storage, release notes, …)
│   └── assets/              # Images, OS logos, settings artwork
├── src-tauri/               # Tauri 2 backend (Rust)
│   ├── src/
│   │   ├── commands/        # Tauri IPC command handlers
│   │   ├── devices/         # Per-OS device enumeration (linux/macos/windows)
│   │   ├── flash/           # Per-OS writers, verification, privilege handling
│   │   ├── qdl/             # EDL/QDL board flashing
│   │   ├── images/          # Image catalog, filters, models
│   │   ├── dev_scenarios/   # Debug-only emulator (devices, flash, network, …)
│   │   ├── paste/           # paste.armbian.com log upload
│   │   └── utils/           # HTTP, path, progress, formatting helpers
│   ├── capabilities/        # Tauri capability manifests
│   ├── icons/               # App icons for every platform (incl. Android / iOS)
│   └── tauri.conf.json      # Tauri configuration (plus dev / debug variants)
├── crates/
│   └── armbian-write-conf/  # ext4 autoconfig writer crate (MIT)
├── scripts/
│   ├── locales/             # Locale sync + parity checks
│   └── setup/               # Dev-environment installers (Linux/macOS/Windows)
├── public/                  # Static frontend assets
└── .github/                 # Issue templates, labels, workflows
```

## Development

Full setup, build instructions, architecture notes, and troubleshooting are in [DEVELOPMENT.md](DEVELOPMENT.md). The short version:

```bash
git clone https://github.com/armbian/imager.git && cd imager
bash scripts/setup/install.sh   # installs system/toolchain deps
npm install
npm run tauri:dev
```

Requirements: Node.js ≥ 20.19.0, Rust ≥ 1.90 (edition 2021), plus the platform toolchain documented in `DEVELOPMENT.md`.

Common scripts:

| Command | Description |
|---------|-------------|
| `npm run dev` | Frontend only (Vite dev server) |
| `npm run tauri:dev` | Full app with hot reload (frontend + Rust) |
| `npm run build` | Production frontend build (`tsc -b && vite build`) |
| `npm run tauri:build` | Production desktop bundle |
| `npm run lint` | ESLint over the frontend |
| `npm run clean` | Remove `node_modules`, `dist`, and `src-tauri/target` |

Debug builds expose a **Dev Scenarios** panel (flask icon / `Cmd/Ctrl+Shift+D`) that drives a backend emulator for fake devices, flash outcomes, and network faults. It is stripped from release builds and verified by CI.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow (branch naming, Conventional Commits, quality checks) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community expectations.

Other ways to help:

- [Become a new board maintainer](https://docs.armbian.com/Board_Maintainers_Procedures_and_Guidelines/)
- [Apply for an open position](https://forum.armbian.com/staffapplications/)
- [Help cover running costs](https://forum.armbian.com/subscriptions/)
- [Help community members in the forum](https://forum.armbian.com/)

## Continuous Integration

All automated builds, release pipelines, locale sync, and maintenance jobs run on GitHub Actions. For a live overview of this repository's workflows and their status, see the [Armbian CI overview for `imager`](https://actions.armbian.com/?repo=imager).

## Why We Sign Our Code

Downloading software shouldn't take a leap of faith. Every Windows release is cryptographically signed, so you can confirm the binary is exactly what we built and hasn't been tampered with on the way to you.

This is possible thanks to [SignPath Foundation](https://signpath.org?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager), which gives free code signing certificates to open source projects, and [SignPath.io](https://signpath.io?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager) for the signing infrastructure.

## License

Armbian Imager is distributed under the GNU General Public License, either version 2 or (at your option) any later version. Version 2 is the floor, so the terms stay compatible with the [Armbian build framework](https://github.com/armbian/build).

SPDX-License-Identifier: `GPL-2.0-or-later`

The full text is in [LICENSE](LICENSE). A few bundled components keep their own terms: the `armbian-write-conf` crate (and its vendored `armbian-ext4fs` fork) is `MIT`, and the language flag graphics from [Twemoji](https://github.com/jdecked/twemoji) are `CC-BY-4.0`. Per-file details are in [`src-tauri/packaging/copyright`](src-tauri/packaging/copyright).

---

<p align="center">
  <sub>Made with ❤️ by the Armbian community</sub>
</p>
