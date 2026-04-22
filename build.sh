#!/bin/bash
#
# MMYCodeSwitch-API Build Script (Mac/Linux)
# One-click build script supporting installer & portable builds.
#
# Usage:
#   ./build.sh                  # DMG installer (Mac) / AppImage (Linux)
#   ./build.sh --portable       # Portable folder (run directly)
#   ./build.sh --clean          # Clean then rebuild
#   ./build.sh --help           # Show help
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
GRAY='\033[0;90m'
NC='\033[0m' # No Color

# Parse arguments
CLEAN=false
PORTABLE=false
MODE="release"

while [[ $# -gt 0 ]]; do
    case $1 in
        --clean) CLEAN=true; shift ;;
        --portable) PORTABLE=true; shift ;;
        --dev) MODE="debug"; shift ;;
        --help|-h)
            echo "MMYCodeSwitch-API Build Script"
            echo ""
            echo "Usage: ./build.sh [options]"
            echo ""
            echo "Options:"
            echo "  --portable    Build portable version (single folder)"
            echo "  --clean       Clean old build artifacts before building"
            echo "  --dev         Build in debug mode"
            echo "  --help        Show this help message"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# Detect OS
OS_TYPE="unknown"
if [[ "$OSTYPE" == "darwin"* ]]; then
    OS_TYPE="mac"
elif [[ "$OSTYPE" == "linux"* ]]; then
    OS_TYPE="linux"
fi

PROJECT_ROOT="$(cd "$(dirname "$0")" && pwd)"
PRODUCT_NAME="MMYCodeSwitch-API"

echo ""
echo -e "${CYAN}==============================================${NC}"
echo -e "${CYAN}  MMYCodeSwitch-API Build Script${NC}"
echo -e "${CYAN}  Mode: ${MODE^^}${NC}"
echo -e "${CYAN}  OS: ${OS_TYPE^^}${NC}"
if $PORTABLE; then
    echo -e "${CYAN}  Type: Portable (no-install)${NC}"
else
    echo -e "${CYAN}  Type: Installer${NC}"
fi
echo -e "${CYAN}==============================================${NC}"
echo ""

# Step 1: Check dependencies
echo -e "${YELLOW}[1/6] Checking dependencies...${NC}"

if ! command -v npm &> /dev/null; then
    echo -e "${RED}[ERROR] Node.js/npm not found.${NC}"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}[ERROR] Rust/Cargo not found.${NC}"
    exit 1
fi

echo -e "${GREEN}[OK] Node.js: $(node --version)${NC}"
echo -e "${GREEN}[OK] Cargo:  $(cargo --version)${NC}"
echo ""

# Step 2: Clean old artifacts
if $CLEAN; then
    echo -e "${YELLOW}[2/6] Cleaning old build artifacts...${NC}"
    rm -rf "$PROJECT_ROOT/dist"
    rm -rf "$PROJECT_ROOT/src-tauri/target/$MODE"
    rm -rf "$PROJECT_ROOT/dist-portable"
    echo -e "${GREEN}[OK] Cleanup done${NC}"
else
    echo -e "${GRAY}[2/6] Skipping cleanup (use --clean to enable)${NC}"
fi
echo ""

# Step 3: Frontend dependencies
echo -e "${YELLOW}[3/6] Checking frontend dependencies...${NC}"
if [[ ! -d "$PROJECT_ROOT/node_modules" ]]; then
    echo "   Installing npm dependencies..."
    cd "$PROJECT_ROOT"
    npm install
    if [[ $? -ne 0 ]]; then
        echo -e "${RED}[ERROR] npm install failed${NC}"
        exit 1
    fi
else
    echo -e "${GRAY}   node_modules exists, skipping${NC}"
fi
echo -e "${GREEN}[OK] Ready${NC}"
echo ""

# Step 4: Compile
echo -e "${YELLOW}[4/6] Compiling application... (this takes a while)${NC}"
cd "$PROJECT_ROOT"

if $PORTABLE; then
    # Compile only, skip bundle
    if [[ "$MODE" == "release" ]]; then
        npx tauri build --no-bundle
    else
        npx tauri build --debug --no-bundle
    fi
else
    # Full build with bundle
    if [[ "$MODE" == "release" ]]; then
        npx tauri build
    else
        npx tauri build --debug
    fi
fi

if [[ $? -ne 0 ]]; then
    echo -e "${RED}[ERROR] Compilation failed!${NC}"
    exit 1
fi

echo -e "${GREEN}[OK] Compilation done${NC}"
echo ""

# Determine binary path
if [[ "$MODE" == "release" ]]; then
    BINARY_PATH="$PROJECT_ROOT/src-tauri/target/release/tauri-app"
