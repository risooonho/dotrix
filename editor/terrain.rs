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

pub struct Terrain {
    pub rings: Vec<HashMap<Vec3i, Chunk>>,
    pub last_viewer_position: Option<Point3>,
    pub update_if_moved_by: f32,
    pub view_distance: i32,
}

impl Terrain {
    pub fn new() -> Self {
        let update_if_moved_by = Chunk::size() as f32 * 0.5;
        Self {
            rings: (0..NUMBER_OF_RINGS).map(|_| HashMap::new()).collect::<Vec<_>>(),
            update_if_moved_by: update_if_moved_by * update_if_moved_by,
            last_viewer_position: None,
            view_distance: 1,
        }
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

    let mut origin_from_x = (viewer_position.x / chunk_size as f32).floor() as i32 * chunk_size as i32;
    let mut origin_from_y = (viewer_position.y / chunk_size as f32).floor() as i32 * chunk_size as i32;
    let mut origin_from_z = (viewer_position.z / chunk_size as f32).floor() as i32 * chunk_size as i32;
    let mut origin_to_x = origin_from_x;
    let mut origin_to_y = origin_from_y;
    let mut origin_to_z = origin_from_z;

    let now = std::time::Instant::now();
    let mut chunks = 0;
    for r in 0..NUMBER_OF_RINGS {
        let scale = (2_i32).pow(r as u32);
        let chunk_size = Chunk::size() as i32 * scale;
        let view_extent = terrain.view_distance * if r == 0 { 2 * chunk_size } else { chunk_size };
        let from_x = origin_from_x - view_extent;
        let to_x = origin_to_x + view_extent;
        let from_y = origin_from_y - view_extent;
        let to_y = origin_to_y + view_extent;
        let from_z = origin_from_z - view_extent;
        let to_z = origin_to_z + view_extent;

        // println!("{} / {} / {}, X: {}..{}, Y: {}..{}, Z: {}..{}", r, chunk_size, view_extent, from_x, to_x, from_y, to_y, from_z, to_z,);

        for x in (from_x..to_x).step_by(chunk_size as usize) {
            let y = 0;
            for y in (from_y..to_y).step_by(chunk_size as usize) {
                for z in (from_z..to_z).step_by(chunk_size as usize) {
                    if r != 0 && x >= origin_from_x && x < origin_to_x
                        && y >= origin_from_y && y < origin_to_y
                        && z >= origin_from_z && z < origin_to_z
                    {
                        continue;
                    }

                    let index = Vec3i::new(x, y, z);

                    // println!(" {}: {:?}", r, index);

                    // get chunk
                    // TODO: ring number here is a LOD, and instead of creating a chunk here,
                    // we should get it or data for its creation from octree or some kind of 
                    // similar storage
                    let chunk = terrain.rings[r].entry(index)
                        .or_insert(Chunk::new(index, r));

                    if !chunk.hollow && chunk.changed {
                        chunk.polygonize(&mut assets, &mut world, &editor.density);
                        chunks += 1;
                    }

                    chunk.disabled = false;
                }
            }
        }
        
        origin_from_x = from_x;
        origin_from_y = from_y;
        origin_from_z = from_z;
        origin_to_x = to_x;
        origin_to_y = to_y;
        origin_to_z = to_z;
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
    println!("{} rings has been built in {}us, {} chunks", NUMBER_OF_RINGS, now.elapsed().as_micros(), chunks);
}


