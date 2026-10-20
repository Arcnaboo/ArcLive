# Technology Decisions

Decisions for ArcLive v1. Claims are limited to what was verified at selection time
(2026-10). Re-check crates before enabling optional features.

## License

**MIT OR Apache-2.0** for ArcLive source. Compatible with egui, cpal, midir, serde,
rtrb, tracing, and CLAP (`clack-host`). Avoid GPL dependencies in the default graph
(`vst3-sys` is GPL-3.0 and will not be used).

## GUI — egui 0.34 + eframe

| Option | Verdict |
|--------|---------|
| **egui / eframe** | **Selected.** Immediate mode fits meters, grids, waveforms, piano roll. Actively maintained (0.34.x). Windows/macOS/Linux. Per-monitor DPI via winit. MIT OR Apache-2.0. |
| Iced | Weaker custom painting story for dense timelines. |
| Slint (OSS) | GPL-3.0 would relicense the UI stack. |

Menus are drawn in-window for consistent Linux/Windows behavior.

## Audio I/O — cpal 0.18 (default host)

| Platform | Default backend | ArcLive v1 |
|----------|-----------------|------------|
| Windows | WASAPI | Enabled and tested |
| macOS | CoreAudio | Same API; not yet built here |
| Linux | ALSA | Same API; not yet built here |

**ASIO:** optional cpal feature exists but pulls Steinberg’s ASIO SDK (proprietary or
GPLv3; SDK redistribution restricted). **Not enabled in default builds.** Do not claim
ASIO support until an optional feature is built and tested with a local driver and
compliant SDK usage.

## MIDI — midir

Standard cross-platform MIDI I/O (WinMM / CoreMIDI / ALSA). Wired from Milestone 6.

## Real-time IPC — rtrb + atomics + ArcSwap

- `rtrb`: bounded SPSC for `EngineCommand` (no alloc on push/pop).
- Atomics: meters, xrun counter, fault flags.
- `arc_swap`: publish graph/clip PCM allocated off the audio thread.

## Serialization — serde + serde_json

Human-readable versioned `project.json` with `schema_version` and ordered migrations.
Binary formats can be added later without changing the domain model.

## Logging — tracing

UI and control threads only. Audio callback uses counters; never synchronous logging.

## Plugins

1. Internal `Device` trait and built-ins (gain, filter, EQ) first.
2. VST3: Steinberg SDK is MIT (3.8+). Evaluate `vst3-host` (MIT) only if a lock-free
   process path is proven; default path locks and must not run in the callback.
3. Prefer CLAP via `clack-host` (MIT OR Apache-2.0) as the first third-party format.
4. Never depend on GPL `vst3-sys` in the default workspace.

## Dialogs — rfd

Native file dialogs for Milestone 8 project open/save.

## Toolchain

- Build with **`cargo +stable`** (verified MSRV: 1.95 for egui).
- Edition **2024**.
- No `rust-toolchain.toml` pin to nightly (nightly may be the machine default).

## MIDI I/O status (2026-10-20)

`midir` 0.9 and 0.10 both conflict with `cpal` 0.18 in Cargo's multi-target resolver
(`alsa-sys` `links` value). ArcLive therefore ships a stub `MidiInputHub` that returns
an empty device list. MIDI **clip** data and pre-rendered audition remain in the engine.
Revisit when crate versions share one `alsa-sys`, or add a Windows-only WinMM adapter
behind `cfg(windows)`.

## WAV I/O — hound 3.x

Chosen for a small pure-Rust WAV reader/writer without pulling a full media stack. Used for Sample browser import into AudioClipData. Raw PCM assets from schema v1 remain supported.

## Warp DSP — original (not Ableton)

Marker map + Seg. BPM math from public Live 7 documentation. Beats/Tones/Texture share an onset-guided stretcher; Re-Pitch is varispeed. Complex is a labeled stub. See docs/warping.md.

