use dotrix::{
    components::{ Light },
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
    input::{ Button, State as InputState },
    services::{ Camera, Frame, Input, World, Renderer },
};

use std::f32::consts::PI;

pub struct Editor {
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
}

impl Editor {
    pub fn new() -> Self {
        Self {
            terrain_size: 64,
            terrain_size_changed: true,
            noise_octaves: 3,
            noise_frequency: 1.0,
            noise_lacunarity: 2.0,
            noise_persistence: 0.5,
            noise_scale: 32.0,
            noise_amplitude: 2.0,
            show_toolbox: true,
            brush_x: 0.0,
            brush_y: 0.0,
            brush_z: 0.0,
            brush_radius: 4.0,
            brush_add: false,
            brush_sub: false,
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

pub fn startup(mut renderer: Mut<Renderer>, mut world: Mut<World>) {
    renderer.add_overlay(Box::new(Egui::default()));

    world.spawn(Some((Light::white([0.0, 500.0, 0.0]),)));
}

pub fn camera_control(mut camera: Mut<Camera>, input: Const<Input>, frame: Const<Frame>) {
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

    camera.set_view();
}
