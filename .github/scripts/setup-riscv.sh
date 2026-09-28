#!/usr/bin/env bash
set -euo pipefail

# Ubuntu's standard mirror serves amd64; RISC-V packages are on ubuntu-ports.
sudo sed -i '/^Architectures:/d; /^Components:/a Architectures: amd64' /etc/apt/sources.list.d/ubuntu.sources
sudo tee /etc/apt/sources.list.d/astra-riscv.sources >/dev/null <<'EOF'
Types: deb
URIs: http://ports.ubuntu.com/ubuntu-ports
Suites: noble noble-updates noble-security
Components: main universe
Architectures: riscv64
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg
EOF
sudo dpkg --add-architecture riscv64
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  gcc-riscv64-linux-gnu g++-riscv64-linux-gnu libc6-dev-riscv64-cross \
  libx11-dev:riscv64 libxi-dev:riscv64 libxcursor-dev:riscv64 \
  libxrandr-dev:riscv64 libgl1-mesa-dev:riscv64 \
  libwayland-dev:riscv64 libxkbcommon-dev:riscv64 libudev-dev:riscv64
