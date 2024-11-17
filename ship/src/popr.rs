use jandering_engine::{
    bind_group::{
        BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutDescriptorEntry,
        BindGroupLayoutEntry,
    },
    renderer::{BindGroupHandle, BufferHandle, Janderer, Renderer},
    types::Vec2,
};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct PoprData {
    pub resolution: Vec2,
    pub screenspace_cube_position: Vec2,
    pub time: f32,
    padding: [f32; 1],
}

pub struct PoprConfig {
    pub data: PoprData,
    pub buffer_handle: BufferHandle,
    bind_group: BindGroupHandle,
}

impl PoprConfig {
    pub fn new(renderer: &mut Renderer) -> Self {
        let data = PoprData {
            resolution: Default::default(),
            screenspace_cube_position: Default::default(),
            time: 0.0,
            padding: Default::default(),
        };

        let buffer_handle = renderer.create_uniform_buffer(bytemuck::cast_slice(&[data]));

        let bind_group = renderer.create_bind_group(BindGroupLayout {
            entries: vec![BindGroupLayoutEntry::Data(buffer_handle)],
        });

        Self {
            data,
            bind_group,
            buffer_handle,
        }
    }

    pub fn get_layout_descriptor() -> jandering_engine::bind_group::BindGroupLayoutDescriptor {
        BindGroupLayoutDescriptor {
            entries: vec![BindGroupLayoutDescriptorEntry::Data { is_uniform: true }],
        }
    }

    pub fn bind_group(&self) -> BindGroupHandle {
        self.bind_group
    }

    pub fn update(&mut self, renderer: &mut Renderer) {
        renderer.write_buffer(self.buffer_handle, bytemuck::cast_slice(&[self.data]));
    }
}
