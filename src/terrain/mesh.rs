//! Converts Marching Cubes output into a Bevy Mesh.

use bevy::{
    prelude::*,
    render::{
        mesh::Indices,
        render_asset::RenderAssetUsages,
        render_resource::PrimitiveTopology,
    },
};

use super::marching_cubes::McOutput;

/// Builds a `Mesh` from marching-cubes output.
///
/// `McOutput::colors` is `[f32; 3]` (RGB) but `Mesh::ATTRIBUTE_COLOR` is
/// `Float32x4`, so each colour is padded with alpha = 1.0.
pub fn build_mesh(mc: McOutput) -> Mesh {
    let colors: Vec<[f32; 4]> = mc
        .colors
        .iter()
        .map(|&[r, g, b]| [r, g, b, 1.0])
        .collect();

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, mc.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, mc.normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(mc.indices));
    mesh
}

/// Returns a `StandardMaterial` that displays per-vertex colours.
///
/// In Bevy 0.18 there is no `vertex_colors` toggle; the pipeline activates
/// vertex colours automatically when `Mesh::ATTRIBUTE_COLOR` is present.
/// Using `base_color: Color::WHITE` ensures colours are not tinted.
pub fn terrain_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.9,
        metallic: 0.0,
        ..default()
    }
}
