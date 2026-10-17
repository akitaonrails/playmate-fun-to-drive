//! Original 16-bit cabinet audio: authored one-shots plus state-driven loops.
//!
//! Audio observes presentation and authoritative simulation events. It never
//! feeds back into game-core, so sound loading and browser autoplay behavior
//! cannot change scoring or fixed-step timing.

use bevy::audio::{AudioSink, AudioSinkPlayback, Volume};
use bevy::prelude::*;
use game_core::{Gear, Phase};

use crate::{bridge, view, EndSession, FrameInput, Sim, TuningRes};

pub const STORAGE_KEY: &str = "fun-to-drive.audio.v1";
const EFFECTS_GAIN: f32 = 0.94;
const MUSIC_GAIN: f32 = 1.18;

pub struct ArcadeAudioPlugin;

impl Plugin for ArcadeAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioEvents>()
            .add_systems(Startup, setup_audio)
            .add_systems(
                Update,
                (
                    unlock_on_input,
                    toggle_mute,
                    adjust_volume,
                    update_mix,
                    schedule_weather_cues,
                    play_cues,
                )
                    .chain()
                    .in_set(crate::AppSet::Audio),
            );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AudioCue {
    Shift,
    Clank,
    Crash,
    NitroPickup,
    NitroStart,
    Thunder,
    MenuMove,
    MenuConfirm,
    RunStart,
    Key,
    Delete,
    Submit,
}

impl AudioCue {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Shift => "shift",
            Self::Clank => "clank",
            Self::Crash => "crash",
            Self::NitroPickup => "nitro_pickup",
            Self::NitroStart => "nitro_start",
            Self::Thunder => "thunder",
            Self::MenuMove => "menu_move",
            Self::MenuConfirm => "menu_confirm",
            Self::RunStart => "run_start",
            Self::Key => "key",
            Self::Delete => "delete",
            Self::Submit => "submit",
        }
    }

    const fn volume(self) -> f32 {
        let base = match self {
            Self::Crash => 0.55,
            Self::Thunder => 0.46,
            Self::NitroStart => 0.45,
            Self::NitroPickup | Self::RunStart => 0.38,
            Self::Shift | Self::Clank | Self::MenuConfirm | Self::Submit => 0.34,
            Self::MenuMove | Self::Key | Self::Delete => 0.25,
        };
        base * EFFECTS_GAIN
    }
}

#[derive(Resource, Default)]
pub(crate) struct AudioEvents(Vec<AudioCue>);

impl AudioEvents {
    pub(crate) fn emit(&mut self, cue: AudioCue) {
        self.0.push(cue);
    }
}

#[derive(Resource)]
pub(crate) struct AudioState {
    pub(crate) unlocked: bool,
    pub(crate) muted: bool,
    pub(crate) music_gain: f32,
    pub(crate) effects_gain: f32,
    pub(crate) level_notice: &'static str,
    pub(crate) level_notice_secs: f32,
    pub(crate) engine_low: f32,
    pub(crate) engine_high: f32,
    pub(crate) wind: f32,
    pub(crate) rain: f32,
    pub(crate) brake: f32,
    pub(crate) skid: f32,
    pub(crate) overrev: f32,
    pub(crate) nitro: f32,
    pub(crate) music_sunset: f32,
    pub(crate) music_neon: f32,
    pub(crate) music_dawn: f32,
    pub(crate) music_track: u8,
    pub(crate) engine_pitch: f32,
    pub(crate) last_cue: &'static str,
    pub(crate) cue_count: u64,
}

impl AudioState {
    pub(crate) fn new(muted: bool) -> Self {
        Self {
            unlocked: false,
            muted,
            music_gain: 1.0,
            effects_gain: 1.0,
            level_notice: "",
            level_notice_secs: 0.0,
            engine_low: 0.0,
            engine_high: 0.0,
            wind: 0.0,
            rain: 0.0,
            brake: 0.0,
            skid: 0.0,
            overrev: 0.0,
            nitro: 0.0,
            music_sunset: 0.0,
            music_neon: 0.0,
            music_dawn: 0.0,
            music_track: 0,
            engine_pitch: 0.65,
            last_cue: "none",
            cue_count: 0,
        }
    }

