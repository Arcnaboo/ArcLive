# Warping in ArcLive

ArcLive’s warp system follows the **publicly documented Live 7 model** (markers + Seg. BPM + mode roles). DSP is original; there is no Ableton code, assets, or bit-exact Complex mode.

## Public model (verified)

- A **Warp Marker** locks a sample frame to a musical beat.
- Between markers, natural tempo is **Seg. BPM** = `(sample_delta / sample_rate) * 60 / beat_delta`.
- Playback advances in **set BPM** (transport). Sample read position comes from the piecewise-linear marker map (`beat_to_sample`).
- **Warp off** + matching sample rate + no transpose ≈ neutral (sample copy / simple varispeed for transpose only).
- **Warp on** with `|set BPM − Seg. BPM| < ε`, mode ≠ Complex, transpose/detune = 0 → **neutral** (no stretch).
- **Complex** is never treated as neutral (falls back to Beats; labeled experimental).

## Modes in ArcLive

| Mode | Behavior |
|------|----------|
| **Beats** | Transient-aware read (onset table + short grain snap); WSOLA-inspired |
| **Tones** | Longer grain preset on the same stretcher |
| **Texture** | Longer still (pads / beds) |
| **Re-Pitch** | Varispeed along the marker map (speed changes pitch) |
| **Complex** | Stub → Beats + UI note |

## Import heuristic

WAV import (`hound`) creates markers at beat 0 and clip end. Clip length in beats = `duration_seconds * project_BPM / 60`. Onsets for Beats mode are detected offline (energy flux) when the graph is published.

## Neutrality tests

Unit tests in `arclive-core` (`is_neutral_playback`, Seg. BPM math, marker map) and `arclive-engine` (`neutrality_matched_tempo_copies`, Re-Pitch advances with tempo).

## Non-goals

- Ableton branding / `.als` / proprietary Complex grain code
- Global Groove (would move markers; deferred)
- Bit-exact Live parity