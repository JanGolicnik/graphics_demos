use jandering_engine::{
    engine::Engine,
    object::{Instance, Object},
    render_pass::RenderPass,
    renderer::Janderer,
    shader::ShaderDescriptor,
    texture::{texture_usage, TextureDescriptor, TextureFormat},
    types::Vec3,
    utils::free_camera::{FreeCameraController, MatrixCamera},
    window::{WindowConfig, WindowManagerTrait, WindowTrait},
};

fn main() {
    let mut engine = pollster::block_on(Engine::default());

    let mut window = engine.spawn_window(
        WindowConfig::default()
            .with_cursor(true)
            .with_resolution(300, 300)
            .with_auto_resolution()
            .with_decorations(false)
            .with_transparency(true)
            .with_fps_preference(jandering_engine::window::FpsPreference::Exact(120))
            .with_title("beast"),
    );

    let renderer = &mut engine.renderer;

    let mut camera = MatrixCamera::with_controller(renderer, FreeCameraController::default());
    camera.make_perspective(40.0, 1.0, 0.01, 10000.0);
    camera.set_position(Vec3::new(10.0, 10.0, 10.0));
    camera.set_direction(-camera.position());

    let depth_texture = renderer.create_texture(TextureDescriptor {
        name: "depth_texture",
        format: TextureFormat::Depth32F,
        usage: texture_usage::GENERIC,
        ..Default::default()
    });

    let shader = renderer.create_shader(ShaderDescriptor {
        name: "main_shader",
        bind_group_layout_descriptors: vec![MatrixCamera::get_layout_descriptor()],
        depth: true,
        ..Default::default()
    });

    let n = 5;
    let instances = (-n..=n)
        .flat_map(|x| {
            (-n..=n)
                .flat_map(|y| {
                    (-n..=n)
                        .map(|z| {
                            Instance::default()
                                .translate(Vec3::new(x as f32, y as f32, z as f32) * 10.0)
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let object = Object::from_obj(
        include_str!("icosphere.obj"),
        renderer,
        instances,
        // vec![Instance::default()],
    );

    let mut last_time = std::time::Instant::now();

    engine.run(|renderer, window_manager| {
        if window.should_close() {
            window_manager.end();
        }

        window.poll_events();
        let events = window.events();

        let current_time = std::time::Instant::now();
        let dt = (current_time - last_time).as_secs_f32();
        last_time = current_time;

        println!("fps: {}", 1.0 / dt);

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
                _ => {}
            }
        }

        camera.update(renderer, events, dt);

        if window.is_initialized() {
            let pass = RenderPass::new(&mut window)
                .set_shader(shader)
                .with_depth(depth_texture, Some(1.0))
                .with_clear_color(0.7, 0.4, 0.3)
                .bind(0, camera.bind_group())
                .render_one(&object);
            renderer.submit_pass(pass);

            window.request_redraw();
        }
    });
}
