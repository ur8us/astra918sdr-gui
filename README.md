# Astra918 controller

Rust/egui light-theme receiver controls for Linux, Windows and macOS. The GUI
controls the same authoritative state as WSJT-X CAT: spectrum center, firmware audio offset,
USB/LSB audio mode and passband, antenna route, automatic/manual gains,
LF/MF capacitor, Save and Retry, with health counters. It has no spectrum or
waterfall. USB I/O runs on a worker thread; window height follows its contents
when connecting, disconnecting or exposing manual gains. The disconnected
window is compact, and a scroll area handles smaller displays.

Keep `astra918sdr` next to this repository: `Cargo.toml` uses its shared host
transport and state codecs. The reference `drm1000-gui` supplied the worker/UI
architecture; its serial protocol is not used. MIT license.

```sh
cargo test --locked
cargo build --locked --release
# Start ../astra918sdr/target/release/astra918-sim in another terminal:
cargo run --locked -- --simulator 127.0.0.1:7350
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

Release builds show a reachable Astra918 SDR Console bridge as a receiver, or
physical receivers when no bridge is available. The bridge owns USB vendor
interface 4 and accepts the GUI's control connection at `127.0.0.1:30433`;
this lets SDR Console and the GUI run concurrently. Simulator UI and the `--simulator`
option are available in debug builds. At startup the GUI discovers receivers
and connects automatically when exactly one identifiable receiver is present,
even if a different serial was selected previously. With zero or multiple
receivers, use **Refresh**, select a serial and **Connect**. Disconnect stays
disconnected until requested; startup discovery is not a reconnect loop.
Connecting adopts receiver settings. Editing
a frequency keeps its draft intact during polling; the applied dial/center
remain visible. **Spectrum center (Hz)** tunes the RF center while preserving
the firmware audio offset. **Firmware USB audio offset (Hz)** moves the audio
channel and CAT dial while keeping that center fixed. The firmware audio/CAT
frequency is always center plus offset. CAT retuning preserves offset and moves
the center. Use **Save to receiver** explicitly to persist all settings. Preferences
store only connection selection, never a stale receiver configuration.

The capacitor slider clamps both dragging and numeric entry to 0-4095. Changes
apply during dragging, at most ten times per second, and the final value is sent
on release. There is no Apply button. Only **Save to receiver** writes flash.

Disconnect SDR++ before connecting this GUI directly by USB: both own the same
vendor interface. When the SDR Console bridge is running, the GUI automatically
uses its proxy instead of claiming USB.
WSJT-X continues using CAT and USB audio independently. Closing this GUI releases
only the vendor interface. See [receiver setup](../astra918sdr/README.md) and
[validation](../astra918sdr/docs/VALIDATION.md).

Linux hardware checks on September 24 verified state adoption, bidirectional
WSJT-X tuning, offset semantics, antenna/gain-mode changes, USB/LSB and explicit
Save. The light window fits both automatic and manual LF controls and shrinks
on disconnect. See the [hardware record](../astra918sdr/docs/HARDWARE-2026-09-24.md).
The current Windows GUI cross-builds and its release CLI was checked under Wine.
Simulator connection checks now use the debug GUI. Native Windows/macOS hardware
checks are still pending. See also the [control update tests](../astra918sdr/docs/CONTROLS-2026-09-24.md).

The subsequent center/firmware-audio-offset UI correction was compiled on Linux
only. No runtime or hardware tests were run for that correction, as requested.

The GitHub workflow expects a sibling GitHub repository named `astra918sdr`
under the same owner. Override repository variable `ASTRA_FIRMWARE_REPOSITORY`
and `ASTRA_FIRMWARE_REF` if published differently. These workflows are provided
but have not run on a remote service in this local-only milestone.
