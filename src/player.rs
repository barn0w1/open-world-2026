//! Player physics, movement, and jump.
//!
//! # Design
//!
//! Physics run in `FixedUpdate` at 64 Hz (Bevy default). The visual `Transform`
//! is a *separate* interpolated representation so rendering stays smooth even
//! when the physics tick rate differs from the frame rate. This pattern is taken
//! directly from `examples/movement/physics_in_fixed_timestep.rs`.
//!
//! Gravity and collision are manual (no external physics crate):
//! - Gravity accelerates `Velocity::y` downward each tick when airborne.
//! - Ground is the plane y = 0; the player's AABB bottom must stay at or above it.
//! - Static obstacles are AABB-resolved by pushing out on the axis of smallest
//!   penetration depth.

use bevy::{
    ecs::schedule::{RunFixedMainLoopSystems, ScheduleLabel},
    prelude::*,
};

use crate::world::StaticObstacle;

// ── Constants ────────────────────────────────────────────────────────────────

/// Gravitational acceleration in m/s².
pub const GRAVITY: f32 = 20.0;

/// Half-height of the player's capsule (metres). The player's origin sits at
/// ground level when `physical_translation.y == PLAYER_HALF_HEIGHT`.
pub const PLAYER_HALF_HEIGHT: f32 = 0.9;

/// Half-width (and depth) of the player's horizontal AABB for collision.
pub const PLAYER_HALF_WIDTH: f32 = 0.35;

/// Horizontal walk speed in m/s.
pub const MOVE_SPEED: f32 = 5.0;

/// Upward velocity applied on jump.
pub const JUMP_IMPULSE: f32 = 8.0;

// ── Components ───────────────────────────────────────────────────────────────

/// Marker for the player root entity.
#[derive(Component, Debug)]
pub struct Player;

/// Authoritative physics-simulation position.
///
/// The visual `Transform.translation` lags behind this by up to one fixed tick
/// and is updated by `interpolate_player_visual` via `lerp`.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut)]
pub struct PhysicalTranslation(pub Vec3);

/// The value `PhysicalTranslation` held at the *start* of the last `FixedUpdate`
/// tick. Used as the "from" end of the visual interpolation lerp.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut)]
pub struct PreviousPhysicalTranslation(pub Vec3);

/// World-space velocity in m/s.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut)]
pub struct Velocity(pub Vec3);

/// Per-frame keyboard input, accumulated once per render frame and consumed by
/// `advance_physics` (which may run 0–N times per render frame).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AccumulatedInput {
    /// Horizontal movement intent: x = strafe (right positive), y = forward.
    pub movement: Vec2,
    /// Set when Space is pressed while grounded; cleared after `advance_physics`
    /// consumes it so a single key press produces exactly one jump.
    pub jump_pressed: bool,
}

/// True while the player is in contact with the ground.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Grounded(pub bool);

// ── Resource ─────────────────────────────────────────────────────────────────

/// Signals to `AfterFixedMainLoop` systems whether `FixedUpdate` actually ran
/// this render frame (it may be skipped when the frame is very short).
///
/// Without this guard, `clear_accumulated_input` would erase input that was
/// never consumed by physics — causing dropped move or jump inputs.
#[derive(Resource, Debug, Default, Deref, DerefMut)]
pub struct DidFixedTimestepRunThisFrame(bool);

// ── Plugin ───────────────────────────────────────────────────────────────────

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DidFixedTimestepRunThisFrame>()
            .add_systems(Startup, spawn_player)
            // Clear the flag at the very start of each render frame.
            .add_systems(PreUpdate, clear_fixed_timestep_flag)
            // Set the flag whenever FixedUpdate actually runs.
            .add_systems(FixedPreUpdate, set_fixed_timestep_flag)
            // Core physics tick.
            .add_systems(FixedUpdate, advance_physics)
            .add_systems(
                RunFixedMainLoop,
                (
                    // Input accumulation runs once per render frame, before any
                    // FixedUpdate ticks that may happen this frame.
                    accumulate_player_input
                        .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
                    // After all FixedUpdate ticks for this frame have run:
                    // 1. Clear accumulated input (only if physics consumed it).
                    // 2. Lerp the visual Transform to the new physics position.
                    (
                        clear_accumulated_input
                            .run_if(did_fixed_timestep_run),
                        interpolate_player_visual,
                    )
                        .chain()
                        .in_set(RunFixedMainLoopSystems::AfterFixedMainLoop),
                ),
            );
    }
}

// ── Condition ────────────────────────────────────────────────────────────────

fn did_fixed_timestep_run(flag: Res<DidFixedTimestepRunThisFrame>) -> bool {
    **flag
}

// ── Systems ──────────────────────────────────────────────────────────────────

fn spawn_player(mut commands: Commands) {
    let start = Vec3::new(0.0, PLAYER_HALF_HEIGHT, 0.0);
    commands.spawn((
        Name::new("Player"),
        Player,
        Transform::from_translation(start),
        Visibility::default(),
        PhysicalTranslation(start),
        PreviousPhysicalTranslation(start),
        Velocity::default(),
        AccumulatedInput::default(),
        Grounded(true),
    ));
}

fn clear_fixed_timestep_flag(mut flag: ResMut<DidFixedTimestepRunThisFrame>) {
    **flag = false;
}

fn set_fixed_timestep_flag(mut flag: ResMut<DidFixedTimestepRunThisFrame>) {
    **flag = true;
}

