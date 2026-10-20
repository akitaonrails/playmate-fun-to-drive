//! M7: the 3D toy view (PLAN.md) — loads the astra/Blender-generated shell
//! (`assets/models/toy.glb`), renders it on a desk, and wiggles the separated
//! wheel/gear parts to prove pivots. T toggles game/toy view.

use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::camera::{Exposure, RenderTarget};

use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::post_process::bloom::{Bloom, BloomPrefilter};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::shader::ShaderRef;
use bevy::world_serialization::WorldAssetRoot;
use game_core::Gear;

use crate::Sim;

pub const CRT_STORAGE_KEY: &str = "fun-to-drive.crt.v1";
pub const LARGE_SCREEN_STORAGE_KEY: &str = "fun-to-drive.large-screen.v1";
const CRT_SHADER_PATH: &str = "shaders/crt_screen.wesl";
const ROADSTER_FRONT_LAMP_X: [f32; 2] = [-0.008, 0.008];
const INTRO_ORBIT_SECS: f32 = 5.2;
const INTRO_CRT_POWER_SECS: f32 = 0.9;
const INTRO_TOTAL_SECS: f32 = INTRO_ORBIT_SECS + INTRO_CRT_POWER_SECS;
const INTRO_CAMERA_START_ANGLE: f32 = -1.30;
const INTRO_CAMERA_START_RADIUS: f32 = 0.78;
const SCREEN_GLOW_INTENSITY: f32 = 88.0;

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
    pub lever_target: f32,
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
            lever_angle: 0.30,
            lever_vel: 0.0,
            lever_target: 0.30,
            key_angle: 0.0,
            key_vel: 0.0,
        }
    }
}

/// Single-pass CRT material. The deliberately inexpensive Lottes-style pass
/// combines tube warp, scanlines, an RGB grille, vignette, and a rolling beam.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CrtScreenMaterial {
    #[texture(0)]
    #[sampler(1)]
    source: Handle<Image>,
    /// x=enabled, y=time, z=look strength, w=backlight exposure
    #[uniform(2)]
    params: Vec4,
}

impl Material for CrtScreenMaterial {
    fn fragment_shader() -> ShaderRef {
        CRT_SHADER_PATH.into()
    }
}

/// Handle of the live material on the toy screen.
#[derive(Resource, Default)]
pub struct ScreenMatHandle(pub Option<Handle<CrtScreenMaterial>>);

#[derive(Resource, Default)]
pub struct ScreenPaintState {
    screen: bool,
    bezel: bool,
}

/// Whole-toy spring rig (M11/M12); logic lives in game-core::dynamics.
#[derive(Resource, Default)]
pub struct BodyRig(pub game_core::BodyDynamics);

/// Eased acceleration pose layered over the normal body dynamics. Positive Z
/// moves the toy toward the camera; negative pitch leans its face back.
#[derive(Resource, Default)]
pub struct NitroPose {
    pub z: f32,
    pub pitch: f32,
    z_velocity: f32,
    pitch_velocity: f32,
}

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

/// One-shot presentation sequence for a fresh page load. It deliberately
/// lives outside the deterministic game state and is not reset with a run.
#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct StartupIntro {
    elapsed: f32,
    enabled: bool,
}

impl StartupIntro {
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            elapsed: if enabled { 0.0 } else { INTRO_TOTAL_SECS },
            enabled,
        }
    }

    pub(crate) fn complete(self) -> bool {
        !self.enabled || self.elapsed >= INTRO_TOTAL_SECS
    }

    fn camera_progress(self) -> f32 {
        smoothstep01(self.elapsed / INTRO_ORBIT_SECS)
    }

    pub(crate) fn screen_power(self) -> f32 {
        if !self.enabled {
            return 1.0;
        }
        smoothstep01((self.elapsed - INTRO_ORBIT_SECS) / INTRO_CRT_POWER_SECS)
    }
}

/// Parent of the whole 3D diorama (desk + toy scene); hidden in game mode.
#[derive(Component)]
pub struct ToyRoot;

#[derive(Component)]
pub struct ScreenGlowLight;

/// Static dorm-room scene root. Steering/crash pose is deliberately applied
/// only to `ToyRoot`, which is a child of this unmoving environment.
#[derive(Component)]
pub struct RoomRoot;

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

// Keep the eye on the authored screen-centered sightline while pushing toward
// the toy. A single ratio makes the crop easy to retune without introducing
// vertical drift: 0.86 retains a narrow room border, the full bezel, and the
// dash Roadster while letting more of the wheel and lower controls fall below
// the narrowest supported 16:10 viewport.
const TOY_CAMERA_BASE_POSITION: Vec3 = Vec3::new(0.0, 0.183, 0.265);
const TOY_CAMERA_TARGET: Vec3 = Vec3::new(0.0, 0.128, 0.02);
const TOY_CAMERA_PUSH_IN_RATIO: f32 = 0.86;

fn toy_camera_position() -> Vec3 {
    TOY_CAMERA_TARGET + (TOY_CAMERA_BASE_POSITION - TOY_CAMERA_TARGET) * TOY_CAMERA_PUSH_IN_RATIO
}

