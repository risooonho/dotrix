mod chunk;

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};
use noise::{ NoiseFn, Fbm, Perlin, Seedable, /* Perlin, Turbulence, Seedable, */ MultiFractal };
use rayon::prelude::*;

use dotrix::{
    assets::{ Id, Mesh },
    components::{ Model },
    ecs::{ Const, Mut, Context },
    math::{ Point3, Vec3, Vec3i },
    renderer::{ Transform },
    services::{ Assets, Camera, World },
    terrain::{
        Density,
        MarchingCubes,
    }
};

use crate::editor::Editor;
use chunk::*;

const NUMBER_OF_RINGS: usize = 3;
const IS_ODD_RING: bool = (NUMBER_OF_RINGS % 2 != 0);
const CHUNKS_IN_RING: usize = 3;

pub struct Terrain {
    pub rings: Vec<HashMap<Vec3i, Chunk>>,
    pub last_viewer_position: Option<Point3>,
    pub update_if_moved_by: f32,
    pub density: Option<Density>,
    pub zero_at: Vec3i,
}

impl Terrain {
    pub fn new() -> Self {
        let update_if_moved_by = Chunk::size() as f32 * 0.5;
        Self {
            rings: (0..NUMBER_OF_RINGS).map(|_| HashMap::new()).collect::<Vec<_>>(),
            update_if_moved_by: update_if_moved_by * update_if_moved_by,
            last_viewer_position: None,
            density: None,
            zero_at: Vec3i::new(0, 0 ,0),
        }
    }

    pub fn set_density(&mut self, density: Density, zero_at: Vec3i) {
        self.density = Some(density);
        self.zero_at = zero_at;
    }


}

impl Default for Terrain {
    fn default() -> Self {
        Self::new()
    }
}

pub fn spawn(
    camera: Const<Camera>,
    mut terrain: Mut<Terrain>,
    mut assets: Mut<Assets>,
    mut editor: Mut<Editor>,
    mut world: Mut<World>,
) {
    let viewer_position = camera.target;

    // check if update is necessary
    if let Some(last_viewer_position) = terrain.last_viewer_position.as_ref() {
        let dx = viewer_position.x - last_viewer_position.x;
        let dy = viewer_position.y - last_viewer_position.y;
        let dz = viewer_position.z - last_viewer_position.z;
        if dx * dx + dy * dy + dz * dz < terrain.update_if_moved_by { return; }
    }

    terrain.last_viewer_position = Some(viewer_position);


    // disable all chunks
    for r in 0..NUMBER_OF_RINGS {
        for chunk in terrain.rings[r].values_mut() {
            chunk.disabled = true;
        }
    }

    // calculate view range and level of details
    let chunk_size = Chunk::size();
    let number_of_rings = NUMBER_OF_RINGS;
    let origin_x = (viewer_position.x / chunk_size as f32).floor() as i32;
    let origin_z = (viewer_position.z / chunk_size as f32).floor() as i32;
    // let from_x = origin_x - CHUNKS_IN_RING as i32 / 2;
    // let from_z = origin_z - CHUNKS_IN_RING as i32 / 2;
    // let to_x = origin_x + (CHUNKS_IN_RING as f32 / 2.0).ceil() as i32;
    // let to_z = origin_z + (CHUNKS_IN_RING as f32 / 2.0).ceil() as i32;
    let mut from_x = origin_x;
    let mut from_z = origin_z;

    let now = std::time::Instant::now();
    for r in 0..NUMBER_OF_RINGS {
        let scale = CHUNKS_IN_RING.pow(r as u32) as i32;
        from_x -= scale;
        from_z -= scale;
        let height = (CHUNKS_IN_RING as f32 / scale as f32).ceil() as i32;
        println!("Ring #{}, scale {}, height {}", r, scale, height);
        for x in 0..CHUNKS_IN_RING {
            let xs = from_x + (x as i32 * scale);
            let y = 0;
            for y in 0..height {
                let ys = y * scale;
                for z in 0..CHUNKS_IN_RING {
                    let zs = from_z + (z as i32 * scale);
                    if r != 0 && z == 1 && x == 1 {
                        continue; 
                    }
                    let index = Vec3i::new(xs, ys, zs) * chunk_size as i32;

                    // get chunk
                    let chunk = terrain.rings[r].entry(index)
                        .or_insert(Chunk::new(index, r));

                    if chunk.changed {
                        chunk.polygonize(&mut assets, &mut world, &editor.density);
                    }

                    chunk.disabled = false;
                    println!("  ({:?})", index);
                }
            }
        }
    }

    let query = world.query::<(&mut Model, &Tile)>();
    for (model, tile) in query {
        model.disabled = terrain.rings[tile.ring].get(&tile.position)
            .map(|chunk| chunk.disabled || model.mesh.is_null())
            .unwrap_or_else(|| {
                /*
                println!("Chunk not found: {:?}", index);
                let mesh = model.mesh;
                if !mesh.is_null() {
                    // assets.remove(mesh);
                }
                model.mesh = Id::new(0);
                unused_entities += 1;
                */
                true
            });
    }
    println!("{} rings has been built in {}us", NUMBER_OF_RINGS, now.elapsed().as_micros());
}


