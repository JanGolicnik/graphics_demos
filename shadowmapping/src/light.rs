use jandering_engine::{
    bind_group::{
        BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutDescriptorEntry,
        BindGroupLayoutEntry, SamplerType,
    },
    renderer::{BindGroupHandle, BufferHandle, Janderer, Renderer, SamplerHandle, TextureHandle},
    texture::{sampler::SamplerDescriptor, texture_usage, TextureDescriptor, TextureFormat},
    types::{Mat4, UVec2, Vec3},
    utils::free_camera::OPENGL_TO_WGPU_MATRIX,
};

const LIGHT_RESOLUTION: u32 = 2048;
const LIGHT_UP: Vec3 = Vec3::Y;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
struct Data {
    up: Vec3,
    up_padding: f32,
    right: Vec3,
    right_padding: f32,
    position: Vec3,
    position_padding: f32,
    direction: Vec3,
    direction_padding: f32,
    view_proj: Mat4,
    texture_size: UVec2,
    fov: f32,
    padding: f32,
}

pub struct Light {
    data: Data,
    proj: Mat4,
    texture: TextureHandle,

    buffer_handle: BufferHandle,
    #[allow(dead_code)]
    texture_handle: TextureHandle,
    #[allow(dead_code)]
    sampler_handle: SamplerHandle,

    data_bind_group: BindGroupHandle,
    light_bind_group: BindGroupHandle,
}

impl Light {
    pub fn cone(renderer: &mut Renderer, fov: f32, position: Vec3, direction: Vec3) -> Self {
        let right = LIGHT_UP.cross(direction).normalize();
        let up = direction.cross(right).normalize();

        let view = Mat4::look_at_rh(position, position + direction, LIGHT_UP);
        // let proj = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.01, 1000.0);
        let proj = Mat4::perspective_rh(fov.to_radians(), 1.0, 0.01, 1000.0);

        let view_proj = OPENGL_TO_WGPU_MATRIX * proj * view;
        // let inverse_view = view.inverse();

        let data = Data {
            up,
            up_padding: 0.0,
            right,
            right_padding: 0.0,
            position,
            position_padding: 0.0,
            direction,
            direction_padding: 0.0,
            view_proj,
            // inverse_view,
            texture_size: UVec2 {
                x: LIGHT_RESOLUTION,
                y: LIGHT_RESOLUTION,
            },
            fov,
            padding: 0.0,
        };

        let buffer_handle = renderer.create_uniform_buffer(bytemuck::cast_slice(&[data]));
        let texture_handle = renderer.create_texture(TextureDescriptor {
            size: UVec2::splat(LIGHT_RESOLUTION),
            format: TextureFormat::Depth32F,
            usage: texture_usage::GENERIC,
            ..Default::default()
        });
        let sampler_handle = renderer.create_sampler(SamplerDescriptor {
            filter: jandering_engine::texture::sampler::SamplerFilterMode::Nearest,
            ..Default::default()
        });

        let light_bind_group = renderer.create_bind_group(BindGroupLayout {
            entries: vec![
                BindGroupLayoutEntry::Data(buffer_handle),
                BindGroupLayoutEntry::Texture {
                    handle: texture_handle,
                    sample_type: jandering_engine::bind_group::TextureSampleType::Depth,
                },
                BindGroupLayoutEntry::Sampler {
                    handle: sampler_handle,
                    sampler_type: SamplerType::NonFiltering,
                },
            ],
        });

        let data_bind_group = renderer.create_bind_group(BindGroupLayout {
            entries: vec![BindGroupLayoutEntry::Data(buffer_handle)],
        });

        Self {
            data,
            proj,
            texture: texture_handle,

            buffer_handle,
            texture_handle,
            sampler_handle,

            data_bind_group,
            light_bind_group,
        }
    }

    pub fn get_data_only_layout_descriptor(
    ) -> jandering_engine::bind_group::BindGroupLayoutDescriptor {
        BindGroupLayoutDescriptor {
            entries: vec![BindGroupLayoutDescriptorEntry::Data { is_uniform: true }],
        }
    }

    pub fn data_only_bind_group(&self) -> BindGroupHandle {
        self.data_bind_group.into()
    }

    pub fn get_layout_descriptor() -> jandering_engine::bind_group::BindGroupLayoutDescriptor {
        BindGroupLayoutDescriptor {
            entries: vec![
                BindGroupLayoutDescriptorEntry::Data { is_uniform: true },
                BindGroupLayoutDescriptorEntry::Texture {
                    sample_type: jandering_engine::bind_group::TextureSampleType::Depth,
                },
                BindGroupLayoutDescriptorEntry::Sampler {
                    sampler_type: SamplerType::NonFiltering,
                },
            ],
        }
    }

    pub fn bind_group(&self) -> BindGroupHandle {
        self.light_bind_group.into()
    }

    pub fn texture(&self) -> TextureHandle {
        self.texture
    }

    pub fn position(&self) -> Vec3 {
        self.data.position
    }

    #[allow(dead_code)]
    pub fn set_position(&mut self, position: Vec3) {
        self.data.position = position;
    }

    pub fn set_direction(&mut self, direction: Vec3) {
        self.data.direction = direction;
    }

    #[allow(dead_code)]
    pub fn direction(&self) -> Vec3 {
        self.data.direction
    }

    pub fn update(&mut self, renderer: &mut Renderer) {
        self.data.right = LIGHT_UP.cross(self.data.direction).normalize();
        self.data.up = self.data.direction.cross(self.data.right).normalize();

        let view = Mat4::look_at_rh(
            self.data.position,
            self.data.position + self.data.direction,
            LIGHT_UP,
        );

        self.data.view_proj = OPENGL_TO_WGPU_MATRIX * self.proj * view;
        // self.data.inverse_view = view.inverse();

        renderer.write_buffer(self.buffer_handle, bytemuck::cast_slice(&[self.data]));
    }
}
