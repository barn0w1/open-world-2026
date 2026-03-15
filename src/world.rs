//! World lighting setup.

use bevy::prelude::*;

// ── Component ─────────────────────────────────────────────────────────────────

/// AABB collision data for a static, axis-aligned box obstacle.
/// Kept for future use; no obstacles are spawned with the procedural terrain.
#[derive(Component, Debug)]
pub struct StaticObstacle {
    pub half_extents: Vec3,
}

// ── Plugin ───────────────────────────────────────────────────────────────────

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_lighting);
    }
}

// ── Systems ──────────────────────────────────────────────────────────────────

fn setup_lighting(mut commands: Commands) {
    // ── Ambient light — soft blue fill, Astroneer-style ────────────────────
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.55, 0.70, 0.95), // cool blue sky fill
        brightness: 120.0,
        ..default()
    });

    // ── Directional "sun" — warm golden angle ──────────────────────────────
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            color: Color::srgb(1.0, 0.90, 0.70), // warm golden-white
            illuminance: 15_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            -std::f32::consts::FRAC_PI_4,       // yaw  45° south-east
            -std::f32::consts::FRAC_PI_3,       // pitch 60° downward — low sun
            0.0,
        )),
    ));
}
