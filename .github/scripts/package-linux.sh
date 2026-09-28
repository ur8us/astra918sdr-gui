#!/usr/bin/env bash
set -euo pipefail

target="$1"
label="$2"
runtime_arch="$3"
repo_dir="$(cd "$(dirname "$0")/../.." && pwd)"
binary="$repo_dir/target/$target/release/astra918-gui"
dist="$repo_dir/dist"
appdir="$repo_dir/target/appdir-$runtime_arch"
tool="$repo_dir/target/appimagetool-$runtime_arch"

test -s "$binary"
mkdir -p "$dist" "$appdir/usr/bin" "$appdir/usr/lib"
install -m 755 "$binary" "$appdir/usr/bin/astra918-gui"
cp "$repo_dir/LICENSE" "$appdir/LICENSE"
cp "$repo_dir/.github/packaging/astra918-gui.desktop" "$appdir/"
cp "$repo_dir/.github/packaging/astra918-gui.svg" "$appdir/.DirIcon"
cp "$repo_dir/.github/packaging/astra918-gui.svg" "$appdir/astra918-gui.svg"
cp "$repo_dir/.github/packaging/AppRun" "$appdir/AppRun"
chmod +x "$appdir/AppRun"

# Bundle the target libudev/libcap needed by the USB stack. Graphics and glibc
# are supplied by the destination Linux system.
case "$runtime_arch" in
  x86_64) triplet=x86_64-linux-gnu; tool_sha=629564ae579fda3323a43f7a2797289971a3571a3ae93ccc8cf5fb6783a9a76c ;;
  aarch64) triplet=aarch64-linux-gnu; tool_sha=c7bcb05e04c1456e50198bc4c6babcdf3e3a5c0f67a45ef927321e9183ffa92e ;;
  riscv64) triplet=riscv64-linux-gnu; tool_sha=629564ae579fda3323a43f7a2797289971a3571a3ae93ccc8cf5fb6783a9a76c ;;
  *) exit 2 ;;
esac
for soname in libudev.so.1 libcap.so.2; do
  source_lib="$(find "/usr/lib/$triplet" -maxdepth 1 -name "$soname" -print -quit)"
  test -n "$source_lib"
  cp -L "$source_lib" "$appdir/usr/lib/$soname"
done

tar -czf "$dist/astra918-gui-Linux-$label.tar.gz" -C "$(dirname "$binary")" astra918-gui

host_arch="$(uname -m)"
curl -fsSL --retry 3 \
  "https://github.com/pkgforge-dev/appimagetool/releases/download/0.5.2/appimagetool-${host_arch}-linux" \
  -o "$tool"
printf '%s  %s\n' "$tool_sha" "$tool" | sha256sum -c -
chmod +x "$tool"
if [[ "$runtime_arch" == riscv64 ]]; then
  # mkdwarfs runs on the x64 host; the embedded runtime is RISC-V.
  mkdwarfs="$repo_dir/target/mkdwarfs-x86_64"
  curl -fsSL --retry 3 \
    https://github.com/mhx/dwarfs/releases/download/v0.15.6/dwarfs-universal-0.15.6-Linux-x86_64 \
    -o "$mkdwarfs"
  printf '%s  %s\n' 50891c38ba359db8271819a6cbf6aaa8068681523f0c4f2b8242007a45edaa28 "$mkdwarfs" | sha256sum -c -
  chmod +x "$mkdwarfs"
  env -u GITHUB_REPOSITORY "$tool" "$appdir" --output "$dist" --name "astra918-gui-Linux-$label.AppImage" \
    --appimage-arch "$runtime_arch" --mkdwarfs "$mkdwarfs"
else
  env -u GITHUB_REPOSITORY "$tool" "$appdir" --output "$dist" --name "astra918-gui-Linux-$label.AppImage" \
    --appimage-arch "$runtime_arch"
fi
test -s "$dist/astra918-gui-Linux-$label.AppImage"
