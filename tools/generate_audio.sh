#!/usr/bin/env bash
set -euo pipefail

# Deterministic, original late-1980s arcade effects for Fun to Drive.
# SoX's -R flag fixes dither/noise seeds, making regenerated assets stable.
project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
output_dir="$project_root/crates/game-app/assets/audio"
work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT

mkdir -p "$output_dir"

tone() {
  local output=$1 duration=$2 type=$3 frequency=$4 gain=$5
  sox -R -n -r 22050 -c 1 "$output" \
    synth "$duration" "$type" "$frequency" gain -n "$gain"
}

noise() {
  local output=$1 duration=$2 colour=$3 gain=$4
  sox -R -n -r 22050 -c 1 "$output" \
    synth "$duration" "${colour}noise" gain -n "$gain"
}

mix_to_ogg() {
  local output=$1
  shift
  sox -R -m "$@" "$output" gain -n -3
}

# Continuous cabinet layers. Two-second beds keep repetition subtle while
# staying tiny enough for an instant browser load.
tone "$work_dir/low_a.wav" 2.0 square 55 -12
tone "$work_dir/low_b.wav" 2.0 sawtooth 110 -19
tone "$work_dir/low_c.wav" 2.0 square 165 -25
sox -R -m "$work_dir/low_a.wav" "$work_dir/low_b.wav" "$work_dir/low_c.wav" \
  "$output_dir/engine_low.ogg" tremolo 11 28 lowpass 1250 gain -n -5

tone "$work_dir/high_a.wav" 2.0 square 92 -15
tone "$work_dir/high_b.wav" 2.0 sawtooth 184 -20
tone "$work_dir/high_c.wav" 2.0 sine 368 -23
sox -R -m "$work_dir/high_a.wav" "$work_dir/high_b.wav" "$work_dir/high_c.wav" \
  "$output_dir/engine_high.ogg" tremolo 17 24 highpass 70 lowpass 2300 gain -n -5

noise "$work_dir/wind.wav" 2.0 pink -11
sox -R "$work_dir/wind.wav" "$output_dir/wind.ogg" highpass 320 lowpass 3800 \
  tremolo 0.65 18 gain -n -8

# Rain should read as a soft sheet on glass/road, not bright radio static.
# Layering filtered pink and brown noise gives it body; two slow, shallow
# modulations avoid the old machine-gun tick pattern while retaining a little
# 16-bit cabinet movement. Four seconds makes repetition much less obvious.
noise "$work_dir/rain_spray.wav" 4.0 pink -10
noise "$work_dir/rain_body.wav" 4.0 brown -13
sox -R "$work_dir/rain_spray.wav" "$work_dir/rain_spray_f.wav" \
  highpass 420 lowpass 4600 tremolo 0.37 11
sox -R "$work_dir/rain_body.wav" "$work_dir/rain_body_f.wav" \
  highpass 110 lowpass 1500 tremolo 0.23 16
sox -R -m "$work_dir/rain_spray_f.wav" "$work_dir/rain_body_f.wav" \
  "$output_dir/rain.ogg" gain -n -8

# Brake pad/tire drag: a restrained, pressure-controlled chip-noise bed. It
# stays distinct from the wetter, brighter skid layer below.
noise "$work_dir/brake_noise.wav" 1.5 pink -12
tone "$work_dir/brake_pulse.wav" 1.5 square 72 -25
sox -R "$work_dir/brake_noise.wav" "$work_dir/brake_noisef.wav" highpass 210 lowpass 1900 \
  tremolo 10 42
sox -R -m "$work_dir/brake_noisef.wav" "$work_dir/brake_pulse.wav" \
  "$output_dir/brake.ogg" lowpass 2200 gain -n -8

noise "$work_dir/skid.wav" 1.5 white -10
sox -R "$work_dir/skid.wav" "$output_dir/wet_skid.ogg" highpass 420 lowpass 3100 \
  tremolo 7 35 gain -n -7

# Dry shoulder gravel: irregular, low-mid chip noise with a faint repeating
# pebble pulse. It deliberately avoids the wet skid's bright continuous hiss.
noise "$work_dir/gravel_body.wav" 2.4 brown -8
noise "$work_dir/gravel_pebbles.wav" 2.4 white -14
sox -R "$work_dir/gravel_body.wav" "$work_dir/gravel_body_f.wav" \
  highpass 120 lowpass 1900 tremolo 13 48
sox -R "$work_dir/gravel_pebbles.wav" "$work_dir/gravel_pebbles_f.wav" \
  highpass 900 lowpass 3900 tremolo 19 72
