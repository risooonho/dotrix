use std::collections::HashMap;
use noise::{ NoiseFn, Fbm, Perlin, Seedable, /* Perlin, Turbulence, Seedable, */ MultiFractal };
use rayon::prelude::*;

use dotrix::{
    assets::{ Id, Mesh },
    components::{ Model },
    ecs::{ Const, Mut, Context },
    math::{ Point3, Vec3 },
    renderer::{ Transform },
    services::{ Assets, Camera, World },
    terrain::{
        Density,
        MarchingCubes,
    }
};

use crate::editor::Editor;


/// Level of details
pub enum Lod {
    Zero,
    First,
    Second,
}

impl Lod {
    pub fn scale(self) -> usize {
        match self {
            Lod::Zero => 1,
            Lod::First => 2,
            Lod::Second => 4,
        }
    }
}

struct Chunk {
    position: Vec3,
    boundary: Vec3,
    size: f32,
    mesh: Option<Id<Mesh>>,
    lod: Lod,
    children: Vec<Chunk>,
    disabled: bool,
    changed: bool,
    index: ChunkIndex,
}

impl Chunk {
    pub fn new(index: ChunkIndex, size: f32) -> Self {
        let world_x = index.0 as f32 * size;
        let world_y = index.1 as f32 * size;
        let world_z = index.2 as f32 * size;
        Self {
            position: Vec3::new(world_x, world_y, world_z),
            boundary: Vec3::new(world_x + size, world_y + size, world_z + size),
            size,
            lod: Lod::Zero,
            children: Vec::new(),
            mesh: None,
            changed: true,
            disabled: false,
            index,
        }
    }

    /*
    pub fn resize_density_map(&mut self, new_size: usize) {
        self.density = Self::default_density(new_size);
        self.size = new_size;
        self.changed = true;
    }

    fn default_density(size: usize) -> Vec<Vec<Vec<f32>>> {
        let size = size + 1;
        let mut map = vec![vec![vec![-1.0; size]; size]; size];
        for x in 0..map.len() {
            for z in 0..map.len() {
                map[x][0][z] = 0.0;
            }
        }
        map
    }
    */

    fn update(
        &mut self,
        index: ChunkIndex,
        unused_entities: usize,
        assets: &mut Assets,
        world: &mut World
    ) {
        let mc = MarchingCubes {
            size: self.size as usize,
            height: self.size as usize,
            ..Default::default()
        };

        let (positions, _) = mc.polygonize(|x, y, z| {
            if y == 0 { 0.0 } else { -1.0 }
        });

        let len = positions.len();
        let uvs = Some(vec![[0.0, 0.0]; len]);

        if let Some(mesh_id) = self.mesh {
            let mesh = assets.get_mut(mesh_id).unwrap();
            mesh.positions = positions;
            mesh.uvs = uvs;
            mesh.normals.take();
            mesh.calculate();
            mesh.unload();

        } else {
            let mut mesh = Mesh {
                positions,
                uvs,
                ..Default::default()
            };
            mesh.calculate();

            let mesh = assets.add(mesh);

            let texture = assets.register("gray");
            assets.import("editor/assets/gray.png");

            let transform = Transform {
                translate: Vec3::new(
                    self.position.x,
                    if (self.position.x / self.size).abs() as usize % 2 == 0
                        && (self.position.z / self.size).abs() as usize % 2 != 0 {
                            5.0
                        } else {
                            0.0
                        },
                    self.position.z
                ), // self.position.clone(),
                ..Default::default()
            };

            println!("Spawn tile @ {:?}", transform.translate);

            if unused_entities == 0 {
                world.spawn(Some(
                    (
                        Model { mesh, texture, transform, ..Default::default() },
                        index,
                    )
                ));
            } else {
                let query = world.query::<(&mut Model, &mut ChunkIndex)>();
                for (model, chunk_index) in query {
                    if model.mesh.is_null() {
                        model.mesh = mesh;
                        model.transform = transform;
                        model.disabled = false;
                        chunk_index.0 = index.0;
                        chunk_index.1 = index.1;
                        chunk_index.2 = index.2;
                        break;
                    }
                }
            }

            self.mesh = Some(mesh);
        }
        self.changed = false;
    }

