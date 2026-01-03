#!/bin/bash
# Tarqeem Installation Script
# ترقيم - أول لغة برمجة عربية مُترجَمة
#
# Usage:
#   ./install.sh              # Install to ~/.tarqeem
#   TARQEEM_HOME=/custom/path ./install.sh  # Install to custom location

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Installation directory (default: ~/.tarqeem)
INSTALL_DIR="${TARQEEM_HOME:-$HOME/.tarqeem}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║${NC}           ${GREEN}ترقيم - Tarqeem Compiler Installation${NC}           ${BLUE}║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if we're in the right directory
if [ ! -f "$SCRIPT_DIR/Cargo.toml" ]; then
    echo -e "${RED}Error: Must run from tarqeem source directory${NC}"
    exit 1
fi

# Check if release binary exists
if [ ! -f "$SCRIPT_DIR/target/release/tarqeem" ]; then
    echo -e "${YELLOW}Release binary not found. Building...${NC}"
    echo ""

    # Build compiler and Rust runtime
    echo -e "${BLUE}Building Tarqeem compiler and runtime (release mode)...${NC}"
    cargo build --release --workspace
    echo ""
fi

# Verify runtime library was built
if [ ! -f "$SCRIPT_DIR/target/release/libtrq.a" ]; then
    echo -e "${RED}Error: Runtime library (libtrq.a) not found. Build may have failed.${NC}"
    exit 1
fi

# Create installation directory structure
echo -e "${BLUE}Installing to: ${GREEN}$INSTALL_DIR${NC}"
echo ""

mkdir -p "$INSTALL_DIR/bin"
mkdir -p "$INSTALL_DIR/lib"
mkdir -p "$INSTALL_DIR/stdlib_trq"

# Copy binary
echo -e "  ${GREEN}✓${NC} Installing tarqeem binary"
cp "$SCRIPT_DIR/target/release/tarqeem" "$INSTALL_DIR/bin/"
chmod +x "$INSTALL_DIR/bin/tarqeem"

# Copy runtime library (Rust implementation)
echo -e "  ${GREEN}✓${NC} Installing runtime library"
cp "$SCRIPT_DIR/target/release/libtrq.a" "$INSTALL_DIR/lib/"

# Copy standard library
echo -e "  ${GREEN}✓${NC} Installing standard library"
cp -r "$SCRIPT_DIR/stdlib_trq/"* "$INSTALL_DIR/stdlib_trq/"

# Create version file
VERSION=$(grep '^version' "$SCRIPT_DIR/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
echo "$VERSION" > "$INSTALL_DIR/VERSION"
echo -e "  ${GREEN}✓${NC} Version: $VERSION"

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║${NC}               Installation Complete! ✓                     ${GREEN}║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "Add the following to your shell profile (${YELLOW}~/.bashrc${NC}, ${YELLOW}~/.zshrc${NC}, etc.):"
echo ""
echo -e "  ${BLUE}export TARQEEM_HOME=\"$INSTALL_DIR\"${NC}"
echo -e "  ${BLUE}export PATH=\"\$TARQEEM_HOME/bin:\$PATH\"${NC}"
echo ""
echo -e "Then reload your shell or run:"
echo ""
echo -e "  ${BLUE}source ~/.bashrc${NC}  # or ~/.zshrc"
echo ""
echo -e "Verify installation with:"
echo ""
echo -e "  ${BLUE}tarqeem --version${NC}"
echo ""