sox -R -m "$work_dir/gravel_body_f.wav" "$work_dir/gravel_pebbles_f.wav" \
  "$output_dir/gravel.ogg" gain -n -9

tone "$work_dir/overrev_a.wav" 1.0 square 152 -14
noise "$work_dir/overrev_b.wav" 1.0 white -18
sox -R "$work_dir/overrev_b.wav" "$work_dir/overrev_bf.wav" highpass 950 lowpass 2800
sox -R -m "$work_dir/overrev_a.wav" "$work_dir/overrev_bf.wav" \
  "$output_dir/overrev.ogg" tremolo 12 78 gain -n -7

noise "$work_dir/nitro_noise.wav" 1.5 pink -8
tone "$work_dir/nitro_tone.wav" 1.5 sawtooth 620 -18
sox -R "$work_dir/nitro_noise.wav" "$work_dir/nitro_noisef.wav" highpass 650 lowpass 7200
sox -R -m "$work_dir/nitro_noisef.wav" "$work_dir/nitro_tone.wav" \
  "$output_dir/nitro_burn.ogg" tremolo 16 20 gain -n -5

# Mechanical and collision one-shots.
tone "$work_dir/shift_a.wav" 0.16 square 240-105 -8
noise "$work_dir/shift_b.wav" 0.055 white -6
sox -R "$work_dir/shift_b.wav" "$work_dir/shift_bf.wav" highpass 1700 fade t 0 0.050 0.040
mix_to_ogg "$output_dir/shift.ogg" "$work_dir/shift_a.wav" "$work_dir/shift_bf.wav"

tone "$work_dir/clank_a.wav" 0.30 square 175-82 -7
tone "$work_dir/clank_b.wav" 0.30 sine 690-285 -10
noise "$work_dir/clank_c.wav" 0.075 white -8
sox -R "$work_dir/clank_c.wav" "$work_dir/clank_cf.wav" highpass 1200 fade t 0 0.075 0.06 pad 0 0.225
mix_to_ogg "$output_dir/clank.ogg" "$work_dir/clank_a.wav" "$work_dir/clank_b.wav" "$work_dir/clank_cf.wav"

tone "$work_dir/crash_a.wav" 0.55 square 105-38 -5
noise "$work_dir/crash_b.wav" 0.55 brown -4
sox -R "$work_dir/crash_b.wav" "$work_dir/crash_bf.wav" highpass 90 lowpass 3600 \
  fade t 0.003 0.54 0.41
mix_to_ogg "$output_dir/crash.ogg" "$work_dir/crash_a.wav" "$work_dir/crash_bf.wav"

# Weather punctuation: a short band-limited noise crack with two chip-like
# echoes, followed by a gated low-noise tail. Avoid a sustained bass note: it
# read as an engine fault rather than distant 16-bit thunder.
noise "$work_dir/thunder_crack.wav" 0.075 white -5
sox -R "$work_dir/thunder_crack.wav" "$work_dir/thunder_crack_f.wav" \
  highpass 720 lowpass 5200 fade t 0.001 0.074 0.045 pad 0 1.325
sox -R "$work_dir/thunder_crack_f.wav" "$work_dir/thunder_echo_a.wav" \
  pad 0.085 0 trim 0 1.4 gain -8
sox -R "$work_dir/thunder_crack_f.wav" "$work_dir/thunder_echo_b.wav" \
  pad 0.17 0 trim 0 1.4 gain -13
noise "$work_dir/thunder_rumble.wav" 1.30 pink -9
sox -R "$work_dir/thunder_rumble.wav" "$work_dir/thunder_rumble_f.wav" \
  highpass 65 lowpass 820 tremolo 12 48 fade t 0.015 1.30 0.62 pad 0.10 0
tone "$work_dir/thunder_pulse.wav" 1.18 square 92-47 -23
sox -R "$work_dir/thunder_pulse.wav" "$work_dir/thunder_pulse_f.wav" \
  lowpass 390 tremolo 9 42 fade t 0.01 1.17 0.55 pad 0.15 0.07
mix_to_ogg "$output_dir/thunder.ogg" "$work_dir/thunder_crack_f.wav" \
  "$work_dir/thunder_echo_a.wav" "$work_dir/thunder_echo_b.wav" \
  "$work_dir/thunder_rumble_f.wav" "$work_dir/thunder_pulse_f.wav"

