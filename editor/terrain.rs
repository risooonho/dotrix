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
    math::{ Point3, Vec3, Vec3i, MetricSpace },
    renderer::{ Transform },
    services::{ Assets, Camera, World },
    terrain::{
        Density,
        MarchingCubes,
    }
};

use crate::{
    editor::Editor,
    octree::{Octree, Node as OctreeNode},
    lod::Lod,
};
use chunk::*;

const NUMBER_OF_RINGS: usize = 2;
const IS_ODD_RING: bool = (NUMBER_OF_RINGS % 2 != 0);

type VoxelMap = [[[f32; 17]; 17]; 17];

pub struct Terrain {
    pub last_viewer_position: Option<Point3>,
    pub update_if_moved_by: f32,
    pub view_distance: i32,
    pub octree: Octree<VoxelMap>,
    pub populated: bool,
    pub last_lod: usize,
}

impl Terrain {
    pub fn new() -> Self {
        let update_if_moved_by = Chunk::size() as f32 * 0.5;
        Self {
            update_if_moved_by: update_if_moved_by * update_if_moved_by,
            last_viewer_position: None,
            view_distance: 1,
            octree: Octree::new(2048),
            populated: false,
            last_lod: 1,
        }
    }

    pub fn populate(&mut self, noise: &Fbm) {
        let node = Vec3i::new(0, 0, 0);
        self.populate_node(noise, node, 0);
        self.populated = true;
    }

    fn populate_node(&mut self, noise: &Fbm, node: Vec3i, depth: usize) {
        let scale = Lod(depth).scale();
        let offset = self.octree.size() as i32 / scale / 2;
        let step = offset / (16 / 2);

        // density.value already applies theoffset
        let mut payload = [[[0.0; 17]; 17]; 17];
        let noise_scale = 4.0;
        for x in 0..17 {
            let xf = (node.x - offset + x * step) as f64 / noise_scale + 0.5;
            for y in 0..17 {
                let yf = (node.y - offset + y * step) as f64 /* noise_scale + 0.5 */;
                for z in 0..17 {
                    let zf = (node.z - offset + z * step) as f64 / noise_scale + 0.5;
                    payload[x as usize][y as usize][z as usize] = 
                        (4.0 * (noise.get([xf, zf]) + 1.0) - yf) as f32;
                }
            }
        }
        if (depth == 1) {
            // println!("Density  {:?} (scale: {}, offset: {}, step: {}):\n\n {:?}\n", node, scale, offset, step, payload);
        }
        // println!("Set density for {:?}: {}", node, depth);
        self.octree.store(node.clone(), payload);

        if depth < 4 {
            let children = OctreeNode::<i32>::children(&node, offset / 2);
            // println!("set_children: {:?}", children);
            for child in children.iter() {
                self.populate_node(noise, *child, depth + 1);

            }
        }
    }

    fn chunks(
        &self,
        viewer_position: &Point3,
        cursor: &Vec3i,
        chunk_view_distance2: i32,
        chunk_offset: i32,
        chunk_lod: usize,
        max_lod: usize,
    ) -> Vec<Vec3i> {

        let distance2 = Point3::new(
            cursor.x as f32,
            cursor.y as f32,
            cursor.z as f32
        ).distance2(*viewer_position);
        println!("{:?}({:?}) -> {:?} < {:?}", cursor, chunk_offset, distance2, chunk_view_distance2);

        if chunk_lod < max_lod && Point3::new(
            cursor.x as f32,
            cursor.y as f32,
            cursor.z as f32
        ).distance2(*viewer_position) < chunk_view_distance2 as f32 {
            if let Some(children) = self.octree.children(&cursor) {
                let child_offset = chunk_offset / 4;
                let child_view_distance2 = 6 * chunk_offset * chunk_offset;
                let child_lod = chunk_lod + 1;
                println!("Going for children for {:?} (lod {} / {})", cursor, chunk_lod, max_lod);
                println!(" . {:?}", children);

                let res = children.iter()
                    .map(|child| self.chunks(
                        viewer_position,
                        child,
                        child_view_distance2,
                        child_offset,
                        child_lod,
                        max_lod
                    )).flatten().collect::<Vec<_>>();
                // println!(" . {:?}", res);

                return res;
            }
        }

        vec![cursor.clone()]
    }
}

