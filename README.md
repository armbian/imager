<h2 align="center">
  <a href=#><img src="https://raw.githubusercontent.com/armbian/.github/master/profile/logosmall.png" alt="Armbian logo"></a>
  <br><br>
</h2>

### About

Armbian Imager is the official tool for downloading and flashing Armbian OS images to single-board computers. It checks the target disk before writing, validates the checksum, and verifies the image after the write, so a bad download or the wrong disk doesn't turn into a broken card.

### Features

- Works with 300+ boards, with filtering and board metadata from armbian.com
- Disk safety checks, checksum validation, and post-write verification
- Native builds for Linux, Windows, and macOS, on x64 and ARM64
- Multi-language interface that follows your system language by default
- Built-in application updates
- Small binary with few runtime dependencies

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
|----------|-------------|-------|
| macOS | Intel x64 | Full support |
| macOS | Apple Silicon | Native ARM64 build, Touch ID support |
| Windows | x64 | Requires Administrator privileges |
| Windows | ARM64 | Native ARM64 build, requires Administrator privileges |
| Linux | x64 | Uses lsblk for detection and UDisks2/polkit for elevated device access |
| Linux | ARM64 | Native ARM64 build |

### Supported Languages

English, Italian, German, French, Spanish, Portuguese, Portuguese (Brazil), Dutch, Polish, Russian, Chinese, Japanese, Korean, Ukrainian, Turkish, Slovenian, Swedish, Croatian

## Why We Sign Our Code

Downloading software shouldn't take a leap of faith. Every Windows release is cryptographically signed, so you can confirm the binary is exactly what we built and hasn't been tampered with on the way to you.

This is possible thanks to [SignPath Foundation](https://signpath.org?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager), which gives free code signing certificates to open source projects, and [SignPath.io](https://signpath.io?utm_source=foundation&utm_medium=github&utm_campaign=armbian-imager) for the signing infrastructure.

## Development

Setup, build instructions, and project layout live in [DEVELOPMENT.md](DEVELOPMENT.md).

---

<p align="center">
  <sub>Made with ❤️ by the Armbian community</sub>
</p>
