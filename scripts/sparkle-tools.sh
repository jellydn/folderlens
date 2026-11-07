#!/bin/sh
# Oak's signing tools, pinned independently of mutable release metadata.
set -eu
cd "$(dirname "$0")/.."
tools="$PWD/target/sparkle-2.10.1"
archive="$tools.tar.xz"
if [ ! -x "$tools/bin/sign_update" ]; then
  mkdir -p "$tools"
  curl --fail --location --proto '=https' --tlsv1.2 \
    https://github.com/sparkle-project/Sparkle/releases/download/2.10.1/Sparkle-2.10.1.tar.xz -o "$archive"
  printf '%s  %s\n' 1a856d2a9edb8e628de88121d7be04cf0e2167f8641650b78fca00b14244cf27 "$archive" | shasum -a 256 -c - >&2
  tar -xf "$archive" -C "$tools"
fi
printf '%s\n' "$tools/bin/sign_update"
