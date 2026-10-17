//! M7: the 3D toy view (PLAN.md) — loads the astra/Blender-generated shell
//! (`assets/models/toy.glb`), renders it on a desk, and wiggles the separated
//! wheel/gear parts to prove pivots. T toggles game/toy view.

use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::camera::RenderTarget;

use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::world_serialization::WorldAssetRoot;
use game_core::Gear;

use crate::Sim;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    Game,
    #[default]
    Toy,
}

#[derive(Resource)]
pub struct ViewState {
    pub mode: ViewMode,
    /// M10 dial: 0 = flat modern look, 1 = full rolling-tube illusion.
    pub toy_look: f32,
    /// M9 spring states for the moving parts (angle in radians).
    pub wheel_angle: f32,
    pub wheel_vel: f32,
    pub lever_angle: f32,
    pub lever_vel: f32,
    /// M13: ignition key twist when the game runs.
    pub key_angle: f32,
    pub key_vel: f32,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            mode: ViewMode::default(),
            // the toy reads best with the rolling-tube look ON from boot
            toy_look: 0.5,
            wheel_angle: 0.0,
            wheel_vel: 0.0,
            lever_angle: 0.0,
            lever_vel: 0.0,
            key_angle: 0.0,
            key_vel: 0.0,
        }
    }
}

/// Handle of the StandardMaterial on the toy screen (M8 pipeline, kept for
/// the M10 warm-lamp tint dial).
#[derive(Resource, Default)]
pub struct ScreenMatHandle(pub Option<Handle<StandardMaterial>>);

/// Whole-toy spring rig (M11/M12); logic lives in game-core::dynamics.
#[derive(Resource, Default)]
pub struct BodyRig(pub game_core::BodyDynamics);

/// Crash pulse captured by the fixed-step sim, consumed by the rig (M12).
#[derive(Resource, Default)]
pub struct CrashPulse {
    pub pending: bool,
    pub dir: f32,
}

#[derive(Component)]
pub struct GameCamera;

#[derive(Component)]
pub struct ToyCamera;

/// Parent of the whole 3D diorama (desk + toy scene); hidden in game mode.
#[derive(Component)]
pub struct ToyRoot;

/// Captured rest rotation of a glTF part; wiggles are applied in local space
/// on top of it every frame (no compounding).
#[derive(Component)]
pub struct RestRotation(Quat);

/// Offscreen target the 2D game renders into while in toy mode (M8).
pub const GAME_W: u32 = 960;
pub const GAME_H: u32 = 600;
// toy screen quad size (must match assets/blender/toy.py PARAMS screen_size);
// aspect-checked against the game canvas in the test below
#[cfg_attr(not(test), expect(dead_code))]
pub const SCREEN_W: f32 = 0.190;
#[cfg_attr(not(test), expect(dead_code))]
pub const SCREEN_H: f32 = 0.11875;

#[derive(Resource)]
pub struct GameImage(pub Handle<Image>);

/// Marker: the toy screen mesh already carries the game texture material.
#[derive(Component)]
pub struct ScreenPainted;

/// Handle to the toy glb; consumed by `spawn_toy` once loaded.
#[derive(Resource)]
pub struct ToyGltf(pub Handle<Gltf>);

/// Embed the generated glb into the binary (works identically native + wasm;
/// no HTTP serving / trunk hooks needed). Dev source of truth stays
/// `assets/models/toy.glb` via include_bytes.
pub fn setup(
    mut commands: Commands,
    registry: Res<EmbeddedAssetRegistry>,
    assets: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.init_resource::<ViewState>();
    let size = Extent3d {
        width: GAME_W,
        height: GAME_H,
        ..default()
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[12, 14, 18, 255],
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::all(),
    );
    image.texture_descriptor.usage |=
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
    let game_image = images.add(image);
    commands.insert_resource(GameImage(game_image));
    registry.insert_asset(
        std::path::PathBuf::from("models/toy.glb"),
        std::path::Path::new("models/toy.glb"),
        include_bytes!("../../../assets/models/toy.glb"),
    );

    commands.insert_resource(ToyGltf(assets.load("embedded://models/toy.glb")));

    // sit in front of the toy; its front faces +Z in the exported glTF
    commands.spawn((
        Camera3d::default(),
        Camera {
            is_active: false,
            ..default()
        },
        Transform::from_xyz(0.0, 0.185, 0.275).looking_at(Vec3::new(0.0, 0.128, 0.02), Vec3::Y),
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        ToyCamera,
    ));

    // key light with shadows (contrast/AO feel)
    commands.spawn((
        Transform::from_xyz(1.1, 1.8, 1.4).looking_at(Vec3::ZERO, Vec3::Y),
        DirectionalLight {
            illuminance: 9_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        bevy::light::CascadeShadowConfig::default(),
    ));
    // warm desk-lamp fill from the player's side
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.88, 0.72),
            intensity: 6.0,
            range: 2.5,
            ..default()
        },
        Transform::from_xyz(0.0, 0.55, 0.55),
    ));
    // cool rim light from behind for edge contrast on the plastic
    commands.spawn((
        PointLight {
            color: Color::srgb(0.55, 0.7, 1.0),
            intensity: 4.0,
            range: 2.0,
            ..default()
        },
        Transform::from_xyz(-0.5, 0.4, -0.6),
    ));
    commands.spawn(AmbientLight {
        color: Color::srgb(0.55, 0.58, 0.66),
        brightness: 50.0,
        ..default()
    });
}

