#!/bin/bash
##############################################################################
# Armbian Imager - Cross-Platform Dependencies Installation Script
#
# This script automatically detects the operating system and runs the
# appropriate installation script for your platform.
#
# Usage: bash scripts/setup/install.sh
##############################################################################

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Get script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

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

print_header() {
    echo -e "${BOLD}$1${NC}"
}

# Display header
clear
echo ""
print_header "╔════════════════════════════════════════════════════════════╗"
print_header "║                                                            ║"
print_header "║        Armbian Imager - Dependency Installation           ║"
print_header "║                                                            ║"
print_header "╚════════════════════════════════════════════════════════════╝"
echo ""

# Detect operating system
detect_os() {
    case "$(uname -s)" in
        Linux*)
            OS="linux"
            ;;
        Darwin*)
            OS="macos"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            OS="windows"
            ;;
        *)
            print_error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac
}

detect_os
print_info "Detected OS: $OS"
echo ""

# Confirm before proceeding
if [ "$OS" = "linux" ]; then
    print_warning "Linux installation requires root privileges (sudo)"
    echo ""
fi

read -p "Do you want to proceed with installation? [y/N] " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_info "Installation cancelled"
    exit 0
fi

echo ""
print_info "Starting installation process..."
echo ""

# Run appropriate installation script
case "$OS" in
    linux)
        if [ "$EUID" -ne 0 ]; then
            print_info "Requesting root privileges..."
            exec sudo bash "$SCRIPT_DIR/install-linux.sh"
        else
            bash "$SCRIPT_DIR/install-linux.sh"
        fi
        ;;

    macos)
        bash "$SCRIPT_DIR/install-macos.sh"
        ;;

    windows)
        print_error "Please run the Windows PowerShell script directly:"
        echo "  powershell -ExecutionPolicy Bypass -File scripts\\setup\\install-windows.ps1"
        echo ""
        print_info "Or run from Git Bash/WSL as administrator"
        exit 1
        ;;

    *)
        print_error "Unknown operating system: $OS"
        exit 1
        ;;
esac

# Script should not reach here, but just in case
exit 0
