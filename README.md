<h2 align="center">
  <a href=#><img src="https://raw.githubusercontent.com/armbian/.github/master/profile/logosmall.png" alt="Armbian logo"></a>
  <br><br>
</h2>

### Purpose of This Repository

**Armbian Imager** is the official tool for downloading and flashing Armbian OS images to single-board computers. It focuses on safe and reliable flashing of Armbian images, with board-aware guidance and verification.

### Key features

- Support for **300+ boards** with smart filtering and board-aware metadata
- Disk safety checks, checksum validation, and post-write **verification**
- Native cross-platform builds for **Linux**, **Windows**, and **macOS** (x64 and ARM64)
- **Multi-language UI** with automatic system language detection
- Automatic application updates
- Small binary size and minimal runtime dependencies

<p align="center">
  <a href=https://github.com/armbian/imager/releases><img src="images/armbian-imager-ani.gif" alt="Armbian Imager"></a>
</p>

### Testimonials

> "What a fantastic tool for getting people started with a non Raspberry PI"
> — *Interfacing Linux*, *Hardware and software guides for Linux creatives.* ([source](https://www.youtube.com/watch?v=RAxQebKsnuc)) 

> “A proper multi-platform desktop app that actually works, which is rarer than you’d think.”
> — *Bruno Verachten*, *Senior Developer Relations Engineer* ([source](https://www.linkedin.com/pulse/adding-risc-v-support-armbian-imager-tale-qemu-tauri-deja-verachten-86fxe))

> "The Upcoming Armbian Imager Tool is a Godsend for Non-Raspberry Pi SBC Owners"
> — *Sourav Rudra*, *It's FOSS* ([source](https://itsfoss.com/news/armbian-imager-quietly-debuts/))

> "According to Armbian, this results in less RAM and storage usage and a faster experience."
> — *Jordan Gloor*, *HowtoGeek.com* ([source](https://www.howtogeek.com/armbians-raspberry-pi-imager-alternative-is-here/))

> "It's super easy to write an operating system... I'm always happy when an Armbian version comes out because you've got more stability and much more compatibility."
> — *leepspvideo*, *Simple Linux install for 300+ Arm devices* ([source](https://www.youtube.com/watch?v=vUvGD2GSALI))

## Download

Prebuilt binaries are available for all supported platforms.

| <a href="https://github.com/armbian/imager/releases"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/apple.svg" width="24"><br><strong>macOS</strong></a> | <a href="https://github.com/armbian/imager/releases"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/windows11.svg" width="24"><br><strong>Windows</strong></a> | <a href="https://github.com/armbian/imager/releases"><img src="https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/linux.svg" width="24"><br><strong>Linux</strong></a> |
|:---:|:---:|:---:|
| Intel & Apple Silicon | x64 & ARM64 | x64 & ARM64 |
| <code>.dmg</code> / <code>.app.zip</code> | <code>.exe</code> / <code>.msi</code> | <code>.deb</code> / <code>.AppImage</code> |

## How It Works

1. **Select Manufacturer** — Choose from 70+ supported SBC manufacturers or load a custom image
2. **Select Board** — Pick your board using real photos and metadata from armbian.com
3. **Select Image** — Choose desktop or server, kernel variant, and stable or nightly builds
4. **Flash** — Download, decompress, write, and verify automatically

## Customization

- **Theme Selection**: Light, dark, or automatic based on system preferences
- **Developer Mode**: Enable detailed logging and view application logs
- **Language Selection**: 17 languages with automatic system detection

## Platform Support

| Platform | Architecture | Notes |
|----------|-------------|-------|
| macOS | Intel x64 | Full support |
| macOS | Apple Silicon | Native ARM64 build, Touch ID support |
| Windows | x64 | Requires Administrator privileges |
| Windows | ARM64 | Native ARM64 build, requires Administrator privileges |
| Linux | x64 | Uses UDisks2 and pkexec for elevated privileges |
| Linux | ARM64 | Native ARM64 build |

### Supported Languages

English, Italian, German, French, Spanish, Portuguese, Dutch, Polish, Russian, Chinese, Japanese, Korean, Ukrainian, Turkish, Slovenian, Swedish, Croatian

## Development

Development setup, build instructions, and project structure are documented in  [DEVELOPMENT.md](DEVELOPMENT.md).

---

<p align="center">
  <sub>Made with ❤️ by the Armbian community</sub>
</p>