    pub(crate) fn unlock(&mut self) {
        self.unlocked = true;
    }
}

const VOLUME_STEP: f32 = 0.1;
const MAX_USER_GAIN: f32 = 1.5;

fn stepped_gain(current: f32, down: bool, up: bool) -> f32 {
    match (down, up) {
        (true, false) => (current - VOLUME_STEP).max(0.0),
        (false, true) => (current + VOLUME_STEP).min(MAX_USER_GAIN),
        _ => current,
    }
}

fn adjust_volume(keys: Res<ButtonInput<KeyCode>>, time: Res<Time>, mut state: ResMut<AudioState>) {
    state.level_notice_secs = (state.level_notice_secs - time.delta_secs()).max(0.0);
    let music = stepped_gain(
        state.music_gain,
        keys.just_pressed(KeyCode::BracketLeft),
        keys.just_pressed(KeyCode::BracketRight),
    );
    let effects = stepped_gain(
        state.effects_gain,
        keys.just_pressed(KeyCode::Minus),
        keys.just_pressed(KeyCode::Equal),
    );
    if music != state.music_gain {
        state.music_gain = music;
        state.level_notice = "MUSIC";
        state.level_notice_secs = 1.4;
    }
    if effects != state.effects_gain {
        state.effects_gain = effects;
        state.level_notice = "SFX";
        state.level_notice_secs = 1.4;
    }
}

#[derive(Resource)]
struct AudioAssets {
    shift: Handle<AudioSource>,
    clank: Handle<AudioSource>,
    crash: Handle<AudioSource>,
    nitro_pickup: Handle<AudioSource>,
    nitro_start: Handle<AudioSource>,
    thunder: Handle<AudioSource>,
    menu_move: Handle<AudioSource>,
    menu_confirm: Handle<AudioSource>,
    run_start: Handle<AudioSource>,
    key: Handle<AudioSource>,
    delete: Handle<AudioSource>,
    submit: Handle<AudioSource>,
}

