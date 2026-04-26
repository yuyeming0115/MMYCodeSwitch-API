#!/bin/bash
#
# MMYCodeSwitch-API Build Script (Mac/Linux)
# One-click build script supporting installer & portable builds.
#
# Usage:
#   bash ./build.sh              # DMG installer (Mac) / AppImage (Linux)
#   bash ./build.sh --portable   # Portable folder (run directly)
#   bash ./build.sh --clean      # Clean then rebuild
#   bash ./build.sh --help       # Show help
#
# NOTE: On macOS, use 'bash ./build.sh' instead of './build.sh'
#       to avoid zsh compatibility issues.
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
GRAY='\033[0;90m'
NC='\033[0m' # No Color

# Helper: uppercase (POSIX compatible, works in both bash & zsh)
to_upper() { echo "$1" | tr '[:lower:]' '[:upper:]'; }

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
            echo "Usage: bash ./build.sh [options]"
            echo ""
            echo "Options:"
            echo "  --portable    Build portable version (single folder with .app)"
            echo "  --clean       Clean old build artifacts before building"
            echo "  --dev         Build in debug mode (faster but larger)"
            echo "  --help        Show this help message"
            echo ""
            echo "Examples:"
            echo "  bash ./build.sh                  # DMG installer (Mac)"
            echo "  bash ./build.sh --portable       # Portable .app folder (Mac)"
            echo "  bash ./build.sh --portable --clean  # Clean rebuild portable"
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
echo -e "${CYAN}  Mode: $(to_upper "$MODE")${NC}"
echo -e "${CYAN}  OS: $(to_upper "$OS_TYPE")${NC}"
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
    rm -rf "$PROJECT_ROOT/$PRODUCT_NAME-Portable"
    rm -rf "$PROJECT_ROOT/$PRODUCT_NAME-Portable-debug"
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

if $PORTABLE && [[ "$OS_TYPE" != "mac" ]]; then
    # Linux: compile only, no bundle needed
    if [[ "$MODE" == "release" ]]; then
        npx tauri build --no-bundle
    else
        npx tauri build --debug --no-bundle
    fi
else
    # Mac (always): full build to get proper .app bundle
    # Linux installer: full build for AppImage/deb
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

# Determine app bundle path (from tauri bundle output)
BUNDLE_DIR="$PROJECT_ROOT/src-tauri/target/$MODE/bundle"
APP_BUNDLE="$BUNDLE_DIR/macos/$PRODUCT_NAME.app"

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

    if [[ "$OS_TYPE" == "mac" ]]; then
        # On Mac: copy the .app bundle from Tauri output
        if [[ -d "$APP_BUNDLE" ]]; then
            cp -R "$APP_BUNDLE" "$PORTABLE_DIR/"
            TOTAL_SIZE=$(du -sh "$PORTABLE_DIR/$PRODUCT_NAME.app" | cut -f1)
            echo -e "${GREEN}[OK] Portable .app created: $PORTABLE_DIR/${PRODUCT_NAME}.app${NC}"
            echo -e "${GRAY}     Size: $TOTAL_SIZE${NC}"
        else
            echo -e "${YELLOW}[WARN] .app bundle not found at expected location.${NC}"
            echo -e "${YELLOW}       Falling back to raw binary...${NC}"
            cp "$BINARY_PATH" "$PORTABLE_DIR/$PRODUCT_NAME"
            chmod +x "$PORTABLE_DIR/$PRODUCT_NAME"
            TOTAL_SIZE=$(du -sh "$PORTABLE_DIR" | cut -f1)
            echo -e "${GREEN}[OK] Portable binary created: $PORTABLE_DIR/${PRODUCT_NAME}${NC}"
            echo -e "${GRAY}     Size: $TOTAL_SIZE${NC}"
        fi
    else
        cp "$BINARY_PATH" "$PORTABLE_DIR/$PRODUCT_NAME"
        chmod +x "$PORTABLE_DIR/$PRODUCT_NAME"
        TOTAL_SIZE=$(du -sh "$PORTABLE_DIR" | cut -f1)
        echo -e "${GREEN}[OK] Portable created: $PORTABLE_DIR${NC}"
        echo -e "${GRAY}     Size: $TOTAL_SIZE${NC}"
    fi

    echo ""
    echo -e "${GREEN}==============================================${NC}"
    echo -e "${GREEN}  Build Success! (Portable)${NC}"
    echo -e "${GREEN}==============================================${NC}"
    echo ""
    echo -e "Output: ${CYAN}$PORTABLE_DIR${NC}"
    echo -e "Size:   ${GRAY}$TOTAL_SIZE${NC}"
    echo ""
    if [[ "$OS_TYPE" == "mac" ]]; then
        if [[ -d "$PORTABLE_DIR/$PRODUCT_NAME.app" ]]; then
            echo "Double-click ${PRODUCT_NAME}.app to launch."
        else
            echo "Run: ./${PRODUCT_NAME}"
        fi
    else
        echo "Run: ./${PRODUCT_NAME}"
    fi

else
    echo -e "${YELLOW}[5/6] Locating output files...${NC}"

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
        if [[ -d "$APP_BUNDLE" ]]; then
            APP_SIZE=$(du -sh "$APP_BUNDLE" | cut -f1)
            echo -e "App Bundle:${NC}"
            echo -e "${CYAN}   $APP_BUNDLE${NC}"
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