# Nitro rewards and ignition.
tone "$work_dir/pickup_a.wav" 0.11 square 440 -13
tone "$work_dir/pickup_b.wav" 0.11 square 660 -12
tone "$work_dir/pickup_c.wav" 0.16 square 990 -10
sox -R "$work_dir/pickup_b.wav" "$work_dir/pickup_bp.wav" pad 0.09 0
sox -R "$work_dir/pickup_c.wav" "$work_dir/pickup_cp.wav" pad 0.18 0
mix_to_ogg "$output_dir/nitro_pickup.ogg" "$work_dir/pickup_a.wav" "$work_dir/pickup_bp.wav" "$work_dir/pickup_cp.wav"

noise "$work_dir/start_noise.wav" 0.46 pink -5
tone "$work_dir/start_tone.wav" 0.46 square 105-520 -10
sox -R "$work_dir/start_noise.wav" "$work_dir/start_noisef.wav" highpass 500 lowpass 6200 \
  fade t 0.006 0.46 0.15
mix_to_ogg "$output_dir/nitro_start.ogg" "$work_dir/start_noisef.wav" "$work_dir/start_tone.wav"

# Rare Jump Kick reward, launch, and clean landing.
tone "$work_dir/jump_pick_a.wav" 0.14 square 330-660 -13
tone "$work_dir/jump_pick_b.wav" 0.22 square 495-990 -15
sox -R "$work_dir/jump_pick_b.wav" "$work_dir/jump_pick_bp.wav" pad 0.10 0
mix_to_ogg "$output_dir/jump_pickup.ogg" "$work_dir/jump_pick_a.wav" "$work_dir/jump_pick_bp.wav"

noise "$work_dir/jump_start_noise.wav" 0.42 pink -8
tone "$work_dir/jump_start_tone.wav" 0.42 square 145-720 -13
sox -R "$work_dir/jump_start_noise.wav" "$work_dir/jump_start_noisef.wav" \
  highpass 480 lowpass 4400 fade t 0.002 0.42 0.16
mix_to_ogg "$output_dir/jump_start.ogg" "$work_dir/jump_start_tone.wav" \
  "$work_dir/jump_start_noisef.wav"

tone "$work_dir/jump_land_tone.wav" 0.24 square 92-48 -10
noise "$work_dir/jump_land_noise.wav" 0.18 brown -8
sox -R "$work_dir/jump_land_noise.wav" "$work_dir/jump_land_noisef.wav" \
  highpass 70 lowpass 950 fade t 0 0.18 0.12 pad 0.02 0.04
mix_to_ogg "$output_dir/jump_land.ogg" "$work_dir/jump_land_tone.wav" \
  "$work_dir/jump_land_noisef.wav"

# Menu and leaderboard UI vocabulary.
sox -R -n -r 22050 -c 1 "$output_dir/menu_move.ogg" \
  synth 0.085 square 330-470 fade t 0.002 0.085 0.035 gain -n -10

tone "$work_dir/confirm_a.wav" 0.20 square 440 -13
tone "$work_dir/confirm_b.wav" 0.20 square 660 -14
mix_to_ogg "$output_dir/menu_confirm.ogg" "$work_dir/confirm_a.wav" "$work_dir/confirm_b.wav"

tone "$work_dir/run_a.wav" 0.34 square 220-660 -13
tone "$work_dir/run_b.wav" 0.34 square 330-990 -16
noise "$work_dir/run_c.wav" 0.34 white -20
sox -R "$work_dir/run_c.wav" "$work_dir/run_cf.wav" highpass 2500 fade t 0.01 0.34 0.10
mix_to_ogg "$output_dir/run_start.ogg" "$work_dir/run_a.wav" "$work_dir/run_b.wav" "$work_dir/run_cf.wav"

sox -R -n -r 22050 -c 1 "$output_dir/key.ogg" \
  synth 0.055 square 720 fade t 0 0.055 0.04 gain -n -13
sox -R -n -r 22050 -c 1 "$output_dir/delete.ogg" \
  synth 0.075 square 360-210 fade t 0 0.075 0.05 gain -n -12
sox -R -n -r 22050 -c 1 "$output_dir/submit.ogg" \
  synth 0.28 square 440-880 fade t 0.003 0.28 0.08 gain -n -11

# Three original 16-second synthwave loops. The deliberately compact forms
# suit short arcade runs; runtime crossfades rotate them per day/night cycle.
music_note() {
  local output=$1 offset=$2 duration=$3 wave=$4 frequency=$5 gain=$6
  sox -R -n -r 22050 -c 1 "$output" \
    synth "$duration" "$wave" "$frequency" fade t 0.008 "$duration" 0.035 \
    gain -n "$gain" pad "$offset" 0
}

