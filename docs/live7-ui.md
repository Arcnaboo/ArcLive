# Live 7 UI Control Inventory (ArcLive target)

Authoritative source: **Ableton Live 7.0.9 Reference Manual** (July 2008), public documentation.
ArcLive recreates **layout, control names, and workflows** only. No Ableton logos, skins, fonts, or bitmaps.
Brand remains **ArcLive**.

## Non-goals (Live 8+)

- Live 8/9/10/11/12 Clip View panel layouts
- Push / Max for Live UI
- Modern “Clip | Sample” tab strip as the organizing metaphor
- Ableton trademarks or product skins

Live 7 Clip View uses **boxes** (Clip / Launch / Envelopes / Sample|Notes) toggled by the **Clip View Box selector**.

## Shell layout

```
┌─ Control Bar ─────────────────────────────────────────────────────────────┐
│ Play Stop Rec | BackToArr | ArrPos | Tempo Tap Nudge | Groove | Quantize │
│ KEY MIDI map                                                               │
├─ Browser ─� Nudge | Groove | Quantize │
│ KEY MIDI map                                                               │
├─ Browser ─┬─ Session tracks (columns) ──────────┬─ Returns ┬─ Master ────┤
│ Categories│ title → slots (△ launch / □ stop)   │          │ Scene △     │
│ Samples   │ Track Status (pie / one-shot / …)   │          │ Master mix  │
│           │ Mixer: Activator Solo Arm Vol Pan   │          │             │
├───────────┴─ Clip View ──────────────────────────────────────────────────┤
│ [Clip][Launch][Envelopes][Sample|Notes] box selector                     │
│ Clip/Launch/Sample boxes │ Sample Display / MIDI Editor                  │
└─ Status ─────────────────────────────────────────────────────────────────┘
```

## Control Bar (Ch. 4/7/9)

| Control | Live 7 behavior |
|---------|-----------------|
| Play | Start transport; stays lit while Arrangement Position runs |
| Stop | Stop; **double-stop** resets Arrangement Position to 1.1.1 |
| Record | Arrangement/Session record arm at transport level |
| Back to Arrangement | Resume Arrangement on tracks diverted by Session clips |
| Arrangement Position | Continuous song time (bars.beats.ticks) |
| Tempo | 20–999 BPM |
| Tap Tempo | Tap to set tempo; 4 taps can count-in in 4/4 |
| Tempo Nudge | Temporary speed up/slow down |
| Global Groove | Amount applied to clip grooves |
| Quantization | Global launch quantize |
| KEY / MIDI Map | Mapping modes |

## Session View (Ch. 7)

- Tracks = **vertical columns** left→right; scenes = **horizontal rows**
- **Scene Launch on Master (rightmost column)**
- Clip: **triangular Launch** (left edge), name, **square Stop**
- Track Status above mixer: loop pie + counts; one-shot remaining; monitor icon; Arrangement mini
- One clip per track at a time

## Mixer (Ch. 13)

Session mixer (vertical under tracks):

1. Meter (RMS/peak)
2. Volume
3. Pan (triangle resets to center)
4. **Track Activator** (mute when off)
5. Solo
6. Arm

View sections: In/Out, Sends, Returns, Mixer, Track Delays, Crossfader.

Returns + Master on the right of Session.

## Clip View (Ch. 8/11/9)

**Boxes** (selector shows/hides):

- **Clip:** Activator, Name, Color, Signature, Groove, Nudge
- **Launch:** Mode (Trigger/Gate/Toggle/Repeat), Clip Quantize, Legato, Follow Action (stub OK)
- **Envelopes:** stub editor chrome
- **Sample** (audio): Warp, Mode, Seg.BPM, Transpose, Detune, Loop/Region + Sample Display
- **Notes** (MIDI): Notes box + MIDI editor

## Verification checklist

- [x] Scenes launch from Master column (right)
- [x] Clip △ launch and □ stop
- [x] Track Status above under-track mixer
- [x] Activator / Solo / Arm on Session mixer
- [x] Control Bar: BTA, Tap, Nudge, Groove, Quantize, KEY/MIDI
- [x] Clip View boxes (not Live 11-style tabs)
- [x] No Ableton branding