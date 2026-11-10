#!/bin/sh
# Build a local, ad-hoc signed app. Distribution still needs Developer ID signing.
set -eu
cd "$(dirname "$0")/.."
export MACOSX_DEPLOYMENT_TARGET=14.0
cargo build --release --locked
metadata=$(cargo metadata --locked --no-deps --format-version 1)
target=$(printf '%s' "$metadata" | plutil -extract target_directory raw -o - -)
version=$(printf '%s' "$metadata" | plutil -extract packages.0.version raw -o - -)
short_version=${version%%-*}
build_version=$(printf '%s' "$version" | sed 's/-beta\./b/')
app="$target/release/FolderLens.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
xcrun swiftc -parse-as-library -O -target arm64-apple-macosx14.0 native/Updater.swift -o "$app/Contents/MacOS/FolderLensUpdater"
"$app/Contents/MacOS/FolderLensUpdater" --self-test
cp native/update-public-key.txt "$app/Contents/Resources/update-public-key.txt"
cp assets/FolderLens.icns "$app/Contents/Resources/FolderLens.icns"
codesign --force --sign - "$app/Contents/MacOS/FolderLensUpdater"
cp "$target/release/folderlens" "$app/Contents/MacOS/FolderLens.new"
mv -f "$app/Contents/MacOS/FolderLens.new" "$app/Contents/MacOS/FolderLens"
cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleIdentifier</key><string>io.github.jellydn.folderlens</string>
  <key>CFBundleName</key><string>FolderLens</string>
  <key>CFBundleDisplayName</key><string>FolderLens</string>
  <key>CFBundleExecutable</key><string>FolderLens</string>
  <key>CFBundleIconFile</key><string>FolderLens.icns</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleShortVersionString</key><string>$short_version</string>
  <key>CFBundleVersion</key><string>$build_version</string>
  <key>FolderLensVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
EOF
plutil -lint "$app/Contents/Info.plist"
codesign --force --sign - "$app"
printf 'Built %s\n' "$app"
