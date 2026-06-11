#!/bin/bash
set -euo pipefail

BINARY_DIR="../../binaries"
MEDIA_DIR="./media"

mkdir -p "$MEDIA_DIR"

declare -a targets=(
  "jinja-lsp-x86_64-unknown-linux-gnu|linux-x64"
  "jinja-lsp-aarch64-unknown-linux-gnu|linux-arm64"
  "jinja-lsp-armv7-unknown-linux-gnueabihf|linux-armhf"
  "jinja-lsp-x86_64-pc-windows-msvc.exe|win32-x64"
  "jinja-lsp-aarch64-pc-windows-msvc.exe|win32-arm64"
  "jinja-lsp-x86_64-apple-darwin|darwin-x64"
  "jinja-lsp-aarch64-apple-darwin|darwin-arm64"
)

for t in "${targets[@]}"; do
  filename="${t%%|*}"
  vsce_target="${t#*|}"
  package_name="${filename%.exe}"

  src="$BINARY_DIR/$filename"
  chmod +x $src
  [ -f "$src" ] || continue

  if [[ "$filename" == *.exe ]]; then
    dest="jinja-lsp.exe"
  else
    dest="jinja-lsp"
  fi
  cp "$src" "$MEDIA_DIR/$dest"

  npm run vscode:prepublish && vsce package -o "$package_name.vsix" --target "$vsce_target"
  rm -rf "$MEDIA_DIR/$dest"
  mv "$package_name.vsix" ../../extensions
done

ls -la "$MEDIA_DIR"