    /*
    fn has_intersection(&self, sphere: Vec3, radius: f32) -> bool {
        let mut dist_sq = radius * radius;
        let half_size = self.size as f32 / 2.0;
        let c1 = Vec3::new(self.world_x - half_size, 0.0, self.world_x - half_size);
        let c2 = Vec3::new(self.world_z - half_size, self.size as f32, self.world_z - half_size);

        if sphere.x < c1.x {
            dist_sq -= (sphere.x - c1.x).powf(2.0);
        } else if sphere.x > c2.x {
            dist_sq -= (sphere.x - c2.x).powf(2.0);
        }

        if sphere.y < c1.y {
            dist_sq -= (sphere.y - c1.y).powf(2.0);
        } else if sphere.y > c2.y {
            dist_sq -= (sphere.y - c2.y).powf(2.0);
        }

        if sphere.z < c1.z {
            dist_sq -= (sphere.z - c1.z).powf(2.0);
        } else if sphere.z > c2.z {
            dist_sq -= (sphere.z - c2.z).powf(2.0);
        }

        dist_sq > 0.0
    }
    */
}

#[derive(Eq, Hash, Debug, Clone, Copy)]
pub struct ChunkIndex(i32, i32, i32);

impl PartialEq for ChunkIndex {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 &&
        self.1 == other.1 &&
        self.2 == other.2
    }
}

pub struct Terrain {
    chunk_size: f32,
    view_distance: f32,
    chunks_in_view_distance: i32,
    chunks: HashMap<ChunkIndex, Chunk>,
    last_viewer_position: Option<Point3>,
    unused_entities: usize,
}

impl Terrain {
    pub fn new(view_distance: f32) -> Self {
        let chunk_size = 32.0;
        let chunks_in_view_distance = (view_distance / chunk_size).floor() as i32;
        Self {
            chunk_size,
            view_distance,
            chunks_in_view_distance,
            chunks: HashMap::new(),
            last_viewer_position: None,
            unused_entities: 0,
        }
    }
}

pub fn spawn(
    camera: Const<Camera>,
    mut terrain: Mut<Terrain>,
    mut assets: Mut<Assets>,
    mut editor: Mut<Editor>,
    mut world: Mut<World>,
) {
    const SQARE_DISTANCE_THRESHOLD: f32 = 16.0;

    let viewer_position = camera.target;
    let chunk_size = terrain.chunk_size; // TODO: add LOD support
    let chunks_in_view_distance = terrain.chunks_in_view_distance;

    // check if update is necessary
    if let Some(last_viewer_position) = terrain.last_viewer_position.as_ref() {
        let dx = viewer_position.x - last_viewer_position.x;
        let dy = viewer_position.y - last_viewer_position.y;
        let dz = viewer_position.z - last_viewer_position.z;
        if dx * dx + dy * dy + dz * dz < SQARE_DISTANCE_THRESHOLD { return; }
    }

    terrain.last_viewer_position = Some(viewer_position);

    // find what chunks has to be visible
    let chunk_x = (viewer_position.x / chunk_size).floor();
    let chunk_z = (viewer_position.z / chunk_size).floor();

    // disable all remaining chunks
    for chunk in terrain.chunks.values_mut() {
        chunk.disabled = true;
    }

    let mut unused_entities = terrain.unused_entities;
    println!("VP: {:?}, {}, {}", viewer_position, chunk_x, chunk_z);
    for x in -chunks_in_view_distance..chunks_in_view_distance {
        for z in -chunks_in_view_distance..chunks_in_view_distance {
            let chunk_x = (x + chunk_x as i32);
            let chunk_z = (z + chunk_z as i32);
            let chunk_index = ChunkIndex(chunk_x, 0, chunk_z);
            let chunk = terrain.chunks.entry(chunk_index)
                .or_insert(Chunk::new(chunk_index, chunk_size));

            // regenerate chunk if changed
            if chunk.changed {
                chunk.update(chunk_index, unused_entities, &mut assets, &mut world);
                if unused_entities > 0 {
                    unused_entities -= 1;
                }
            }

            chunk.disabled = false;
        }
    }

    // clean up far away chunks
    println!("chunks in list before cleanup: {}", terrain.chunks.len());
    let chunks_in_view_distance_2x = (2 * chunks_in_view_distance).pow(2);
    terrain.chunks.retain(|&index, _| {
        let dx = (chunk_x as i32 - index.0);
        let dz = (chunk_z as i32 - index.2);

        let res = dx * dx + dz * dz < chunks_in_view_distance_2x;
        if !res {
            println!("dx {}, dz {} < {}", dx, dz, chunks_in_view_distance_2x);
        }
        res
    });
    println!("chunks in list after cleanup: {}", terrain.chunks.len());

    // hide disabled chunks
    let query = world.query::<(&mut Model, &ChunkIndex)>();
    let mut unused_entities = 0;
    for (model, index) in query {
        model.disabled = terrain.chunks.get(index)
            .map(|chunk| chunk.disabled || model.mesh.is_null())
            .unwrap_or_else(|| {
                println!("Chunk not found: {:?}", index);
                let mesh = model.mesh;
                if !mesh.is_null() {
                    // assets.remove(mesh);
                }
                model.mesh = Id::new(0);
                unused_entities += 1;
                true
            });
    }
    terrain.unused_entities = unused_entities;
    println!("Unused: {}", terrain.unused_entities);
}

