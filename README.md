<h2 align="center">
  <a href=#><img src="https://raw.githubusercontent.com/armbian/.github/master/profile/logosmall.png" alt="Armbian logo"></a>
  <br><br>
</h2>

# Armbian Imager

## Purpose of This Repository

Armbian Imager is the official cross-platform desktop application for downloading and flashing Armbian OS images to SD cards, USB drives, and EDL-capable boards. It handles disk selection safety checks, checksum validation, decompression, writing, and post-write verification for 300+ single-board computers.

## Features

- Works with 300+ boards, with filtering and board metadata sourced from armbian.com
- Disk safety checks, checksum validation, and post-write verification
- Native builds for Linux, Windows, and macOS on x64 and ARM64
- Multi-language interface (18 languages) that follows the system locale by default
- Built-in application updates
- QDL (Qualcomm Device Loader) support for EDL-based boards (e.g. Arduino UNO Q)
- First-boot autoconfig injection into ext4 rootfs via the in-repo `armbian-write-conf` crate

## Download

Prebuilt binaries are published on the [GitHub Releases](https://github.com/armbian/imager/releases) page.

| Platform | Architectures | Bundle formats |
|----------|---------------|----------------|
| macOS | Intel x64, Apple Silicon | `.dmg`, `.app` |
| Windows | x64, ARM64 (code-signed) | `.exe` (NSIS), `.msi` |
| Linux | x64, ARM64 | `.deb`, `.AppImage` |

## How It Works

1. **Pick a manufacturer.** Choose one of the supported SBC vendors, or load your own image file.
2. **Pick a board.** Boards show real photos and metadata from armbian.com.
3. **Pick an image.** Desktop or server, a kernel branch, and a stable, nightly, or rolling release build.
4. **Flash.** The app downloads, decompresses, writes, and verifies for you.

## Supported Languages

English, Italian, German, French, Spanish, Portuguese, Portuguese (Brazil), Dutch, Polish, Russian, Chinese, Japanese, Korean, Ukrainian, Turkish, Slovenian, Swedish, Croatian.

Translation files live in [`src/locales/`](src/locales/). Missing keys are auto-synced from `en.json` daily via a scheduled workflow.

## Tech Stack

Armbian Imager is a [Tauri 2](https://tauri.app/) application.

- **Frontend:** TypeScript + React 19, built with Vite; internationalization via `i18next` / `react-i18next`; icons from `lucide-react`.
- **Backend:** Rust (edition 2021, MSRV 1.85.0) using Tauri plugins (`shell`, `dialog`, `updater`, `process`, `store`), `tokio`, `reqwest` (rustls), and decompression crates (`lzma-rust2`, `xz2`, `bzip2`, `flate2`, `zstd`).
- **Platform integration:**
  - Linux: `libc`, UDisks2/polkit via `udisks2` + `zbus`
  - macOS: `security-framework`, `core-foundation`
  - Windows: `windows-sys`
- **Auxiliary crate:** [`crates/armbian-write-conf`](crates/armbian-write-conf/) — MIT-licensed helper that writes a first-boot config file into an ext4 rootfs of a raw disk image in userspace, then validates it.

## Repository Layout

```
.
├── src/                    # React + TypeScript frontend
│   ├── components/         # UI (flash, layout, modals, settings, shared)
│   ├── hooks/              # Custom React hooks (Tauri IPC, settings, flash)
│   ├── contexts/           # Motion, Theme, Update contexts
│   ├── config/             # Constants, badges, i18n, os-info, qdlBoards, …
│   ├── locales/            # 18 translation JSON files
│   ├── styles/             # CSS with design tokens
│   ├── assets/             # Logos and OS icons
│   └── utils/              # Color, device, distro theme, error helpers
├── src-tauri/              # Rust backend (Tauri host)
│   ├── src/
│   │   ├── commands/       # Tauri IPC command handlers
│   │   ├── devices/        # Per-OS block device enumeration
│   │   ├── flash/          # Per-OS write + verify pipelines
│   │   ├── images/         # Image catalog models + filters
│   │   ├── qdl/            # Qualcomm Device Loader (EDL) support
│   │   ├── paste/          # paste.armbian.com log upload
│   │   └── utils/          # Format, HTTP, path, progress, system
│   ├── icons/              # App icons (desktop, iOS, Android)
│   ├── capabilities/       # Tauri capability manifests
│   └── tauri.conf.json     # Tauri app configuration
├── crates/
│   └── armbian-write-conf/ # In-repo Rust crate (autoconfig injector)
├── scripts/
│   ├── locales/            # sync-locales.js (AI-assisted i18n sync)
│   └── setup/              # install.sh, install-linux.sh, install-macos.sh,
│                           # install-windows.ps1
├── public/                 # Static frontend assets
├── index.html              # Vite entry HTML
├── vite.config.ts          # Vite configuration
├── eslint.config.js        # ESLint flat config
├── package.json            # Frontend manifest & scripts
└── tsconfig*.json          # TypeScript project config
```

## Development

Full setup, prerequisites, and build instructions are in [DEVELOPMENT.md](DEVELOPMENT.md). Quick start:

```bash
git clone https://github.com/armbian/imager.git && cd imager
bash scripts/setup/install.sh
npm install
npm run tauri:dev
```

Requirements at a glance:

- Node.js ≥ 20.19.0 (npm 10+)
- Rust ≥ 1.85.0 (stable toolchain)
- Platform toolchain: GTK/WebKit2GTK on Linux, Xcode CLT on macOS, VS Build Tools 2022 + WebView2 on Windows

### Common npm scripts

| Script | Purpose |
|--------|---------|
| `npm run dev` | Vite dev server (frontend only) |
| `npm run tauri:dev` | Full app with hot reload (frontend + Rust) |
| `npm run build` | Production frontend build (`tsc -b && vite build`) |
| `npm run tauri:build` | Production distributable |
| `npm run lint` | ESLint |
| `npm run clean` | Remove `node_modules`, `dist`, `src-tauri/target` |

### Quality checks

Before opening a PR, run:

```bash
# Frontend
npm run lint
npx tsc --noEmit

# Backend
cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

PRs must land with zero lint errors and zero warnings.

## Continuous Integration

Build, release, PR check, and maintenance workflows for this repository are aggregated in the Armbian CI dashboard:

<https://actions.armbian.com/?repo=imager>

## Contributing

Bug reports, feature requests, translations, and pull requests are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow, branch naming, and commit conventions, and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community expectations.

Other ways to help the wider project:

- [Become a board maintainer](https://docs.armbian.com/Board_Maintainers_Procedures_and_Guidelines/)
- [Help cover running costs](https://forum.armbian.com/subscriptions/)
- [Help community members in the forum](https://forum.armbian.com/)

## Why We Sign Our Code

Every Windows release is cryptographically signed so you can verify the binary is exactly what was built. This is possible thanks to the [SignPath Foundation](https://signpath.org?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager), which provides free code-signing certificates to open source projects, and [SignPath.io](https://signpath.io?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager) for the signing infrastructure.

## License

Armbian Imager is distributed under the GNU General Public License, version 2 or (at your option) any later version — the v2 floor keeps it compatible with the [Armbian build framework](https://github.com/armbian/build).

SPDX-License-Identifier: `GPL-2.0-or-later`

Full text: [LICENSE](LICENSE). Bundled components keep their own terms — the `armbian-write-conf` crate is `MIT` — and per-file details are recorded in [`src-tauri/packaging/copyright`](src-tauri/packaging/copyright).

---

<p align="center">
  <sub>Made with ❤️ by the Armbian community</sub>
</p>
