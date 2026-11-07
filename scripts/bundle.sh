#!/bin/sh
# Build a local, ad-hoc signed app. Distribution still needs Developer ID signing.
set -eu
cd "$(dirname "$0")/.."
export MACOSX_DEPLOYMENT_TARGET=14.0
cargo build --release --locked
metadata=$(cargo metadata --locked --no-deps --format-version 1)
target=$(printf '%s' "$metadata" | plutil -extract target_directory raw -o - -)
version=$(printf '%s' "$metadata" | plutil -extract packages.0.version raw -o - -)
app="$target/release/Shelf.app"
mkdir -p "$app/Contents/MacOS"
cp "$target/release/shelf" "$app/Contents/MacOS/Shelf.new"
mv -f "$app/Contents/MacOS/Shelf.new" "$app/Contents/MacOS/Shelf"
cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleIdentifier</key><string>io.github.jellydn.shelf</string>
  <key>CFBundleName</key><string>Shelf</string>
  <key>CFBundleDisplayName</key><string>Shelf</string>
  <key>CFBundleExecutable</key><string>Shelf</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
EOF
plutil -lint "$app/Contents/Info.plist"
codesign --force --sign - "$app"
printf 'Built %s\n' "$app"
