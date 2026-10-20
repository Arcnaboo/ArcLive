# Progress

Filled only with commands that actually ran.

## Milestone status

| Milestone | Status | Notes |
|-----------|--------|-------|
| 1 Foundation | Complete | egui shell, transport tests, Windows launch |
| 2 Audio engine | Complete | WASAPI via cpal; process() + test tone; device open |
| 3 Tracks / mixer | Complete | gain/pan/mute/solo, meters, add/remove track |
| 4 Session | Complete | clip grid, scene launch, quantized launch, clip audio |
| 5 Arrangement | Complete | timeline, loop, zoom, clip move, arrangement mode |
| 6 MIDI | Partial | MIDI clips + piano roll + offline preview; live MIDI I/O deferred (midir/cpal alsa-sys conflict) |
| 7 Devices | Complete | Gain/Filter/EQ Device trait in track chain |
| 8 Projects | Complete | .arclive JSON, open/save, autosave, v0->v1 migration, missing assets |

## Platforms tested

| Platform | Built | App launched |
|----------|-------|--------------|
| Windows 11 x64 | Yes (`cargo +stable`) | Yes (2026-10-20) |
| macOS | Not run | Not run |
| Linux | Not run | Not run |

## Milestone 1-8 verification (Windows)

Commands:

```
cargo +stable fmt --all
cargo +stable check --workspace
cargo +stable clippy --workspace --all-targets -- -D warnings
cargo +stable test --workspace
cargo +stable run -p arclive-app
```

Results (2026-10-20):

- `fmt`: applied
- `check`: ok
- `clippy -D warnings`: ok
- `test --workspace`: **21 passed** (core 8, engine 8, engine offline 2, project 3)
- App launch: window painted Session View; audio device `L27i-40 (NVIDIA High Definition Audio)`; transport/BPM/mixer/device chain visible

### Relevant source

- `crates/arclive-core/` — time, transport, song
- `crates/arclive-engine/` — process, host, devices, session/arrangement render
- `crates/arclive-project/` — serialization + migrations
- `crates/arclive-ui/` — egui shell
- `crates/arclive-app/` — binary bridge

### Known limitations

- ASIO not in default build
- `midir` not linked (Cargo `alsa-sys` links conflict with cpal 0.18 across targets); MIDI clip playback works offline
- Tempo changes musical rate; no clip resampling yet
- Plugin hosting (CLAP/VST3) researched only; not in RT path
- macOS/Linux not built in this workspace

### Remaining tasks

- Resolve MIDI I/O dependency (Windows-only midir or aligned alsa versions)
- Optional ASIO feature with compliant SDK workflow
- WAV asset import; richer piano-roll editing
- RT-safe third-party plugin adapter spike

## Live 7 UI + Warp (2026-10-20)

- Session View: vertical track columns, under-track mixer, Live-era charcoal/amber theme
- Clip View: Clip / Sample tabs, waveform, Warp toggle, mode, Seg.BPM, transpose/detune, loop, markers
- Domain: WarpMarker / WarpMode, AudioClipData warp fields; project schema **v2** migration
- Engine: marker map, Re-Pitch varispeed, Beats onset-guided stretch; neutrality helpers + tests
- WAV: hound import, browser list (samples/), drag-drop into slots + clip-slot moves
- Docs: docs/warping.md
- `cargo test --workspace`: all passed (27 tests incl. warp + schema v2)
