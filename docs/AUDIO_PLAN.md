# Arcade Audio Plan

## Direction

Create an original late-1980s, 16-bit arcade sound set: short envelopes,
square/saw oscillators, filtered noise, light pitch stepping, and restrained
bit-crushing. The goal is the energy and clarity of an OutRun-era cabinet,
without sampling or reproducing Sega audio, melodies, or specific effects.

Generate every source deterministically with a checked-in SoX script, retain
the generation recipe as the editable source, and ship compact Ogg Vorbis
assets. Bevy 0.20 loads Ogg with its default `vorbis` feature, so this requires
no additional runtime dependency. Prefer 44.1 kHz assets, with deliberately
lower-rate synthesis stages where they create the period texture.

## Sound set

| Cue | Behavior | Synthesis concept |
|---|---|---|
| Engine low | Seamless loop; follows speed in LOW | Layered pulse/saw motor with a short repeating firing pattern |
| Engine high | Crossfades in after shifting HIGH | Brighter pulse layer and octave whine, less low-end weight |
| Acceleration | Engine pitch and high-frequency layer rise continuously | Live `AudioSink` speed and volume, smoothed to avoid zipper noise |
| Accepted shift | One-shot on an actual core gear change | Two-stage mechanical click plus a brief pitch drop |
| Early HIGH request | One-shot only when HIGH is rejected | Hollow metallic clank; rate-limited and distinct from a crash |
| LOW redline warning | Intermittent rattle near LOW maximum | Quiet syncopated mechanical chatter, gated by over-rev intensity |
| Crash/bump | One-shot per authoritative bump event | Noise impact, low square thud, and rapidly descending pitch |
| Nitro pickup | One-shot when the charge count increases | Fast three-note rising arpeggio with a bright noise sparkle |
| Nitro ignition | One-shot at boost start | Rising filtered-noise blast and square-wave punch |
| Nitro burn | Seamless loop during active boost | Bright turbulent noise plus a high turbine tone; fades on release |
| Wind | Continuous loop while moving | Filtered noise whose volume and brightness follow speed |
| Rain | Continuous loop weighted by visual rain blend | Sparse chip-noise droplets over a soft noise bed |
| Wet-road skid | Conditional loop/short bursts | Band-passed noise when steering hard or off-road during rain |
| Menu move | One-shot per changed difficulty card | Short upward/downward blip matching navigation direction |
| Menu confirm | One-shot when the selected run is accepted | Two-note confirmation chord |
| Run start | One-shot after confirmation | Compact rising fanfare/noise gate, separate from menu confirm |
| Leaderboard input | One-shot per accepted letter/delete/submit | Soft key tick, lower delete tick, brighter submit cadence |

## Runtime architecture

Add `crates/game-app/src/audio.rs` with an `ArcadeAudioPlugin` and keep audio
strictly in the presentation crate. The pure game core remains deterministic
and audio-free.

- `AudioAssets` owns handles for every generated Ogg asset.
- `AudioMemory` stores the previous authoritative snapshot: phase, gear,
  speed, bump/pass counts, nitro charge/timer, weather, menu selection, and
  initials length. Transitions create one-shot cues exactly once.
- Long-running tagged entities provide the LOW engine, HIGH engine, wind,
  rain, wet skid, over-rev rattle, and nitro burn layers.
- One update system computes target pitch/volume from game state; a second
  applies attack/release smoothing before calling `AudioSink::set_speed` and
  `set_volume`.
- Engine LOW/HIGH layers crossfade around the accepted gear state. Nitro adds
  its own layer instead of over-pitching the engine into an irritating range.
- Pause, difficulty selection, and terminal screens fade continuous driving
  layers rather than abruptly stopping them. One-shots use despawning playback.
- Keep the mix non-spatial and cabinet-like. Stereo width belongs in the
  authored weather/skid assets; the player car should remain centered.

Suggested initial mix targets: engine -12 dB, wind -22 to -12 dB over the
speed range, weather -20 dB, nitro burn -10 dB, and one-shots peaking below
-3 dBFS. Reserve headroom for crash plus engine plus weather.

## Browser behavior

Web Audio may begin suspended until the user interacts with the page. Treat
the difficulty-confirm action (Enter, Start, or gamepad South) as the audio
unlock point. Do not rely on page-load autoplay. Show a small muted-speaker
indicator only if audio remains unavailable after that interaction.

Add `M` / gamepad-compatible UI support for mute and persist only the audio
preference locally. Audio must never affect simulation, score, or input timing.

## Implementation sequence

1. Add the deterministic generator and auditionable source WAVs; encode the
   approved versions as Ogg and document their duration and loop points.
2. Land menu navigation/confirm/start cues first. This proves browser unlock,
   asset loading, native playback, and WASM playback with minimal mixing risk.
3. Add the two-layer engine mixer, gear-change cue, rejected-HIGH clank, and
   over-rev rattle. Tune against actual LOW/HIGH speed ranges.
4. Add crash, pickup, nitro ignition, and nitro burn transitions.
5. Add wind, rain, and wet-road skid ambience, driven by the same weather
   blend used by visuals so sound and image crossfade together.
6. Balance the full mix in clear and rainy conditions, with and without nitro,
   then add mute/volume persistence and final browser QA.

## Verification path

- Unit-test transition detection: no cue on steady state, exactly one cue per
  accepted shift/bump/pickup/menu move, and a distinct clank for rejected HIGH.
- Unit-test mixer curves at stopped, LOW limit, HIGH maximum, nitro, rain
  crossfade, wet skid, pause, and time-up boundaries.
- Publish compact QA state (`audio_unlocked`, active layers, target pitch and
  volume, last cue, cue count) so browser tests can observe behavior without
  trying to classify recorded sound.
- Validate every asset loads in native and release WASM builds. In Chromium,
  verify silence before user interaction, successful unlock on confirmation,
  and correct mute/resume behavior.
- Record representative clear, rainy, crash-recovery, rejected-shift, and
  nitro runs; inspect for clipping and listen for audible loop seams.

## Scope boundary

This phase adds sound effects and ambience, not a soundtrack. Music can be
planned separately after the effects mix establishes how much frequency and
dynamic space remains.
