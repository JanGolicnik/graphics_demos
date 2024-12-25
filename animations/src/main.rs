use animated_object::{AnimatedObject, AnimatedVertex};
use jandering_engine::{
    engine::{Engine, EngineConfig}, object::{Instance, Vertex}, render_pass::RenderPass, renderer::Janderer, shader::ShaderDescriptor, texture::{texture_usage, TextureDescriptor, TextureFormat}, types::Vec3, utils::free_camera::{FreeCameraController, MatrixCamera}, window::{InputState, Key, WindowConfig, WindowManagerTrait, WindowTrait}
};

mod animated_object;

fn main() {
    let mut engine = pollster::block_on(Engine::new(EngineConfig {
        writable_storage: true,
    }));

    let mut window = engine.spawn_window(
        WindowConfig::default()
            .with_cursor(true)
            .with_resolution(300, 300)
            .with_auto_resolution()
            .with_transparency(true)
            .with_decorations(true)
            .with_title("beast"),
    );

    let renderer = &mut engine.renderer;

    let mut camera = MatrixCamera::with_controller(renderer, FreeCameraController::default());
    camera.make_perspective(40.0, 1.0, 0.01, 10000.0);
    camera.set_position(Vec3::new(10.0, 10.0, 10.0));
    camera.set_direction(-camera.position().normalize());

    let depth_texture = renderer.create_texture(TextureDescriptor {
        name: "depth_texture",
        format: TextureFormat::Depth32F,
        usage: texture_usage::GENERIC,
        ..Default::default()
    });

    let shader = renderer.create_shader(ShaderDescriptor {
        name: "main_shader",
        source: jandering_engine::shader::ShaderSource::File(
            jandering_engine::utils::FilePath::FileName("shader.wgsl"),
        ),
        descriptors: vec![AnimatedVertex::desc(), AnimatedObject::instance_desc()],
        bind_group_layout_descriptors: vec![MatrixCamera::get_layout_descriptor(), AnimatedObject::joint_data_bind_group_layout_descriptor()],
        depth: true,
        ..Default::default()
    });

    let debug_shader = renderer.create_shader(ShaderDescriptor {
        name: "debug_shader",
        descriptors: vec![Vertex::desc(), Instance::desc()],
        bind_group_layout_descriptors: vec![MatrixCamera::get_layout_descriptor()],
        depth: true,
        ..Default::default()
    });


    let mut animated_object = pollster::block_on(AnimatedObject::from_gltf(renderer, "character.gltf"));
    let mut time = 0.0;
    let mut last_time = std::time::Instant::now();

    let mut frame_counter = 0;
    let mut frame_accumulator = 0.0;

    engine.run_with_events(|renderer, window_manager, events| {
        if window.should_close() {
            window_manager.end();
        }

        for event in events {
            match event {
                jandering_engine::engine::EngineEvent::FileChanged(file_name) => {
                    if file_name == "shader.wgsl" {
                        renderer.reload_shader(shader);
                    }
                }
            }
        }

        window.poll_events();
        let events = window.events().clone();

        let current_time = std::time::Instant::now();
        let dt = (current_time - last_time).as_secs_f32();
        last_time = current_time;
        time += dt;

        frame_accumulator += dt;
        frame_counter += 1;
        if frame_accumulator > 1.0 {
            println!("fps: {}", frame_counter as f32 / frame_accumulator);
            frame_accumulator = 0.0;
            frame_counter = 0;
        }

        for event in events.iter() {
            match event {
                jandering_engine::window::WindowEvent::WindowInitialized => {
                    renderer.register_window(&window)
                }
                jandering_engine::window::WindowEvent::Resized((width, height)) => {
                    renderer.resize(&window, *width, *height);
                    camera.make_perspective(40.0, *width as f32 / *height as f32, 0.01, 10000.0);
                    renderer.re_create_texture(
                        TextureDescriptor {
                            name: "depth_texture",
                            size: window.size().into(),
                            format: TextureFormat::Depth32F,
                            usage: texture_usage::GENERIC,
                            ..Default::default()
                        },
                        depth_texture,
                    );
                }
                jandering_engine::window::WindowEvent::KeyInput {
                    key,
                    state: InputState::Pressed,
                } => match key {
                    Key::Q => {
                        animated_object.current_animation = if animated_object.current_animation == 0 {
                            animated_object.animations.len() - 1
                        }else {
                            animated_object.current_animation - 1
                        };
                    }
                    Key::E => {
                        animated_object.current_animation = if animated_object.current_animation + 1 >= animated_object.animations.len() {
                            0
                        }else {
                            animated_object.current_animation + 1
                        };
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        animated_object.update(renderer, dt);

        camera.update(renderer, &events, dt);

        if window.is_initialized() {

            let animated_meshes = animated_object.meshes.iter().collect::<Vec<_>>();

            let main_pass = RenderPass::new(&mut window)
                .set_shader(shader)
                .with_depth(depth_texture, Some(1.0))
                .with_clear_color(0.6, 0.5, 0.4)
                .bind(0, camera.bind_group())
                .bind(1, animated_object.joint_data_bind_group)
                .render(&animated_meshes)
                .set_shader(debug_shader)
                .render_one(&animated_object.debug_object);
            renderer.submit_pass(main_pass);

            window.request_redraw();
        }
    });
}