build_track() {
  local name=$1 bass_text=$2 lead_text=$3 chord_text=$4
  local -a bass lead chords parts
  read -r -a bass <<< "$bass_text"
  read -r -a lead <<< "$lead_text"
  read -r -a chords <<< "$chord_text"
  parts=()

  for step in $(seq 0 31); do
    local offset frequency file
    offset=$(awk -v n="$step" 'BEGIN { printf "%.3f", n * 0.5 }')
    frequency=${bass[$((step % ${#bass[@]}))]}
    file="$work_dir/${name}_bass_${step}.wav"
    music_note "$file" "$offset" 0.43 square "$frequency" -17
    parts+=("$file")
  done

  for step in $(seq 0 63); do
    local offset frequency file
    frequency=${lead[$((step % ${#lead[@]}))]}
    if [[ "$frequency" == "0" ]]; then
      continue
    fi
    offset=$(awk -v n="$step" 'BEGIN { printf "%.3f", n * 0.25 }')
    file="$work_dir/${name}_lead_${step}.wav"
    music_note "$file" "$offset" 0.20 square "$frequency" -23
    parts+=("$file")
  done

  for bar in $(seq 0 7); do
    local offset root fifth root_file fifth_file
    offset=$(awk -v n="$bar" 'BEGIN { printf "%.3f", n * 2.0 }')
    root=${chords[$((bar % ${#chords[@]}))]}
    fifth=$(awk -v f="$root" 'BEGIN { printf "%.3f", f * 1.498307 }')
    root_file="$work_dir/${name}_pad_root_${bar}.wav"
    fifth_file="$work_dir/${name}_pad_fifth_${bar}.wav"
    music_note "$root_file" "$offset" 1.92 sawtooth "$root" -27
    music_note "$fifth_file" "$offset" 1.92 sine "$fifth" -25
    parts+=("$root_file" "$fifth_file")
  done

  for beat in $(seq 0 31); do
    local offset kick_file hat_file
    offset=$(awk -v n="$beat" 'BEGIN { printf "%.3f", n * 0.5 }')
    kick_file="$work_dir/${name}_kick_${beat}.wav"
    sox -R -n -r 22050 -c 1 "$kick_file" synth 0.12 sine 96-42 \
      fade t 0.002 0.12 0.07 gain -n -13 pad "$offset" 0
    parts+=("$kick_file")
    if (( beat % 2 == 1 )); then
      local snare_file
      snare_file="$work_dir/${name}_snare_${beat}.wav"
      sox -R -n -r 22050 -c 1 "$snare_file" synth 0.10 whitenoise \
        gain -20 highpass 1150 lowpass 5800 fade t 0.001 0.10 0.07 pad "$offset" 0
      parts+=("$snare_file")
    fi
    hat_file="$work_dir/${name}_hat_${beat}.wav"
    sox -R -n -r 22050 -c 1 "$hat_file" synth 0.045 whitenoise \
      gain -27 highpass 4200 fade t 0 0.045 0.03 pad "$offset" 0
    parts+=("$hat_file")
  done

  sox -R -m "${parts[@]}" "$output_dir/${name}.ogg" pad 0 0.10 trim 0 16.0 gain -n -5
}

build_track "music_sunset_drive" \
  "65.41 65.41 77.78 98.00 58.27 58.27 77.78 87.31" \
  "311.13 0 392.00 466.16 0 392.00 349.23 0 293.66 0 349.23 392.00 0 466.16 392.00 0" \
  "130.81 116.54 98.00 116.54"

build_track "music_neon_midnight" \
  "87.31 87.31 103.83 130.81 77.78 77.78 103.83 116.54" \
  "415.30 0 523.25 622.25 0 523.25 466.16 415.30 0 349.23 415.30 466.16 0 523.25 0 466.16" \
  "174.61 155.56 130.81 155.56"

build_track "music_dawn_circuit" \
  "55.00 82.41 110.00 82.41 65.41 98.00 130.81 98.00" \
  "440.00 0 523.25 659.25 0 587.33 523.25 0 392.00 440.00 493.88 0 659.25 0 587.33 523.25" \
  "110.00 130.81 146.83 98.00"

printf 'Generated %s original arcade audio assets in %s\n' \
  "$(find "$output_dir" -maxdepth 1 -name '*.ogg' | wc -l)" "$output_dir"