fn smoothstep01(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

fn intro_camera_transform(progress: f32) -> Transform {
    let progress = progress.clamp(0.0, 1.0);
    let final_position = toy_camera_position();
    let radius = INTRO_CAMERA_START_RADIUS
        + (final_position.z - TOY_CAMERA_TARGET.z - INTRO_CAMERA_START_RADIUS) * progress;
    let angle = INTRO_CAMERA_START_ANGLE * (1.0 - progress);
    let height = 0.34 + (final_position.y - 0.34) * progress;
    let position = Vec3::new(
        TOY_CAMERA_TARGET.x + radius * angle.sin(),
        height,
        TOY_CAMERA_TARGET.z + radius * angle.cos(),
    );
    Transform::from_translation(position).looking_at(TOY_CAMERA_TARGET, Vec3::Y)
}

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
    quality: Res<crate::RenderQuality>,
    intro: Res<StartupIntro>,
) {
    commands.init_resource::<ViewState>();
    let size = Extent3d {
        width: quality.game_width,
        height: quality.game_height,
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
    let mut toy_camera = commands.spawn((
        Camera3d::default(),
        Camera {
            is_active: false,
            ..default()
        },
        intro_camera_transform(intro.camera_progress()),
        // ACES is shader-only, preserving HDR-to-display contrast without the
        // compressed LUT assets (and zstd decoder) required by TonyMcMapface.
        bevy::core_pipeline::tonemapping::Tonemapping::AcesFitted,
        // One stop darker than Bevy's indoor preset: bright enough for the
        // toy-scale practical lights to register, while retaining the dorm-at-
        // night mood and keeping the emissive screen as the brightest surface.
        Exposure { ev100: 8.0 },
        ToyCamera,
    ));
    if quality.bloom {
        toy_camera.insert(Bloom {
            intensity: 0.035,
            // Keep white plastic below bloom: only the HDR screen emitter
            // should produce the soft old-backlight bleed.
            prefilter: BloomPrefilter {
                threshold: 1.05,
                threshold_softness: 0.12,
            },
            ..Bloom::OLD_SCHOOL
        });
    }

    // Dim blue window/moon fill establishes a dorm room at night without
    // flattening the toy's deliberately simple plastic forms.
    commands.spawn((
        Transform::from_xyz(1.1, 1.8, 1.4).looking_at(Vec3::ZERO, Vec3::Y),
        DirectionalLight {
            color: Color::srgb(0.42, 0.52, 0.78),
            illuminance: 300.0,
            shadow_maps_enabled: quality.bloom,
            ..default()
        },
        bevy::light::CascadeShadowConfig::default(),
    ));
    // A very dim overhead lamp grazes the toy from the upper-right. The
    // off-axis pool breaks the bilateral symmetry and reveals the red shell,
    // while its short range leaves the rear wall in nighttime darkness.
    commands.spawn((
        SpotLight {
            color: Color::srgb(1.0, 0.80, 0.58),
            intensity: 1_100.0,
            // Stops before the rear wall, so it reads as a pool on the toy
            // and desk instead of making the entire room uniformly bright.
            range: 0.72,
            radius: 0.08,
            inner_angle: 0.30,
            outer_angle: 0.62,
            shadow_maps_enabled: quality.bloom,
            ..default()
        },
        Transform::from_xyz(0.30, 0.48, 0.24).looking_at(Vec3::new(-0.035, 0.065, 0.0), Vec3::Y),
    ));
    // Low cool rim separates the dark shell from the wall.
    commands.spawn((
        PointLight {
            color: Color::srgb(0.35, 0.53, 1.0),
            intensity: 40.0,
            range: 0.8,
            ..default()
        },
        Transform::from_xyz(-0.35, 0.32, -0.42),
    ));
    // A small amber practical-light spill paints one side of the rear wall.
    // It is intentionally behind the toy so the room gains depth and a lived-
    // in dorm mood without competing with the screen or flattening the shell.
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.58, 0.34),
            intensity: 160.0,
            range: 0.85,
            radius: 0.12,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.32, 0.42, -0.28),
    ));
    commands.spawn(AmbientLight {
        color: Color::srgb(0.24, 0.29, 0.48),
        brightness: 8.0,
        ..default()
    });
}

/// Turbo lamp on the dash (M12): small emissive dot, lit in HIGH gear.
#[derive(Resource)]
pub struct TurboLamp(pub Handle<StandardMaterial>);

#[derive(Resource)]
pub struct RoadsterHeadlights(pub Handle<StandardMaterial>);

#[derive(Component)]
pub(crate) struct GearKnobShine;

#[derive(Component)]
pub(crate) struct SteeringWheelShine;

#[derive(Component)]
pub struct SportsCarOrnament;