/*

pub fn draw(
    mut ctx: Context<Terrain>,
    mut assets: Mut<Assets>,
    mut editor: Mut<Editor>,
    mut world: Mut<World>,
) {
    let mut changed = resize(&mut ctx, &editor);

    editor.terrain_size_changed = false;

    if editor.brush_add || editor.brush_sub {
        brush(&mut ctx, &editor);
        editor.brush_add = false;
        editor.brush_sub = false;
        editor.brush_changed = false;
        changed = true;
    }

    if changed {
        generate(&mut ctx, &mut assets, &mut editor, &mut world);
    }
}

fn resize(terrain: &mut Terrain, editor: &Editor) -> bool {
    let terrain_size = editor.terrain_size;
    let mut changed = false;

    // resize chunk
    if editor.terrain_size_changed && terrain_size != terrain.size {
        for chunk in terrain.chunks.iter_mut() {
            chunk.resize_density_map(terrain_size);
        }
        changed = true;
        terrain.size = editor.terrain_size;
    }

    changed
}

struct Position {
    world_x: f32,
    world_z: f32,
}

pub fn generate(
    terrain: &mut Terrain,
    assets: &mut Assets,
    editor: &mut Editor,
    world: &mut World,
) {

    // Push chunks added from the UI
    if terrain.chunks.len() == 0 {
        let size = 8;
        let half_size = size as f32 / 2.0;
        terrain.chunks.push(Chunk::new(terrain.size, 0.0, 0.0, 0.0));
        /* for x in 0..size {
            let xf = (x as f32 - half_size) * terrain.size as f32;
            for z in 0..size {
                let zf = (z as f32 - half_size) * terrain.size as f32;
                println!("Add chunk ({},{})", xf, zf);
                terrain.chunks.push(Chunk::new(terrain.size, xf, 0.0, zf));
            }
        }*/
    }

    for chunk in terrain.chunks.iter_mut() {
        if !chunk.changed { continue; }

        let mc = MarchingCubes {
            size: terrain.size,
            height: chunk.size,
            ..Default::default()
        };

        let (positions, _) = mc.polygonize_map(&chunk.density);
        let len = positions.len();
        let uvs = Some(vec![[0.0, 0.0]; len]);

        if let Some(mesh_id) = chunk.mesh {
            let mesh = assets.get_mut(mesh_id).unwrap();
            mesh.positions = positions;
            mesh.uvs = uvs;
            mesh.normals.take();
            mesh.calculate();
            mesh.unload();

        } else {
            let mut mesh = Mesh {
                positions,
                uvs,
                ..Default::default()
            };
            mesh.calculate();

            let mesh = assets.store(mesh, "Terrain");

            let texture = assets.register("gray");
            assets.import("editor/assets/gray.png");

            let transform = Transform {
                translate: chunk.position.clone(),
                ..Default::default()
            };

            world.spawn(Some(
                (
                    Model { mesh, texture, transform, ..Default::default() },
                    /*Position {
                        world_x: chunk.position.x,
                        world_y: chunk.position.y,
                        world_z: chunk.position.z,
                    }*/
                )
            ));

            chunk.mesh = Some(mesh);
        }
        chunk.changed = false;
    }

    /*
    let query = world.query::<(&mut Model, &Position)>();
    for (model, position) in query {
        let shift_x = position.world_x - terrain.size as f32 / 2.0;
        let shift_z = position.world_z - terrain.size as f32 / 2.0;
        model.transform = Transform {
            translate: Vec3::new(shift_x, 0.0, shift_z),
            ..Default::default()
        };
    }
    */
}

