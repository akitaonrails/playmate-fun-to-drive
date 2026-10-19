#!/usr/bin/env bash
set -euo pipefail

# Six deterministic, original 100-second PSG arrangements. They use broad
# late-'80s/early-'00s driving-game vocabularies without borrowing melodies.
project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
output_dir="$project_root/crates/game-app/assets/audio"
work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT
mkdir -p "$output_dir"

note() {
  local output=$1 offset=$2 duration=$3 wave=$4 frequency=$5 gain=$6
  sox -R -n -r 22050 -c 1 "$output" \
    synth "$duration" "$wave" "$frequency" fade t 0.006 "$duration" 0.028 \
    gain -n "$gain" pad "$offset" 0
}

build_section() {
  local name=$1 bass_text=$2 lead_a_text=$3 lead_b_text=$4 chord_text=$5 lead_wave=$6 drum_style=$7
  local -a bass lead_a lead_b chords parts
  read -r -a bass <<< "$bass_text"
  read -r -a lead_a <<< "$lead_a_text"
  read -r -a lead_b <<< "$lead_b_text"
  read -r -a chords <<< "$chord_text"
  parts=()

  # Syncopated PSG bass with a quieter octave layer for arcade-cabinet weight.
  for step in $(seq 0 39); do
    local offset frequency octave bass_file octave_file
    offset=$(awk -v n="$step" 'BEGIN { printf "%.3f", n * 0.5 }')
    frequency=${bass[$((step % ${#bass[@]}))]}
    octave=$(awk -v f="$frequency" 'BEGIN { printf "%.3f", f * 2.0 }')
    bass_file="$work_dir/${name}_bass_${step}.wav"
    octave_file="$work_dir/${name}_bass_oct_${step}.wav"
    note "$bass_file" "$offset" 0.43 square "$frequency" -16
    note "$octave_file" "$offset" 0.24 sawtooth "$octave" -29
    parts+=("$bass_file" "$octave_file")
  done

  # Two original ten-second motifs give every section a call/response shape.
  for step in $(seq 0 79); do
    local offset frequency lead_file shadow_file motif_step
    offset=$(awk -v n="$step" 'BEGIN { printf "%.3f", n * 0.25 }')
    motif_step=$((step % 40))
    if (( step < 40 )); then
      frequency=${lead_a[$((motif_step % ${#lead_a[@]}))]}
    else
      frequency=${lead_b[$((motif_step % ${#lead_b[@]}))]}
    fi
    [[ "$frequency" == "0" ]] && continue
    lead_file="$work_dir/${name}_lead_${step}.wav"
    shadow_file="$work_dir/${name}_shadow_${step}.wav"
    note "$lead_file" "$offset" 0.205 "$lead_wave" "$frequency" -20
    note "$shadow_file" "$offset" 0.16 triangle \
      "$(awk -v f="$frequency" 'BEGIN { printf "%.3f", f * 0.5 }')" -31
    parts+=("$lead_file" "$shadow_file")
  done

  # Long root/fifth pads and a quiet four-note PSG arpeggiator make the harmony
  # audible even under engine and weather layers.
  local -a arp_ratios=(1.0 1.259921 1.498307 2.0)
  for bar in $(seq 0 7); do
    local offset root fifth root_file fifth_file
    offset=$(awk -v n="$bar" 'BEGIN { printf "%.3f", n * 2.5 }')
    root=${chords[$((bar % ${#chords[@]}))]}
    fifth=$(awk -v f="$root" 'BEGIN { printf "%.3f", f * 1.498307 }')
    root_file="$work_dir/${name}_pad_root_${bar}.wav"
    fifth_file="$work_dir/${name}_pad_fifth_${bar}.wav"
    note "$root_file" "$offset" 2.42 sawtooth "$root" -28
    note "$fifth_file" "$offset" 2.42 sine "$fifth" -26
    parts+=("$root_file" "$fifth_file")
  done
  for step in $(seq 0 79); do
    local offset root ratio frequency arp_file
    offset=$(awk -v n="$step" 'BEGIN { printf "%.3f", n * 0.25 }')
    root=${chords[$(((step / 10) % ${#chords[@]}))]}
    ratio=${arp_ratios[$((step % 4))]}
    frequency=$(awk -v f="$root" -v r="$ratio" 'BEGIN { printf "%.3f", f * r * 2.0 }')
    arp_file="$work_dir/${name}_arp_${step}.wav"
    note "$arp_file" "$offset" 0.105 triangle "$frequency" -32
    parts+=("$arp_file")
  done

  # Gated chip drums. Styles alter kick weight, snare brightness, and hat
  # density while preserving a stable 120 BPM section grid.
  for beat in $(seq 0 39); do
    local offset kick_file kick_gain snare_gain
    offset=$(awk -v n="$beat" 'BEGIN { printf "%.3f", n * 0.5 }')
    kick_gain=-13
    snare_gain=-20
    [[ "$drum_style" == "rock" ]] && kick_gain=-10
    [[ "$drum_style" == "industrial" ]] && snare_gain=-16
    kick_file="$work_dir/${name}_kick_${beat}.wav"
    sox -R -n -r 22050 -c 1 "$kick_file" synth 0.13 sine 104-43 \
      fade t 0.002 0.13 0.075 gain -n "$kick_gain" pad "$offset" 0
    parts+=("$kick_file")
    if (( beat % 2 == 1 )); then
      local snare_file
      snare_file="$work_dir/${name}_snare_${beat}.wav"
      sox -R -n -r 22050 -c 1 "$snare_file" synth 0.115 whitenoise \
        gain "$snare_gain" highpass 1050 lowpass 6200 fade t 0.001 0.115 0.075 \
        pad "$offset" 0
      parts+=("$snare_file")
    fi
  done
  for hat in $(seq 0 79); do
    if [[ "$drum_style" == "breeze" ]] && (( hat % 2 == 1 )); then
      continue
    fi
    local offset hat_file
    offset=$(awk -v n="$hat" 'BEGIN { printf "%.3f", n * 0.25 }')
    hat_file="$work_dir/${name}_hat_${hat}.wav"
    sox -R -n -r 22050 -c 1 "$hat_file" synth 0.042 whitenoise \
      gain -29 highpass 4300 fade t 0 0.042 0.028 pad "$offset" 0
    parts+=("$hat_file")
  done

  local silence_file="$work_dir/${name}_silence.wav"
  sox -R -n -r 22050 -c 1 "$silence_file" trim 0 20.0
  parts+=("$silence_file")
  sox -R -m "${parts[@]}" "$work_dir/${name}_section.wav" trim 0 20.0 gain -n -5
}

finish_track() {
  local name=$1
  local section="$work_dir/${name}_section.wav"
  sox -R "$section" "$work_dir/${name}_intro.wav" lowpass 2900 gain -3
  sox -R "$section" "$work_dir/${name}_open.wav" treble 1.5 3200
  sox -R "$section" "$work_dir/${name}_bridge.wav" bass -1 180 lowpass 3900 gain -1
  sox -R "$section" "$work_dir/${name}_peak.wav" bass 1.5 160 treble 2.5 3000
  sox -R "$section" "$work_dir/${name}_final.wav" treble 1 2800
  sox -R "$work_dir/${name}_intro.wav" "$work_dir/${name}_open.wav" \
    "$work_dir/${name}_bridge.wav" "$work_dir/${name}_peak.wav" \
    "$work_dir/${name}_final.wav" "$output_dir/${name}.ogg" \
    trim 0 100.0 fade t 0.08 100.0 0.45 gain -n -5
}

# Bright gated pop-rock: buoyant bass, brass-like square lead, big backbeat.
build_section "music_voltage_hearts" \
  "82.41 82.41 123.47 164.81 73.42 73.42 110.00 146.83" \
  "329.63 0 415.30 493.88 0 415.30 369.99 329.63 0 277.18 329.63 369.99 415.30 0 493.88 0" \
  "493.88 440.00 415.30 0 369.99 329.63 0 277.18 329.63 369.99 415.30 493.88 0 554.37 493.88 0" \
  "164.81 220.00 138.59 185.00" square pop
finish_track "music_voltage_hearts"

# Tropical arcade fusion: syncopated rests, sunny pentatonic lead, light kit.
build_section "music_coastal_splash" \
  "73.42 110.00 146.83 110.00 82.41 123.47 164.81 123.47" \
  "587.33 0 659.25 0 739.99 659.25 587.33 0 493.88 0 554.37 587.33 0 493.88 440.00 0" \
  "440.00 0 493.88 554.37 0 659.25 739.99 0 659.25 587.33 0 554.37 493.88 0 440.00 0" \
  "146.83 196.00 164.81 220.00" triangle breeze
finish_track "music_coastal_splash"

# Breezy city-pop/fusion: relaxed hats, major-seventh color, lyrical response.
build_section "music_passing_lights" \
  "65.41 98.00 130.81 98.00 73.42 110.00 146.83 110.00" \
  "392.00 0 440.00 493.88 0 523.25 493.88 440.00 392.00 0 349.23 392.00 0 440.00 0 349.23" \
  "523.25 0 493.88 440.00 392.00 0 349.23 329.63 0 392.00 440.00 493.88 0 440.00 392.00 0" \
  "130.81 164.81 146.83 196.00" sine breeze
finish_track "music_passing_lights"

# Dramatic racing rock: minor pedal, organ-like saw lead, ascending answer.
build_section "music_redline_overture" \
  "82.41 82.41 82.41 123.47 73.42 73.42 110.00 146.83" \
  "329.63 392.00 440.00 493.88 0 440.00 392.00 369.99 329.63 0 293.66 329.63 369.99 392.00 440.00 0" \
  "246.94 293.66 329.63 369.99 392.00 440.00 493.88 0 587.33 493.88 440.00 392.00 0 369.99 329.63 0" \
  "164.81 146.83 130.81 185.00" sawtooth rock
finish_track "music_redline_overture"

# Hard-rock road anthem: power-chord bass motion and a short, punchy hook.
build_section "music_overpass_anthem" \
  "55.00 55.00 82.41 110.00 65.41 65.41 98.00 130.81" \
  "440.00 440.00 0 523.25 493.88 440.00 392.00 0 329.63 329.63 392.00 440.00 0 523.25 0 493.88" \
  "659.25 587.33 523.25 0 493.88 440.00 392.00 329.63 0 392.00 440.00 493.88 523.25 0 440.00 0" \
  "110.00 146.83 130.81 174.61" square rock
finish_track "music_overpass_anthem"

# Tense industrial alt-rock: chromatic bass, clipped motif, brighter snare.
build_section "music_static_signal" \
  "92.50 92.50 98.00 87.31 92.50 123.47 116.54 98.00" \
  "369.99 0 369.99 392.00 0 349.23 329.63 0 311.13 0 369.99 349.23 0 293.66 311.13 0" \
  "466.16 0 440.00 0 415.30 392.00 369.99 0 349.23 0 311.13 329.63 0 392.00 369.99 0" \
  "185.00 174.61 196.00 164.81" sawtooth industrial
finish_track "music_static_signal"

printf 'Generated 6 original 100-second PSG tracks in %s\n' "$output_dir"
