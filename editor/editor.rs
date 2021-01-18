use dotrix::{
    assets::{ Mesh },
    components::{ Light, Model },
    ecs::{ Mut, Const },
    egui::{
        Egui,
        CollapsingHeader,
        Label,
        TopPanel,
        Separator,
        Slider,
        Window
    },
    math::{Vec3i, Vec3},
    renderer::{ Transform },
    input::{ Button, State as InputState, Mapper, KeyCode },
    services::{ Assets, Camera, Frame, Input, World, Renderer },
    terrain::{ Density, HeightMap },
};

use crate::controls::Action;

use noise::{ NoiseFn, Fbm, Perlin, Seedable, /* Perlin, Turbulence, Seedable, */ MultiFractal };
use std::f32::consts::PI;

pub struct Editor {
    pub density: Density,
    pub heightmap: Option<HeightMap<u8>>,
    pub heightmap_size_x: usize,
    pub heightmap_size_z: usize,
    pub sea_level: u8,
    pub terrain_size: usize,
    pub terrain_size_changed: bool,
    pub noise_octaves: usize,
    pub noise_frequency: f64,
    pub noise_lacunarity: f64,
    pub noise_persistence: f64,
    pub noise_scale: f64,
    pub noise_amplitude: f64,
    pub show_toolbox: bool,
    pub brush_x: f32,
    pub brush_y: f32,
    pub brush_z: f32,
    pub brush_radius: f32,
    pub brush_add: bool,
    pub brush_sub: bool,
    pub brush_changed: bool,
    pub apply_noise: bool,
}

impl Editor {
    pub fn new() -> Self {
        let mut density = Density::new(2048, 64, 2048, Vec3i::new(1024, 0, 1024));

        let noise_octaves = 3;
        let noise_frequency = 0.5;
        let noise_lacunarity = 1.0;
        let noise_persistence = 0.5;
        let noise_scale = 4.0;
        let noise_amplitude: f64 = 2.0;

        let noise = Fbm::new();
        let noise = noise.set_octaves(noise_octaves);
        let noise = noise.set_frequency(noise_frequency);
        let noise = noise.set_lacunarity(noise_lacunarity);
        let noise = noise.set_persistence(noise_persistence);

        density.set(|x, y, z| {
            let xf = x as f64 / noise_scale + 0.5;
            let zf = z as f64 / noise_scale + 0.5;
            let yf = y as f64;
            let n = 4.0 * (noise.get([xf, zf]) + 1.0);
            let value = /* noise_amplitude.exp() **/ n - yf;
            value as f32
        });

        Self {
            density,
            heightmap: None,
            heightmap_size_x: 2048,
            heightmap_size_z: 2048,
            sea_level: 0,
            terrain_size: 64,
            terrain_size_changed: true,
            noise_octaves,
            noise_frequency,
            noise_lacunarity,
            noise_persistence,
            noise_scale,
            noise_amplitude,
            show_toolbox: true,
            brush_x: 0.0,
            brush_y: 10.0,
            brush_z: 0.0,
            brush_radius: 5.0,
            brush_add: false,
            brush_sub: false,
            brush_changed: false,
            apply_noise: true,
        }
    }
}

