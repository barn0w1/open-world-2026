//! First-person camera with mouse-look and browser pointer lock.
//!
//! # Hierarchy
//!
//! ```text
//! Player entity  (root)
//!   └─ Transform.rotation = yaw-only (Y-axis)
//!
//!   FpsCamera entity  (child of Player)
//!     └─ Transform.translation = EYE_HEIGHT_OFFSET  (parent-local)
//!        Transform.rotation    = pitch-only (X-axis), clamped ±85°
//! ```
//!
//! Separating yaw and pitch into two entities means the walk-direction logic in
//! `player.rs` only ever needs to read the player's Y-axis rotation — it never
//! sees pitching, so looking up/down does not affect movement.
//!
//! # Pointer lock
//!
//! Browser security requires pointer lock to be requested from inside a user
//! gesture callback. Bevy's winit backend translates `CursorGrabMode::Locked`
//! into `canvas.requestPointerLock()`, which only succeeds when triggered by a
//! user event — hence the left-click trigger below.
//!
//! On desktop, `CursorGrabMode::Confined` would also work, but `Locked` is the
//! correct mode for FPS-style infinite mouse movement in a browser.

use std::f32::consts::FRAC_PI_2;

use bevy::{
    ecs::schedule::RunFixedMainLoopSystems,
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

use crate::player::Player;

// ── Constants ────────────────────────────────────────────────────────────────

/// Camera position relative to the player's origin (parent-local space).
/// Positioned near the top of the capsule to simulate eye height.
pub const EYE_HEIGHT_OFFSET: Vec3 = Vec3::new(0.0, 0.8, 0.0);

/// Pitch is clamped to ±(90° − ε) so the camera can never flip past vertical.
const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;

// ── Components ───────────────────────────────────────────────────────────────

/// Marker for the first-person camera entity.
#[derive(Component, Debug)]
pub struct FpsCamera;

/// Stores the current pitch angle (radians) of the camera entity so we can
/// clamp it without re-decomposing the quaternion every frame.
#[derive(Component, Debug)]
pub struct CameraPitch(pub f32);

/// Mouse sensitivity: x = horizontal (yaw), y = vertical (pitch), in radians
/// per raw pixel. Matched to `examples/camera/first_person_view_model.rs`.
#[derive(Component, Debug, Deref, DerefMut)]
pub struct CameraSensitivity(pub Vec2);

impl Default for CameraSensitivity {
    fn default() -> Self {
        // Horizontal slightly faster than vertical — mirrors common FPS feel.
        Self(Vec2::new(0.003, 0.002))
    }
}

// ── Plugin ───────────────────────────────────────────────────────────────────

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            // `PostStartup` is used instead of `Startup` to guarantee the
            // Player entity (spawned in `PlayerPlugin::Startup`) exists before
            // we try to attach the camera as a child.
            .add_systems(PostStartup, spawn_camera)
            .add_systems(
                RunFixedMainLoop,
                // Both systems run in `BeforeFixedMainLoop` so they execute
                // once per render frame, before any FixedUpdate physics ticks.
                // Pointer lock must be processed first so `update_mouse_look`
                // immediately sees the new grab state on the same frame as the click.
                (update_pointer_lock, update_mouse_look)
                    .chain()
                    .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
            );
    }
}

// ── Systems ──────────────────────────────────────────────────────────────────

fn spawn_camera(mut commands: Commands, player: Single<Entity, With<Player>>) {
    commands.entity(*player).with_children(|parent| {
        parent.spawn((
            Name::new("FpsCamera"),
            FpsCamera,
            Camera3d::default(),
            Projection::from(PerspectiveProjection {
                fov: 90.0_f32.to_radians(),
                ..default()
            }),
            // Position at eye height within the player capsule (parent-local).
            Transform::from_translation(EYE_HEIGHT_OFFSET),
            CameraPitch(0.0),
            CameraSensitivity::default(),
        ));
    });
}

/// Handles pointer lock for WASM mouse capture.
///
/// - Left-click → grab (request pointer lock from inside a user gesture).
/// - Escape      → release.
///
/// Pattern copied verbatim from `examples/input/mouse_grab.rs`.
fn update_pointer_lock(
    mut cursor_options: Single<&mut CursorOptions>,
    mouse: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        cursor_options.grab_mode = CursorGrabMode::Locked;
        cursor_options.visible = false;
    }
    // Note: in a browser, pressing Escape also releases pointer lock at the
    // browser level before Bevy sees it, so the system below is somewhat
    // redundant but ensures `visible` is correctly restored.
    if key.just_pressed(KeyCode::Escape) {
        cursor_options.grab_mode = CursorGrabMode::None;
        cursor_options.visible = true;
    }
}

/// Applies mouse motion to camera look.
///
/// - Mouse X → yaw on the Player root entity (Y-axis rotation).
/// - Mouse Y → pitch on the FpsCamera child entity (X-axis rotation), clamped.
///
/// Only active while the cursor is locked; raw mouse deltas are meaningless
/// (and confusing) when the cursor is free.
fn update_mouse_look(
    motion: Res<AccumulatedMouseMotion>,
    cursor_options: Single<&CursorOptions>,
    // `Without<FpsCamera>` makes this query disjoint from the camera query below
    // so Bevy can verify at startup that both `&mut Transform` borrows are safe.
    mut player: Single<&mut Transform, (With<Player>, Without<FpsCamera>)>,
    mut camera: Single<
        (&mut Transform, &mut CameraPitch, &CameraSensitivity),
        (With<FpsCamera>, Without<Player>),
    >,
) {
    // Guard: only rotate while pointer-locked.
    if cursor_options.grab_mode != CursorGrabMode::Locked {
        return;
    }

    let delta = motion.delta;
    if delta == Vec2::ZERO {
        return;
    }

    let (mut cam_tf, mut cam_pitch, sensitivity) = camera.into_inner();

    // ── Yaw ─────────────────────────────────────────────────────────────────
    // Modify only the yaw (Y) component; preserve any existing roll (should
    // always be 0 but guard against floating-point drift).
    let (yaw, _pitch, roll) = player.rotation.to_euler(EulerRot::YXZ);
    let new_yaw = yaw - delta.x * sensitivity.x;
    // Player pitch is always 0 — the camera child owns all pitch.
    player.rotation = Quat::from_euler(EulerRot::YXZ, new_yaw, 0.0, roll);

    // ── Pitch ────────────────────────────────────────────────────────────────
    cam_pitch.0 = (cam_pitch.0 - delta.y * sensitivity.y).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    cam_tf.rotation = Quat::from_rotation_x(cam_pitch.0);
}