impl AudioAssets {
    fn source(&self, cue: AudioCue) -> Handle<AudioSource> {
        match cue {
            AudioCue::Shift => self.shift.clone(),
            AudioCue::Clank => self.clank.clone(),
            AudioCue::Crash => self.crash.clone(),
            AudioCue::NitroPickup => self.nitro_pickup.clone(),
            AudioCue::NitroStart => self.nitro_start.clone(),
            AudioCue::Thunder => self.thunder.clone(),
            AudioCue::MenuMove => self.menu_move.clone(),
            AudioCue::MenuConfirm => self.menu_confirm.clone(),
            AudioCue::RunStart => self.run_start.clone(),
            AudioCue::Key => self.key.clone(),
            AudioCue::Delete => self.delete.clone(),
            AudioCue::Submit => self.submit.clone(),
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum AudioLayer {
    EngineLow,
    EngineHigh,
    Wind,
    Rain,
    Brake,
    Skid,
    Overrev,
    Nitro,
    MusicSunset,
    MusicNeon,
    MusicDawn,
}

fn setup_audio(mut commands: Commands, assets: Res<AssetServer>) {
    let load = |name: &'static str| assets.load(format!("audio/{name}.ogg"));
    let one_shots = AudioAssets {
        shift: load("shift"),
        clank: load("clank"),
        crash: load("crash"),
        nitro_pickup: load("nitro_pickup"),
        nitro_start: load("nitro_start"),
        thunder: load("thunder"),
        menu_move: load("menu_move"),
        menu_confirm: load("menu_confirm"),
        run_start: load("run_start"),
        key: load("key"),
        delete: load("delete"),
        submit: load("submit"),
    };

    for (layer, source) in [
        (AudioLayer::EngineLow, load("engine_low")),
        (AudioLayer::EngineHigh, load("engine_high")),
        (AudioLayer::Wind, load("wind")),
        (AudioLayer::Rain, load("rain")),
        (AudioLayer::Brake, load("brake")),
        (AudioLayer::Skid, load("wet_skid")),
        (AudioLayer::Overrev, load("overrev")),
        (AudioLayer::Nitro, load("nitro_burn")),
        (AudioLayer::MusicSunset, load("music_sunset_drive")),
        (AudioLayer::MusicNeon, load("music_neon_midnight")),
        (AudioLayer::MusicDawn, load("music_dawn_circuit")),
    ] {
        commands.spawn((
            AudioPlayer::new(source),
            PlaybackSettings::LOOP.with_volume(Volume::SILENT),
            layer,
        ));
    }
    commands.insert_resource(one_shots);
}

/// Browsers require audio to be resumed from a user gesture. Unlock on the
/// first keyboard or gamepad press so menu cues and gameplay audio cannot be
/// lost merely because a specific confirmation path was not used.
fn unlock_on_input(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut state: ResMut<AudioState>,
) {
    let pressed = keys.get_just_pressed().next().is_some()
        || pads
            .iter()
            .any(|pad| pad.get_just_pressed().next().is_some());
    if pressed {
        // Retrying matters on the web: an earlier AudioContext::resume promise
        // may have been rejected because it raced browser user activation.
        bridge::resume_audio();
        state.unlock();
    }
}

fn toggle_mute(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    end: Res<EndSession>,
    mut state: ResMut<AudioState>,
) {
    // M remains available for leaderboard initials while that editor is open.
    if end.0.mode == crate::session::EndMode::NameEntry {
        return;
    }
    let requested = keys.just_pressed(KeyCode::KeyM)
        || pads.iter().any(|pad| pad.just_pressed(GamepadButton::East));
    if requested {
        state.unlocked = true;
        state.muted = !state.muted;
        let _ = bridge::save_local(STORAGE_KEY, if state.muted { "muted" } else { "sound" });
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct MixTargets {
    engine_low: f32,
    engine_high: f32,
    wind: f32,
    rain: f32,
    brake: f32,
    skid: f32,
    overrev: f32,
    nitro: f32,
    music: [f32; 3],
    engine_pitch: f32,
}

#[derive(Debug, Clone, Copy)]
struct MixInput {
    active: bool,
    music_active: bool,
    speed: f32,
    max_speed: f32,
    gear: Gear,
    overrev: f32,
    rain: f32,
    steer: f32,
    brake: f32,
    off_road: f32,
    nitro: bool,
    music_track: usize,
}

fn mix_targets(input: MixInput) -> MixTargets {
    let speed = (input.speed / input.max_speed.max(0.01)).clamp(0.0, 1.3);
    let gear_pitch = if input.gear == Gear::High { 0.05 } else { 0.0 };
    let pitch = (0.65 + speed * 0.72 + gear_pitch).clamp(0.65, 1.48);
    let music = {
        let mut levels = [0.0; 3];
        if input.music_active {
            levels[input.music_track.min(2)] =
                (if input.nitro { 0.075 } else { 0.095 }) * MUSIC_GAIN;
        }
        levels
    };
    if !input.active {
        return MixTargets {
            music,
            engine_pitch: pitch,
            ..default()
        };
    }

    let engine = (0.055 + 0.115 * speed.min(1.0)) * EFFECTS_GAIN;
    let (engine_low, engine_high) = match input.gear {
        Gear::Low => (engine, engine * 0.08),
        Gear::High => (engine * 0.12, engine * 1.05),
    };
    let slip = ((input.steer.abs() - 0.52) / 0.48)
        .max(input.off_road)
        .max((input.brake - 0.40) / 0.60)
        .clamp(0.0, 1.0);

    MixTargets {
        engine_low,
        engine_high,
        wind: (0.015 + 0.115 * speed.powf(1.45)).min(0.15) * EFFECTS_GAIN,
        rain: 0.115 * input.rain.clamp(0.0, 1.0) * EFFECTS_GAIN,
        brake: 0.13 * input.brake.clamp(0.0, 1.0).powf(0.72) * speed.min(1.0) * EFFECTS_GAIN,
        skid: 0.14 * input.rain.clamp(0.0, 1.0) * slip * speed.min(1.0) * EFFECTS_GAIN,
        overrev: 0.105 * input.overrev.clamp(0.0, 1.0) * EFFECTS_GAIN,
        nitro: if input.nitro {
            0.24 * EFFECTS_GAIN
        } else {
            0.0
        },
        music,
        engine_pitch: pitch,
    }
}

#[allow(clippy::too_many_arguments)]
fn update_mix(
    time: Res<Time>,
    sim: Res<Sim>,
    tuning: Res<TuningRes>,
    frame: Res<FrameInput>,
    end: Res<EndSession>,
    weather: Res<view::WeatherVisuals>,
    mut state: ResMut<AudioState>,
    mut layers: Query<(&AudioLayer, &mut AudioSink)>,
) {
    let audible = state.unlocked && !state.muted;
    let active = audible
        && end.0.mode == crate::session::EndMode::Playing
        && matches!(sim.0.phase, Phase::Running)
        && !sim.0.paused;
    let target = mix_targets(MixInput {
        active,
        music_active: audible,
        speed: sim.0.speed,
        max_speed: sim.0.high_speed_target(&tuning.0),
        gear: sim.0.gear(),
        overrev: sim.0.low_overrev(&tuning.0),
        rain: weather.rain,
        steer: frame.state.steer,
        brake: frame.state.brake,
        off_road: sim.0.off_road,
        nitro: crate::nitro_effects_active(&sim.0),
        music_track: (sim.0.day_cycles % 3) as usize,
    });

    let alpha = 1.0 - (-time.delta_secs() * 9.0).exp();
    state.engine_low += (target.engine_low - state.engine_low) * alpha;
    state.engine_high += (target.engine_high - state.engine_high) * alpha;
    state.wind += (target.wind - state.wind) * alpha;
    state.rain += (target.rain - state.rain) * alpha;
    state.brake += (target.brake - state.brake) * alpha;
    state.skid += (target.skid - state.skid) * alpha;
    state.overrev += (target.overrev - state.overrev) * alpha;
    state.nitro += (target.nitro - state.nitro) * alpha;
    state.engine_pitch += (target.engine_pitch - state.engine_pitch) * alpha;
    // Slower gain movement gives soundtrack changes a deliberate crossfade.
    let music_alpha = 1.0 - (-time.delta_secs() * 1.25).exp();
    state.music_sunset += (target.music[0] - state.music_sunset) * music_alpha;
    state.music_neon += (target.music[1] - state.music_neon) * music_alpha;
    state.music_dawn += (target.music[2] - state.music_dawn) * music_alpha;
    state.music_track = (sim.0.day_cycles % 3) as u8;

    for (layer, mut sink) in &mut layers {
        let volume = match layer {
            AudioLayer::EngineLow => state.engine_low,
            AudioLayer::EngineHigh => state.engine_high,
            AudioLayer::Wind => state.wind,
            AudioLayer::Rain => state.rain,
            AudioLayer::Brake => state.brake,
            AudioLayer::Skid => state.skid,
            AudioLayer::Overrev => state.overrev,
            AudioLayer::Nitro => state.nitro,
            AudioLayer::MusicSunset => state.music_sunset,
            AudioLayer::MusicNeon => state.music_neon,
            AudioLayer::MusicDawn => state.music_dawn,
        };
        let user_gain = match layer {
            AudioLayer::MusicSunset | AudioLayer::MusicNeon | AudioLayer::MusicDawn => {
                state.music_gain
            }
            _ => state.effects_gain,
        };
        sink.set_volume(Volume::Linear(volume * user_gain));
        match layer {
            AudioLayer::EngineLow => sink.set_speed(state.engine_pitch),
            AudioLayer::EngineHigh => sink.set_speed((state.engine_pitch * 1.03).min(1.52)),
            AudioLayer::Wind => sink.set_speed((0.82 + sim.0.speed / 80.0).min(1.18)),
            _ => {}
        }
    }
}

#[derive(Default)]
struct ThunderSchedule {
    slot: Option<u64>,
    emitted: u8,
}

fn thunder_strike_times(seed: u64, slot: u64) -> [Option<f32>; 2] {
    let mixed = seed
        .wrapping_add(slot.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .rotate_left(23)
        ^ 0xD1B5_4A32_D192_ED03;
    let first = 7.0 + (mixed & 0x7ff) as f32 / 2047.0 * 8.0;
    let second = (mixed & 0x3 == 0).then(|| 20.0 + ((mixed >> 13) & 0x7ff) as f32 / 2047.0 * 7.0);
    [Some(first), second]
}

fn schedule_weather_cues(
    sim: Res<Sim>,
    end: Res<EndSession>,
    weather: Res<view::WeatherVisuals>,
    mut schedule: Local<ThunderSchedule>,
    mut events: ResMut<AudioEvents>,
) {
    let transition = sim.0.weather();
    if schedule.slot != Some(transition.slot) {
        schedule.slot = Some(transition.slot);
        schedule.emitted = 0;
    }
    if end.0.mode != crate::session::EndMode::Playing
        || !matches!(sim.0.phase, Phase::Running)
        || sim.0.paused
        || weather.rain_visible < 0.55
    {
        return;
    }

    let slot_time =
        sim.0.environment_time - transition.slot as f32 * game_core::environment::WEATHER_SLOT_SECS;
    for (index, strike_time) in thunder_strike_times(sim.0.seed, transition.slot)
        .into_iter()
        .enumerate()
    {
        let bit = 1 << index;
        if schedule.emitted & bit == 0 && strike_time.is_some_and(|time| slot_time >= time) {
            events.emit(AudioCue::Thunder);
            schedule.emitted |= bit;
        }
    }
}

fn play_cues(
    mut commands: Commands,
    assets: Option<Res<AudioAssets>>,
    mut events: ResMut<AudioEvents>,
    mut state: ResMut<AudioState>,
) {
    let Some(assets) = assets else {
        events.0.clear();
        return;
    };
    for cue in events.0.drain(..) {
        state.last_cue = cue.name();
        state.cue_count = state.cue_count.saturating_add(1);
        if state.unlocked && !state.muted {
            commands.spawn((
                AudioPlayer::new(assets.source(cue)),
                PlaybackSettings::DESPAWN
                    .with_volume(Volume::Linear(cue.volume() * state.effects_gain)),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn driving() -> MixInput {
        MixInput {
            active: true,
            music_active: true,
            speed: 12.0,
            max_speed: 24.0,
            gear: Gear::Low,
            overrev: 0.0,
            rain: 0.0,
            steer: 0.0,
            brake: 0.0,
            off_road: 0.0,
            nitro: false,
            music_track: 0,
        }
    }

    #[test]
    fn silent_when_session_is_not_active() {
        let targets = mix_targets(MixInput {
            active: false,
            music_active: false,
            ..driving()
        });
        assert_eq!(targets.engine_low, 0.0);
        assert_eq!(targets.wind, 0.0);
        assert_eq!(targets.rain, 0.0);
        assert_eq!(targets.brake, 0.0);
        assert_eq!(targets.music, [0.0; 3]);
    }

    #[test]
    fn soundtrack_keeps_playing_outside_the_driving_session() {
        let targets = mix_targets(MixInput {
            active: false,
            music_active: true,
            music_track: 1,
            ..driving()
        });
        assert_eq!(targets.engine_low, 0.0);
        assert_eq!(targets.wind, 0.0);
        assert_eq!(
            targets.music.iter().filter(|level| **level > 0.0).count(),
            1
        );
        assert!(targets.music[1] > 0.09);
    }

    #[test]
    fn gears_crossfade_engine_layers() {
        let low = mix_targets(driving());
        let high = mix_targets(MixInput {
            gear: Gear::High,
            ..driving()
        });
        assert!(low.engine_low > low.engine_high * 5.0);
        assert!(high.engine_high > high.engine_low * 5.0);
        assert!(high.engine_pitch > low.engine_pitch);
    }

    #[test]
    fn mix_balance_softens_effects_and_lifts_music() {
        let targets = mix_targets(driving());
        let raw_engine = 0.055 + 0.115 * 0.5;
        assert!(targets.engine_low < raw_engine);
        assert!(targets.music[0] > 0.095);
        assert!(AudioCue::Crash.volume() < 0.55);
    }

    #[test]
    fn thunder_schedule_is_deterministic_sparse_and_inside_each_weather_slot() {
        for seed in 0..16 {
            for slot in 0..16 {
                let strikes = thunder_strike_times(seed, slot);
                assert_eq!(strikes, thunder_strike_times(seed, slot));
                assert!(strikes[0].is_some_and(|time| (7.0..=15.0).contains(&time)));
                assert!(strikes[1].is_none_or(|time| (20.0..=27.0).contains(&time)));
            }
        }
    }

    #[test]
    fn weather_skid_and_nitro_are_conditional() {
        let dry = mix_targets(driving());
        let wet = mix_targets(MixInput {
            rain: 1.0,
            steer: 1.0,
            nitro: true,
            ..driving()
        });
        assert_eq!(dry.rain, 0.0);
        assert_eq!(dry.skid, 0.0);
        assert_eq!(dry.nitro, 0.0);
        assert!(wet.rain > 0.0);
        assert!(wet.skid > 0.0);
        assert!(wet.nitro > 0.0);
        assert!(wet.music[0] > 0.0);
        assert!(
            wet.music[0] < dry.music[0],
            "nitro should gently duck music"
        );
    }

    #[test]
    fn brake_layer_tracks_pressure_and_requires_motion() {
        let coasting = mix_targets(driving());
        let braking = mix_targets(MixInput {
            brake: 0.65,
            ..driving()
        });
        let stopped = mix_targets(MixInput {
            speed: 0.0,
            brake: 1.0,
            ..driving()
        });
        assert_eq!(coasting.brake, 0.0);
        assert!(braking.brake > 0.0);
        assert_eq!(stopped.brake, 0.0);
    }

    #[test]
    fn overrev_layer_tracks_authoritative_intensity() {
        let quiet = mix_targets(driving());
        let early_warning = mix_targets(MixInput {
            overrev: 0.4,
            ..driving()
        });
        let warning = mix_targets(MixInput {
            overrev: 0.8,
            ..driving()
        });
        assert_eq!(quiet.overrev, 0.0);
        assert!(early_warning.overrev > 0.0);
        assert!((warning.overrev - early_warning.overrev * 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn user_volume_steps_are_clamped_and_conflicts_are_ignored() {
        assert!((stepped_gain(1.0, true, false) - 0.9).abs() < f32::EPSILON);
        assert!((stepped_gain(1.0, false, true) - 1.1).abs() < f32::EPSILON);
        assert_eq!(stepped_gain(0.0, true, false), 0.0);
        assert_eq!(stepped_gain(MAX_USER_GAIN, false, true), MAX_USER_GAIN);
        assert_eq!(stepped_gain(0.7, true, true), 0.7);
    }

    #[test]
    fn soundtrack_selects_exactly_one_track_per_cycle() {
        for track in 0..3 {
            let targets = mix_targets(MixInput {
                music_track: track,
                ..driving()
            });
            assert_eq!(
                targets.music.iter().filter(|level| **level > 0.0).count(),
                1
            );
            assert!(targets.music[track] > 0.09);
        }
    }
}
