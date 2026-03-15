//! Static world geometry and lighting.
//!
//! Spawns:
//! - A 50×50 m flat ground plane.
//! - Four box obstacles of varying sizes to test movement and collision.
//! - Ambient light (global fill) and a directional "sun" light with shadows.

use bevy::prelude::*;

// ── Component ─────────────────────────────────────────────────────────────────

/// AABB collision data for a static, axis-aligned box obstacle.
///
/// `half_extents` are the per-axis half-sizes of the box centred at the entity's
/// `Transform.translation`. They must match the visual mesh dimensions.
/// `player.rs::resolve_aabb` reads this component to perform push-out resolution.
#[derive(Component, Debug)]
pub struct StaticObstacle {
    pub half_extents: Vec3,
}

// ── Plugin ───────────────────────────────────────────────────────────────────

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_world_geometry, setup_lighting));
    }
}

// ── Systems ──────────────────────────────────────────────────────────────────

fn spawn_world_geometry(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // ── Ground plane ─────────────────────────────────────────────────────────
    // `Plane3d::new(normal, half_size)` — half_size = 25.0 → visible 50×50 m.
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(25.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.28, 0.50, 0.28), // muted green
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::IDENTITY,
    ));

    // ── Obstacles ─────────────────────────────────────────────────────────────
    // Each entry: (name, world_center, half_extents).
    // `half_extents` drive both the `Cuboid` mesh (full = 2 * half) and the
    // `StaticObstacle` component used for AABB collision.
    let obstacle_color = materials.add(StandardMaterial {
        base_color: Color::srgb(0.70, 0.50, 0.22),
        perceptual_roughness: 0.6,
        ..default()
    });

    let obstacles: &[(&str, Vec3, Vec3)] = &[
        // A compact cube near the spawn — easy "wall" to test head-on collision.
        ("Box_A", Vec3::new(5.0, 0.5, -5.0), Vec3::new(1.0, 0.5, 1.0)),
        // A wider block off to the left — tests strafing around a larger body.
        ("Box_B", Vec3::new(-6.0, 0.75, 4.0), Vec3::new(1.5, 0.75, 1.5)),
        // A taller, narrow pillar — tests looking up and collision at head height.
        ("Box_C", Vec3::new(8.0, 1.0, 3.0), Vec3::new(0.8, 1.0, 0.8)),
        // A long, low slab — tests sliding along a flat surface.
        ("Box_D", Vec3::new(-3.0, 0.5, -8.0), Vec3::new(2.0, 0.5, 0.5)),
    ];

    for (name, center, half) in obstacles {
        commands.spawn((
            Name::new(*name),
            Mesh3d(meshes.add(Cuboid::new(
                half.x * 2.0,
                half.y * 2.0,
                half.z * 2.0,
            ))),
            MeshMaterial3d(obstacle_color.clone()),
            // `center` already has y = half.y so the box sits on the ground.
            Transform::from_translation(*center),
            StaticObstacle {
                half_extents: *half,
            },
        ));
    }
}

fn setup_lighting(mut commands: Commands) {
    // ── Ambient light ─────────────────────────────────────────────────────────
    // `GlobalAmbientLight` is a `Resource`, not a component. It provides a
    // uniform base illumination across the whole scene.
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 150.0,
        ..default()
    });

    // ── Directional "sun" light ───────────────────────────────────────────────
    // Rotated to point roughly south-east and downward, casting soft side shadows
    // that make the box geometry easier to read.
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: 10_000.0, // lux — roughly an overcast day
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            -std::f32::consts::FRAC_PI_4, // yaw  45° south-east
            -std::f32::consts::FRAC_PI_4, // pitch 45° downward
            0.0,
        )),
    ));
}
