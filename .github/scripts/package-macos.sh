#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "$0")/../.." && pwd)"
app="$repo_dir/target/Astra918 GUI.app"
macos="$app/Contents/MacOS"
mkdir -p "$macos" "$repo_dir/dist"
lipo -create \
  "$repo_dir/target/aarch64-apple-darwin/release/astra918-gui" \
  "$repo_dir/target/x86_64-apple-darwin/release/astra918-gui" \
  -output "$macos/astra918-gui"
lipo "$macos/astra918-gui" -verify_arch arm64 x86_64
cp "$repo_dir/.github/packaging/Info.plist" "$app/Contents/Info.plist"
codesign --force --deep --sign - "$app"
stage="$repo_dir/target/dmg-stage"
mkdir -p "$stage"
ln -s /Applications "$stage/Applications"
cp -R "$app" "$stage/"
hdiutil create -volname 'Astra918 GUI' -srcfolder "$stage" -ov -format UDZO \
  "$repo_dir/dist/astra918-gui-macOS-Universal.dmg"
test -s "$repo_dir/dist/astra918-gui-macOS-Universal.dmg"
