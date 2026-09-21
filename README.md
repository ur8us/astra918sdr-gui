# Astra918 controller

Rust/egui light-theme receiver controls for Linux, Windows and macOS. The GUI
controls the same authoritative state as WSJT-X CAT: dial, channel offset,
USB/LSB audio mode and passband, antenna route, automatic/manual gains,
LF/MF capacitor, Save and Retry, with health counters. It has no spectrum or
waterfall. USB I/O runs on a worker thread; window height follows its contents,
and a scroll area handles smaller displays.

Keep `astra918sdr` next to this repository: `Cargo.toml` uses its shared host
transport and state codecs. The reference `drm1000-gui` supplied the worker/UI
architecture; its serial protocol is not used. MIT license.

```sh
cargo test --locked
cargo build --locked --release
# Start ../astra918sdr/target/release/astra918-sim in another terminal:
target/release/astra918-gui --simulator 127.0.0.1:7350
```

Build prerequisites: Rust/rustup (pinned 1.90.0), a native C toolchain and normal
platform graphics support. Linux needs development headers for X11/Wayland,
OpenGL and xkbcommon; Ubuntu packages include `build-essential pkg-config
libx11-dev libxi-dev libxcursor-dev libxrandr-dev libgl1-mesa-dev
libwayland-dev libxkbcommon-dev`. Windows supports the MSVC or GNU Rust target;
install the matching compiler. macOS needs Xcode command-line tools. libusb is
built from the vendored source. Native macOS execution remains unverified.

The release executable is in `target/release/` (`.exe` on Windows). A Windows
GNU cross-build is also available under `target/x86_64-pc-windows-gnu/release/`
in the implementation workspace. `--smoke-test` opens the UI, exercises the
worker and closes after approximately eight seconds; check its connection
report in stderr.

To connect hardware later, clear **Offline simulator**, click **Refresh**,
choose a serial and **Connect**. Connecting adopts receiver settings. Editing
a frequency keeps its draft intact during polling; the applied dial/center
remain visible. Ordinary tuning preserves offset; changing offset preserves
dial. Use **Save to receiver** explicitly to persist all settings. Preferences
store only connection selection, never a stale receiver configuration.

Disconnect SDR++ before connecting this GUI: both own the same vendor interface.
WSJT-X continues using CAT and USB audio independently. Closing this GUI releases
only the vendor interface. See [receiver setup](../astra918sdr/README.md) and
[validation](../astra918sdr/docs/VALIDATION.md).

The GitHub workflow expects a sibling GitHub repository named `astra918sdr`
under the same owner. Override repository variable `ASTRA_FIRMWARE_REPOSITORY`
and `ASTRA_FIRMWARE_REF` if published differently. These workflows are provided
but have not run on a remote service in this local-only milestone.
