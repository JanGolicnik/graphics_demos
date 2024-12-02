use history_instance::HistoryInstance;
use jandering_engine::{
    bind_group::{
        BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutDescriptorEntry,
        BindGroupLayoutEntry,
    },
    engine::{Engine, EngineConfig},
    object::{Instance, Object, Vertex},
    render_pass::RenderPass,
    renderer::{BindGroupHandle, BufferHandle, Janderer, Renderer},
    shader::ShaderDescriptor,
    texture::{sampler::SamplerDescriptor, texture_usage, TextureDescriptor, TextureFormat},
    types::{Mat4, Qua, Vec3},
    utils::{
        free_camera::{FreeCameraController, MatrixCamera},
        texture::{StorageTextureBindGroup, TextureSamplerBindGroup},
    },
    window::{InputState, WindowConfig, WindowManagerTrait, WindowTrait},
};

mod history_instance;

struct PrevCameraMatBindGroup {
    pub mat: Mat4,
    pub buffer_handle: BufferHandle,
    pub bind_group: BindGroupHandle,
}

impl PrevCameraMatBindGroup {
    pub fn new(renderer: &mut Renderer) -> Self {
        let mat = Mat4::IDENTITY;
        let buffer_handle = renderer.create_uniform_buffer(bytemuck::cast_slice(&[mat]));
        let bind_group = renderer.create_bind_group(BindGroupLayout {
            entries: vec![BindGroupLayoutEntry::Data(buffer_handle)],
        });

        Self {
            mat,
            buffer_handle,
            bind_group,
        }
    }

    pub fn get_layout_descriptor() -> BindGroupLayoutDescriptor {
        BindGroupLayoutDescriptor {
            entries: vec![BindGroupLayoutDescriptorEntry::Data { is_uniform: true }],
        }
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
            .with_fps_preference(jandering_engine::window::FpsPreference::Exact(120))
            .with_title("beast"),
    );

    let renderer = &mut engine.renderer;

    let mut camera = MatrixCamera::with_controller(renderer, FreeCameraController::default());
    camera.make_perspective(40.0, 1.0, 0.01, 10000.0);
    camera.set_position(Vec3::new(10.0, 10.0, 10.0));
    camera.set_direction(-camera.position());

    let mut prev_camera_mat = PrevCameraMatBindGroup::new(renderer);

    let depth_texture = renderer.create_texture(TextureDescriptor {
        name: "depth_texture",
        format: TextureFormat::Depth32F,
        usage: texture_usage::GENERIC,
        ..Default::default()
    });

    let mut target_textures = [
        {
            let texture_handle = renderer.create_texture(TextureDescriptor {
                name: "target_texture1",
                format: TextureFormat::Bgra8U,
                usage: texture_usage::GENERIC,
                ..Default::default()
            });
            let sampler_handle = renderer.create_sampler(SamplerDescriptor::default());
            TextureSamplerBindGroup::new(renderer, texture_handle, sampler_handle)
        },
        {
            let texture_handle = renderer.create_texture(TextureDescriptor {
                name: "target_texture2",
                format: TextureFormat::Bgra8U,
                usage: texture_usage::GENERIC,
                ..Default::default()
            });
            let sampler_handle = renderer.create_sampler(SamplerDescriptor::default());
            TextureSamplerBindGroup::new(renderer, texture_handle, sampler_handle)
        },
        {
            let texture_handle = renderer.create_texture(TextureDescriptor {
                name: "target_texture3",
                format: TextureFormat::Bgra8U,
                usage: texture_usage::GENERIC,
                ..Default::default()
            });
            let sampler_handle = renderer.create_sampler(SamplerDescriptor::default());
            TextureSamplerBindGroup::new(renderer, texture_handle, sampler_handle)
        },
    ];

    let mut storage_texture = {
        let texture_handle = renderer.create_texture(TextureDescriptor {
            name: "storage_texture",
            format: TextureFormat::Rg32F,
            usage: texture_usage::GENERIC_STORAGE,
            ..Default::default()
        });
        StorageTextureBindGroup::new(
            renderer,
            texture_handle,
            jandering_engine::bind_group::StorageTextureAccessType::ReadWrite,
            TextureFormat::Rg32F,
        )
    };

    let (shader, random_color_shader) = {
        let desc = ShaderDescriptor {
            name: "main_shader",
            source: jandering_engine::shader::ShaderSource::File(
                jandering_engine::utils::FilePath::FileName("shader.wgsl"),
            ),
            descriptors: vec![Vertex::desc(), HistoryInstance::desc()],
            bind_group_layout_descriptors: vec![
                MatrixCamera::get_layout_descriptor(),
                PrevCameraMatBindGroup::get_layout_descriptor(),
                storage_texture.get_layout_descriptor(),
            ],
            depth: true,
            target_texture_format: Some(TextureFormat::Bgra8U),
            ..Default::default()
        };

        (
            renderer.create_shader(ShaderDescriptor {
                fs_entry: "fs_random_colors",
                ..desc.clone()
            }),
            renderer.create_shader(desc),
        )
    };

    let popr_shader = renderer.create_shader(ShaderDescriptor {
        name: "popr_shader",
        source: jandering_engine::shader::ShaderSource::File(
            jandering_engine::utils::FilePath::FileName("popr_shader.wgsl"),
        ),
        bind_group_layout_descriptors: vec![
            TextureSamplerBindGroup::get_layout_descriptor(),
            storage_texture.get_layout_descriptor(),
        ],
        fs_entry: "fs_datamosh",
        backface_culling: false,
        ..Default::default()
    });

