# Audio Engine

## Contract

Public entry used by tests and by the cpal adapter:

```rust
Engine::process(&mut self, output: &mut [f32], frames: usize, channels: usize)
```

Interleaved `f32` output. The callback must never allocate, take a blocking mutex,
touch the filesystem or network, or call `tracing`.

## Research unknowns

Live 7’s exact callback entry, buffer graph, and RT memory discipline were **not**
recovered. The design below is original.

## Processing model

1. Drain `EngineCommand` from an `rtrb` producer (non-blocking).
2. Apply transport commands at buffer boundaries (Milestone 2); sample-accurate
   quantized launch from Milestone 4.
3. Load published graph snapshot (`ArcSwap`) if present.
4. For each track that should sound (solo/mute rules), render clip PCM or silence,
   apply device chain, gain/pan, sum into master.
5. Update peak meters (atomics) and advance sample position when playing.

## Transport

| State | Behavior |
|-------|----------|
| Stopped | Output silence (or test-tone off); position held |
| Playing | Advance samples; render session and/or arrangement sources |
| Recording | Armed in UI; capture path expands in later milestones |

Tempo (BPM) and time signature live in the song. Musical position converts via
`samples_per_beat = sample_rate * 60 / bpm`.

## Device faults

cpal stream errors set an atomic fault flag. A control path in `arclive-app`
reopens the stream. The callback never reopens devices.

## ASIO

Not part of the default engine build. See [technology-decisions.md](technology-decisions.md).