#[derive(Component)]
pub struct RoomShootingStar {
    event: u8,
    segment: u8,
}

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
    let body = materials.add(StandardMaterial {
        base_color: Color::srgb(0.82, 0.85, 0.86),
        metallic: 0.32,
        perceptual_roughness: 0.34,
        ..default()
    });
    let body_shadow = materials.add(StandardMaterial {
        base_color: Color::srgb(0.40, 0.44, 0.47),
        metallic: 0.26,
        perceptual_roughness: 0.44,
        ..default()
    });
    let canopy = materials.add(StandardMaterial {
        base_color: Color::srgb(0.06, 0.10, 0.17),
        metallic: 0.18,
        perceptual_roughness: 0.28,
        ..default()
    });
    let tire = materials.add(StandardMaterial {
        base_color: Color::srgb(0.025, 0.032, 0.045),
        perceptual_roughness: 0.9,
        ..default()
    });
    let headlight = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.88, 0.48),
        emissive: LinearRgba::new(0.9, 0.55, 0.12, 1.0),
        perceptual_roughness: 0.3,
        ..default()
    });
    commands.insert_resource(RoadsterHeadlights(headlight.clone()));
    let window_glass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.018, 0.032, 0.070),
        emissive: LinearRgba::new(0.004, 0.009, 0.026, 1.0),
        perceptual_roughness: 0.22,
        ..default()
    });
    let room_trim = materials.add(StandardMaterial {
        base_color: Color::srgb(0.095, 0.085, 0.105),
        perceptual_roughness: 0.82,
        ..default()
    });
    let curtain = materials.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.025, 0.052),
        perceptual_roughness: 0.92,
        ..default()
    });
    let skyline = materials.add(StandardMaterial {
        base_color: Color::srgb(0.012, 0.018, 0.036),
        perceptual_roughness: 1.0,
        ..default()
    });
    let warm_glow = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.58, 0.22),
        emissive: LinearRgba::new(0.48, 0.19, 0.035, 1.0),
        perceptual_roughness: 0.48,
        ..default()
    });
    let shooting_star = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.88, 1.0),
        emissive: LinearRgba::new(2.8, 3.6, 5.4, 1.0),
        perceptual_roughness: 0.18,
        ..default()
    });
    let shelf_wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.075, 0.038),
        perceptual_roughness: 0.78,
        ..default()
    });
    let poster_navy = materials.add(StandardMaterial {
        base_color: Color::srgb(0.055, 0.075, 0.13),
        perceptual_roughness: 0.9,
        ..default()
    });
    let poster_coral = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.16, 0.19),
        perceptual_roughness: 0.85,
        ..default()
    });
    let poster_cream = materials.add(StandardMaterial {
        base_color: Color::srgb(0.80, 0.67, 0.46),
        perceptual_roughness: 0.9,
        ..default()
    });
    let book_materials = [
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.28, 0.35),
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.56, 0.16, 0.12),
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.46, 0.36, 0.12),
            ..default()
        }),
    ];
    commands
        .spawn((Visibility::Hidden, RoomRoot, Transform::IDENTITY))
        .with_children(|parent| {
            parent
                .spawn((Transform::IDENTITY, Visibility::Visible, ToyRoot))
                .with_children(|toy| {
                    // The tube is a real local light source, not just a bright
                    // pixel surface. It moves with the toy, unlike the room.
                    toy.spawn((
                        PointLight {
                            color: Color::srgb(0.45, 0.72, 1.0),
                            intensity: 0.0,
                            range: 0.38,
                            radius: 0.035,
                            shadow_maps_enabled: false,
                            ..default()
                        },
                        Transform::from_xyz(0.0, 0.177, 0.074),
                        ScreenGlowLight,
                    ));
                    toy.spawn((
                        Transform::from_xyz(0.048, 0.0815, 0.050)
                            // The camera sits on +Z while the model nose is -Z.
                            // Keep it 25 degrees off a head-on view for a front
                            // three-quarter hero angle.
                            .with_rotation(Quat::from_rotation_y(155.0_f32.to_radians())),
                        Visibility::Visible,
                        SportsCarOrnament,
                    ))
                    .with_children(|car| {
                        // Black undertray below a long, extremely low silver wedge.
                        car.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.031, 0.003, 0.044))),
                            MeshMaterial3d(tire.clone()),
                            Transform::from_xyz(0.0, 0.003, 0.0),
                        ));
                        car.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.029, 0.005, 0.040))),
                            MeshMaterial3d(body.clone()),
                            Transform::from_xyz(0.0, 0.006, -0.001),
                        ));
                        car.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.025, 0.004, 0.019))),
                            MeshMaterial3d(body_shadow.clone()),
                            Transform::from_xyz(0.0, 0.009, -0.010)
                                .with_rotation(Quat::from_rotation_x(-0.10)),
                        ));
                        car.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.019, 0.005, 0.019))),
                            MeshMaterial3d(canopy.clone()),
                            Transform::from_xyz(0.0, 0.011, -0.002)
                                .with_rotation(Quat::from_rotation_x(-0.06)),
                        ));
                        // Huge wheels with visible silver hubs.
                        for x in [-0.014, 0.014] {
                            for z in [-0.012, 0.012] {
                                car.spawn((
                                    Mesh3d(meshes.add(Cylinder::new(0.0062, 0.004))),
                                    MeshMaterial3d(tire.clone()),
                                    Transform::from_xyz(x, 0.006, z).with_rotation(
                                        Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                                    ),
                                ));
                                car.spawn((
                                    Mesh3d(meshes.add(Cylinder::new(0.0032, 0.0043))),
                                    MeshMaterial3d(body_shadow.clone()),
                                    Transform::from_xyz(x, 0.006, z).with_rotation(
                                        Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                                    ),
                                ));
                            }
                        }
                        // Raised rear wing, its two supports, and the uninterrupted
                        // red blade make the model identifiable even at 4 cm long.
                        car.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.035, 0.0018, 0.004))),
                            MeshMaterial3d(tire.clone()),
                            Transform::from_xyz(0.0, 0.016, 0.020),
                        ));
                        for x in [-0.011, 0.011] {
                            car.spawn((
                                Mesh3d(meshes.add(Cuboid::new(0.0018, 0.007, 0.0018))),
                                MeshMaterial3d(tire.clone()),
                                Transform::from_xyz(x, 0.0125, 0.019),
                            ));
                        }
                        car.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.027, 0.0016, 0.0014))),
                            MeshMaterial3d(lamp.clone()),
                            Transform::from_xyz(0.0, 0.009, 0.0206),
                        ));
                        // Four rear outlets under the light blade.
                        for x in [-0.009, -0.003, 0.003, 0.009] {
                            car.spawn((
                                Mesh3d(meshes.add(Cylinder::new(0.0017, 0.0012))),
                                MeshMaterial3d(tire.clone()),
                                Transform::from_xyz(x, 0.006, 0.0215).with_rotation(
                                    Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                                ),
                            ));
                        }
                        // Thin triangular-looking front lamps are approximated by
                        // angled light slivers on the low nose.
                        for x in ROADSTER_FRONT_LAMP_X {
                            car.spawn((
                                Mesh3d(meshes.add(Cuboid::new(0.007, 0.0013, 0.0012))),
                                MeshMaterial3d(headlight.clone()),
                                Transform::from_xyz(x, 0.008, -0.0212)
                                    .with_rotation(Quat::from_rotation_y(-x.signum() * 0.18)),
                            ));
                        }
                    });
                });
            parent.spawn((
                Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::ONE))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.110, 0.080, 0.065),
                    perceptual_roughness: 0.72,
                    ..default()
                })),
                Transform::from_scale(Vec3::splat(1.4)),
            ));
            // backdrop wall so the void behind the toy reads like a room
            parent.spawn((
                Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::ONE))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.075, 0.085, 0.135),
                    perceptual_roughness: 1.0,
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.25, -0.55),
            ));

            // Moonlit dorm window: simple layered cuboids keep the physical
            // room readable without introducing textures or asset loads.
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.35, 0.225, 0.008))),
                MeshMaterial3d(room_trim.clone()),
                Transform::from_xyz(-0.305, 0.335, -0.535),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.315, 0.192, 0.008))),
                MeshMaterial3d(window_glass.clone()),
                Transform::from_xyz(-0.305, 0.335, -0.526),
            ));
            for (size, position) in [
                (
                    Vec3::new(0.012, 0.205, 0.010),
                    Vec3::new(-0.305, 0.335, -0.516),
                ),
                (
                    Vec3::new(0.328, 0.010, 0.010),
                    Vec3::new(-0.305, 0.335, -0.516),
                ),
            ] {
                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                    MeshMaterial3d(room_trim.clone()),
                    Transform::from_translation(position),
                ));
            }
            for index in 0..11 {
                let width = 0.018 + (index * 7 % 12) as f32 * 0.001;
                let height = 0.025 + (index * 11 % 48) as f32 * 0.001;
                let x = -0.445 + index as f32 * 0.028;
                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(width, height, 0.006))),
                    MeshMaterial3d(skyline.clone()),
                    Transform::from_xyz(x, 0.255 + height * 0.5, -0.510),
                ));
                if index % 2 == 0 {
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.004, 0.004, 0.004))),
                        MeshMaterial3d(warm_glow.clone()),
                        Transform::from_xyz(x, 0.272 + height * 0.5, -0.505),
                    ));
                }
            }
            // Sparse offset schedules animate across the glass. Segmented
            // streaks are cheaper than particles and remain clipped by the
            // physical window trim and curtains.
            for event in 0..2 {
                for segment in 0..5 {
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.022, 0.0022, 0.002))),
                        MeshMaterial3d(shooting_star.clone()),
                        Transform::from_xyz(-0.44, 0.41, -0.508)
                            .with_rotation(Quat::from_rotation_z(-0.43)),
                        Visibility::Hidden,
                        RoomShootingStar { event, segment },
                    ));
                }
            }
            for x in [-0.492, -0.118] {
                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.052, 0.25, 0.018))),
                    MeshMaterial3d(curtain.clone()),
                    Transform::from_xyz(x, 0.335, -0.505),
                ));
            }

            // A small asymmetrical shelf and abstract racing print explain the
            // warm practical light while keeping the screen-center wall quiet.
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.285, 0.014, 0.055))),
                MeshMaterial3d(shelf_wood.clone()),
                Transform::from_xyz(0.315, 0.365, -0.505),
            ));
            for (index, x) in [0.235, 0.280, 0.326].into_iter().enumerate() {
                let height = 0.070 + index as f32 * 0.012;
                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.034, height, 0.035))),
                    MeshMaterial3d(book_materials[index].clone()),
                    Transform::from_xyz(x, 0.407 + height * 0.5, -0.500),
                ));
            }
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.19, 0.13, 0.008))),
                MeshMaterial3d(room_trim.clone()),
                Transform::from_xyz(0.325, 0.205, -0.526),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.170, 0.110, 0.007))),
                MeshMaterial3d(poster_navy.clone()),
                Transform::from_xyz(0.325, 0.205, -0.516),
            ));
            for (material, size, position) in [
                (
                    poster_coral.clone(),
                    Vec3::new(0.13, 0.018, 0.005),
                    Vec3::new(0.315, 0.225, -0.510),
                ),
                (
                    poster_cream.clone(),
                    Vec3::new(0.085, 0.014, 0.005),
                    Vec3::new(0.350, 0.190, -0.510),
                ),
            ] {
                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                    MeshMaterial3d(material),
                    Transform::from_translation(position),
                ));
            }
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.78, 0.002, 0.003))),
                MeshMaterial3d(room_trim.clone()),
                Transform::from_xyz(0.0, 0.490, -0.520),
            ));
            for index in 0..9 {
                parent.spawn((
                    Mesh3d(meshes.add(Sphere::new(0.006).mesh().ico(1).unwrap())),
                    MeshMaterial3d(warm_glow.clone()),
                    Transform::from_xyz(-0.36 + index as f32 * 0.09, 0.485, -0.510),
                ));
            }
        });
}

