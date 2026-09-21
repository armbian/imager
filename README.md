<h2 align="center">
  <a href=#><img src="https://raw.githubusercontent.com/armbian/.github/master/profile/logosmall.png" alt="Armbian logo"></a>
  <br><br>
</h2>

# Armbian Imager

## Purpose of This Repository

Armbian Imager is the official cross-platform desktop tool for downloading and flashing Armbian OS images to SD cards and USB drives. It handles vendor and board discovery, image download and decompression, safe writing to the target disk, and post-write verification.

## About

Armbian Imager checks the target disk before writing, validates the checksum, and verifies the image after the write, so a bad download or the wrong disk doesn't turn into a broken card.

## Features

- Works with 300+ boards, with filtering and board metadata from armbian.com
- Disk safety checks, checksum validation, and post-write verification
- Native builds for Linux, Windows, and macOS, on x64 and ARM64
- Multi-language interface (18 languages) that follows your system language by default
- Built-in application updates via the Tauri updater
- QDL (Qualcomm Device Loader) support for EDL-based boards (e.g. Arduino UNO Q)
- First-boot autoconfig injection into an image's ext4 rootfs before writing

## Testimonials

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
- Language: 18 languages, auto-detected from your system

## Platform Support

| Platform | Architecture | Notes |
|----------|--------------|-------|
| macOS    | Intel x64    | Full support |
| macOS    | Apple Silicon | Native ARM64 build, Touch ID support |
| Windows  | x64          | Requires Administrator privileges |
| Windows  | ARM64        | Native ARM64 build, requires Administrator privileges |
| Linux    | x64          | Uses `lsblk` for detection and UDisks2/polkit for elevated device access |
| Linux    | ARM64        | Native ARM64 build |

### Supported Languages

English, Italian, German, French, Spanish, Portuguese, Portuguese (Brazil), Dutch, Polish, Russian, Chinese, Japanese, Korean, Ukrainian, Turkish, Slovenian, Swedish, Croatian.

## Tech Stack

Armbian Imager is a [Tauri 2](https://tauri.app) desktop application.

- **Frontend:** TypeScript + React 19, built with Vite; internationalization via `i18next` / `react-i18next`; icons from `lucide-react`. Linted with ESLint (`typescript-eslint`, React Hooks rules).
- **Backend:** Rust (edition 2021, MSRV 1.85). Uses `tokio` for async, `reqwest` (rustls) for downloads, `sha2` for checksums, and multi-threaded decompression via `lzma-rust2`, `xz2`, `bzip2`, `flate2`, and `zstd`. Platform integration through `udisks2`/`zbus` on Linux, `security-framework`/`core-foundation` on macOS, and `windows-sys` on Windows.
- **QDL:** EDL-based flashing via the `qdl` crate and `nusb`.
- **In-repo crate:** [`armbian-write-conf`](crates/armbian-write-conf/) — writes a first-boot config file into an ext4 rootfs of a RAW disk image, in userspace, then validates the result (MIT-licensed).

## Repository Layout

```
.
├── crates/
│   └── armbian-write-conf/   # Rust crate: inject autoconfig into ext4 rootfs
├── public/                   # Static assets served by Vite
├── scripts/
│   ├── locales/              # Locale sync helper (Node.js)
│   └── setup/                # Per-OS dev environment installers
├── src/                      # React 19 + TypeScript frontend
│   ├── components/           # UI: flash, layout, modals, settings, shared
│   ├── config/               # App constants, i18n config, board/OS metadata
│   ├── contexts/             # React contexts (theme, motion, update)
│   ├── hooks/                # Custom React hooks (Tauri IPC, settings, ...)
│   ├── locales/              # Translation JSON files (18 languages)
│   ├── styles/               # CSS with design tokens
│   ├── types/                # Shared TypeScript types
│   └── utils/                # Frontend utilities
└── src-tauri/                # Rust backend + Tauri app configuration
    ├── capabilities/         # Tauri capability manifests
    ├── icons/                # App icons (desktop, Android, iOS)
    ├── packaging/            # Packaging metadata (copyright, ...)
    └── src/                  # Rust sources
        ├── commands/         # Tauri IPC command handlers
        ├── config/           # Backend configuration
        ├── devices/          # Per-OS block-device enumeration
        ├── flash/            # Per-OS writer, verify, privilege escalation
        ├── images/           # Image catalog models and filters
        ├── qdl/              # Qualcomm EDL flashing pipeline
        ├── logging/          # Structured logging
        ├── paste/            # Log upload to paste.armbian.com
        └── utils/            # HTTP, path, format, progress helpers
```

## Development

Setup, build instructions, and a deeper architecture walkthrough live in [DEVELOPMENT.md](DEVELOPMENT.md).

Quick start:

```bash
git clone https://github.com/armbian/imager.git && cd imager
bash scripts/setup/install.sh
npm install
npm run tauri:dev
```

The `scripts/setup/` directory also contains platform-specific installers: [`install-linux.sh`](scripts/setup/install-linux.sh), [`install-macos.sh`](scripts/setup/install-macos.sh), and [`install-windows.ps1`](scripts/setup/install-windows.ps1).

Common npm scripts (from `package.json`):

| Command | Description |
|---------|-------------|
| `npm run dev`             | Frontend only (Vite dev server) |
| `npm run tauri:dev`       | Full app with hot reload (frontend + Rust) |
| `npm run build`           | Production frontend build |
| `npm run tauri:build`     | Production distributable |
| `npm run tauri:build:dev` | Debug build with symbols |
| `npm run lint`            | Run ESLint |
| `npm run clean`           | Remove `node_modules`, `dist`, and `src-tauri/target` |

Requires Node.js ≥ 20.19.0 and Rust ≥ 1.85.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow, commit conventions, and required quality checks. All contributors are expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Continuous Integration

Build, release, and maintenance pipelines run on GitHub Actions. For a live overview of workflows and their status, see the Armbian CI dashboard for this repository:

<https://actions.armbian.com/?repo=imager>

## Why We Sign Our Code

Downloading software shouldn't take a leap of faith. Every Windows release is cryptographically signed, so you can confirm the binary is exactly what we built and hasn't been tampered with on the way to you.

This is possible thanks to [SignPath Foundation](https://signpath.org?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager), which gives free code signing certificates to open source projects, and [SignPath.io](https://signpath.io?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager) for the signing infrastructure.

## License

Armbian Imager is distributed under the GNU General Public License, either version 2 or (at your option) any later version. Version 2 is the floor, so the terms stay compatible with the [Armbian build framework](https://github.com/armbian/build).

SPDX-License-Identifier: `GPL-2.0-or-later`

The full text is in [LICENSE](LICENSE). A few bundled components keep their own terms: the `armbian-write-conf` crate is `MIT`, and the language flag graphics from [Twemoji](https://github.com/jdecked/twemoji) are `CC-BY-4.0`. Per-file details are in [`src-tauri/packaging/copyright`](src-tauri/packaging/copyright).

---

<p align="center">
  <sub>Made with ❤️ by the Armbian community</sub>
</p>
