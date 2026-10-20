# Cross-Platform Strategy

ArcLive is designed as a cross-platform product. **Only Windows 11 x64 has been
built and run in this workspace** unless [progress.md](progress.md) records otherwise.

## Platform matrix

| Concern | Windows | macOS | Linux |
|---------|---------|-------|-------|
| GUI | egui/eframe (winit) | Same API | Same API |
| Audio default | WASAPI via cpal | CoreAudio via cpal | ALSA via cpal |
| ASIO | Optional; **disabled** by default | N/A | N/A |
| MIDI | midir / WinMM | CoreMIDI | ALSA |
| File dialogs | rfd | rfd | rfd |
| High DPI | winit | winit | winit |
| Plugins | Internal devices; CLAP/VST3 later | Same | Same |

## Rules

1. `arclive-core` and domain logic must not call Win32 / Cocoa / X11 APIs.
2. Platform adapters live in `arclive-engine` (audio/MIDI) and `arclive-app` / `rfd` (dialogs).
3. Successful Windows compilation alone is never reported as “cross-platform tested.”
4. Record each OS/arch that was actually built in `docs/progress.md`.

## ASIO note

Enabling cpal’s `asio` feature requires a compliant Steinberg ASIO SDK setup and
must not redistribute the SDK. Document any future feature gate separately; do not
enable it in CI or default developer builds without explicit license compliance.
