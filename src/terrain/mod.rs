//! Terrain plugin — procedural world generation and real-time deformation.

pub mod chunk_manager;
pub mod deform;
pub mod r#gen;
pub mod marching_cubes;
pub mod mesh;

pub use chunk_manager::{ChunkManager, TerrainChunk};
pub use deform::{DeformEvent, VoxelStore};
pub use r#gen::{VoxelChunk, CHUNK_SIZE, VOXEL_SCALE};

use bevy::prelude::*;
use std::collections::HashMap;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ChunkManager {
                loaded:        HashMap::new(),
                view_distance: 4,
                seed:          42,
            })
            .insert_resource(VoxelStore::default())
            .add_message::<DeformEvent>()
            .add_systems(Update, (
                chunk_manager::update_loaded_chunks,
                deform::emit_deform_events,
                deform::apply_deform,
            ));
    }
}
