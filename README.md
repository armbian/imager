<h2 align="center">
  <a href=#><img src="https://raw.githubusercontent.com/armbian/.github/master/profile/logosmall.png" alt="Armbian logo"></a>
  <br><br>
</h2>

# Armbian Imager

## Purpose of This Repository

Armbian Imager is the official cross-platform desktop application for downloading, verifying, and flashing Armbian OS images to SD cards, USB drives, and EDL-capable boards. This repository contains the full source for the app: a React/TypeScript frontend, a Rust/Tauri backend, and a companion Rust crate that writes first-boot configuration into an image's ext4 rootfs.

## Features

- Supports 300+ boards with vendor filtering and board metadata from armbian.com
- Disk safety checks, SHA checksum validation, and post-write image verification
- Native builds for Linux, Windows, and macOS, on x64 and ARM64
- Multi-threaded decompression (xz, bzip2, gzip, zstd)
- QDL / EDL flashing path for Qualcomm-based boards
- First-boot profiles: user, locale, network (incl. Wi-Fi), SSH keys
- Built-in application updates (signed via the Tauri updater)
- Localized UI in 20 languages, auto-detected from the system

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

Prebuilt binaries are available on the [releases page](https://github.com/armbian/imager/releases).

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
| macOS | Intel x64 | Full support |
| macOS | Apple Silicon | Native ARM64 build |
| Windows | x64 | Requires Administrator privileges |
| Windows | ARM64 | Native ARM64 build, requires Administrator privileges |
| Linux | x64 | Uses UDisks2/polkit for elevated device access |
| Linux | ARM64 | Native ARM64 build |

### Supported Languages

English, Italian, German, French, Spanish, Portuguese, Portuguese (Brazil), Dutch, Polish, Russian, Chinese, Japanese, Korean, Ukrainian, Turkish, Slovenian, Swedish, Croatian, Greek, Serbian.

## Tech Stack

- **Frontend:** TypeScript + React 19, built with Vite; i18n via i18next; linted with ESLint 9 and typescript-eslint.
- **Backend:** Rust (edition 2021, rust-version `1.85`) using the [Tauri 2](https://tauri.app) framework with plugins for shell, dialog, updater, process, and store.
- **Image writing:** in-repo [`armbian-write-conf`](crates/armbian-write-conf) crate (MIT) uses `gptman`, `mbrman`, `ext4-view`, and a vendored `armbian-ext4fs` to write first-boot config into an image's ext4 rootfs without mounting it.
- **Decompression:** `lzma-rust2`, `xz2`, `bzip2`, `flate2`, `zstd`.
- **Platform integration:** `udisks2` + `zbus` on Linux; `security-framework` + Core Foundation on macOS; `windows-sys` on Windows.
- **QDL / EDL:** `qdl` and `nusb` for Qualcomm-based boards.
- **Build tooling:** `@tauri-apps/cli`, `cargo`, `cargo tauri`, and shell/PowerShell setup scripts under [`scripts/setup/`](scripts/setup).

## Repository Layout

```
.
├── src/                 # React/TypeScript frontend (components, hooks, locales, styles)
├── src-tauri/           # Rust/Tauri backend (commands, flash, devices, qdl, updater)
├── crates/
│   └── armbian-write-conf/   # Rust crate: inject first-boot config into ext4 rootfs
├── scripts/
│   ├── locales/         # Locale parity check and AI sync
│   └── setup/           # Per-OS dev dependency installers
├── public/              # Static assets served by Vite
└── .github/             # Issue templates, labels, workflows
```

## Development

Prerequisites, step-by-step setup, build instructions, and project structure live in [DEVELOPMENT.md](DEVELOPMENT.md). The short version:

```bash
git clone https://github.com/armbian/imager.git && cd imager
bash scripts/setup/install.sh
npm install
npm run tauri:dev
```

Common scripts from [`package.json`](package.json):

| Command | Description |
|---------|-------------|
| `npm run dev` | Vite dev server (frontend only) |
| `npm run tauri:dev` | Full app with hot reload (frontend + Rust) |
| `npm run build` | Production frontend build (`tsc -b && vite build`) |
| `npm run tauri:build` | Production distributable |
| `npm run lint` | Run ESLint |
| `npm run clean` | Remove `node_modules`, `dist`, `src-tauri/target` |

See [CONTRIBUTING.md](CONTRIBUTING.md) for the contribution workflow, quality checks, and translation guidelines, and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community expectations.

## Continuous Integration

All automation lives under [`.github/workflows/`](.github/workflows). For a live view of jobs, status, and artifacts across Armbian's CI, use the dedicated overview page:

**[Armbian CI overview — imager](https://actions.armbian.com/?repo=imager)**

## Why We Sign Our Code

Downloading software shouldn't take a leap of faith. Every Windows release is cryptographically signed, so you can confirm the binary is exactly what we built and hasn't been tampered with on the way to you.

This is possible thanks to [SignPath Foundation](https://signpath.org?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager), which gives free code signing certificates to open source projects, and [SignPath.io](https://signpath.io?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager) for the signing infrastructure.

## License

Armbian Imager is distributed under the GNU General Public License, either version 2 or (at your option) any later version — compatible with the [Armbian build framework](https://github.com/armbian/build).

SPDX-License-Identifier: `GPL-2.0-or-later`

The full text is in [LICENSE](LICENSE). A few bundled components keep their own terms: the `armbian-write-conf` crate and its vendored `armbian-ext4fs` fork are `MIT`, and the language flag graphics from [Twemoji](https://github.com/jdecked/twemoji) are `CC-BY-4.0`. Per-file details are in [`src-tauri/packaging/copyright`](src-tauri/packaging/copyright).

---

<p align="center">
  <sub>Made with ❤️ by the Armbian community</sub>
</p>
