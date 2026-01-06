#!/bin/bash
set -e

# Check for source image
SOURCE_ICON="$1"

if [ -z "$SOURCE_ICON" ]; then
    echo "Usage: $0 <path-to-source-1024x1024.png>"
    echo "Example: $0 assets/logo.png"
    exit 1
fi

if [ ! -f "$SOURCE_ICON" ]; then
    echo "Error: File $SOURCE_ICON not found."
    exit 1
fi

echo "Generating .icns file from $SOURCE_ICON..."

ICONSET_DIR="assets/icon.iconset"
mkdir -p "$ICONSET_DIR"

# Resize to standard macOS icon sizes
sips -z 16 16     "$SOURCE_ICON" --out "$ICONSET_DIR/icon_16x16.png"
sips -z 32 32     "$SOURCE_ICON" --out "$ICONSET_DIR/icon_16x16@2x.png"
sips -z 32 32     "$SOURCE_ICON" --out "$ICONSET_DIR/icon_32x32.png"
sips -z 64 64     "$SOURCE_ICON" --out "$ICONSET_DIR/icon_32x32@2x.png"
sips -z 128 128   "$SOURCE_ICON" --out "$ICONSET_DIR/icon_128x128.png"
sips -z 256 256   "$SOURCE_ICON" --out "$ICONSET_DIR/icon_128x128@2x.png"
sips -z 256 256   "$SOURCE_ICON" --out "$ICONSET_DIR/icon_256x256.png"
sips -z 512 512   "$SOURCE_ICON" --out "$ICONSET_DIR/icon_256x256@2x.png"
sips -z 512 512   "$SOURCE_ICON" --out "$ICONSET_DIR/icon_512x512.png"
sips -z 1024 1024 "$SOURCE_ICON" --out "$ICONSET_DIR/icon_512x512@2x.png"

# Convert iconset to icns using macOS native tool
iconutil -c icns "$ICONSET_DIR" -o assets/icon.icns

# Clean up
rm -rf "$ICONSET_DIR"

echo "✅ Generated assets/icon.icns"
echo "Note: This is the main app icon. For menu bar icons (assets/icon_*.png), you may want to generate separate simple monochrome 18x18 versions."
