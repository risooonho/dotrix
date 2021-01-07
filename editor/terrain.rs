use dotrix::{
    assets::{ Id, Mesh },
    components::{ Model },
    ecs::{ Mut, Context },
    math::{ Vec3 },
    renderer::{ Transform },
    services::{ Assets, World },
    terrain::{
        MarchingCubes,
    }
};

use rayon::prelude::*;

use crate::editor::Editor;
use noise::{ NoiseFn, Fbm, Perlin, Seedable, /* Perlin, Turbulence, Seedable, */ MultiFractal };

struct Tile {
    /// tile X offset
    world_x: f32,
    /// tile Z offset
    world_z: f32,
    /// tile height: bigger if there are mountains
    height: usize,
    /// mesh id, if tile is in the world already
    mesh: Option<Id<Mesh>>,
    /// density values map
    density: Vec<Vec<Vec<f32>>>,
    /// changes flag
    changed: bool,
}

impl Tile {
    pub fn new(size: usize, world_x: f32, world_z: f32) -> Self {
        Self {
            world_x,
            world_z,
            height: size,
            mesh: None,
            density: Self::default_density(size),
            changed: true,
        }
    }

    pub fn resize_density_map(&mut self, new_size: usize) {
        self.density = Self::default_density(new_size);
        self.height = 1;
        self.changed = true;
    }

    fn default_density(size: usize) -> Vec<Vec<Vec<f32>>> {
        let size = size + 1;
        let mut map = vec![vec![vec![-1.0; size]; size]; size];
        for x in 0..size {
            for z in 0..size {
                map[0][x][z] = 0.0;
            }
        }
        map
    }
}

#[derive(Default)]
pub struct Terrain {
    tiles: Vec<Tile>,
    size: usize,
}

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
        for tile in terrain.tiles.iter_mut() {
            tile.resize_density_map(terrain_size);
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

    // Push tiles added from the UI
    if terrain.tiles.len() == 0 {
        terrain.tiles.push(Tile::new(terrain.size, 0.0, 0.0));
    }

    for tile in terrain.tiles.iter_mut() {
        if !tile.changed { continue; }

        let mc = MarchingCubes {
            size: terrain.size,
            height: tile.height,
            ..Default::default()
        };

        let (positions, _) = mc.polygonize_map(&tile.density);
        let len = positions.len();
        let uvs = Some(vec![[0.0, 0.0]; len]);

        if let Some(mesh_id) = tile.mesh {
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

            world.spawn(Some(
                (
                    Model { mesh, texture, ..Default::default() },
                    Position { world_x: tile.world_x, world_z: tile.world_z }
                )
            ));

            tile.mesh = Some(mesh);
        }
        tile.changed = false;
    }

    let query = world.query::<(&mut Model, &Position)>();
    for (model, position) in query {
        let shift_x = position.world_x - terrain.size as f32 / 2.0;
        let shift_z = position.world_z - terrain.size as f32 / 2.0;
        model.transform = Transform {
            translate: Vec3::new(shift_x, 0.0, shift_z),
            ..Default::default()
        };
    }
}

fn brush(terrain: &mut Terrain, editor: &Editor) {
    for tile in terrain.tiles.iter_mut() {
        tile.density[0][31][31] = 1.0;
        tile.density[0][31][32] = 1.0;
        tile.density[0][32][31] = 1.0;
        tile.density[0][32][32] = 1.0;
        tile.density[1][31][31] = -0.5;
        tile.density[1][31][32] = 0.0;
        tile.density[1][32][31] = 0.0;
        tile.density[1][32][32] = -0.5;
        tile.changed = true;
    }
}

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

let tiles = (x0..x_size).into_par_iter().map(|world_x|
            (z0..z_size).into_par_iter().map(|world_z|
                Tile {
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