impl Default for Terrain {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
pub struct Spawner {
    pub chunks: HashMap<Vec3i, Chunk>
}

pub fn spawn(
    mut ctx: Context<Spawner>,
    camera: Const<Camera>,
    mut terrain: Mut<Terrain>,
    mut assets: Mut<Assets>,
    mut editor: Mut<Editor>,
    mut world: Mut<World>,
) {
    if !terrain.populated {
        terrain.populate(&editor.noise());
    }
    let viewer_position = camera.target;

    // check if update is necessary
    if let Some(last_viewer_position) = terrain.last_viewer_position.as_ref() {
        let dx = viewer_position.x - last_viewer_position.x;
        let dy = viewer_position.y - last_viewer_position.y;
        let dz = viewer_position.z - last_viewer_position.z;
        if dx * dx + dy * dy + dz * dz < terrain.update_if_moved_by
            && terrain.last_lod == editor.lod {
            return;
        }
    }
    terrain.last_lod = editor.lod;

    terrain.last_viewer_position = Some(viewer_position);

    // disable all chunks
    for chunk in ctx.chunks.values_mut() {
        chunk.disabled = true;
    }

    let terrain_size = terrain.octree.size() as i32;
    let lod_by_view_distance = Lod(editor.lod); // TODO: get it from camera distance
    let scale = lod_by_view_distance.scale();
    let chunk_size = terrain_size / scale;
    let chunk_offset = chunk_size / 2;
    let high_lod = editor.lod + 2;

    let mut origin_x = (viewer_position.x / chunk_size as f32).floor() as i32 * chunk_size + chunk_offset;
    let mut origin_y = (viewer_position.y / chunk_size as f32).floor() as i32 * chunk_size + chunk_offset;
    let mut origin_z = (viewer_position.z / chunk_size as f32).floor() as i32 * chunk_size + chunk_offset;

    let chunk_view_distance2 = 6 * chunk_offset * chunk_offset;

    let limit = match lod_by_view_distance.0 {
        0 => 1,
        1 => 1,
        _ => 2
    };

    let chunks = (-limit..limit).into_par_iter().map(|xi| {
        let x = origin_x + chunk_size * xi;
        (-limit..limit).into_par_iter().map(|yi| {
            let y = origin_y + chunk_size * yi;
            (-limit..limit).into_par_iter().map(|zi| {
                let z = origin_z + chunk_size * zi;
                terrain.chunks(
                    &viewer_position,
                    &Vec3i::new(x, y, z),
                    chunk_view_distance2,
                    chunk_offset,
                    editor.lod,
                    high_lod
                )
            }).flatten().collect::<Vec<_>>()
        }).flatten().collect::<Vec<_>>()
    }).flatten().collect::<Vec<_>>();

    println!("chunks to show for {:?}", viewer_position);

    for index in chunks {
        if let Some((index, lod, voxel_map)) = terrain.octree.find(&index) {
            println!(" > {:?}: {}", index, lod);
            let chunk = ctx.chunks.entry(index)
                .or_insert(Chunk::new(index, lod, terrain.octree.size() / Lod(lod).scale() as usize));

            if !chunk.hollow && chunk.changed {

                // println!("  {:?} (polygonize)", index);
                if index.x == 512 && index.y == 512 && index.z == 512 {
                    // println!("    {:?}", voxel_map);
                }
                chunk.polygonize(&mut assets, &mut world, voxel_map);
                /* if !chunk.hollow {
                    println!("  {:?} (polygonize)", index);
                } else {
                    println!("  {:?} (hollow)", index);
                } */
            } else {
                // println!("  {:?} (hollow)", index);
            }

            chunk.disabled = false;
        } else {
            println!("  {:?} (not found in octree)", index);
        }
    }

    let query = world.query::<(&mut Model, &Tile)>();
    for (model, tile) in query {
        model.disabled = ctx.chunks.get(&tile.position)
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
    // println!("{} rings has been built in {}us, {} chunks", NUMBER_OF_RINGS, now.elapsed().as_micros(), chunks);
    println!("target {:?}", viewer_position);
}


