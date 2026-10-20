# Implementation Roadmap

Incremental, testable milestones. A milestone is complete only when its behavior is
exercised and `fmt` / `check` / `clippy` / `test` pass. See [progress.md](progress.md)
for actual results.

## Milestone 1 — Foundation

- Rust workspace and crate boundaries
- egui main window: menu, transport, browser, session/arrangement, bottom editor, status
- Transport state machine unit tests
- Logging (`tracing`) on UI/control threads

## Milestone 2 — Audio Engine

- Enumerate devices; open WASAPI stereo output
- `Engine::process` with play/stop/seek and test tone
- Device-less unit tests; skip device tests when no output

## Milestone 3 — Tracks and Mixer

- Multiple audio tracks, gain, pan, mute, solo, master sum
- Peak meters via atomics
- Add/remove tracks via published graph snapshot

## Milestone 4 — Session View

- Clip-slot grid and scenes
- Launch/stop audio clips with bar/beat quantization
- Tempo drives musical rate (document resampling limitations)

## Milestone 5 — Arrangement View

- Timeline placement, move clips, loop region, zoom/scroll
- Arrangement playback distinct from session

## Milestone 6 — MIDI

- midir enumeration and input
- MIDI tracks, piano-roll editing, clip playback from transport

## Milestone 7 — Plugins and Devices

- Internal `Device` trait; built-in gain, filter, EQ
- Documented hosting spike (CLAP preferred; VST3 only if RT-safe)

## Milestone 8 — Projects

- New / open / save / save as
- Versioned JSON, missing-asset placeholders, autosave, migration test
