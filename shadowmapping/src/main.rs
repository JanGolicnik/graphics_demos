use std::f32::consts::PI;

use jandering_engine::{
    engine::Engine,
    object::{Instance, Object, Vertex},
    render_pass::RenderPass,
    renderer::{Janderer, TargetTexture},
    shader::ShaderDescriptor,
    texture::{
        texture_usage::{self},
        TextureDescriptor, TextureFormat,
    },
    types::{UVec2, Vec2, Vec3},
    utils::free_camera::MatrixCamera,
    window::{InputState, Key, WindowConfig, WindowEvent, WindowManagerTrait, WindowTrait},
};
use light::Light;

mod light;

const CAMERA_NEAR: f32 = 0.01;
const CAMERA_FAR: f32 = 100000.0;

const CAMERA_ZOOM: f32 = 27.0;

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

    let mut camera = MatrixCamera::new(renderer);
    // let mut camera = MatrixCamera::with_controller(renderer, FreeCameraController::default());
    camera.set_position(Vec3::new(70.0, 30.0, 70.0));
    camera.set_direction(-camera.position().normalize());

    // let shader_source = pollster::block_on(load_text(jandering_engine::utils::FilePath::FileName(
    //     "shader.wgsl",
    // )))
    // .unwrap();
    let shader_source = include_str!("shader.wgsl").to_string();

    // let light_shader_source = pollster::block_on(load_text(
    //     jandering_engine::utils::FilePath::FileName("light_shader.wgsl"),
    // ))
    // .unwrap();
    let light_shader_source = include_str!("light_shader.wgsl").to_string();

    let shader = renderer.create_shader(ShaderDescriptor {
        source: jandering_engine::shader::ShaderSource::Code(shader_source.clone()),
        descriptors: vec![Vertex::desc(), Instance::desc()],
        bind_group_layout_descriptors: vec![
            Light::get_layout_descriptor(),
            MatrixCamera::get_layout_descriptor(),
        ],
        backface_culling: false,
        depth: true,
        ..Default::default()
    });

    let light_shader = renderer.create_shader(ShaderDescriptor {
        source: jandering_engine::shader::ShaderSource::Code(light_shader_source.clone()),
        fs_entry: "fs_main",
        descriptors: vec![Vertex::desc(), Instance::desc()],
        bind_group_layout_descriptors: vec![
            Light::get_data_only_layout_descriptor(),
            MatrixCamera::get_layout_descriptor(),
        ],
        depth: true,
        backface_culling: true,
        target_texture_format: None,
        ..Default::default()
    });

    let popr_shader = renderer.create_shader(ShaderDescriptor {
        source: jandering_engine::shader::ShaderSource::Code(
            include_str!("popr_shader.wgsl").to_string(),
        ),
        descriptors: vec![Vertex::desc(), Instance::desc()],
        bind_group_layout_descriptors: vec![Light::get_layout_descriptor()],
        depth: false,
        backface_culling: false,
        ..Default::default()
    });

    // let light_pos = Vec3::new(-70.0, 60.0, -70.0);
    let light_pos = Vec3::new(50.0, 100.0, 50.0);
    let mut light = Light::cone(renderer, 2.0, light_pos, -light_pos.normalize());
    let mut objects = {
        let instances_vec: Vec<Instance> = (-10..=10)
            .flat_map(|x| {
                (-10..=10)
                    .map(|z| {
                        Instance::default().translate(Vec3::new(x as f32, 0.0, z as f32) * 5.0)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        Object::from_obj(include_str!("icosphere.obj"), renderer, instances_vec)
    };

    let floor = Object::quad(
        renderer,
        vec![Instance::default()
            .rotate(90.0f32.to_radians(), Vec3::X)
            .translate(Vec3::new(-50.0, -3.0, -50.0))
            .scale(100.0)],
    );
    let fullscreen_quad = Object::quad(
        renderer,
        vec![Instance::default()
            .translate(Vec3::new(-1.0, -1.0, 0.0))
            .scale(1.0)],
    );

    let mut show_shadow_camera_view = false;

    let mut mouse_position = Vec2::default();

    let mut time = 0.0;

    let mut last_time = std::time::Instant::now();
    engine.run(move |renderer, window_manager| {
        if window.should_close() {
            window_manager.end();
        }

        window.poll_events();

        let current_time = std::time::Instant::now();
        let dt = (current_time - last_time).as_secs_f32();
        last_time = current_time;
        time += dt;

        let events = window.events().clone();

        for event in events.iter() {
            match event {
                WindowEvent::WindowInitialized => renderer.register_window(&window),
                WindowEvent::Resized(resolution) => {
                    let aspect_ratio = resolution.0 as f32 / resolution.1 as f32;

                    camera.make_ortho(
                        -CAMERA_ZOOM * 0.5 * aspect_ratio,
                        CAMERA_ZOOM * 0.5 * aspect_ratio,
                        -CAMERA_ZOOM * 0.5,
                        CAMERA_ZOOM * 0.5,
                        CAMERA_NEAR,
                        CAMERA_FAR,
                    );
                    // camera.make_perspective(40.0, aspect_ratio, CAMERA_NEAR, CAMERA_FAR);
                    renderer.re_create_texture(
                        TextureDescriptor {
                            size: window.size().into(),
                            format: TextureFormat::Depth32F,
                            usage: texture_usage::GENERIC,
                            ..Default::default()
                        },
                        depth_texture,
                    );
                }
                WindowEvent::MouseMotion(position) => {
                    mouse_position = (*position).into();
                }
                WindowEvent::KeyInput { key, state } => match state {
                    InputState::Released => {
                        if let Key::Q = key {
                            show_shadow_camera_view = false;
                        }
                    }
                    InputState::Pressed => {
                        if let Key::Q = key {
                            show_shadow_camera_view = true;
                        }
                    }
                },
                _ => {}
            }
        }

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

        light.set_direction(mouse_floor - light.position());
        light.update(renderer);

        for (i, instance) in objects.instances.iter_mut().enumerate() {
            instance.set_rotation(PI * dt, Vec3::Y);
            instance.set_position(
                instance.position()
                    + Vec3::new(0.0, (time + i as f32 * 0.03).sin() * 2.0 * dt, 0.0),
            );
        }

        objects.update(renderer);

        if window.is_initialized() {
            let pass = RenderPass::new(&mut window)
                .with_depth(light.texture(), Some(1.0))
                .with_clear_color(0.0, 0.0, 0.0)
                .with_target_texture_resolve(TargetTexture::None, None)
                .set_shader(light_shader)
                .bind(0, light.data_only_bind_group())
                .bind(1, camera.bind_group())
                .render(&[&objects, &floor]);
            renderer.submit_pass(pass);
            let pass = RenderPass::new(&mut window)
                .with_depth(depth_texture, Some(1.0))
                .with_target_texture_resolve(
                    jandering_engine::renderer::TargetTexture::Screen,
                    None,
                )
                // .with_alpha(0.0)
                .bind(0, light.bind_group())
                .bind(1, camera.bind_group())
                .set_shader(shader)
                .render(&[&objects, &floor]);
            renderer.submit_pass(pass);
            if show_shadow_camera_view {
                let pass = RenderPass::new(&mut window)
                    .without_depth()
                    .set_shader(popr_shader)
                    .bind(0, light.bind_group())
                    .render(&[&fullscreen_quad]);
                renderer.submit_pass(pass);
            }
        }

        window.request_redraw();
    });
}
