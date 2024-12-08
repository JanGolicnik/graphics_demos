use jandering_engine::{
    engine::{Engine, EngineConfig},
    object::{primitives::plane_data, Instance, Object, Vertex},
    render_pass::RenderPass,
    renderer::Janderer,
    shader::ShaderDescriptor,
    texture::{texture_usage, TextureDescriptor, TextureFormat},
    types::{Mat4, Vec2, Vec3},
    utils::free_camera::{FreeCameraController, MatrixCamera},
    window::{self, WindowConfig, WindowEvent::Resized, WindowManagerTrait, WindowTrait},
};

struct Heightfield<const W: usize, const H: usize> {
    data: [[f32; W]; H],
}

impl<const W: usize, const H: usize> Heightfield<W, H> {
    pub fn new() -> Self {
        let mut data = [[0.0; W]; H];

        for y in 0..H {
            for x in 0..W {
                data[y][x] = (Vec2::new(y as f32 / H as f32, x as f32 / W as f32) - 0.5).length();
            }
        }

        Heightfield { data }
    }

    fn sample(&self, mut uv: Vec2) -> f32 {
        uv = uv.fract() * Vec2::new(W as f32, H as f32);

        let x0 = uv.x.floor() as usize;
        let y0 = uv.y.floor() as usize;

        let x1 = (x0 + 1) % W;
        let y1 = (y0 + 1) % H;

        let fx = uv.x - uv.x.floor();
        let fy = uv.y - uv.y.floor();

        let bottomleft = self.data[y0][x0] as f32;
        let bottomright = self.data[y0][x1] as f32;
        let topleft = self.data[y1][x0] as f32;
        let topright = self.data[y1][x1] as f32;

        let bottom = bottomleft * (1.0 - fx) + bottomright * fx;
        let top = topleft * (1.0 - fx) + topright * fx;

        bottom * (1.0 - fy) + top * fy
    }
}

fn main() {
    let mut engine = pollster::block_on(Engine::new(EngineConfig {
        writable_storage: true,
    }));

    let mut window = engine.spawn_window(
        WindowConfig::default()
            .with_cursor(true)
            .with_resolution(300, 300)
            .with_auto_resolution()
            .with_decorations(false)
            .with_fps_preference(window::FpsPreference::Exact(120))
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
        source: jandering_engine::shader::ShaderSource::File(
            jandering_engine::utils::FilePath::FileName("shader.wgsl"),
        ),
        descriptors: vec![Vertex::desc(), Instance::desc()],
        bind_group_layout_descriptors: vec![MatrixCamera::get_layout_descriptor()],
        depth: true,
        backface_culling: false,
        target_texture_format: Some(TextureFormat::Bgra8U),
        ..Default::default()
    });

    let n = 10;
    let instances = (-n..=n)
        .flat_map(|x| {
            (-n..=n)
                .flat_map(|y| {
                    (-n..=n)
                        .map(|z| {
                            let model = Mat4::from_translation(
                                Vec3::new(x as f32, y as f32, z as f32) * 10.0,
                            );
                            Instance {
                                model,
                                inv_model: model.inverse(),
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let mut plane = {
        const RESOLUTION: usize = 7;
        const N_VERTICES: usize = 2usize.pow(RESOLUTION as u32);
        let (mut vertices, indices) = plane_data(RESOLUTION as u32, true);
        let heightfield = Heightfield::<N_VERTICES, N_VERTICES>::new();
        //        let heightfield = generate_heightfield(2 ^ resolution, 2 ^ resolution);

        for vertex in vertices.iter_mut() {
            vertex.position.z += heightfield.sample(vertex.uv);
        }

        Object::new(
            renderer,
            vertices,
            indices,
            vec![Instance::default()
                .rotate(90.0f32.to_radians(), Vec3::X)
                .scale(100.0)],
        )
    };

    let mut object = Object::triangle(renderer, instances);

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
                window::WindowEvent::WindowInitialized => renderer.register_window(&window),
                Resized((width, height)) => {
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

        plane.instances[0].set_rotation(30.0f32.to_radians() * dt, Vec3::Y);
        plane.update(renderer);
        //object.update(renderer);

        camera.update(renderer, &events, dt);

        if window.is_initialized() {
            let main_pass = RenderPass::new(&mut window)
                .set_shader(shader)
                .with_depth(depth_texture, Some(1.0))
                .with_clear_color(0.7, 0.4, 0.3)
                .bind(0, camera.bind_group())
                .render_one(&plane);
            renderer.submit_pass(main_pass);

            window.request_redraw();
        }
    });
}