fn shooting_star_progress(elapsed: f32, event: u8, segment: u8) -> Option<f32> {
    let (period, offset) = if event == 0 {
        (47.0, 17.0)
    } else {
        (71.0, 39.0)
    };
    let local = (elapsed - offset).rem_euclid(period) - segment as f32 * 0.035;
    (0.0..0.82).contains(&local).then_some(local / 0.82)
}

pub fn animate_room_shooting_stars(
    time: Res<Time>,
    mut stars: Query<(&RoomShootingStar, &mut Transform, &mut Visibility)>,
) {
    for (star, mut transform, mut visibility) in &mut stars {
        let Some(progress) = shooting_star_progress(time.elapsed_secs(), star.event, star.segment)
        else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let lane = star.event as f32 * 0.025;
        transform.translation.x = -0.438 + progress * 0.258;
        transform.translation.y = 0.407 - progress * 0.112 - lane;
        *visibility = Visibility::Visible;
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn toggle_mode(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ViewState>,
    game_image: Res<GameImage>,
    mut game_cam: Query<(Entity, &mut Camera), (With<GameCamera>, Without<ToyCamera>)>,
    mut toy_cam: Query<&mut Camera, (With<ToyCamera>, Without<GameCamera>)>,
    mut room_root: Query<&mut Visibility, With<RoomRoot>>,
    mut hud_texts: Query<&mut Visibility, (With<crate::view::StatusText>, Without<RoomRoot>)>,
    mut applied_mode: Local<Option<ViewMode>>,
) {
    if keys.just_pressed(KeyCode::KeyT) {
        state.mode = match state.mode {
            ViewMode::Game => ViewMode::Toy,
            ViewMode::Toy => ViewMode::Game,
        };
    }
    if *applied_mode == Some(state.mode) {
        return;
    }
    *applied_mode = Some(state.mode);
    apply_mode(
        &mut state,
        &mut commands,
        &game_image,
        &mut game_cam,
        &mut toy_cam,
        &mut room_root,
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
    room_root: &mut Query<&mut Visibility, With<RoomRoot>>,
    hud_texts: &mut Query<&mut Visibility, (With<crate::view::StatusText>, Without<RoomRoot>)>,
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
    if let Ok(mut vis) = room_root.single_mut() {
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
fn lever_target_for(gear: Gear) -> f32 {
    match gear {
        Gear::Low => 0.30,
        Gear::High => -0.30,
    }
}

pub fn animate_parts(
    mut state: ResMut<ViewState>,
    sim: Res<Sim>,
    frame: Res<crate::FrameInput>,
    time: Res<Time>,
    mut commands: Commands,
    mut parts: Query<(Entity, &Name, &mut Transform, Option<&RestRotation>), Without<ToyRoot>>,
) {
    let lever_target = lever_target_for(sim.0.gear());
    state.lever_target = lever_target;
    if state.mode != ViewMode::Toy {
        return;
    }
    let dt = time.delta_secs().min(0.05);
    // invert so turning LEFT rotates the wheel counter-clockwise as the
    // player sees it (positive local-Z rotation is CCW from the player side)
    let wheel_target = -frame.state.steer * 1.35; // ~77 deg full lock, gentler
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
    )
    .clamp(-0.349, 0.349);

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

/// Small glossy glints keep the black controls legible in the dim room. Each
/// glint is parented to its animated control, so it follows steering and gear
/// changes without becoming a separate light source.
#[allow(clippy::type_complexity)]
pub fn decorate_control_shines(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    levers: Query<(Entity, &Name), (With<Mesh3d>, Without<GearKnobShine>)>,
    wheels: Query<(Entity, &Name), (With<Mesh3d>, Without<SteeringWheelShine>)>,
) {
    for (entity, name) in &levers {
        if !name.as_str().starts_with("GearLever") {
            continue;
        }
        let shine = materials.add(StandardMaterial {
            base_color: Color::srgb(0.72, 0.78, 0.88),
            emissive: LinearRgba::new(0.055, 0.070, 0.10, 1.0),
            metallic: 0.62,
            perceptual_roughness: 0.12,
            ..default()
        });
        commands.entity(entity).insert(GearKnobShine).with_child((
            Mesh3d(meshes.add(Sphere::new(0.0022).mesh().ico(2).unwrap())),
            MeshMaterial3d(shine),
            Transform::from_xyz(-0.0028, 0.0435, 0.0022).with_scale(Vec3::new(0.58, 1.35, 0.42)),
        ));
    }
    for (entity, name) in &wheels {
        if !name.as_str().starts_with("SteeringWheel") {
            continue;
        }
        let shine = materials.add(StandardMaterial {
            base_color: Color::srgb(0.62, 0.68, 0.76),
            emissive: LinearRgba::new(0.014, 0.018, 0.026, 1.0),
            metallic: 0.58,
            perceptual_roughness: 0.18,
            ..default()
        });
        commands
            .entity(entity)
            .insert(SteeringWheelShine)
            .with_children(|wheel| {
                for (x, y, angle) in [(-0.029, 0.034, -0.70), (0.024, 0.038, 0.56)] {
                    wheel.spawn((
                        Mesh3d(meshes.add(Sphere::new(0.0008).mesh().ico(2).unwrap())),
                        MeshMaterial3d(shine.clone()),
                        Transform::from_xyz(x, y, 0.0054)
                            .with_rotation(Quat::from_rotation_z(angle))
                            .with_scale(Vec3::new(1.6, 0.34, 0.24)),
                    ));
                }
            });
    }
}

/// M8/M10: put the live game texture on the toy's screen quad via the
/// rolling-tube material (toy_look dial in `update_screen_mat`).
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub fn paint_screen(
    mut commands: Commands,
    game_image: Res<GameImage>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut crt_materials: ResMut<Assets<CrtScreenMaterial>>,
    display: Res<crate::DisplaySettings>,
    intro: Res<StartupIntro>,
    mut mat_handle: ResMut<ScreenMatHandle>,
    mut paint_state: ResMut<ScreenPaintState>,
    screens: Query<(Entity, &Name), (With<Mesh3d>, Without<ScreenPainted>)>,
) {
    if paint_state.screen && paint_state.bezel {
        return;
    }
    for (entity, name) in screens.iter() {
        if name.as_str() == "ScreenFrame" && !paint_state.bezel {
            // The original toy surrounds its display with molded near-black
            // plastic. Keep this separate from the emissive screen material.
            let bezel = materials.add(StandardMaterial {
                base_color: Color::srgb(0.025, 0.03, 0.035),
                perceptual_roughness: 0.62,
                metallic: 0.04,
                ..default()
            });
            commands
                .entity(entity)
                .insert(MeshMaterial3d(bezel))
                .insert(ScreenPainted);
            paint_state.bezel = true;
        } else if name.as_str().starts_with("Screen") && !paint_state.screen {
            let mat = crt_materials.add(CrtScreenMaterial {
                source: game_image.0.clone(),
                params: Vec4::new(
                    if display.crt_enabled { 1.0 } else { 0.0 },
                    0.0,
                    0.5,
                    intro.screen_power(),
                ),
            });
            mat_handle.0 = Some(mat.clone());
            commands
                .entity(entity)
                .remove::<MeshMaterial3d<StandardMaterial>>()
                .insert(MeshMaterial3d(mat))
                .insert(ScreenPainted);
            paint_state.screen = true;
        }
    }
}

/// M10: drive the screen shader uniforms; `L` cycles the toy_look dial
/// (0.0 -> 0.5 -> 1.0 -> 0.0) for manual comparison.
pub fn update_screen_mat(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    display: Res<crate::DisplaySettings>,
    intro: Res<StartupIntro>,
    mut view: ResMut<ViewState>,
    mat_handle: Res<ScreenMatHandle>,
    mut materials: ResMut<Assets<CrtScreenMaterial>>,
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
        mat.params = Vec4::new(
            if display.crt_enabled { 1.0 } else { 0.0 },
            time.elapsed_secs(),
            view.toy_look,
            intro.screen_power(),
        );
    }
}

/// Holds on a wide three-quarter view until the toy and its screen are ready,
/// then arcs into the authored play camera. The screen remains electrically
/// dark during the orbit and powers up only after the camera settles.
pub fn animate_startup_intro(
    time: Res<Time>,
    paint: Res<ScreenPaintState>,
    mut intro: ResMut<StartupIntro>,
    mut camera: Query<&mut Transform, With<ToyCamera>>,
    mut screen_light: Query<&mut PointLight, With<ScreenGlowLight>>,
    mut audio_events: ResMut<crate::audio::AudioEvents>,
) {
    if !paint.screen {
        return;
    }
    if intro.enabled && intro.elapsed < INTRO_TOTAL_SECS {
        let previous = intro.elapsed;
        intro.elapsed = (intro.elapsed + time.delta_secs()).min(INTRO_TOTAL_SECS);
        if previous < INTRO_ORBIT_SECS && intro.elapsed >= INTRO_ORBIT_SECS {
            crate::bridge::resume_audio();
            audio_events.emit(crate::audio::AudioCue::ToyPower);
        }
    }
    if let Ok(mut transform) = camera.single_mut() {
        *transform = intro_camera_transform(intro.camera_progress());
    }
    if let Ok(mut light) = screen_light.single_mut() {
        light.intensity = SCREEN_GLOW_INTENSITY * intro.screen_power();
    }
}

#[allow(clippy::too_many_arguments)]
pub fn apply_body_dynamics(
    mut rig: ResMut<BodyRig>,
    mut nitro_pose: ResMut<NitroPose>,
    mut pulse: ResMut<CrashPulse>,
    pads: Query<Entity, With<Gamepad>>,
    mut rumble: bevy::ecs::message::MessageWriter<GamepadRumbleRequest>,
    sim: Res<Sim>,
    frame: Res<crate::FrameInput>,
    effects: Res<crate::VisualEffects>,
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
    let speed_norm = (sim.0.speed / sim.0.high_speed_target(&tuning.0)).clamp(0.0, 1.0);
    rig.0.step(
        frame.state.steer,
        speed_norm,
        sim.0.speed,
        time.elapsed_secs(),
        &tuning.0,
    );
    let (jx, jz, jroll) = rig.0.shake_offset(time.elapsed_secs(), &tuning.0);
    let (drive_z, drive_pitch, drive_roll) = drive_pose_offsets(
        effects.downshift_secs,
        effects.downshift_severity,
        sim.0.low_overrev(&tuning.0),
        time.elapsed_secs(),
    );
    let dt = time.delta_secs().min(0.05);
    let nitro_active = crate::nitro_effects_active(&sim.0);
    let (target_z, target_pitch) = nitro_pose_targets(nitro_active);
    let stiffness = if nitro_active { 150.0 } else { 95.0 };
    let damping = if nitro_active { 20.0 } else { 19.0 };
    nitro_pose.z = game_core::dynamics::spring_step(
        nitro_pose.z,
        &mut nitro_pose.z_velocity,
        target_z,
        stiffness,
        damping,
        dt,
    );
    nitro_pose.pitch = game_core::dynamics::spring_step(
        nitro_pose.pitch,
        &mut nitro_pose.pitch_velocity,
        target_pitch,
        stiffness,
        damping,
        dt,
    );
    if let Ok(mut tf) = root.single_mut() {
        tf.rotation = Quat::from_rotation_z(rig.0.roll + jroll + drive_roll)
            * Quat::from_rotation_x(rig.0.pitch + nitro_pose.pitch + drive_pitch);
        tf.translation = Vec3::new(
            rig.0.shift + jx,
            0.0,
            rig.0.heave + jz + nitro_pose.z + drive_z,
        );
    }
}

fn drive_pose_offsets(
    remaining: f32,
    severity: Option<game_core::DownshiftSeverity>,
    overrev: f32,
    elapsed: f32,
) -> (f32, f32, f32) {
    let jolt = (remaining / 0.38).clamp(0.0, 1.0);
    let severity_gain = match severity {
        Some(game_core::DownshiftSeverity::Unsafe) => 1.0,
        Some(game_core::DownshiftSeverity::Normal) => 0.52,
        None => 0.0,
    };
    let kick = (jolt * std::f32::consts::PI).sin() * severity_gain;
    let shake = overrev.clamp(0.0, 1.0);
    (
        -0.009 * kick + (elapsed * 41.0).sin() * 0.0008 * shake,
        0.055 * kick,
        (elapsed * 53.0).sin() * 0.012 * shake,
    )
}

fn nitro_pose_targets(active: bool) -> (f32, f32) {
    if active {
        (0.018, -0.040)
    } else {
        (0.0, 0.0)
    }
}

/// M12: turbo lamp follows the gear (nods to the Tomy's HIGH-gear light).
pub fn turbo_lamp(
    sim: Res<Sim>,
    time: Res<Time>,
    menu_idle: Res<crate::MenuIdle>,
    lamp: Res<TurboLamp>,
    headlights: Res<RoadsterHeadlights>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let nitro_active = crate::nitro_effects_active(&sim.0);
    let daylight = game_core::environment::daylight(sim.0.environment_time);
    let night = (1.0 - daylight).clamp(0.0, 1.0);
    if let Some(mut mat) = materials.get_mut(&lamp.0) {
        let on = matches!(sim.0.gear(), Gear::High) || nitro_active;
        let pulse = if nitro_active {
            0.88 + 0.12 * (time.elapsed_secs() * 7.0).sin()
        } else {
            1.0
        };
        let drive_gain = if on { 0.55 } else { 0.0 };
        let glow = (0.04 + night * 1.35 + drive_gain) * pulse;
        mat.emissive = LinearRgba::new(glow * 1.45, glow * 0.07, glow * 0.025, 1.0);
    }
    if let Some(mut mat) = materials.get_mut(&headlights.0) {
        let idle_blink = menu_idle.headlight_blink();
        let glow = 0.10 + night * 1.65 + idle_blink * 4.25;
        mat.emissive = LinearRgba::new(glow, glow * 0.91, glow * 0.68, 1.0);
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

    #[test]
    fn startup_camera_reveals_the_whole_toy_then_lands_on_the_authored_view() {
        let start = intro_camera_transform(0.0);
        let middle = intro_camera_transform(0.5);
        let end = intro_camera_transform(1.0);
        let final_view = Transform::from_translation(toy_camera_position())
            .looking_at(TOY_CAMERA_TARGET, Vec3::Y);

        assert!(
            start.translation.distance(TOY_CAMERA_TARGET)
                > final_view.translation.distance(TOY_CAMERA_TARGET) * 2.5
        );
        assert!(start.translation.x < -0.5);
        assert!(middle.translation.x < -0.1);
        assert!(end.translation.abs_diff_eq(final_view.translation, 1e-6));
        assert!(end.rotation.abs_diff_eq(final_view.rotation, 1e-6));
    }

    #[test]
    fn authored_view_pushes_in_without_changing_the_screen_sightline() {
        let base_delta = TOY_CAMERA_BASE_POSITION - TOY_CAMERA_TARGET;
        let final_delta = toy_camera_position() - TOY_CAMERA_TARGET;

        assert!((final_delta.length() / base_delta.length() - 0.86).abs() < 1e-6);
        assert!(final_delta
            .normalize()
            .abs_diff_eq(base_delta.normalize(), 1e-6));
    }

    #[test]
    fn startup_screen_stays_dark_until_camera_lands_then_opens() {
        let mut intro = StartupIntro::new(true);
        assert_eq!(intro.screen_power(), 0.0);
        assert!(!intro.complete());

        intro.elapsed = INTRO_ORBIT_SECS - 0.01;
        assert_eq!(intro.screen_power(), 0.0);
        intro.elapsed = INTRO_ORBIT_SECS + INTRO_CRT_POWER_SECS * 0.5;
        assert!((intro.screen_power() - 0.5).abs() < 1e-5);
        intro.elapsed = INTRO_TOTAL_SECS;
        assert_eq!(intro.screen_power(), 1.0);
        assert!(intro.complete());
    }

    #[test]
    fn qa_mode_bypasses_the_page_load_intro() {
        let intro = StartupIntro::new(false);
        assert!(intro.complete());
        assert_eq!(intro.camera_progress(), 1.0);
        assert_eq!(intro.screen_power(), 1.0);
    }

    #[test]
    fn nitro_pose_moves_toward_camera_and_leans_back() {
        let (z, pitch) = nitro_pose_targets(true);
        assert!(z > 0.0, "the +Z camera side must make the screen larger");
        assert!(pitch < 0.0, "negative X pitch leans the toy face back");
        assert_eq!(nitro_pose_targets(false), (0.0, 0.0));
    }

    #[test]
    fn unsafe_downshift_jolt_is_stronger_and_overrev_shakes() {
        let normal = drive_pose_offsets(0.19, Some(game_core::DownshiftSeverity::Normal), 0.0, 0.0);
        let unsafe_shift =
            drive_pose_offsets(0.19, Some(game_core::DownshiftSeverity::Unsafe), 0.0, 0.0);
        assert!(unsafe_shift.0.abs() > normal.0.abs());
        assert!(unsafe_shift.1 > normal.1);
        assert_ne!(drive_pose_offsets(0.0, None, 1.0, 0.123).2, 0.0);
    }

    #[test]
    fn lever_target_is_authoritative_for_each_core_gear() {
        assert_eq!(lever_target_for(Gear::Low), 0.30);
        assert_eq!(lever_target_for(Gear::High), -0.30);
        assert!(lever_target_for(Gear::Low) > lever_target_for(Gear::High));
    }

    #[test]
    fn shooting_stars_are_sparse_staggered_and_window_bounded() {
        assert!(shooting_star_progress(16.99, 0, 0).is_none());
        assert_eq!(shooting_star_progress(17.0, 0, 0), Some(0.0));
        assert!(shooting_star_progress(17.0, 0, 4).is_none());
        assert!(shooting_star_progress(18.0, 0, 0).is_none());

        for step in 0..83 {
            let progress = step as f32 / 100.0;
            let x = -0.438 + progress * 0.258;
            let y = 0.407 - progress * 0.112 - 0.025;
            assert!((-0.46..=-0.15).contains(&x));
            assert!((0.27..=0.43).contains(&y));
        }
    }
}