pub fn ui(mut editor: Mut<Editor>, renderer: Mut<Renderer>) {
    let egui = renderer.overlay_provider::<Egui>()
        .expect("Renderer does not contain an Overlay instance");

    TopPanel::top("side_panel").show(&egui.ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button("🗋").clicked { println!("New"); }
            if ui.button("🖴").clicked { println!("Save"); }
            if ui.button("🗁").clicked { println!("Open"); }
            if ui.button("🛠").clicked { editor.show_toolbox = !editor.show_toolbox; }
            if ui.button("ℹ").clicked { println!("Info"); }
        });
    });

    let mut show_toolbox = editor.show_toolbox;

    Window::new("Toolbox").open(&mut show_toolbox).show(&egui.ctx, |ui| {
        CollapsingHeader::new("Height Map").default_open(true).show(ui, |ui| {
            ui.add(Label::new("Size by X axis"));
            ui.add(Slider::usize(&mut editor.heightmap_size_x, 256..=8192).text("meters"));
            ui.add(Label::new("Size by Z axis"));
            ui.add(Slider::usize(&mut editor.heightmap_size_z, 256..=8192).text("meters"));
            ui.horizontal(|ui| {
                let apply_noise = editor.apply_noise;
                if ui.button("Update").clicked {
                    /* editor.heightmap = Some(if apply_noise {

                    } else {
                    }); */
                }
                ui.checkbox(&mut editor.apply_noise, "Apply noise");
            });
        });

        CollapsingHeader::new("Terrain")
            .default_open(true)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.add(Label::new("Size:"));
                        if ui.button("Resize").clicked { editor.terrain_size_changed = true; }
                    });
                    ui.add(Slider::usize(&mut editor.terrain_size, 8..=256).text("Meters"));

                    ui.add(Separator::new());

                    ui.horizontal(|ui| {
                        ui.add(Label::new("Brush:"));
                        if ui.button("Add").clicked { editor.brush_add = true; }
                        if ui.button("Sub").clicked { editor.brush_sub = true; }
                    });

                    ui.add(Slider::f32(&mut editor.brush_x, -64.0..=64.0).text("X"));
                    ui.add(Slider::f32(&mut editor.brush_y, -64.0..=64.0).text("Y"));
                    ui.add(Slider::f32(&mut editor.brush_z, -64.0..=64.0).text("Z"));
                    ui.add(Slider::f32(&mut editor.brush_radius, 1.0..=16.0).text("Radius"));

                    ui.add(Separator::new());

                    ui.add(Label::new("Noise:"));
                    ui.add(Slider::f64(&mut editor.noise_scale, 1.0..=256.0).text("Scale"));
                    ui.add(Slider::f64(&mut editor.noise_amplitude, 1.0..=256.0).text("Amplitude"));
                    ui.add(Slider::usize(&mut editor.noise_octaves, 1..=10).text("Octaves"));
                    ui.add(Slider::f64(&mut editor.noise_frequency, 0.1..=10.0).text("Frequency"));
                    ui.add(Slider::f64(&mut editor.noise_lacunarity, 0.1..=10.0).text("Lacunarity"));
                    ui.add(Slider::f64(&mut editor.noise_persistence, 0.1..=10.0).text("Persistence"));
                });
            });
                // ui.label(format!("Hello '{}', age {}", name, age));
    });

    editor.show_toolbox = show_toolbox;
}

const ROTATE_SPEED: f32 = PI / 10.0;
const ZOOM_SPEED: f32 = 10.0;
const MOVE_SPEED: f32 = 64.0;

pub struct Cursor(Vec3);

pub fn startup(
    mut assets: Mut<Assets>,
    mut input: Mut<Input>,
    mut renderer: Mut<Renderer>,
    mut world: Mut<World>
) {

    assets.import("editor/assets/terrain.png");
    renderer.add_overlay(Box::new(Egui::default()));

    world.spawn(Some((Light::white([0.0, 500.0, 0.0]),)));

    input.mapper_mut::<Mapper<Action>>()
        .set(vec![
            (Action::Move, Button::Key(KeyCode::W)),
        ]);

    let cursor = assets.store(Mesh::cube());
    assets.import("assets/green.png");
    let texture = assets.register("green");
    let transform = Transform {
        translate: Vec3::new(0.0, 0.5, 0.0),
        scale: Vec3::new(0.05, 0.05, 0.05),
        ..Default::default()
    };

    world.spawn(
        Some((
            Model { mesh: cursor, texture, transform, ..Default::default() },
            Cursor(Vec3::new(0.0, 0.0, 0.0))
        ))
    );
}

pub fn camera_control(
    mut camera: Mut<Camera>,
    input: Const<Input>,
    frame: Const<Frame>,
    mut world: Mut<World>,
) {
    let time_delta = frame.delta().as_secs_f32();
    let mouse_delta = input.mouse_delta();
    let mouse_scroll = input.mouse_scroll();

    let distance = camera.distance - ZOOM_SPEED * mouse_scroll * time_delta;
    camera.distance = if distance > -1.0 { distance } else { -1.0 };

    if input.button_state(Button::MouseRight) == Some(InputState::Hold) {
        camera.y_angle += mouse_delta.x * ROTATE_SPEED * time_delta;

        let xz_angle = camera.xz_angle + mouse_delta.y * ROTATE_SPEED * time_delta;
        let half_pi = PI / 2.0;

        camera.xz_angle = if xz_angle >= half_pi {
            half_pi - 0.01
        } else if xz_angle <= -half_pi {
            -half_pi + 0.01
        } else {
            xz_angle
        };
    }

    // move
    let distance = if input.is_action_hold(Action::Move) {
        MOVE_SPEED * frame.delta().as_secs_f32()
    } else {
        0.0
    };

    if distance > 0.00001 {
        let y_angle = camera.y_angle;

        let dx = distance * y_angle.cos();
        let dz = distance * y_angle.sin();

        camera.target.x -= dx;
        camera.target.z -= dz;

        let query = world.query::<(&mut Model, &Cursor)>();
        for (model, _) in query {
            model.transform.translate.x = camera.target.x;
            model.transform.translate.z = camera.target.z;
        }
    }

    camera.set_view();
}
