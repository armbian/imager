<h2 align="center">
  <a href=#><img src="https://raw.githubusercontent.com/armbian/.github/master/profile/logosmall.png" alt="Armbian logo"></a>
  <br><br>
</h2>

# Armbian Imager

## Purpose of This Repository

Armbian Imager is the official desktop application for downloading and flashing Armbian OS images to SD cards and USB drives on single-board computers. It checks the target disk before writing, validates the checksum, and verifies the image after the write, so a bad download or the wrong disk doesn't turn into a broken card.

## Features

- Works with 300+ boards, with filtering and board metadata from armbian.com
- Disk safety checks, checksum validation, and post-write verification
- First-boot autoconfig: writes a configuration file into the image's ext4 rootfs before flashing
- QDL (Qualcomm Device Loader) support for EDL-based boards (e.g. Arduino UNO Q)
- Native builds for Linux, Windows, and macOS, on x64 and ARM64
- Multi-language interface (20 languages) that follows your system language by default
- Built-in application updates

## Testimonials

> "What a fantastic tool for getting people started with a non Raspberry PI"
> *Interfacing Linux* ([source](https://www.youtube.com/watch?v=RAxQebKsnuc))

> "A proper multi-platform desktop app that actually works, which is rarer than you'd think."
> *Bruno Verachten*, Senior Developer Relations Engineer ([source](https://www.linkedin.com/pulse/adding-risc-v-support-armbian-imager-tale-qemu-tauri-deja-verachten-86fxe))

> "The Upcoming Armbian Imager Tool is a Godsend for Non-Raspberry Pi SBC Owners"
> *Sourav Rudra*, It's FOSS ([source](https://itsfoss.com/news/armbian-imager-quietly-debuts/))

> "According to Armbian, this results in less RAM and storage usage and a faster experience."
> *Jordan Gloor*, HowtoGeek.com ([source](https://www.howtogeek.com/armbians-raspberry-pi-imager-alternative-is-here/))

> "It's super easy to write an operating system... I'm always happy when an Armbian version comes out because you've got more stability and much more compatibility."
> *leepspvideo* ([source](https://www.youtube.com/watch?v=vUvGD2GSALI))

## Download

Prebuilt binaries are available for every supported platform from the [Releases page](https://github.com/armbian/imager/releases).

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
|----------|-------------|-------|
| macOS | Intel x64 | Full support |
| macOS | Apple Silicon | Native ARM64 build |
| Windows | x64 | Requires Administrator privileges |
| Windows | ARM64 | Native ARM64 build, requires Administrator privileges |
| Linux | x64 | Uses UDisks2/polkit for elevated device access |
| Linux | ARM64 | Native ARM64 build |

### Supported Languages

English, Italian, German, French, Spanish, Portuguese, Portuguese (Brazil), Dutch, Polish, Russian, Chinese, Japanese, Korean, Ukrainian, Turkish, Slovenian, Swedish, Croatian, Greek, Serbian.

## Repository Layout

```
imager/
├── src/                        # React 19 + TypeScript frontend (Vite)
│   ├── components/             # UI: autoconfig, dev, flash, layout, modals, settings, shared
│   ├── hooks/                  # Custom React hooks (Tauri IPC, flash lifecycle, settings, …)
│   ├── contexts/               # React contexts (theme, motion, updates)
│   ├── config/                 # App config and metadata (i18n, boards, filters, …)
│   ├── locales/                # 20 translation JSON files
│   ├── styles/                 # CSS
│   ├── utils/                  # Frontend utilities
│   └── assets/                 # Logos, OS icons, settings artwork
├── src-tauri/                  # Tauri 2 backend (Rust)
│   ├── src/
│   │   ├── commands/           # Tauri command handlers
│   │   ├── devices/            # Platform-specific device enumeration (linux/macos/windows)
│   │   ├── flash/              # Flash writers, verification, privilege handling
│   │   ├── qdl/                # Qualcomm EDL board support
│   │   ├── dev_scenarios/      # Debug-only device/network/flash emulator
│   │   ├── images/             # Image models and filters
│   │   └── …                   # cache, download, decompress, logging, paste upload, …
│   ├── capabilities/           # Tauri capability definitions
│   ├── icons/                  # App icons (desktop, iOS, Android)
│   ├── packaging/copyright     # Per-file licensing details
│   └── tauri.conf.json         # Tauri configuration (plus .dev / .debug variants)
├── crates/
│   └── armbian-write-conf/     # Rust crate: writes first-boot config into ext4 rootfs of raw image
├── scripts/
│   ├── locales/                # Locale parity check and AI-assisted sync
│   └── setup/                  # Install helpers (install.sh, install-linux.sh, install-macos.sh, install-windows.ps1)
├── public/                     # Static web assets
└── .github/                    # Issue templates, labels, workflows
```

## Tech Stack

- **Frontend:** React 19, TypeScript, Vite 8, i18next, Lucide icons
- **Backend/Shell:** Rust (edition 2021, MSRV 1.85.0), Tauri 2 with plugins for `shell`, `dialog`, `updater`, `process`, `store`, and `mcp-bridge`
- **Networking & I/O:** `reqwest` (rustls), `tokio`, `futures-util`
- **Decompression:** `lzma-rust2`, `xz2`, `bzip2`, `flate2`, `zstd`
- **Platform integration:**
  - Linux: `libc`, `udisks2`, `zbus`
  - macOS: `security-framework`, `core-foundation`
  - Windows: `windows-sys` (file system, I/O control)
- **QDL / EDL:** `qdl` (qdlrs) and `nusb`
- **Image writing:** in-repo [`armbian-write-conf`](crates/armbian-write-conf/) crate, using `gptman`, `mbrman`, and `ext4-view`
- **Tooling:** ESLint 9, `typescript-eslint`, `cargo fmt`, `cargo clippy`

## Development

Full setup, build instructions, and architecture notes live in [DEVELOPMENT.md](DEVELOPMENT.md). Quick start:

```bash
git clone https://github.com/armbian/imager.git && cd imager
bash scripts/setup/install.sh
npm install
npm run tauri:dev
```

Common scripts (see [`package.json`](package.json)):

| Command | Description |
|---------|-------------|
| `npm run dev` | Frontend-only Vite dev server |
| `npm run tauri:dev` | Full app with hot reload (frontend + Rust) |
| `npm run build` | Production frontend build |
| `npm run tauri:build` | Production distributable |
| `npm run lint` | Run ESLint |
| `npm run clean` | Remove `node_modules`, `dist`, and `src-tauri/target` |

Prerequisites: Node.js ≥ 20.19.0, Rust ≥ 1.85.0, and the platform-specific system packages listed in [DEVELOPMENT.md](DEVELOPMENT.md).

## Contributing

Contributions are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow, commit style, and quality checks, and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community expectations.

When adding or removing a translation key, update all 20 files under [`src/locales/`](src/locales/) so they keep the same set of keys. The repository ships a parity check at [`scripts/locales/check-parity.mjs`](scripts/locales/check-parity.mjs) that runs in CI.

## Continuous Integration

Builds, PR checks, release automation, label and locale sync all run from GitHub Actions. For a live overview of every workflow and recent runs for this repository, see:

👉 **[Armbian CI overview for `imager`](https://actions.armbian.com/?repo=imager)**

## Why We Sign Our Code

Downloading software shouldn't take a leap of faith. Every Windows release is cryptographically signed, so you can confirm the binary is exactly what we built and hasn't been tampered with on the way to you.

This is possible thanks to [SignPath Foundation](https://signpath.org?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager), which gives free code signing certificates to open source projects, and [SignPath.io](https://signpath.io?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager) for the signing infrastructure.

## License

Armbian Imager is distributed under the GNU General Public License, either version 2 or (at your option) any later version. Version 2 is the floor, so the terms stay compatible with the [Armbian build framework](https://github.com/armbian/build).

SPDX-License-Identifier: `GPL-2.0-or-later`

The full text is in [LICENSE](LICENSE). A few bundled components keep their own terms: the [`armbian-write-conf`](crates/armbian-write-conf/) crate is `MIT`, and additional per-file licensing details are in [`src-tauri/packaging/copyright`](src-tauri/packaging/copyright).

---

<p align="center">
  <sub>Made with ❤️ by the Armbian community</sub>
</p>
