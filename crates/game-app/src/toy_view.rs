//! M7: the 3D toy view (PLAN.md) — loads the astra/Blender-generated shell
//! (`assets/models/toy.glb`), renders it on a desk, and wiggles the separated
//! wheel/gear parts to prove pivots. T toggles game/toy view.

use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::world_serialization::WorldAssetRoot;
use game_core::Gear;

use crate::Sim;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Game,
    Toy,
}

#[derive(Resource, Default)]
pub struct ViewState {
    pub mode: ViewMode,
    /// M9 spring states for the moving parts (angle in radians).
    pub wheel_angle: f32,
    pub wheel_vel: f32,
    pub lever_angle: f32,
    pub lever_vel: f32,
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
pub const SCREEN_W: f32 = 0.16;
#[cfg_attr(not(test), expect(dead_code))]
pub const SCREEN_H: f32 = 0.10;

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
    commands.insert_resource(GameImage(images.add(image)));
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
        Transform::from_xyz(0.0, 0.31, 0.42).looking_at(Vec3::new(0.0, 0.105, 0.0), Vec3::Y),
        ToyCamera,
    ));

    commands.spawn((
        Transform::from_xyz(1.5, 2.2, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        DirectionalLight {
            illuminance: 14_000.0,
            ..default()
        },
    ));
    commands.spawn(AmbientLight {
        color: Color::srgb(0.95, 0.93, 0.9),
        brightness: 130.0,
        ..default()
    });
}

pub fn setup_diorama(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn((Visibility::Hidden, ToyRoot, Transform::IDENTITY))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::ONE))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.24, 0.22, 0.20),
                    perceptual_roughness: 0.9,
                    ..default()
                })),
                Transform::from_scale(Vec3::splat(1.2)),
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
    mut hud_texts: Query<
        &mut Visibility,
        (
            Or<(
                With<crate::view::ScoreText>,
                With<crate::view::GearText>,
                With<crate::view::StatusText>,
            )>,
            Without<ToyRoot>,
        ),
    >,
) {
    if keys.just_pressed(KeyCode::KeyT) {
        state.mode = match state.mode {
            ViewMode::Game => ViewMode::Toy,
            ViewMode::Toy => ViewMode::Game,
        };
    }
    let toy = state.mode == ViewMode::Toy;
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
    // window-space HUD text only in game mode; in toy mode the game texture
    // on the toy screen carries the world sprites (road, car, fuel bar)
    for mut vis in hud_texts.iter_mut() {
        *vis = if toy {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
}

/// Springy under-damped second-order step (shared by wheel & lever).
fn spring_toward(current: f32, vel: &mut f32, target: f32, k: f32, c: f32, dt: f32) -> f32 {
    *vel += (k * (target - current) - c * *vel) * dt;
    current + *vel * dt
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
    let wheel_target = frame.state.steer * 2.1; // ~120 deg full lock
    let lever_target = match sim.0.gear() {
        Gear::Low => -0.30,
        Gear::High => 0.30,
    };
    state.wheel_angle = spring_toward(
        state.wheel_angle,
        &mut state.wheel_vel,
        wheel_target,
        90.0,
        13.0,
        dt,
    );
    state.lever_angle = spring_toward(
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

/// M8: put the live game texture on the toy's screen quad (unlit so it reads
/// like a backlit CRT regardless of scene lighting).
#[allow(clippy::type_complexity)]
pub fn paint_screen(
    mut commands: Commands,
    game_image: Res<GameImage>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    screens: Query<(Entity, &Name), (With<Mesh3d>, Without<ScreenPainted>)>,
) {
    for (entity, name) in screens.iter() {
        if name.as_str().starts_with("Screen") && name.as_str() != "ScreenFrame" {
            let mat = materials.add(StandardMaterial {
                base_color_texture: Some(game_image.0.clone()),
                unlit: true,
                ..default()
            });
            commands
                .entity(entity)
                .insert(MeshMaterial3d(mat))
                .insert(ScreenPainted);
            return; // one screen per toy
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_converges_with_slight_overshoot() {
        let mut angle = 0.0;
        let mut vel = 0.0;
        for _ in 0..240 {
            angle = spring_toward(angle, &mut vel, 1.0, 90.0, 13.0, 1.0 / 60.0);
        }
        assert!(
            (angle - 1.0).abs() < 1e-3,
            "should settle at target: {angle}"
        );
    }

    #[test]
    fn spring_is_underdamped_but_stable() {
        let mut angle = 0.0;
        let mut vel = 0.0;
        let mut peak: f32 = 0.0;
        for _ in 0..120 {
            angle = spring_toward(angle, &mut vel, 1.0, 90.0, 13.0, 1.0 / 60.0);
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
