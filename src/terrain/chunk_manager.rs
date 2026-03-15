//! Loads and unloads terrain chunks around the player.

use bevy::prelude::*;
use std::collections::HashMap;

use crate::player::Player;
use super::r#gen::{VoxelChunk, CHUNK_SIZE, VOXEL_SCALE};
use super::marching_cubes::extract;
use super::mesh::{build_mesh, terrain_material};

// ── Resource ──────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct ChunkManager {
    pub loaded:        HashMap<IVec3, Entity>,
    pub view_distance: i32,
    pub seed:          u32,
}

impl Default for ChunkManager {
    fn default() -> Self {
        Self {
            loaded:        HashMap::new(),
            view_distance: 4,
            seed:          42,
        }
    }
}

// ── Component ─────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct TerrainChunk {
    pub chunk_pos: IVec3,
}

// ── System ────────────────────────────────────────────────────────────────────

/// Generates and spawns chunks inside `view_distance`; despawns those outside.
pub fn update_loaded_chunks(
    player:        Single<&Transform, With<Player>>,
    mut manager:   ResMut<ChunkManager>,
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let chunk_pos = (player.translation / (CHUNK_SIZE as f32 * VOXEL_SCALE))
        .floor()
        .as_ivec3();

    // Copy out primitive fields before the mutable borrow of `manager`.
    let vd   = manager.view_distance;
    let seed = manager.seed;

    // Build the full set of desired chunk positions.
    let mut desired: Vec<IVec3> = Vec::new();
    for dz in -vd..=vd {
        for dy in -vd..=vd {
            for dx in -vd..=vd {
                desired.push(chunk_pos + IVec3::new(dx, dy, dz));
            }
        }
    }

    // Spawn any chunk not yet loaded.
    for pos in desired {
        if manager.loaded.contains_key(&pos) {
            continue;
        }

        let chunk = VoxelChunk::generate(pos, seed);
        let mc    = extract(&chunk);

        // Skip empty chunks (air regions produce zero triangles).
        if mc.indices.is_empty() {
            continue;
        }

        let mesh_handle     = meshes.add(build_mesh(mc));
        let material_handle = materials.add(terrain_material());
        let world_origin    = pos.as_vec3() * CHUNK_SIZE as f32 * VOXEL_SCALE;

        let entity = commands
            .spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(material_handle),
                Transform::from_translation(world_origin),
                TerrainChunk { chunk_pos: pos },
            ))
            .id();

        manager.loaded.insert(pos, entity);
    }

    // Collect keys whose Chebyshev distance exceeds unload_dist.
    let unload_dist = vd + 1;
    let to_remove: Vec<IVec3> = manager
        .loaded
        .keys()
        .filter(|&&p| {
            let d = p - chunk_pos;
            d.x.abs() > unload_dist || d.y.abs() > unload_dist || d.z.abs() > unload_dist
        })
        .copied()
        .collect();

    for pos in to_remove {
        if let Some(entity) = manager.loaded.remove(&pos) {
            commands.entity(entity).despawn();
        }
    }
}