/// Turbo lamp on the dash (M12): small emissive dot, lit in HIGH gear.
#[derive(Resource)]
pub struct TurboLamp(pub Handle<StandardMaterial>);

pub fn setup_diorama(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let lamp = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.05, 0.02),
        emissive: LinearRgba::new(0.0, 0.0, 0.0, 1.0),
        ..default()
    });
    commands.insert_resource(TurboLamp(lamp.clone()));
    commands
        .spawn((Visibility::Hidden, ToyRoot, Transform::IDENTITY))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.0032))),
                MeshMaterial3d(lamp),
                Transform::from_translation(Vec3::new(0.05, 0.086, 0.048)),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::ONE))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.16, 0.14, 0.12),
                    perceptual_roughness: 0.65,
                    ..default()
                })),
                Transform::from_scale(Vec3::splat(1.4)),
            ));
            // backdrop wall so the void behind the toy reads like a room
            parent.spawn((
                Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::ONE))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.10, 0.09, 0.085),
                    perceptual_roughness: 1.0,
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.25, -0.55),
            ));
        });
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn toggle_mode(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ViewState>,
    game_image: Res<GameImage>,
    mut game_cam: Query<(Entity, &mut Camera), (With<GameCamera>, Without<ToyCamera>)>,
    mut toy_cam: Query<&mut Camera, (With<ToyCamera>, Without<GameCamera>)>,
    mut toy_root: Query<&mut Visibility, With<ToyRoot>>,
    mut hud_texts: Query<&mut Visibility, (With<crate::view::StatusText>, Without<ToyRoot>)>,
) {
    if keys.just_pressed(KeyCode::KeyT) {
        state.mode = match state.mode {
            ViewMode::Game => ViewMode::Toy,
            ViewMode::Toy => ViewMode::Game,
        };
    }
    apply_mode(
        &mut state,
        &mut commands,
        &game_image,
        &mut game_cam,
        &mut toy_cam,
        &mut toy_root,
        &mut hud_texts,
    );
}

