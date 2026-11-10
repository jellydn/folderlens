#!/bin/sh
# Package the same verified, ad-hoc signed bundle as a DMG and a ZIP.
set -eu
cd "$(dirname "$0")/.."
metadata=$(cargo metadata --locked --no-deps --format-version 1)
target=$(printf '%s' "$metadata" | plutil -extract target_directory raw -o - -)
version=$(printf '%s' "$metadata" | plutil -extract packages.0.version raw -o - -)
if [ "${GITHUB_REF_TYPE:-}" = tag ] && [ "${GITHUB_REF_NAME:-}" != "v$version" ]; then
  echo "Release tag must match Cargo.toml: v$version" >&2
  exit 1
fi

sh scripts/bundle.sh
app="$target/release/FolderLens.app"
codesign --verify --deep --strict "$app"
lipo "$app/Contents/MacOS/FolderLens" -verify_arch arm64
assets="$target/release/assets"
mkdir -p "$assets"
staging=$(mktemp -d "$target/release/dmg-root.XXXXXX")
trap 'rm -rf "$staging"' EXIT
ditto "$app" "$staging/FolderLens.app"
ln -s /Applications "$staging/Applications"
hdiutil create -volname FolderLens -srcfolder "$staging" -ov -format UDZO \
  "$assets/FolderLens-$version-macos-arm64.dmg"
ditto -c -k --sequesterRsrc --keepParent "$app" "$assets/FolderLens-$version-macos-arm64.zip"
(
  cd "$assets"
  shasum -a 256 "FolderLens-$version-macos-arm64.dmg" "FolderLens-$version-macos-arm64.zip" > SHA256SUMS
)
printf 'Release assets: %s\n' "$assets"
