use constants::{CAMERA_FAR, CAMERA_NEAR, CAMERA_ZOOM, RESOLUTION_FACTOR};
use jandering_engine::{
    engine::Engine,
    object::{Instance, Object, Vertex},
    render_pass::RenderPass,
    renderer::Janderer,
    shader::ShaderDescriptor,
    texture::{
        sampler::SamplerDescriptor,
        texture_usage::{self},
        TextureDescriptor, TextureFormat,
    },
    types::{UVec2, Vec2, Vec3},
    utils::{
        free_camera::{CameraController, FreeCameraController, MatrixCamera},
        texture::{TextureSamplerBindGroup, UnfilteredTextureSamplerBindGroup},
    },
    window::{InputState, Key, WindowConfig, WindowEvent, WindowManagerTrait, WindowTrait},
};

use ocean::{Ocean, WaveDataBindGroup};
use popr::PoprConfig;
use ship::Ship;

mod constants;
mod ocean;
mod popr;
mod ship;

fn main() {
    let mut engine = pollster::block_on(Engine::default());

    let mut window = engine.spawn_window(
        WindowConfig::default()
            .with_cursor(true)
            .with_resolution(300, 300)
            .with_auto_resolution()
            .with_decorations(false)
            .with_transparency(true)
            .with_title("beast"),
    );

    let renderer = &mut engine.renderer;

    let depth_texture = renderer.create_texture(TextureDescriptor {
        size: UVec2::splat(512),
        format: TextureFormat::Depth32F,
        usage: texture_usage::GENERIC,
        ..Default::default()
    });

    let mut target_texture_bind_group = {
        let target_texture = renderer.create_texture(TextureDescriptor {
            format: TextureFormat::Bgra8U,
            usage: texture_usage::GENERIC,
            ..Default::default()
        });
        let target_texture_sampler = renderer.create_sampler(SamplerDescriptor {
            filter: jandering_engine::texture::sampler::SamplerFilterMode::Nearest,
            ..Default::default()
        });
        TextureSamplerBindGroup::new(renderer, target_texture, target_texture_sampler)
    };

    let mut camera_controller: Option<Box<dyn CameraController>> =
        Some(Box::new(FreeCameraController::default()));
    let mut camera = MatrixCamera::new(renderer);
    camera.set_position(Vec3::new(100.0, 55.0, 100.0));
    camera.set_direction(-camera.position().normalize());

    let shader = renderer.create_shader(ShaderDescriptor {
        name: "main_shader",
        source: jandering_engine::shader::ShaderSource::File(
            jandering_engine::utils::FilePath::FileName("shader.wgsl"),
        ),
        descriptors: vec![Vertex::desc(), Instance::desc()],
        bind_group_layout_descriptors: vec![MatrixCamera::get_layout_descriptor()],
        backface_culling: false,
        depth: true,
        ..Default::default()
    });

    let water_shader = renderer.create_shader(ShaderDescriptor {
        name: "water_shader",
        source: jandering_engine::shader::ShaderSource::File(
            jandering_engine::utils::FilePath::FileName("water_shader.wgsl"),
        ),
        descriptors: vec![Vertex::desc(), Instance::desc()],
        bind_group_layout_descriptors: vec![
            MatrixCamera::get_layout_descriptor(),
            UnfilteredTextureSamplerBindGroup::get_layout_descriptor(),
            WaveDataBindGroup::get_layout_descriptor(),
        ],
        backface_culling: true,
        depth: true,
        ..Default::default()
    });

    let popr_shader = renderer.create_shader(ShaderDescriptor {
        source: jandering_engine::shader::ShaderSource::File(
            jandering_engine::utils::FilePath::FileName("popr_shader.wgsl"),
        ),
        descriptors: vec![Vertex::desc(), Instance::desc()],
        bind_group_layout_descriptors: vec![
            TextureSamplerBindGroup::get_layout_descriptor(),
            PoprConfig::get_layout_descriptor(),
        ],
        depth: false,
        backface_culling: false,
        ..Default::default()
    });

    let cube_shader = renderer.create_shader(ShaderDescriptor {
        name: "cube_shader",
        source: jandering_engine::shader::ShaderSource::File(
            jandering_engine::utils::FilePath::FileName("shader.wgsl"),
        ),
        descriptors: vec![Vertex::desc(), Instance::desc()],
        bind_group_layout_descriptors: vec![MatrixCamera::get_layout_descriptor()],
        fs_entry: "fs_cube",
        backface_culling: true,
        depth: true,
        ..Default::default()
    });

    // let mut debug_cube = Object::from_obj(
    //     include_str!("cube.obj"),
    //     renderer,
    //     (-50..=50)
    //         .flat_map(|z| {
    //             (-50..=50)
    //                 .map(|x| {
    //                     Instance::default()
    //                         .scale(0.4)
    //                         .translate(Vec3::new(x as f32, 0.0, z as f32) * 4.0)
    //                 })
    //                 .collect::<Vec<_>>()
    //         })
    //         .collect(),
    // );

    let mut bigass_cube = Object::from_obj(
        include_str!("cube.obj"),
        renderer,
        vec![Instance::default()
            .translate(Vec3::new(0.0, 5.0, 0.0))
            .rotate(35.0f32.to_radians(), Vec3::Y)
            .scale(7.5)],
    );

    let mut ship = Ship::new(renderer);

    let mut ocean = Ocean::new(renderer);

    let mut popr_data = PoprConfig::new(renderer);

    let fullscreen_quad = Object::quad(
        renderer,
        vec![Instance::default()
            .translate(Vec3::new(-1.0, -1.0, 0.0))
            .scale(2.0)],
    );

    let bigass_cube_position = Vec3::new(50.0, 5.0, 50.0);

    let zoom = 1.0;

    let mut shift_held = false;
    let mut mouse_position = Vec2::default();

    #[allow(unused_variables)]
    let mut time = 0.0;

    let mut last_time = web_time::Instant::now();
    engine.run_with_events(move |renderer, window_manager, events| {
        if window.should_close() {
            window_manager.end();
        }

        for event in events {
            match event {
                jandering_engine::engine::EngineEvent::FileChanged(file_name) => {
                    if file_name == "shader.wgsl" {
                        renderer.reload_shader(shader);
                        renderer.reload_shader(cube_shader);
                    } else if file_name == "water_shader.wgsl" {
                        renderer.reload_shader(water_shader)
                    } else if file_name == "popr_shader.wgsl" {
                        renderer.reload_shader(popr_shader)
                    }
                }
            }
        }

        window.poll_events();

        let current_time = web_time::Instant::now();
        let dt = (current_time - last_time).as_secs_f32();
        last_time = current_time;
        time += dt;

        let events = window.events().clone();

        for event in events.iter() {
            match event {
                WindowEvent::WindowInitialized => renderer.register_window(&window),
                WindowEvent::Resized(_) => {
                    let size = UVec2 {
                        x: (window.width() as f32 / RESOLUTION_FACTOR) as u32,
                        y: (window.height() as f32 / RESOLUTION_FACTOR) as u32,
                    };
                    renderer.re_create_texture(
                        TextureDescriptor {
                            size,
                            format: TextureFormat::Depth32F,
                            usage: texture_usage::GENERIC,
                            ..Default::default()
                        },
                        depth_texture,
                    );

                    renderer.re_create_texture(
                        TextureDescriptor {
                            size,
                            format: TextureFormat::Bgra8U,
                            usage: texture_usage::GENERIC,
                            ..Default::default()
                        },
                        target_texture_bind_group.texture_handle,
                    );

                    target_texture_bind_group.re_create(
                        renderer,
                        target_texture_bind_group.texture_handle,
                        target_texture_bind_group.sampler_handle,
                    );

                    popr_data.data.resolution =
                        Vec2::new(window.width() as f32, window.height() as f32);
                }
                WindowEvent::MouseMotion(position) => {
                    mouse_position = (*position).into();
                }
                WindowEvent::Scroll((_, y)) => {
                    // ocean.wave_data.speed *= 1.0 + y * 0.1;
                    ocean.wave_data.strength *= 1.0 + y * 0.1;
                }
                WindowEvent::KeyInput { state, key } => match (key, state) {
                    (Key::Alt, InputState::Pressed) => {
                        if shift_held {
                            let aspect = window.width() as f32 / window.height() as f32;
                            if camera_controller.is_some() {
                                camera.attach_controller(camera_controller.take().unwrap());
                                camera.make_perspective(40.0, aspect, 0.001, 1000.0);
                            } else {
                                camera_controller = camera.take_controller();
                                camera_controller.as_mut().unwrap().clear_mouse_pos();
                            }
                        }
                    }
                    (Key::Shift, _) => shift_held = matches!(state, InputState::Pressed),
                    _ => {}
                },
                _ => {}
            }
        }

        // zoom = time.sin() * 0.5 + 1.0;
        if camera_controller.is_some() {
            let aspect = window.width() as f32 / window.height() as f32;
            camera.make_ortho(
                -CAMERA_ZOOM * 0.5 * zoom * aspect,
                CAMERA_ZOOM * 0.5 * zoom * aspect,
                -CAMERA_ZOOM * 0.5 * zoom,
                CAMERA_ZOOM * 0.5 * zoom,
                CAMERA_NEAR,
                CAMERA_FAR,
            );
        }

        // for instance in debug_cube.instances.iter_mut() {
        //     let pos = instance.position();
        //     instance.set_position(Vec3::new(
        //         pos.x,
        //         ocean.position_at(instance.position()).y + 1.0,
        //         pos.z,
        //     ));
        // }
        // debug_cube.update(renderer);

        ocean.wave_data.time += dt * ocean.wave_data.speed;

        camera.update(renderer, &events, dt);

        let resolution: UVec2 = window.size().into();
        let aspect = resolution.x as f32 / resolution.y as f32;

        let mut normalized_mouse =
            mouse_position / Vec2::new(resolution.x as f32, resolution.y as f32);
        normalized_mouse = -normalized_mouse * 2.0 + 1.0;

        let mouse_view_plane = Vec2::new(
            normalized_mouse.x * CAMERA_ZOOM * 0.5 * aspect,
            normalized_mouse.y * CAMERA_ZOOM * 0.5,
        );

        let mouse_world = camera.position()
            + mouse_view_plane.x * camera.right()
            + mouse_view_plane.y * camera.up();

        let t = -mouse_world.y / camera.direction().y;

        let mouse_floor = mouse_world + t * camera.direction();

        ship.update(&ocean, mouse_floor, &events, renderer, dt);

        bigass_cube.instances[0]
            .set_position(bigass_cube_position - Vec3::new(ship.position.x, 0.0, ship.position.z));
        bigass_cube.update(renderer);

        let screenspace_cube_position =
            camera.matrix() * bigass_cube.instances[0].position().extend(1.0);
        popr_data.data.screenspace_cube_position =
            Vec2::new(screenspace_cube_position.x, -screenspace_cube_position.y)
                / screenspace_cube_position.w;
        popr_data.data.time = time;
        popr_data.update(renderer);

        ocean.wave_data.position_offset = ship.position;
        ocean.update(dt, renderer);

        if window.is_initialized() {
            let main_pass = RenderPass::new(&mut window)
                .with_depth(depth_texture, Some(1.0))
                .with_target_texture_resolve(
                    jandering_engine::renderer::TargetTexture::Handle(
                        target_texture_bind_group.texture_handle,
                    ),
                    None,
                )
                .with_alpha(0.0)
                .bind(0, camera.bind_group())
                .set_shader(shader)
                .render_one(&ship.mesh)
                .set_shader(cube_shader)
                .render_one(&bigass_cube)
                .set_shader(water_shader)
                .bind(1, ocean.noise_texture.bind_group)
                .bind(2, ocean.wave_data_bind_group)
                .render_one(&ocean.mesh);
            renderer.submit_pass(main_pass);

            let popr_pass = RenderPass::new(&mut window)
                .without_depth()
                .set_shader(popr_shader)
                .bind(0, target_texture_bind_group.bind_group)
                .bind(1, popr_data.bind_group())
                .render(&[&fullscreen_quad]);
            renderer.submit_pass(popr_pass);
        }

        window.request_redraw();
    });
}