#[allow(clippy::type_complexity)]
fn apply_mode(
    state: &mut ResMut<ViewState>,
    commands: &mut Commands,
    game_image: &Res<GameImage>,
    game_cam: &mut Query<(Entity, &mut Camera), (With<GameCamera>, Without<ToyCamera>)>,
    toy_cam: &mut Query<&mut Camera, (With<ToyCamera>, Without<GameCamera>)>,
    toy_root: &mut Query<&mut Visibility, With<ToyRoot>>,
    hud_texts: &mut Query<&mut Visibility, (With<crate::view::StatusText>, Without<ToyRoot>)>,
) {
    let toy = state.mode == ViewMode::Toy;
    let _ = toy;
    if let Ok((entity, mut cam)) = game_cam.single_mut() {
        cam.is_active = true; // always renders: window in game mode, texture in toy mode
        let target = if toy {
            RenderTarget::Image(bevy::camera::ImageRenderTarget {
                handle: game_image.0.clone(),
                scale_factor: 1.0,
            })
        } else {
            RenderTarget::Window(bevy::window::WindowRef::default())
        };
        commands.entity(entity).insert(target);
    }
    if let Ok(mut cam) = toy_cam.single_mut() {
        cam.is_active = toy;
    }
    if let Ok(mut vis) = toy_root.single_mut() {
        *vis = if toy {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // the lap/score/gear HUD is drawn INTO the game texture (Text2d), so
    // it shows on the toy screen itself; the window-space status hint is
    // always visible
    for mut vis in hud_texts.iter_mut() {
        *vis = Visibility::Visible;
    }
}

/// M9: wheel + gear lever bound to live input. The wheel tracks
/// `InputState.steer` (full lock ≈ 120°) with mechanical lag; the lever
/// snaps LOW↔HIGH with a springy overshoot.
pub fn animate_parts(
    mut state: ResMut<ViewState>,
    sim: Res<Sim>,
    frame: Res<crate::FrameInput>,
    time: Res<Time>,
    mut commands: Commands,
    mut parts: Query<(Entity, &Name, &mut Transform, Option<&RestRotation>), Without<ToyRoot>>,
) {
    if state.mode != ViewMode::Toy {
        return;
    }
    let dt = time.delta_secs().min(0.05);
    // invert so turning LEFT rotates the wheel counter-clockwise as the
    // player sees it (positive local-Z rotation is CCW from the player side)
    let wheel_target = -frame.state.steer * 1.35; // ~77 deg full lock, gentler
    let lever_target = match sim.0.gear() {
        Gear::Low => -0.30,
        Gear::High => 0.30,
    };
    state.wheel_angle = game_core::dynamics::spring_step(
        state.wheel_angle,
        &mut state.wheel_vel,
        wheel_target,
        90.0,
        13.0,
        dt,
    );
    state.lever_angle = game_core::dynamics::spring_step(
        state.lever_angle,
        &mut state.lever_vel,
        lever_target,
        140.0,
        11.0,
        dt,
    );

    for (entity, name, mut tf, rest) in parts.iter_mut() {
        let Some(rest) = rest else {
            commands.entity(entity).insert(RestRotation(tf.rotation));
            continue;
        };
        let base = rest.0;
        match name.as_str() {
            n if n.starts_with("SteeringWheel") => {
                tf.rotation = base * Quat::from_rotation_z(state.wheel_angle);
            }
            n if n.starts_with("GearLever") => {
                tf.rotation = base * Quat::from_rotation_x(state.lever_angle);
            }
            n if n.starts_with("IgnitionKey") => {
                let target = match sim.0.phase {
                    game_core::Phase::Ready => 0.0,
                    _ => 0.85,
                };
                state.key_angle = game_core::dynamics::spring_step(
                    state.key_angle,
                    &mut state.key_vel,
                    target,
                    140.0,
                    11.0,
                    dt,
                );
                tf.rotation = base * Quat::from_rotation_x(state.key_angle);
            }
            _ => {}
        }
    }
}

/// Spawns the toy scene under `ToyRoot` once the glb finishes loading.
/// (Direct labeled loads like `toy.glb#Scene0` trip the asset meta-lens on
/// wasm — loading the Gltf asset and cloning its scene handle works.)
pub fn spawn_toy(
    mut commands: Commands,
    handle: Option<Res<ToyGltf>>,
    gltfs: Res<Assets<Gltf>>,
    root: Query<Entity, With<ToyRoot>>,
) {
    let Some(handle) = handle else { return };
    let Some(gltf) = gltfs.get(&handle.0) else {
        return;
    };
    let Some(scene) = gltf.scenes.first() else {
        return;
    };
    if let Ok(root) = root.single() {
        commands.spawn((
            WorldAssetRoot(scene.clone()),
            Transform::IDENTITY,
            ChildOf(root),
        ));
    }
    commands.remove_resource::<ToyGltf>();
}

/// M8/M10: put the live game texture on the toy's screen quad via the
/// rolling-tube material (toy_look dial in `update_screen_mat`).
#[allow(clippy::type_complexity)]
pub fn paint_screen(
    mut commands: Commands,
    game_image: Res<GameImage>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mat_handle: ResMut<ScreenMatHandle>,
    screens: Query<(Entity, &Name), (With<Mesh3d>, Without<ScreenPainted>)>,
) {
    for (entity, name) in screens.iter() {
        if name.as_str().starts_with("Screen") && name.as_str() != "ScreenFrame" {
            let mat = materials.add(StandardMaterial {
                base_color_texture: Some(game_image.0.clone()),
                // the game image itself is the light source: backlit drum
                emissive_texture: Some(game_image.0.clone()),
                emissive: LinearRgba::new(0.9, 0.8, 0.62, 1.0),
                // NOT unlit: unlit skips the emissive pass, which is the
                // whole backlight — the screen must be a lit emitter
                ..default()
            });
            mat_handle.0 = Some(mat.clone());
            commands
                .entity(entity)
                .insert(MeshMaterial3d(mat))
                .insert(ScreenPainted);
            return; // one screen per toy
        }
    }
}

/// M10: drive the screen shader uniforms; `L` cycles the toy_look dial
/// (0.0 -> 0.5 -> 1.0 -> 0.0) for manual comparison.
pub fn update_screen_mat(
    keys: Res<ButtonInput<KeyCode>>,
    mut view: ResMut<ViewState>,
    mat_handle: Res<ScreenMatHandle>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if keys.just_pressed(KeyCode::KeyL) {
        view.toy_look = match view.toy_look {
            x if x < 0.25 => 0.5,
            x if x < 0.75 => 1.0,
            _ => 0.0,
        };
    }
    let Some(handle) = mat_handle.0.as_ref() else {
        return;
    };
    if let Some(mut mat) = materials.get_mut(handle) {
        // rolling-tube approximation on the proven StandardMaterial path:
        // warm lamp tint + emissive backlit glow scale with the dial (band
        // overlays live in the 2D drum view; see view.rs)
        let look = view.toy_look;
        mat.base_color = Color::srgb(1.0, 1.0, 1.0).mix(&Color::srgb(0.92, 0.85, 0.70), look * 0.6);
        mat.emissive = LinearRgba::new(look * 0.18, look * 0.16, look * 0.12, 1.0);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn apply_body_dynamics(
    mut rig: ResMut<BodyRig>,
    mut pulse: ResMut<CrashPulse>,
    pads: Query<Entity, With<Gamepad>>,
    mut rumble: bevy::ecs::message::MessageWriter<GamepadRumbleRequest>,
    sim: Res<Sim>,
    frame: Res<crate::FrameInput>,
    tuning: Res<crate::TuningRes>,
    time: Res<Time>,
    mut root: Query<&mut Transform, With<ToyRoot>>,
) {
    if pulse.pending {
        rig.0.crash_impulse(pulse.dir, &tuning.0);
        pulse.pending = false;
        // M12: shake the player's hands too (all connected pads)
        for pad in pads.iter() {
            rumble.write(GamepadRumbleRequest::Add {
                duration: std::time::Duration::from_millis(450),
                intensity: GamepadRumbleIntensity::STRONG_MAX,
                gamepad: pad,
            });
        }
    }
    let speed_norm = (sim.0.speed / tuning.0.scroll_speed_high).clamp(0.0, 1.0);
    rig.0.step(
        frame.state.steer,
        speed_norm,
        sim.0.speed,
        time.elapsed_secs(),
        &tuning.0,
    );
    let (jx, jz, jroll) = rig.0.shake_offset(time.elapsed_secs(), &tuning.0);
    if let Ok(mut tf) = root.single_mut() {
        tf.rotation =
            Quat::from_rotation_z(rig.0.roll + jroll) * Quat::from_rotation_x(rig.0.pitch);
        // Nitro throws the physical toy away from the player, then lets it
        // ease forward as the five-second charge drains.
        let nitro_kick = if sim.0.nitro_active() {
            -0.014 * (sim.0.nitro_t / tuning.0.nitro_duration_secs).sqrt()
        } else {
            0.0
        };
        tf.translation = Vec3::new(rig.0.shift + jx, 0.0, rig.0.heave + jz + nitro_kick);
    }
}

/// M12: turbo lamp follows the gear (nods to the Tomy's HIGH-gear light).
pub fn turbo_lamp(
    sim: Res<Sim>,
    lamp: Res<TurboLamp>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if let Some(mut mat) = materials.get_mut(&lamp.0) {
        let on = matches!(sim.0.gear(), Gear::High) || sim.0.nitro_active();
        mat.emissive = if on {
            if sim.0.nitro_active() {
                LinearRgba::new(2.0, 0.55, 0.10, 1.0)
            } else {
                LinearRgba::new(1.2, 0.35, 0.08, 1.0)
            }
        } else {
            LinearRgba::new(0.0, 0.0, 0.0, 1.0)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_converges_with_slight_overshoot() {
        let mut angle: f32 = 0.0;
        let mut vel: f32 = 0.0;
        for _ in 0..240 {
            angle = game_core::dynamics::spring_step(angle, &mut vel, 1.0, 90.0, 13.0, 1.0 / 60.0);
        }
        assert!(
            (angle - 1.0).abs() < 1e-3,
            "should settle at target: {angle}"
        );
    }

    #[test]
    fn spring_is_underdamped_but_stable() {
        let mut angle: f32 = 0.0;
        let mut vel: f32 = 0.0;
        let mut peak: f32 = 0.0;
        for _ in 0..120 {
            angle = game_core::dynamics::spring_step(angle, &mut vel, 1.0, 90.0, 13.0, 1.0 / 60.0);
            peak = peak.max(angle);
        }
        assert!(peak > 1.02, "wheel spring should visibly overshoot: {peak}");
        assert!(peak < 1.6, "overshoot must stay bounded: {peak}");
    }

    /// M8 exit criterion: the game canvas and the toy screen quad must share
    /// aspect so the render-to-texture mapping needs no letterboxing.
    #[test]
    fn game_and_screen_aspects_match() {
        let game = GAME_W as f32 / GAME_H as f32;
        let screen = SCREEN_W / SCREEN_H;
        assert!((game - screen).abs() < 1e-3, "{game} vs {screen}");
    }
}