    let blit_shader = renderer.create_shader(ShaderDescriptor {
        name: "blit_shader",
        source: jandering_engine::shader::ShaderSource::File(
            jandering_engine::utils::FilePath::FileName("popr_shader.wgsl"),
        ),
        bind_group_layout_descriptors: vec![TextureSamplerBindGroup::get_layout_descriptor()],
        fs_entry: "fs_blit",
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
                            let model = Mat4::from_translation(
                                Vec3::new(x as f32, y as f32, z as f32) * 10.0,
                            );
                            HistoryInstance {
                                model,
                                inv_model: model.inverse(),
                                prev_model: model,
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let mut object = Object::from_obj(
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

    let mut time = 0.0;
    let mut last_time = std::time::Instant::now();

    let mut frame_counter = 0;
    let mut frame_accumulator = 0.0;

    let mut current_clear_color = 0;
    let clear_colors = [
        Vec3::new(0.7, 0.4, 0.3),
        Vec3::new(0.3, 0.7, 0.4),
        Vec3::new(0.4, 0.3, 0.7),
        Vec3::new(0.4, 0.7, 0.3),
    ];

    let mut refresh = true;
    let mut no_camera = false;
    let mut random_colors = false;
    let mut alpha0 = true;

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

                    for target_texture in target_textures.iter_mut() {
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
                    }

                    renderer.re_create_texture(
                        TextureDescriptor {
                            name: "storage_texture",
                            size: window.size().into(),
                            format: TextureFormat::Rg32F,
                            usage: texture_usage::GENERIC_STORAGE,
                            ..Default::default()
                        },
                        storage_texture.texture_handle,
                    );
                    storage_texture.re_create(
                        renderer,
                        storage_texture.texture_handle,
                        storage_texture.access_type,
                        storage_texture.format,
                    );
                }
                jandering_engine::window::WindowEvent::KeyInput {
                    key,
                    state: InputState::Pressed,
                } => match key {
                    jandering_engine::window::Key::Key1 => camera.set_position(Vec3::ZERO),
                    jandering_engine::window::Key::Key2 => {
                        refresh = true;
                        current_clear_color = (current_clear_color + 1) % clear_colors.len();
                    }
                    jandering_engine::window::Key::Key3 => no_camera = !no_camera,
                    jandering_engine::window::Key::Key4 => random_colors = !random_colors,
                    jandering_engine::window::Key::Key5 => alpha0 = !alpha0,
                    _ => {}
                },
                _ => {}
            }
        }

        for instance in object.instances.iter_mut() {
            let (scale, mut rotation, mut translation) =
                instance.model.to_scale_rotation_translation();

            rotation *= Qua::from_axis_angle(-translation.normalize(), 30.0f32.to_radians() * dt);

            translation += (time * 1.2).sin() * dt * -translation * 0.1;

            let model = Mat4::from_scale_rotation_translation(scale, rotation, translation);

            *instance = HistoryInstance {
                model,
                inv_model: model.inverse(),
                prev_model: instance.model,
            };
        }

        object.update(renderer);

        prev_camera_mat.mat = camera.matrix();
        camera.update(renderer, &events, dt);

        if no_camera {
            prev_camera_mat.mat = camera.matrix()
        }
        renderer.write_buffer(
            prev_camera_mat.buffer_handle,
            bytemuck::cast_slice(&[prev_camera_mat.mat]),
        );

        if window.is_initialized() {
            let clear_color = clear_colors[current_clear_color];

            let main_shader = if random_colors {
                shader
            } else {
                random_color_shader
            };

            let alpha = if alpha0 { 0.0 } else { 1.0 };

            renderer.clear_texture(storage_texture.texture_handle);
            let main_pass = RenderPass::new(&mut window)
                .set_shader(main_shader)
                .with_target_texture_resolve(
                    jandering_engine::renderer::TargetTexture::Handle(
                        target_textures[0].texture_handle,
                    ),
                    None,
                )
                .with_depth(depth_texture, Some(1.0))
                .with_clear_color(clear_color.x, clear_color.y, clear_color.z)
                .with_alpha(alpha)
                .bind(0, camera.bind_group())
                .bind(1, prev_camera_mat.bind_group)
                .bind(2, storage_texture.bind_group)
                .render_one(&object);
            renderer.submit_pass(main_pass);

            if refresh {
                refresh = false;
                renderer.blit_textures(
                    target_textures[0].texture_handle,
                    target_textures[1].texture_handle,
                );
            }

            let popr_pass = RenderPass::new(&mut window)
                .set_shader(popr_shader)
                .with_target_texture_resolve(
                    jandering_engine::renderer::TargetTexture::Handle(
                        target_textures[2].texture_handle,
                    ),
                    None,
                )
                .bind(0, target_textures[1].bind_group)
                .bind(1, storage_texture.bind_group)
                .render_one(&fullscreen_quad)
                .set_shader(blit_shader)
                .with_target_texture_resolve(
                    jandering_engine::renderer::TargetTexture::Screen,
                    None,
                )
                .bind(0, target_textures[2].bind_group)
                .render_one(&fullscreen_quad);
            renderer.submit_pass(popr_pass);

            renderer.blit_textures(
                target_textures[2].texture_handle,
                target_textures[1].texture_handle,
            );

            window.request_redraw();
        }
    });
}
