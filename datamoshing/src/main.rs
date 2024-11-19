use jandering_engine::{
    engine::{Engine, EngineConfig},
    object::{Instance, Object},
    render_pass::RenderPass,
    renderer::Janderer,
    shader::ShaderDescriptor,
    texture::{sampler::SamplerDescriptor, texture_usage, TextureDescriptor, TextureFormat},
    types::Vec3,
    utils::{
        free_camera::{FreeCameraController, MatrixCamera},
        texture::{StorageTextureBindGroup, TextureSamplerBindGroup},
    },
    window::{WindowConfig, WindowManagerTrait, WindowTrait},
};

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

    let mut target_texture = {
        let texture_handle = renderer.create_texture(TextureDescriptor {
            name: "target_texture",
            format: TextureFormat::Bgra8U,
            usage: texture_usage::GENERIC,
            ..Default::default()
        });
        let sampler_handle = renderer.create_sampler(SamplerDescriptor::default());
        TextureSamplerBindGroup::new(renderer, texture_handle, sampler_handle)
    };

    let mut storage_textures = [
        {
            let texture_handle = renderer.create_texture(TextureDescriptor {
                name: "storage_texture1",
                format: TextureFormat::Rgba32F,
                usage: texture_usage::GENERIC_STORAGE,
                ..Default::default()
            });
            StorageTextureBindGroup::new(
                renderer,
                texture_handle,
                jandering_engine::bind_group::StorageTextureAccessType::ReadWrite,
                TextureFormat::Rgba32F,
            )
        },
        {
            let texture_handle = renderer.create_texture(TextureDescriptor {
                name: "storage_texture2",
                format: TextureFormat::Rgba32F,
                usage: texture_usage::GENERIC_STORAGE,
                ..Default::default()
            });
            StorageTextureBindGroup::new(
                renderer,
                texture_handle,
                jandering_engine::bind_group::StorageTextureAccessType::ReadWrite,
                TextureFormat::Rgba32F,
            )
        },
    ];
    let mut current_storage_tex = 0;

    let shader = renderer.create_shader(ShaderDescriptor {
        name: "main_shader",
        source: jandering_engine::shader::ShaderSource::File(
            jandering_engine::utils::FilePath::FileName("shader.wgsl"),
        ),
        bind_group_layout_descriptors: vec![
            MatrixCamera::get_layout_descriptor(),
            storage_textures[0].get_layout_descriptor(),
        ],
        depth: true,
        target_texture_format: Some(TextureFormat::Bgra8U),
        ..Default::default()
    });

    let popr_shader = renderer.create_shader(ShaderDescriptor {
        name: "popr_shader",
        source: jandering_engine::shader::ShaderSource::File(
            jandering_engine::utils::FilePath::FileName("popr_shader.wgsl"),
        ),
        bind_group_layout_descriptors: vec![
            TextureSamplerBindGroup::get_layout_descriptor(),
            storage_textures[0].get_layout_descriptor(),
            storage_textures[1].get_layout_descriptor(),
        ],
        backface_culling: false,
        ..Default::default()
    });

    let n = 10;
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

    let fullscreen_quad = Object::quad(
        renderer,
        vec![Instance::default()
            .translate(Vec3::new(-1.0, -1.0, 0.0))
            .scale(2.0)],
    );

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
                    } else if file_name == "popr_shader.wgsl" {
                        renderer.reload_shader(popr_shader)
                    }
                }
            }
        }

        window.poll_events();
        let events = window.events();

        let current_time = std::time::Instant::now();
        let dt = (current_time - last_time).as_secs_f32();
        last_time = current_time;

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
                    renderer.re_create_texture(
                        TextureDescriptor {
                            name: "target_texture",
                            size: window.size().into(),
                            format: TextureFormat::Bgra8U,
                            usage: texture_usage::GENERIC,
                            ..Default::default()
                        },
                        target_texture.texture_handle,
                    );
                    target_texture.re_create(
                        renderer,
                        target_texture.texture_handle,
                        target_texture.sampler_handle,
                    );

                    for tex in storage_textures.iter_mut() {
                        renderer.re_create_texture(
                            TextureDescriptor {
                                name: "storage_texture",
                                size: window.size().into(),
                                format: TextureFormat::Rgba32F,
                                usage: texture_usage::GENERIC_STORAGE,
                                ..Default::default()
                            },
                            tex.texture_handle,
                        );
                        tex.re_create(renderer, tex.texture_handle, tex.access_type, tex.format);
                    }
                }
                _ => {}
            }
        }

        camera.update(renderer, events, dt);

        if window.is_initialized() {
            let other_storage_tex = if current_storage_tex == 0 { 1 } else { 0 };
            renderer.clear_texture(storage_textures[current_storage_tex].texture_handle);
            let main_pass = RenderPass::new(&mut window)
                .set_shader(shader)
                .with_target_texture_resolve(
                    jandering_engine::renderer::TargetTexture::Handle(
                        target_texture.texture_handle,
                    ),
                    None,
                )
                .with_depth(depth_texture, Some(1.0))
                .with_clear_color(0.7, 0.4, 0.3)
                .bind(0, camera.bind_group())
                .bind(1, storage_textures[current_storage_tex].bind_group)
                .render_one(&object);
            renderer.submit_pass(main_pass);

            let popr_pass = RenderPass::new(&mut window)
                .set_shader(popr_shader)
                .bind(0, target_texture.bind_group)
                .bind(1, storage_textures[current_storage_tex].bind_group)
                .bind(2, storage_textures[other_storage_tex].bind_group)
                .render_one(&fullscreen_quad);
            renderer.submit_pass(popr_pass);

            window.request_redraw();

            current_storage_tex = other_storage_tex;
        }
    });
}
