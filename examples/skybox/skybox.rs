use dotrix::{
    Dotrix,
    assets::Texture,
    components::SkyBox,
    ecs::{ Mut, RunLevel, System },
    services::{ Assets, Camera, World },
    systems::{ camera_control, world_renderer }
};

fn main() {

    Dotrix::application("SkyBox Example")
        .with_system(System::from(world_renderer).with(RunLevel::Render))
        .with_system(System::from(startup).with(RunLevel::Startup))
        .with_system(System::from(camera_control))
        .with_service(Assets::new())
        .with_service(Camera::new())
        .with_service(World::new())
        .run();

}

fn startup(mut world: Mut<World>, mut assets: Mut<Assets>) {
    let primary_texture = [
        assets.register::<Texture>("skybox_right"),
        assets.register::<Texture>("skybox_left"),
        assets.register::<Texture>("skybox_top"),
        assets.register::<Texture>("skybox_bottom"),
        assets.register::<Texture>("skybox_back"),
        assets.register::<Texture>("skybox_front"),
    ];

    // The skybox cubemap was downloaded from https://opengameart.org/content/elyvisions-skyboxes
    // These files were licensed as CC-BY 3.0 Unported on 2012/11/7
    assets.import("assets/skybox/skybox_right.png");
    assets.import("assets/skybox/skybox_left.png");
    assets.import("assets/skybox/skybox_top.png");
    assets.import("assets/skybox/skybox_bottom.png");
    assets.import("assets/skybox/skybox_front.png");
    assets.import("assets/skybox/skybox_back.png");

    world.spawn(vec![
        (SkyBox { primary_texture, ..Default::default() },),
    ]);
}

