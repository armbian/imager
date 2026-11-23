<h2 align="center">
  <a href=#><img src="https://raw.githubusercontent.com/armbian/.github/master/profile/logosmall.png" alt="Armbian logo"></a>
  <br><br>
</h2>

# Armbian Imager

## Purpose of This Repository

Armbian Imager is the official cross-platform desktop application for downloading and flashing Armbian OS images to SD cards and USB drives. It checks the target disk before writing, validates the checksum, and verifies the image after the write, so a bad download or the wrong disk doesn't turn into a broken card.

## Features

- Works with 300+ boards, with filtering and board metadata from armbian.com
- Disk safety checks, checksum validation, and post-write verification
- Native builds for Linux, Windows, and macOS, on x64 and ARM64
- Multi-language interface that follows your system language by default (20 languages)
- Built-in application updates
- First-boot profile editor (user, locale, network, SSH keys) that writes directly into the image's ext4 rootfs
- Small binary with few runtime dependencies

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
| Linux | x64 | Uses lsblk for detection and UDisks2/polkit for elevated device access |
| Linux | ARM64 | Native ARM64 build |

### Supported Languages

English, Italian, German, French, Spanish, Portuguese, Portuguese (Brazil), Dutch, Polish, Russian, Chinese, Japanese, Korean, Ukrainian, Turkish, Slovenian, Swedish, Croatian, Greek, Serbian.

Translations live in [`src/locales/`](src/locales/) and are kept in sync with `en.json` via a scheduled job and the `scripts/locales/` helpers.

## Built With

The app is a [Tauri 2](https://tauri.app) desktop application: a Rust backend driving a React frontend inside the system webview.

- **Frontend:** TypeScript, React 19, Vite 8, i18next, ESLint. Entry point [`src/main.tsx`](src/main.tsx), app shell [`src/App.tsx`](src/App.tsx).
- **Backend:** Rust (edition 2021, MSRV 1.90), Tauri 2 with the `shell`, `dialog`, `updater`, `process`, and `store` plugins. Entry point [`src-tauri/src/main.rs`](src-tauri/src/main.rs).
- **Flashing:** multi-threaded decompression (xz, bzip2, gzip, zstd), SHA verification, and per-platform writers under [`src-tauri/src/flash/`](src-tauri/src/flash/) (Linux via UDisks2/polkit, macOS via Authorization Services, Windows via `windows-sys`).
- **Qualcomm EDL support:** [`qdl`](https://github.com/qualcomm/qdlrs) for flashing boards such as the Arduino UNO Q, wired up under [`src-tauri/src/qdl/`](src-tauri/src/qdl/).
- **First-boot config:** in-repo crate [`crates/armbian-write-conf/`](crates/armbian-write-conf/) writes a small config file into the ext4 rootfs of a RAW disk image from userspace, then validates it.
- **Shell:** setup helpers in [`scripts/setup/`](scripts/setup/) for Linux, macOS, and Windows (PowerShell).

## Repository Layout

```
.
├── src/                       # React 19 + TypeScript frontend
│   ├── components/            #   UI (layout, flash, settings, modals, shared, autoconfig, dev)
│   ├── hooks/                 #   React hooks, including Tauri IPC wrappers
│   ├── config/                #   Static config (i18n, badges, OS info, QDL boards, ...)
│   ├── contexts/              #   Theme, Motion, Update contexts
│   ├── locales/               #   20 translation JSON files
│   ├── styles/                #   CSS modules
│   └── utils/                 #   Shared helpers
├── src-tauri/                 # Rust backend (Tauri 2)
│   ├── src/commands/          #   IPC commands exposed to the frontend
│   ├── src/flash/             #   Platform-specific writers (linux, macos, windows) + verify
│   ├── src/devices/           #   Device enumeration per OS
│   ├── src/qdl/               #   Qualcomm EDL flashing
│   ├── src/dev_scenarios/     #   Debug-only emulator for devices, flashes, network
│   ├── src/images/            #   Image listing, filters, models
│   ├── capabilities/          #   Tauri capability files
│   ├── icons/                 #   App icons for all platforms
│   └── tauri.conf.json        #   Tauri configuration
├── crates/armbian-write-conf/ # Userspace ext4 writer for first-boot autoconfig
├── scripts/
│   ├── locales/               #   check-parity.mjs, sync-locales.js
│   └── setup/                 #   install-linux.sh, install-macos.sh, install-windows.ps1, install.sh
├── public/                    # Static assets served by Vite
└── index.html                 # Vite entry HTML
```

## Development

Setup, build instructions, and architecture notes live in [DEVELOPMENT.md](DEVELOPMENT.md). The short version:

```bash
git clone https://github.com/armbian/imager.git && cd imager
bash scripts/setup/install.sh
npm install
npm run tauri:dev
```

Prerequisites: Node.js ≥ 20.19.0 and a stable Rust toolchain. Platform-specific system packages (WebKitGTK on Linux, Xcode Command Line Tools on macOS, Visual Studio Build Tools + WebView2 on Windows) are documented in [DEVELOPMENT.md](DEVELOPMENT.md).

Common scripts from [`package.json`](package.json):

| Command | Description |
|---------|-------------|
| `npm run dev` | Frontend only (Vite dev server) |
| `npm run tauri:dev` | Full app with hot reload (frontend + Rust) |
| `npm run build` | Production frontend build |
| `npm run tauri:build` | Production distributable |
| `npm run lint` | Run ESLint |
| `npm run clean` | Remove `node_modules`, `dist`, and `src-tauri/target` |

Before sending a PR, run the same checks CI runs (ESLint, `tsc --noEmit`, locale parity, `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`). See [CONTRIBUTING.md](CONTRIBUTING.md) for the details.

## Continuous Integration

All workflows under [`.github/workflows/`](.github/workflows/) build, test, release, and sync translations for this repository. For a live overview of workflow status and recent runs, see the Armbian CI dashboard:

**[actions.armbian.com/?repo=imager](https://actions.armbian.com/?repo=imager)**

## Why We Sign Our Code

Downloading software shouldn't take a leap of faith. Every Windows release is cryptographically signed, so you can confirm the binary is exactly what we built and hasn't been tampered with on the way to you.

This is possible thanks to [SignPath Foundation](https://signpath.org?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager), which gives free code signing certificates to open source projects, and [SignPath.io](https://signpath.io?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager) for the signing infrastructure.

## Contributing

Bug reports, feature requests, pull requests, and translation updates are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for our community guidelines.

## License

Armbian Imager is distributed under the GNU General Public License, either version 2 or (at your option) any later version. Version 2 is the floor, so the terms stay compatible with the [Armbian build framework](https://github.com/armbian/build).

SPDX-License-Identifier: `GPL-2.0-or-later`

The full text is in [LICENSE](LICENSE). A few bundled components keep their own terms: the `armbian-write-conf` crate and its vendored `armbian-ext4fs` fork are `MIT`, and the language flag graphics from [Twemoji](https://github.com/jdecked/twemoji) are `CC-BY-4.0`. Per-file details are in [`src-tauri/packaging/copyright`](src-tauri/packaging/copyright).

---

<p align="center">
  <sub>Made with ❤️ by the Armbian community</sub>
</p>