else
    BINARY_PATH="$PROJECT_ROOT/src-tauri/target/debug/tauri-app"
fi

# Step 5: Create output
if $PORTABLE; then
    echo -e "${YELLOW}[5/6] Creating portable package...${NC}"

    if [[ "$MODE" == "release" ]]; then
        PORTABLE_DIR="$PROJECT_ROOT/$PRODUCT_NAME-Portable"
    else
        PORTABLE_DIR="$PROJECT_ROOT/$PRODUCT_NAME-Portable-debug"
    fi

    rm -rf "$PORTABLE_DIR"
    mkdir -p "$PORTABLE_DIR"

    # Copy the binary
    if [[ "$OS_TYPE" == "mac" ]]; then
        # On Mac, find the .app bundle
        APP_PATH="$PROJECT_ROOT/src-tauri/target/$MODE/bundle/macos/$PRODUCT_NAME.app"
        if [[ -d "$APP_PATH" ]]; then
            cp -R "$APP_PATH" "$PORTABLE_DIR/"
        else
            # Fallback: copy binary
            cp "$BINARY_PATH" "$PORTABLE_DIR/$PRODUCT_NAME"
        fi
    else
        cp "$BINARY_PATH" "$PORTABLE_DIR/$PRODUCT_NAME"
    fi

    # Calculate size
    TOTAL_SIZE=$(du -sh "$PORTABLE_DIR" | cut -f1)

    echo -e "${GREEN}[OK] Portable created: $PORTABLE_DIR${NC}"
    echo -e "${GRAY}     Size: $TOTAL_SIZE${NC}"
    echo ""

    echo -e "${GREEN}==============================================${NC}"
    echo -e "${GREEN}  Build Success! (Portable)${NC}"
    echo -e "${GREEN}==============================================${NC}"
    echo ""
    echo -e "Portable folder: ${CYAN}$PORTABLE_DIR${NC}"
    echo -e "Size: ${GRAY}$TOTAL_SIZE${NC}"
    echo ""
    echo "Double-click to run."

else
    echo -e "${YELLOW}[5/6] Locating output files...${NC}"

    BUNDLE_DIR="$PROJECT_ROOT/src-tauri/target/$MODE/bundle"

    echo ""
    echo -e "${GREEN}==============================================${NC}"
    echo -e "${GREEN}  Build Success!${NC}"
    echo -e "${GREEN}==============================================${NC}"
    echo ""

    if [[ "$OS_TYPE" == "mac" ]]; then
        # DMG
        DMG_PATH=$(find "$BUNDLE_DIR/dmg" -name "*.dmg" 2>/dev/null | head -1)
        if [[ -f "$DMG_PATH" ]]; then
            DMG_SIZE=$(du -sh "$DMG_PATH" | cut -f1)
            echo -e "DMG Installer:${NC}"
            echo -e "${CYAN}   $DMG_PATH${NC}"
            echo -e "${GRAY}   Size: $DMG_SIZE${NC}"
            echo ""
        fi

        # App
        APP_PATH="$BUNDLE_DIR/macos/$PRODUCT_NAME.app"
        if [[ -d "$APP_PATH" ]]; then
            APP_SIZE=$(du -sh "$APP_PATH" | cut -f1)
            echo -e "App Bundle:${NC}"
            echo -e "${CYAN}   $APP_PATH${NC}"
            echo -e "${GRAY}   Size: $APP_SIZE${NC}"
            echo ""
        fi

        echo "Double-click DMG to install, or drag .app to Applications."

    elif [[ "$OS_TYPE" == "linux" ]]; then
        # AppImage
        APPIMAGE_PATH=$(find "$BUNDLE_DIR/appimage" -name "*.AppImage" 2>/dev/null | head -1)
        if [[ -f "$APPIMAGE_PATH" ]]; then
            APPIMAGE_SIZE=$(du -sh "$APPIMAGE_PATH" | cut -f1)
            echo -e "AppImage:${NC}"
            echo -e "${CYAN}   $APPIMAGE_PATH${NC}"
            echo -e "${GRAY}   Size: $APPIMAGE_SIZE${NC}"
            echo ""
        fi

        # Deb
        DEB_PATH=$(find "$BUNDLE_DIR/deb" -name "*.deb" 2>/dev/null | head -1)
        if [[ -f "$DEB_PATH" ]]; then
            DEB_SIZE=$(du -sh "$DEB_PATH" | cut -f1)
            echo -e "Deb Package:${NC}"
            echo -e "${CYAN}   $DEB_PATH${NC}"
            echo -e "${GRAY}   Size: $DEB_SIZE${NC}"
            echo ""
        fi

        echo "Double-click AppImage to run, or install .deb package."
    fi
fi

echo ""
echo -e "${GRAY}[6/6] Done${NC}"