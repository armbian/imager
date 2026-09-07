<h2 align="center">
  <a href=#><img src="https://raw.githubusercontent.com/armbian/.github/master/profile/logosmall.png" alt="Armbian logo"></a>
  <br><br>
</h2>

# Armbian Imager

## Purpose of This Repository

Armbian Imager is the official cross-platform desktop tool for downloading and flashing Armbian OS images to SD cards and USB drives. It checks the target disk before writing, validates the checksum, and verifies the image after the write, so a bad download or the wrong disk doesn't turn into a broken card.

## Features

- Works with 300+ boards, with filtering and board metadata from armbian.com
- Disk safety checks, checksum validation, and post-write verification
- Native builds for Linux, Windows, and macOS on x64 and ARM64
- Multi-language interface (18 locales), auto-detected from the system
- First-boot autoconfig injection into the image's ext4 rootfs
- Support for Qualcomm EDL (QDL) flashing for compatible boards (e.g. Arduino UNO Q)
- Built-in application updates, code-signed on Windows and macOS
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
|----------|--------------|-------|
| macOS | Intel x64 | Full support |
| macOS | Apple Silicon | Native ARM64 build |
| Windows | x64 | Requires Administrator privileges |
| Windows | ARM64 | Native ARM64 build, requires Administrator privileges |
| Linux | x64 | Uses UDisks2/polkit for elevated device access |
| Linux | ARM64 | Native ARM64 build |

### Supported Languages

English, Italian, German, French, Spanish, Portuguese, Portuguese (Brazil), Dutch, Polish, Russian, Chinese, Japanese, Korean, Ukrainian, Turkish, Slovenian, Swedish, Croatian.

Locale files live in [`src/locales/`](src/locales/), with `en.json` as the source of truth. Missing keys in other locales are auto-synced (see the CI overview link below).

## Tech Stack

Armbian Imager is a [Tauri 2](https://tauri.app) desktop app:

- **Frontend** — TypeScript, React 19, Vite 8, i18next, and Lucide icons. Source in [`src/`](src/), with styles in [`src/styles/`](src/styles/).
- **Backend** — Rust (edition 2021, MSRV 1.85.0) using Tauri plugins for shell, dialog, updater, process, and store; `reqwest` + `rustls` for HTTP; `tokio` for async; and `lzma-rust2`, `xz2`, `bzip2`, `flate2`, and `zstd` for image decompression. Source in [`src-tauri/src/`](src-tauri/src/).
- **In-repo Rust crate** — [`crates/armbian-write-conf/`](crates/armbian-write-conf/): writes a first-boot autoconfig file into an image's ext4 rootfs (in userspace) and validates it.
- **Platform-specific device I/O** — [`src-tauri/src/devices/`](src-tauri/src/) and [`src-tauri/src/flash/`](src-tauri/src/flash/) contain per-OS implementations (`linux.rs`, `macos/`, `windows.rs`).
- **QDL support** — Qualcomm EDL flashing via the `qdl` crate; see [`src-tauri/src/qdl/`](src-tauri/src/qdl/).
- **Setup scripts** — Bash and PowerShell in [`scripts/setup/`](scripts/setup/) to install system prerequisites.

## Development

Environment setup, build instructions, and a full walkthrough of the project layout live in [DEVELOPMENT.md](DEVELOPMENT.md).

Quick start:

```bash
git clone https://github.com/armbian/imager.git && cd imager
bash scripts/setup/install.sh
npm install
npm run tauri:dev
```

Contributions are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md) and our [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Continuous Integration

Builds, releases, PR checks, locale sync, and release cleanup all run on GitHub Actions. See the Armbian CI overview for this repository for the current status of each workflow:

- <https://actions.armbian.com/?repo=imager>

## Why We Sign Our Code

Downloading software shouldn't take a leap of faith. Every Windows release is cryptographically signed, so you can confirm the binary is exactly what we built and hasn't been tampered with on the way to you.

This is possible thanks to [SignPath Foundation](https://signpath.org?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager), which gives free code signing certificates to open source projects, and [SignPath.io](https://signpath.io?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager) for the signing infrastructure.

## License

Armbian Imager is distributed under the GNU General Public License, either version 2 or (at your option) any later version. Version 2 is the floor, so the terms stay compatible with the [Armbian build framework](https://github.com/armbian/build).

SPDX-License-Identifier: `GPL-2.0-or-later`

The full text is in [LICENSE](LICENSE). A few bundled components keep their own terms: the `armbian-write-conf` crate is `MIT`. Per-file details are in [`src-tauri/packaging/copyright`](src-tauri/packaging/copyright).

---

<p align="center">
  <sub>Made with ❤️ by the Armbian community</sub>
</p>