/// Reads the keyboard state and writes intent into `AccumulatedInput`.
///
/// Runs in `BeforeFixedMainLoop` — once per render frame, before any physics
/// ticks. The `movement` field is overwritten every frame (not accumulated),
/// which means the last held direction before a physics tick is used. `jump_pressed`
/// is latched and only cleared by `advance_physics`.
fn accumulate_player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player: Single<(&mut AccumulatedInput, &Grounded)>,
) {
    let (mut input, grounded) = player.into_inner();

    input.movement = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        input.movement.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        input.movement.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        input.movement.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        input.movement.x += 1.0;
    }

    // Latch jump: set once, consumed in advance_physics.
    // Check `grounded` here so a mid-air Space press is ignored.
    if keyboard.just_pressed(KeyCode::Space) && grounded.0 {
        input.jump_pressed = true;
    }
}

fn clear_accumulated_input(mut input: Single<&mut AccumulatedInput>) {
    // Reset movement; jump_pressed was already cleared by advance_physics.
    input.movement = Vec2::ZERO;
}

/// The core physics tick: gravity, movement, collision.
///
/// Reads yaw from the player's *visual* `Transform.rotation` which is updated
/// by `update_mouse_look` in `BeforeFixedMainLoop` (earlier this frame), so
/// this always has the frame's latest look direction.
fn advance_physics(
    fixed_time: Res<Time<Fixed>>,
    mut player: Single<
        (
            &Transform,   // used to read yaw for movement direction
            &mut PhysicalTranslation,
            &mut PreviousPhysicalTranslation,
            &mut Velocity,
            &mut AccumulatedInput,
            &mut Grounded,
        ),
        With<Player>,
    >,
    obstacles: Query<(&Transform, &StaticObstacle)>,
) {
    let (tf, mut pos, mut prev_pos, mut vel, mut input, mut grounded) =
        player.into_inner();
    let dt = fixed_time.delta_secs();

    // Save previous position for the visual interpolation lerp.
    prev_pos.0 = pos.0;

    // ── Horizontal movement ──────────────────────────────────────────────────
    // Decompose yaw from the player's rotation (the pitch lives on the camera
    // child and must NOT influence the walk direction).
    let (yaw, _pitch, _roll) = tf.rotation.to_euler(EulerRot::YXZ);
    let yaw_quat = Quat::from_rotation_y(yaw);

    // Local space: x = strafe, z = forward (Bevy's -Z is "into the screen").
    let local_dir = Vec3::new(input.movement.x, 0.0, -input.movement.y);
    // `clamp_length_max(1)` normalises diagonal input without snapping axes.
    let world_dir = yaw_quat * local_dir.clamp_length_max(1.0);

    vel.0.x = world_dir.x * MOVE_SPEED;
    vel.0.z = world_dir.z * MOVE_SPEED;

    // ── Jump ─────────────────────────────────────────────────────────────────
    if input.jump_pressed && grounded.0 {
        vel.0.y = JUMP_IMPULSE;
        grounded.0 = false;
        input.jump_pressed = false; // consume — prevents double-jump on slow ticks
    }

    // ── Gravity ──────────────────────────────────────────────────────────────
    if !grounded.0 {
        vel.0.y -= GRAVITY * dt;
    }

    // ── Integrate ────────────────────────────────────────────────────────────
    pos.0 += vel.0 * dt;

    // ── Ground plane (y = 0) collision ───────────────────────────────────────
    if pos.0.y <= PLAYER_HALF_HEIGHT {
        pos.0.y = PLAYER_HALF_HEIGHT;
        vel.0.y = 0.0;
        grounded.0 = true;
    }

    // ── Static obstacle AABB resolution ──────────────────────────────────────
    for (obs_tf, obs) in obstacles.iter() {
        resolve_aabb(&mut pos.0, &mut vel.0, obs_tf.translation, obs.half_extents);
    }
}

/// Push the player out of an AABB obstacle.
///
/// Uses the Separating Axis Theorem: find the axis with the smallest overlap
/// and push out along it. Also zeroes the velocity component on that axis to
/// prevent the player from "sliding through" corners.
fn resolve_aabb(player_pos: &mut Vec3, player_vel: &mut Vec3, obs_center: Vec3, obs_half: Vec3) {
    let player_half = Vec3::new(PLAYER_HALF_WIDTH, PLAYER_HALF_HEIGHT, PLAYER_HALF_WIDTH);

    // Vector from obstacle centre to player centre.
    let delta = *player_pos - obs_center;

    // Overlap on each axis (positive = penetrating).
    let overlap = (player_half + obs_half) - delta.abs();

    // No contact if any axis is non-overlapping.
    if overlap.x <= 0.0 || overlap.y <= 0.0 || overlap.z <= 0.0 {
        return;
    }

    // Resolve along the axis of minimum penetration.
    if overlap.x <= overlap.y && overlap.x <= overlap.z {
        player_pos.x += overlap.x * delta.x.signum();
        player_vel.x = 0.0;
    } else if overlap.y <= overlap.x && overlap.y <= overlap.z {
        player_pos.y += overlap.y * delta.y.signum();
        player_vel.y = 0.0;
    } else {
        player_pos.z += overlap.z * delta.z.signum();
        player_vel.z = 0.0;
    }
}

/// Lerps the visual `Transform.translation` between the previous and current
/// physics positions using the fixed-update overstep fraction.
///
/// Only `translation` is touched here — `rotation` (player yaw) is managed
/// separately by `update_mouse_look` in `camera.rs`.
fn interpolate_player_visual(
    fixed_time: Res<Time<Fixed>>,
    mut player: Single<
        (&mut Transform, &PhysicalTranslation, &PreviousPhysicalTranslation),
        With<Player>,
    >,
) {
    let (mut tf, current, previous) = player.into_inner();
    let alpha = fixed_time.overstep_fraction();
    tf.translation = previous.0.lerp(current.0, alpha);
}
