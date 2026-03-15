//! Real-time terrain deformation (dig and fill).

use bevy::{prelude::*, window::{CursorGrabMode, CursorOptions}};

use crate::camera::FpsCamera;
use crate::player::Player;
use super::chunk_manager::{ChunkManager, TerrainChunk};
use super::gen::{VoxelChunk, CHUNK_SIZE, VOXEL_SCALE};
use super::marching_cubes::extract;
use super::mesh::build_mesh;

// ── Event ─────────────────────────────────────────────────────────────────────

#[derive(Event, Clone)]
pub struct DeformEvent {
    pub world_pos: Vec3,
    pub radius:    f32,
    /// Negative = dig, positive = fill.
    pub delta:     f32,
}

// ── Resource ──────────────────────────────────────────────────────────────────

/// Authoritative voxel density store. Holds all chunks that have been
/// generated or deformed. `update_loaded_chunks` populates this lazily via
/// `apply_deform`, which regenerates a chunk from seed the first time it is
/// touched by a deformation event.
#[derive(Resource, Default)]
pub struct VoxelStore {
    pub chunks: std::collections::HashMap<IVec3, VoxelChunk>,
}

// ── Systems ──────────────────────────────────────────────────────────────────

/// Fires [`DeformEvent`]s while the cursor is pointer-locked.
///
/// - LMB held → dig  (delta < 0)
/// - RMB held → fill (delta > 0)
///
/// The target point is 4 m ahead of the player along the camera's look axis.
pub fn emit_deform_events(
    mouse:          Res<ButtonInput<MouseButton>>,
    time:           Res<Time>,
    cursor_options: Single<&CursorOptions>,
    camera_gt:      Single<&GlobalTransform, With<FpsCamera>>,
    player:         Single<&Transform, With<Player>>,
    mut events:     EventWriter<DeformEvent>,
) {
    if cursor_options.grab_mode != CursorGrabMode::Locked {
        return;
    }

    // Dir3 derefs to Vec3 (Deref<Target = Vec3> impl in bevy_math).
    let forward: Vec3 = *camera_gt.forward();
    let target = player.translation + forward * 4.0;
    let dt = time.delta_secs();

    if mouse.pressed(MouseButton::Left) {
        events.write(DeformEvent { world_pos: target, radius: 3.0, delta: -2.0 * dt });
    }
    if mouse.pressed(MouseButton::Right) {
        events.write(DeformEvent { world_pos: target, radius: 3.0, delta:  2.0 * dt });
    }
}

/// Applies pending [`DeformEvent`]s to the voxel density field and rebuilds
/// the affected chunk meshes.
pub fn apply_deform(
    mut events:  EventReader<DeformEvent>,
    mut store:   ResMut<VoxelStore>,
    manager:     Res<ChunkManager>,
    chunks_q:    Query<(Entity, &TerrainChunk, &Mesh3d)>,
    mut meshes:  ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    // Collect first so we release the EventReader borrow before the main work.
    let events_vec: Vec<DeformEvent> = events.read().cloned().collect();
    if events_vec.is_empty() {
        return;
    }

    for event in &events_vec {
        for chunk_pos in affected_chunks(event.world_pos, event.radius) {
            // Lazily populate VoxelStore from the noise seed.
            if !store.chunks.contains_key(&chunk_pos) {
                store.chunks.insert(
                    chunk_pos,
                    VoxelChunk::generate(chunk_pos, manager.seed),
                );
            }

            let chunk = store.chunks.get_mut(&chunk_pos).unwrap();
            let mut modified = false;

            for iz in 0..CHUNK_SIZE {
                for iy in 0..CHUNK_SIZE {
                    for ix in 0..CHUNK_SIZE {
                        let wx = (chunk_pos.x * CHUNK_SIZE as i32 + ix as i32) as f32 * VOXEL_SCALE;
                        let wy = (chunk_pos.y * CHUNK_SIZE as i32 + iy as i32) as f32 * VOXEL_SCALE;
                        let wz = (chunk_pos.z * CHUNK_SIZE as i32 + iz as i32) as f32 * VOXEL_SCALE;

                        let dist = (Vec3::new(wx, wy, wz) - event.world_pos).length();
                        if dist >= event.radius {
                            continue;
                        }

                        let falloff = 1.0 - (dist / event.radius).powi(2);
                        let idx = ix + iy * CHUNK_SIZE + iz * CHUNK_SIZE * CHUNK_SIZE;
                        chunk.density[idx] =
                            (chunk.density[idx] + event.delta * falloff).clamp(-2.0, 2.0);
                        modified = true;
                    }
                }
            }

            if !modified {
                continue;
            }

            let mc = extract(chunk);

            // Find the spawned entity for this chunk position.
            let found = chunks_q
                .iter()
                .find(|(_, tc, _)| tc.chunk_pos == chunk_pos);

            if let Some((entity, _, mesh3d)) = found {
                if mc.indices.is_empty() {
                    // Chunk carved to air — remove the entity.
                    // ChunkManager.loaded retains a stale entry; chunk_manager
                    // will skip re-spawning it because it is still "loaded".
                    commands.entity(entity).despawn();
                } else if let Some(mesh) = meshes.get_mut(mesh3d.0.id()) {
                    *mesh = build_mesh(mc);
                }
            }
            // If entity was never spawned (all-air chunk before deform), the
            // mesh is ready in VoxelStore; chunk_manager will spawn it with the
            // updated density on the next Update tick.
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns every chunk position whose AABB overlaps the sphere (world_pos, radius).
fn affected_chunks(world_pos: Vec3, radius: f32) -> Vec<IVec3> {
    let scale = CHUNK_SIZE as f32 * VOXEL_SCALE;
    let min = ((world_pos - Vec3::splat(radius)) / scale).floor().as_ivec3();
    let max = ((world_pos + Vec3::splat(radius)) / scale).floor().as_ivec3();

    let mut result = Vec::new();
    for z in min.z..=max.z {
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                result.push(IVec3::new(x, y, z));
            }
        }
    }
    result
}
