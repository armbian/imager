<h2 align="center">
  <a href=#><img src="https://raw.githubusercontent.com/armbian/.github/master/profile/logosmall.png" alt="Armbian logo"></a>
  <br><br>
</h2>

# Armbian Imager

## Purpose of This Repository

Armbian Imager is the official cross-platform desktop application for downloading, verifying, and flashing Armbian OS images to SD cards, USB drives, and EDL-based boards. This repository contains the Tauri 2 application: a React/TypeScript frontend and a Rust backend, plus the in-repo `armbian-write-conf` crate used to inject first-boot configuration into image rootfs.

### About

Armbian Imager downloads and flashes Armbian OS images to single-board computers. It checks the target disk before writing, validates the checksum, and verifies the image after the write, so a bad download or the wrong disk doesn't turn into a broken card.

### Features

- Works with 300+ boards, with filtering and board metadata from armbian.com
- Disk safety checks, checksum validation, and post-write verification
- Native builds for Linux, Windows, and macOS, on x64 and ARM64
- Multi-language interface that follows your system language by default
- Built-in application updates via the Tauri updater
- First-boot autoconfig profiles (user, access, network, region) written directly into the image's ext4 rootfs
- QDL (Qualcomm Device Loader) support for EDL-based boards such as Arduino UNO Q

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
| Linux | x64 | UDisks2/polkit for elevated device access |
| Linux | ARM64 | Native ARM64 build |

### Supported Languages

20 locales, each file lives under `src/locales/`:

English, Italian, German, French, Spanish, Portuguese, Portuguese (Brazil), Dutch, Polish, Russian, Chinese, Japanese, Korean, Ukrainian, Turkish, Slovenian, Swedish, Croatian, Greek, Serbian.

## Repository Layout

```
.
├── src/                     # React 19 + TypeScript frontend
│   ├── App.tsx
│   ├── main.tsx
│   ├── i18n.ts
│   ├── components/          # UI (autoconfig, dev, flash, layout, modals, settings, shared)
│   ├── config/              # Static config (i18n, filters, profiles, boards, ...)
│   ├── contexts/            # React contexts (Motion, Theme, Update)
│   ├── hooks/               # Custom hooks (Tauri IPC, async data, flash lifecycle, ...)
│   ├── locales/             # 20 translation JSON files
│   ├── styles/              # CSS
│   ├── types/               # Shared TypeScript types
│   └── utils/               # Frontend helpers
├── src-tauri/               # Tauri 2 / Rust backend
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/        # Tauri IPC command handlers
│   │   ├── config/
│   │   ├── devices/         # Linux/macOS/Windows device enumeration
│   │   ├── flash/           # Per-platform writers and verify
│   │   ├── images/          # Image models and filters
│   │   ├── qdl/             # Qualcomm Device Loader (EDL) flashing
│   │   ├── dev_scenarios/   # Debug-only simulated devices/network/images
│   │   ├── paste/           # paste.armbian.com log upload
│   │   └── utils/
│   ├── capabilities/        # Tauri capability manifests
│   ├── icons/               # App icons (desktop, Android, iOS)
│   ├── packaging/
│   ├── tauri.conf.json
│   ├── tauri.dev.conf.json
│   └── tauri.debug.conf.json
├── crates/
│   └── armbian-write-conf/  # Rust crate: write first-boot config into ext4 rootfs
├── scripts/
│   ├── locales/             # Locale sync + parity scripts (Node.js)
│   └── setup/               # Dev environment setup scripts
├── public/
├── index.html
├── package.json
├── vite.config.ts
├── eslint.config.js
├── tsconfig*.json
├── DEVELOPMENT.md
├── CONTRIBUTING.md
└── CODE_OF_CONDUCT.md
```

## Tech Stack

Built with:

- **Frontend:** React 19, TypeScript 5.9, Vite 8, i18next / react-i18next, lucide-react, qrcode
- **Backend:** Rust 2021 (MSRV 1.85), Tauri 2 with the `shell`, `dialog`, `updater`, `process`, `store`, and `mcp-bridge` plugins
- **Networking & I/O:** `reqwest` (rustls), `tokio`, `futures-util`
- **Decompression:** `lzma-rust2`, `xz2`, `bzip2`, `flate2`, `zstd`
- **Hashing:** `sha2`, `hex`
- **Device access:** Linux `libc` + `udisks2` + `zbus`; macOS `security-framework` + `core-foundation`; Windows `windows-sys`
- **QDL (EDL flashing):** [`qdlrs`](https://github.com/qualcomm/qdlrs), `nusb`
- **In-repo crate:** `armbian-write-conf` (ext4 writer, uses `gptman`, `mbrman`, `ext4-view`, and a vendored `armbian-ext4fs`)
- **Lint & format:** ESLint 9 with `typescript-eslint` and React hooks plugins; `cargo fmt` + `cargo clippy`
- **CI tooling:** Node.js scripts under `scripts/locales/`; shell/PowerShell setup scripts under `scripts/setup/`

## Development

Setup, build instructions, available scripts, and architecture details live in [DEVELOPMENT.md](DEVELOPMENT.md).

Quick start:

```bash
git clone https://github.com/armbian/imager.git && cd imager
bash scripts/setup/install.sh
npm install
npm run tauri:dev
```

Requirements (see [DEVELOPMENT.md](DEVELOPMENT.md) for the full list):

- Node.js ≥ 20.19.0 (from `package.json` `engines`)
- Rust stable, MSRV 1.85.0 (from `src-tauri/Cargo.toml`)
- Platform build tools: webkit2gtk/GTK on Linux, Xcode CLT on macOS, VS Build Tools + WebView2 on Windows

Common npm scripts:

| Command | Description |
|---------|-------------|
| `npm run dev` | Vite dev server (frontend only) |
| `npm run tauri:dev` | Full app with hot reload |
| `npm run build` | Production frontend build |
| `npm run tauri:build` | Production distributable |
| `npm run lint` | ESLint |
| `npm run clean` | Remove `node_modules`, `dist`, `src-tauri/target` |

## Contributing

Contributions are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Translations live in `src/locales/`; all 20 locale files must stay in parity with `en.json` (checked by `scripts/locales/check-parity.mjs` in CI).

## Continuous Integration

Build, lint, release, locale sync, and label maintenance are all handled by GitHub Actions. For an overview of workflow status for this repository, see:

<https://actions.armbian.com/?repo=imager>

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