fn brush(terrain: &mut Terrain, editor: &Editor) {

    let size = (2.0 * editor.brush_radius).ceil() as usize + 1;
    let radius = editor.brush_radius;
    let brush = (0..size).into_par_iter().map(|x| {
        let xf = x as f32 - radius;
        (0..size).into_par_iter().map(|y| {
            let yf = y as f32 - radius;
            (0..size).into_par_iter().map(|z| {
                let zf = z as f32 - radius;
                radius - (xf.powf(2.0) + yf.powf(2.0) + zf.powf(2.0)).sqrt()
            }).collect::<Vec<_>>()
        }).collect::<Vec<_>>()
    }).collect::<Vec<_>>();

    let brush_radius = editor.brush_radius;
    let brush_x0 = (editor.brush_x - brush_radius).floor() as i32;
    let brush_x1 = (editor.brush_x + brush_radius).ceil() as i32;
    let brush_y0 = (editor.brush_y - brush_radius).floor() as i32;
    let brush_y1 = (editor.brush_y + brush_radius).ceil() as i32;
    let brush_z0 = (editor.brush_z - brush_radius).floor() as i32;
    let brush_z1 = (editor.brush_z + brush_radius).ceil() as i32;

    for chunk in terrain.chunks.iter_mut() {
        let chunk_x0: f32 = chunk.position.x;
        let chunk_x1: f32 = chunk.boundary.x;
        let chunk_y0: f32 = chunk.position.y;
        let chunk_y1: f32 = chunk.boundary.y;
        let chunk_z0: f32 = chunk.position.z;
        let chunk_z1: f32 = chunk.boundary.z;

        let x0 = brush_x0 - chunk_x0.floor() as i32;
        let x1 = x0 + brush.len() as i32;
        let y0 = brush_y0 - chunk_y0.floor() as i32;
        let y1 = y0 + brush.len() as i32;
        let z0 = brush_z0 - chunk_z0.floor() as i32;
        let z1 = z0 + brush.len() as i32;

        let brush_x0 = if x0 < 0 { x0.abs() } else { 0 };
        let brush_y0 = if y0 < 0 { y0.abs() } else { 0 };
        let brush_z0 = if z0 < 0 { z0.abs() } else { 0 };

        let x0 = (if x0 < 0 { 0 } else { x0 }) as usize;
        let y0 = (if y0 < 0 { 0 } else { y0 }) as usize;
        let z0 = (if z0 < 0 { 0 } else { z0 }) as usize;

        let x1 = (if x1 < terrain.size as i32 { x1 } else { terrain.size as i32 }) as usize;
        let y1 = (if y1 < terrain.size as i32 { y1 } else { terrain.size as i32 }) as usize;
        let z1 = (if z1 < terrain.size as i32 { z1 } else { terrain.size as i32 }) as usize;

        println!("x: {}..{}, y: {}..{}, z: {}..{}", x0, x1, y0, y1, z0, z1 );
        let mut brush_x = brush_x0;
        for x in x0..x1 {
            let mut brush_y = brush_y0;
            for y in y0..y1 {
                let mut brush_z = brush_z0;
                for z in z0..z1 {
                    println!("brush[{}][{}][{}] -> {}", brush_y, brush_x, brush_z, brush.len());
                    let value = brush[brush_x as usize][brush_y as usize][brush_z as usize];
                    let old_value = chunk.density[x][y][z];
                    if editor.brush_add {
                        if value > old_value {
                            chunk.density[x][y][z] = value;
                        }
                    } else {
                        let value = -value;
                        if value < old_value {
                            chunk.density[x][y][z] = value;
                        }
                    }
                    brush_z += 1;
                }
                brush_y += 1;
            }
            brush_x += 1;
        }
        chunk.changed = true;
    }
}
*/


/*
 *
 *
    let mc = MarchingCubes {
        size: editor.terrain_chunk_size,
        ..Default::default()
    };
    let noise = Fbm::new();
    let noise = noise.set_octaves(editor.noise_octaves);
    let noise = noise.set_frequency(editor.noise_frequency);
    let noise = noise.set_lacunarity(editor.noise_lacunarity);
    let noise = noise.set_persistence(editor.noise_persistence);

let chunks = (x0..x_size).into_par_iter().map(|world_x|
            (z0..z_size).into_par_iter().map(|world_z|
                Chunk {
                    world_x,
                    world_z,
                    mesh: None,
                    density: mc.get_density_map(|x, y, z| {
                    // let island_size = (editor.terrain_chunk_size) as f32;
                    // let distance_x = (x as f32 - island_size * 0.5).abs();
                    // let distance_z = (z as f32 - island_size * 0.5).abs();
                    // let distance = (distance_x * distance_x + distance_z * distance_z).sqrt(); // circle mask

                    // let distance = f32::max(distance_x, distance_z); // square mask

                    // let max_width = island_size * 0.5 - 4.0;
                    // let delta = distance / max_width;
                    // let gradient = delta * delta;

                    // let island_noise = f32::max(0.0, 1.0 - gradient) as f64;
                        let x = (x + world_x) as f64;
                        let z = (z + world_z) as f64;
                        let y = y as f64
                        let scale = editor.noise_scale;
                        let value = editor.noise_amplitude.exp() * /* island_noise */ (noise.get([
                            (x / scale) + 0.5,
                        //     (y as f64 / div_h) + 0.5,
                            (z / scale) + 0.5,
                        ]) - y);
                        value as f32
                    // (if value < -(y as f64) { -(y as f64) } else { value }) as f32
                    }),
                }
            ).collect::<Vec<_>>()
        ).collect::<Vec<_>>();
        */
