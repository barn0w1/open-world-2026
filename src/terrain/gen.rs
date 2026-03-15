//! Noise-based voxel density field generation.

use bevy::math::IVec3;
use noise::{NoiseFn, Perlin};

pub const CHUNK_SIZE: usize = 32;
pub const VOXEL_SCALE: f32  = 1.0; // metres per voxel
pub const CHUNK_VOL:  usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

// Terrain shape parameters
const SEA_LEVEL:      f32 = 0.0;
const TERRAIN_HEIGHT: f32 = 24.0;

pub struct VoxelChunk {
    pub origin:  IVec3,
    pub density: Box<[f32; CHUNK_VOL]>,
}

impl VoxelChunk {
    pub fn generate(origin: IVec3, seed: u32) -> Self {
        let perlin = Perlin::new(seed);
        let mut density = Box::new([0.0f32; CHUNK_VOL]);

        for iz in 0..CHUNK_SIZE {
            for iy in 0..CHUNK_SIZE {
                for ix in 0..CHUNK_SIZE {
                    let wx = (origin.x * CHUNK_SIZE as i32 + ix as i32) as f64
                        * VOXEL_SCALE as f64;
                    let wy = (origin.y * CHUNK_SIZE as i32 + iy as i32) as f64
                        * VOXEL_SCALE as f64;
                    let wz = (origin.z * CHUNK_SIZE as i32 + iz as i32) as f64
                        * VOXEL_SCALE as f64;

                    let fbm = {
                        let o1 = perlin.get([wx * 0.03, wy * 0.03, wz * 0.03]) * 1.000;
                        let o2 = perlin.get([wx * 0.06, wy * 0.06, wz * 0.06]) * 0.500;
                        let o3 = perlin.get([wx * 0.12, wy * 0.12, wz * 0.12]) * 0.250;
                        let o4 = perlin.get([wx * 0.24, wy * 0.24, wz * 0.24]) * 0.125;
                        (o1 + o2 + o3 + o4) / (1.0 + 0.5 + 0.25 + 0.125)
                    };

                    let d = fbm as f32 - (wy as f32 - SEA_LEVEL) / TERRAIN_HEIGHT;
                    density[ix + iy * CHUNK_SIZE + iz * CHUNK_SIZE * CHUNK_SIZE] = d;
                }
            }
        }

        VoxelChunk { origin, density }
    }

    #[inline]
    pub fn density_at(&self, ix: usize, iy: usize, iz: usize) -> f32 {
        self.density[ix + iy * CHUNK_SIZE + iz * CHUNK_SIZE * CHUNK_SIZE]
    }

    pub fn density_at_or_air(&self, ix: i32, iy: i32, iz: i32) -> f32 {
        let s = CHUNK_SIZE as i32;
        if ix < 0 || iy < 0 || iz < 0 || ix >= s || iy >= s || iz >= s {
            return -1.0;
        }
        self.density_at(ix as usize, iy as usize, iz as usize)
    }
}
