#!/bin/bash
##############################################################################
# Armbian Imager - Linux Dependencies Installation Script
#
# This script installs all system dependencies required for development
# on Linux distributions (Debian/Ubuntu-based, Fedora, Arch)
#
# Usage: sudo bash scripts/setup/install-linux.sh
##############################################################################

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored messages
print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    print_error "This script must be run as root (use sudo)"
    exit 1
fi

print_info "Armbian Imager - Linux Dependencies Installation"
echo ""

# Detect Linux distribution
if [ -f /etc/os-release ]; then
    . /etc/os-release
    DISTRO=$ID
    print_info "Detected distribution: $DISTRO"
else
    print_error "Cannot detect Linux distribution"
    exit 1
fi

# Update package list
print_info "Updating package list..."
if [ "$DISTRO" = "ubuntu" ] || [ "$DISTRO" = "debian" ] || [ "$DISTRO" = "zorin" ]; then
    apt-get update
    print_success "Package list updated"
else
    print_warning "Package update skipped for $DISTRO"
fi

# Install dependencies based on distribution
print_info "Installing system dependencies..."

if [ "$DISTRO" = "ubuntu" ] || [ "$DISTRO" = "debian" ] || [ "$DISTRO" = "pop" ] || [ "$DISTRO" = "linuxmint" ] || [ "$DISTRO" = "zorin" ]; then
    # Debian/Ubuntu-based distributions
    apt-get install -y \
        libglib2.0-dev \
        libgtk-3-dev \
        libwebkit2gtk-4.1-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        build-essential \
        curl \
        wget \
        file \
        libssl-dev \
        libxcb-render0-dev \
        libxcb-shape0-dev \
        libxcb-xfixes0-dev \
        libxkbcommon-dev \
        libxkbcommon-x11-dev

elif [ "$DISTRO" = "fedora" ]; then
    # Fedora
    dnf install -y \
        glib2-devel \
        gtk3-devel \
        webkit2gtk4.1-devel \
        libappindicator-gtk3-devel \
        librsvg2-devel \
        gcc \
        gcc-c++ \
        curl \
        wget \
        openssl-devel \
        libxcb-devel \
        libxkbcommon-devel

elif [ "$DISTRO" = "arch" ] || [ "$DISTRO" = "manjaro" ]; then
    # Arch Linux/Manjaro
    pacman -S --noconfirm \
        glib2 \
        gtk3 \
        webkit2gtk-4.1 \
        libappindicator-gtk3 \
        librsvg \
        base-devel \
        curl \
        wget \
        openssl \
        libxcb \
        libxkbcommon

else
    print_error "Unsupported distribution: $DISTRO"
    print_info "Please manually install the following dependencies:"
    echo "  - libglib2.0-dev (or glib2-devel)"
    echo "  - libgtk-3-dev (or gtk3-devel)"
    echo "  - libwebkit2gtk-4.1-dev (or webkit2gtk4.1-devel)"
    echo "  - libayatana-appindicator3-dev (or libappindicator-gtk3-devel)"
    echo "  - librsvg2-dev (or librsvg2-devel)"
    exit 1
fi

print_success "System dependencies installed"
echo ""

# Check if Node.js is installed
print_info "Checking Node.js installation..."
if ! command -v node &> /dev/null; then
    print_warning "Node.js is not installed"
    print_info "Installing Node.js via NodeSource repository..."

    if [ "$DISTRO" = "ubuntu" ] || [ "$DISTRO" = "debian" ] || [ "$DISTRO" = "pop" ] || [ "$DISTRO" = "zorin" ]; then
        # Install Node.js 20.x LTS
        curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
        apt-get install -y nodejs
    elif [ "$DISTRO" = "fedora" ]; then
        dnf install -y nodejs
    elif [ "$DISTRO" = "arch" ] || [ "$DISTRO" = "manjaro" ]; then
        pacman -S --noconfirm nodejs npm
    fi

    print_success "Node.js installed: $(node --version)"
else
    NODE_VERSION=$(node --version)
    print_success "Node.js is already installed: $NODE_VERSION"
fi

# Check if Rust/Cargo is installed
print_info "Checking Rust/Cargo installation..."
if ! command -v cargo &> /dev/null; then
    print_warning "Rust/Cargo is not installed"
    print_info "Installing Rust via rustup..."

    # Install Rust as non-root user
    RUST_USER=${SUDO_USER:-$USER}
    if [ -n "$RUST_USER" ] && [ "$RUST_USER" != "root" ]; then
        su - $RUST_USER -c 'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y'
        print_success "Rust installed for user: $RUST_USER"
        print_info "Please run: source \$HOME/.cargo/env"
    else
        print_warning "Cannot install Rust for root user"
        print_info "Please run as normal user: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    fi
else
    RUST_VERSION=$(rustc --version)
    print_success "Rust is already installed: $RUST_VERSION"
fi

echo ""
print_success "Installation completed!"
echo ""
print_info "Next steps:"
echo "  1. If Rust was just installed, run: source \$HOME/.cargo/env"
echo "  2. Install npm dependencies: npm install"
echo "  3. Start development server: npm run tauri:dev"
echo ""
