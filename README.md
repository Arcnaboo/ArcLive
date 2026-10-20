# ArcLive

**ArcLive** is an open-source, cross-platform digital audio workstation written in Rust.
It combines clip-based Session View workflows with a traditional Arrangement timeline,
multitrack audio and MIDI editing, a real-time audio engine, built-in devices, and
versioned project documents.

ArcLive is an **independent original product**. It is informed by public architectural
research notes about historical DAW designs, but it does **not** copy proprietary code,
assets, branding, or file formats from any third-party DAW. ArcLive is not affiliated
with Ableton AG or any other commercial DAW vendor.

## Status

| Target | Build | Runtime tested |
|--------|-------|----------------|
| Windows 11 x64 (primary) | Yes (see [docs/progress.md](docs/progress.md)) | Yes |
| macOS | Architectural target | Not yet built/tested in this workspace |
| Linux | Architectural target | Not yet built/tested in this workspace |

Milestone progress is recorded in [docs/progress.md](docs/progress.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Requirements

- Rust **stable** 1.95 or newer (`cargo +stable`)
- Windows 11 x64 for the currently verified desktop path
- A working WASAPI output device for audio playback (Milestone 2+)

ASIO is **not** enabled in the default build. See [docs/technology-decisions.md](docs/technology-decisions.md).

## Build and run

```bash
cargo +stable run -p arclive-app
```

Release build:

```bash
cargo +stable run -p arclive-app --release
```

## Tests and checks

```bash
cargo +stable fmt --all -- --check
cargo +stable check --workspace
cargo +stable clippy --workspace --all-targets -- -D warnings
cargo +stable test --workspace
```

## Workspace layout

```text
crates/
  arclive-core/      Musical time, transport, song model
  arclive-engine/    Real-time audio engine, mixer, session, devices
  arclive-project/   Versioned project serialization
  arclive-ui/        egui desktop shell
  arclive-app/       Binary entry point
docs/                Architecture and progress
analysis/            Read-only research artifacts (preserved)
```

## Documentation

- [Architecture](docs/architecture.md)
- [Technology decisions](docs/technology-decisions.md)
- [Audio engine](docs/audio-engine.md)
- [Cross-platform](docs/cross-platform.md)
- [Implementation roadmap](docs/implementation-roadmap.md)
- [Progress](docs/progress.md)
- Research reference (preserved): [Live 7 architecture notes](docs/live7-architecture.md)

## Contributing

Contributions are welcome under the dual MIT / Apache-2.0 license.
Keep the audio callback free of allocations, blocking locks, filesystem access,
network I/O, and synchronous logging. Prefer small, tested vertical slices over
large scaffolding.
