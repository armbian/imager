#!/bin/bash
##############################################################################
# Armbian Imager - macOS Dependencies Installation Script
#
# This script installs all system dependencies required for development
# on macOS
#
# Usage: bash scripts/setup/install-macos.sh
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

print_info "Armbian Imager - macOS Dependencies Installation"
echo ""

# Check if running on macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    print_error "This script is designed for macOS only"
    print_info "Use install-linux.sh for Linux or install-windows.ps1 for Windows"
    exit 1
fi

# Detect macOS version
MACOS_VERSION=$(sw_vers -productVersion)
print_info "macOS Version: $MACOS_VERSION"

# Check if Xcode Command Line Tools are installed
print_info "Checking Xcode Command Line Tools..."
if ! command -v xcode-select &> /dev/null || ! xcode-select -p &> /dev/null; then
    print_warning "Xcode Command Line Tools are not installed"
    print_info "Installing Xcode Command Line Tools..."

    xcode-select --install

    print_warning "Please complete the Xcode Command Line Tools installation in the dialog window"
    print_info "Then run this script again"
    exit 0
else
    print_success "Xcode Command Line Tools are installed"
fi

# Check if Homebrew is installed
print_info "Checking Homebrew installation..."
if ! command -v brew &> /dev/null; then
    print_warning "Homebrew is not installed"
    print_info "Installing Homebrew..."

    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

    # Add Homebrew to PATH for Apple Silicon Macs
    if [[ $(uname -m) == 'arm64' ]]; then
        echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile
        eval "$(/opt/homebrew/bin/brew shellenv)"
    fi

    print_success "Homebrew installed"
else
    HOMEBREW_VERSION=$(brew --version | head -n 1)
    print_success "Homebrew is already installed: $HOMEBREW_VERSION"
fi

# Update Homebrew
print_info "Updating Homebrew..."
brew update

# Check if Node.js is installed
print_info "Checking Node.js installation..."
if ! command -v node &> /dev/null; then
    print_warning "Node.js is not installed"
    print_info "Installing Node.js via Homebrew..."

    brew install node

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

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

    # Load Cargo environment
    source "$HOME/.cargo/env"

    print_success "Rust installed: $(rustc --version)"
else
    RUST_VERSION=$(rustc --version)
    print_success "Rust is already installed: $RUST_VERSION"
fi

echo ""
print_success "Installation completed!"
echo ""
print_info "Next steps:"
echo "  1. If Rust was just installed, restart your terminal or run: source \$HOME/.cargo/env"
echo "  2. Install npm dependencies: npm install"
echo "  3. Start development server: npm run tauri:dev"
echo ""
