<h2 align="center">
  <a href=#><img src="https://raw.githubusercontent.com/armbian/.github/master/profile/logosmall.png" alt="Armbian logo"></a>
  <br><br>
</h2>

# Armbian Imager

## Purpose of This Repository

Armbian Imager is the official desktop application for downloading, verifying, and flashing Armbian OS images to SD cards, USB drives, and EDL-based boards. It provides a native cross-platform GUI backed by a Rust core that handles device discovery, image decompression, writing, and post-write verification.

## About

Armbian Imager checks the target disk before writing, validates the download's checksum, and verifies the image after the write, so a bad download or the wrong disk doesn't turn into a broken card. It covers 300+ boards with metadata and photos pulled from armbian.com.

## Features

- Works with 300+ boards, with filtering and board metadata from armbian.com
- Disk safety checks, checksum validation, and post-write verification
- Native builds for Linux, Windows, and macOS, on x64 and ARM64
- Code-signed Windows releases (thanks to [SignPath Foundation](https://signpath.org))
- Multi-language interface (20 languages) that follows your system language
- Built-in application updates via the Tauri updater
- First-boot autoconfig: pre-fills user, SSH keys, Wi-Fi and locale into the ext4 rootfs of a RAW image before flashing
- QDL (Qualcomm EDL) flashing path for boards like Arduino UNO Q

## Download

Prebuilt binaries are available on the [Releases](https://github.com/armbian/imager/releases) page.

| <a href="https://github.com/armbian/imager/releases"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/apple.svg" width="24"><br><strong>macOS</strong></a> | <a href="https://github.com/armbian/imager/releases"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/windows11.svg" width="24"><br><strong>Windows</strong></a> | <a href="https://github.com/armbian/imager/releases"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/linux.svg" width="24"><br><strong>Linux</strong></a> |
|:---:|:---:|:---:|
| Intel & Apple Silicon | x64 & ARM64 (code-signed) | x64 & ARM64 |
| `.dmg` / `.app.zip` | `.exe` / `.msi` | `.deb` / `.AppImage` |

## How It Works

1. **Pick a manufacturer.** Choose one of the supported SBC vendors, or load your own image file.
2. **Pick a board.** Boards show real photos and metadata from armbian.com.
3. **Pick an image.** Desktop or server, a kernel branch, and a stable, nightly, or rolling release build.
4. **Flash.** The app downloads, decompresses, writes, and verifies for you.

## Platform Support

| Platform | Architecture | Notes |
|----------|--------------|-------|
| macOS | Intel x64 | Full support |
| macOS | Apple Silicon | Native ARM64 build |
| Windows | x64 | Requires Administrator privileges |
| Windows | ARM64 | Native ARM64 build, requires Administrator |
| Linux | x64 | Uses `lsblk` for detection and UDisks2/polkit for elevated device access |
| Linux | ARM64 | Native ARM64 build |

### Supported Languages

English, Italian, German, French, Spanish, Portuguese, Portuguese (Brazil), Dutch, Polish, Russian, Chinese, Japanese, Korean, Ukrainian, Turkish, Slovenian, Swedish, Croatian, Greek, Serbian. Translation files live in [`src/locales/`](src/locales/).

## Tech Stack

- **Frontend:** React 19 + TypeScript, bundled with Vite 8, internationalised with i18next
- **Backend / shell:** [Tauri 2](https://tauri.app) in Rust (edition 2021, MSRV 1.85.0)
- **Decompression:** `xz2`, `lzma-rust2`, `bzip2`, `flate2`, `zstd`
- **Device access:** `windows-sys` on Windows; `libc` + `udisks2`/`zbus` on Linux; `security-framework` + `core-foundation` on macOS
- **QDL flashing:** [`qdl`](https://github.com/qualcomm/qdlrs) + `nusb`
- **In-repo crate:** [`armbian-write-conf`](crates/armbian-write-conf/) — writes a config file into the ext4 rootfs of a RAW disk image in userspace, with partition parsing via `gptman` / `mbrman` and ext4 validation via `ext4-view`
- **Tooling:** ESLint 9, `typescript-eslint`, `cargo fmt`, `cargo clippy`

## Repository Layout

```
.
├── src/                         # React frontend (TypeScript)
│   ├── components/              # UI components (flash/, settings/, modals/, layout/, shared/, autoconfig/, dev/)
│   ├── hooks/                   # Custom React hooks
│   ├── contexts/                # Theme, motion, and update React contexts
│   ├── config/                  # Static config (i18n, boards, filters, hero art, ...)
│   ├── locales/                 # 20 translation files (en.json is the source of truth)
│   ├── styles/                  # CSS
│   ├── utils/                   # Shared utilities
│   └── App.tsx, main.tsx, i18n.ts
├── src-tauri/                   # Tauri shell (Rust)
│   ├── src/
│   │   ├── commands/            # Tauri IPC commands exposed to the frontend
│   │   ├── flash/               # Per-OS flashing (linux/, macos/, windows.rs) + verify
│   │   ├── devices/             # Device enumeration per OS
│   │   ├── decompress.rs, download.rs, cache.rs
│   │   ├── qdl/                 # Qualcomm EDL flashing
│   │   ├── dev_scenarios/       # Debug-only emulator (devices, flash, network, vdisks)
│   │   ├── images/, paste/, logging/, utils/
│   │   └── main.rs
│   ├── capabilities/            # Tauri capability manifests
│   ├── icons/                   # App icons (desktop, Android, iOS)
│   ├── packaging/copyright      # Per-file licence breakdown
│   └── tauri.conf.json, tauri.dev.conf.json, tauri.debug.conf.json
├── crates/
│   └── armbian-write-conf/      # Ext4 rootfs config writer + tests
├── scripts/
│   ├── setup/                   # install.sh, install-linux.sh, install-macos.sh, install-windows.ps1
│   └── locales/                 # check-parity.mjs, sync-locales.js
├── public/                      # Static assets
├── index.html, vite.config.ts, eslint.config.js
├── tsconfig.json, tsconfig.app.json, tsconfig.node.json
├── package.json
└── .github/                     # Issue templates, labels, workflows
```

## Development

Full setup, build instructions, and architecture notes live in [DEVELOPMENT.md](DEVELOPMENT.md). The short version:

```bash
git clone https://github.com/armbian/imager.git && cd imager
bash scripts/setup/install.sh
npm install
npm run tauri:dev
```

Prerequisites: Node.js ≥ 20.19.0, Rust (stable, ≥ 1.85.0 for the Tauri crate), and the platform-specific system packages listed in [DEVELOPMENT.md](DEVELOPMENT.md).

### Common scripts

| Command | What it does |
|---------|--------------|
| `npm run dev` | Vite dev server, frontend only |
| `npm run tauri:dev` | Full app with hot reload (debug build; includes dev scenarios panel) |
| `npm run build` | Production frontend build (`tsc -b && vite build`) |
| `npm run tauri:build` | Production distributable |
| `npm run lint` | ESLint over the frontend |
| `npm run clean` | Remove `node_modules`, `dist`, and `src-tauri/target` |

### Quality checks

Run these locally before opening a PR; CI enforces them with zero warnings:

```bash
npm run lint
npx tsc --noEmit -p tsconfig.app.json
node scripts/locales/check-parity.mjs

cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

## Continuous Integration

CI covers PR checks, test builds, releases, scheduled locale sync and release pruning. See the live overview at [actions.armbian.com/?repo=imager](https://actions.armbian.com/?repo=imager).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow, branch naming, Conventional Commit scopes, and translation guidelines. All participants are expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

Related reading:
- [Armbian documentation](https://docs.armbian.com)
- [Armbian build framework](https://github.com/armbian/build)
- [Armbian forum](https://forum.armbian.com/)

## Why We Sign Our Code

Downloading software shouldn't take a leap of faith. Every Windows release is cryptographically signed, so you can confirm the binary is exactly what we built and hasn't been tampered with on the way to you.

This is possible thanks to [SignPath Foundation](https://signpath.org?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager), which gives free code signing certificates to open source projects, and [SignPath.io](https://signpath.io?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager) for the signing infrastructure.

## License

Armbian Imager is distributed under the GNU General Public License, either version 2 or (at your option) any later version. Version 2 is the floor, so the terms stay compatible with the [Armbian build framework](https://github.com/armbian/build).

SPDX-License-Identifier: `GPL-2.0-or-later`

The full text is in [LICENSE](LICENSE). A few bundled components keep their own terms: the `armbian-write-conf` crate and its vendored `armbian-ext4fs` fork are `MIT`, and the language flag graphics from [Twemoji](https://github.com/jdecked/twemoji) are `CC-BY-4.0`. Per-file details are in [`src-tauri/packaging/copyright`](src-tauri/packaging/copyright).

---

<p align="center">
  <sub>Made with ❤️ by the Armbian community</sub>
</p>
