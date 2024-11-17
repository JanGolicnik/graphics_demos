use jandering_engine::{
    bind_group::{
        BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutDescriptorEntry,
        BindGroupLayoutEntry,
    },
    object::{Instance, Object},
    renderer::{BindGroupHandle, BufferHandle, Janderer, Renderer},
    texture::{sampler::SamplerDescriptor, texture_usage, TextureDescriptor, TextureFormat},
    types::{UVec2, Vec2, Vec3},
    utils::texture::UnfilteredTextureSamplerBindGroup,
};
use noise::utils::NoiseMap;

use crate::constants::{NOISE_SCALE, NOISE_TEXTURE_SIZE};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct WaveData {
    pub time: f32,
    pub strength: f32,
    pub speed: f32,
    pub direction: f32,
    pub position_offset: Vec3,
    pub ocean_size_meters: f32,
    pub noise_scale: f32,
    padding: [f32; 3],
}

pub struct Ocean {
    pub mesh: Object<Instance>,
    pub map: NoiseMap,

    pub noise_texture: UnfilteredTextureSamplerBindGroup,

    pub wave_data_bind_group: BindGroupHandle,
    pub wave_data: WaveData,
    pub wave_data_buffer_handle: BufferHandle,
}

impl Ocean {
    pub fn new(renderer: &mut Renderer) -> Self {
        use noise::utils::PlaneMapBuilder;
        let hasher = noise::permutationtable::PermutationTable::new(0);
        let map =
            PlaneMapBuilder::new_fn(|point| noise::core::perlin::perlin_2d(point.into(), &hasher))
                .set_size(NOISE_TEXTURE_SIZE, NOISE_TEXTURE_SIZE)
                // .set_x_bounds(-10.0, 10.0)
                // .set_y_bounds(-10.0, 10.0)
                .set_x_bounds(-3.0, 3.0)
                .set_y_bounds(-3.0, 3.0)
                .set_is_seamless(true)
                .build();

        let noise_texture_data = map.iter().map(|e| *e as f32).collect::<Vec<_>>();

        let noise_texture = renderer.create_texture(TextureDescriptor {
            name: "noise",
            size: UVec2::splat(NOISE_TEXTURE_SIZE as u32),
            data: Some(bytemuck::cast_slice(&noise_texture_data[..])),
            format: TextureFormat::F32,
            usage: texture_usage::GENERIC,
            ..Default::default()
        });
        let noise_texture_sampler = renderer.create_sampler(SamplerDescriptor {
            address_mode: jandering_engine::texture::sampler::SamplerAddressMode::Repeat,
            filter: jandering_engine::texture::sampler::SamplerFilterMode::Nearest,
            ..Default::default()
        });
        let noise_texture =
            UnfilteredTextureSamplerBindGroup::new(renderer, noise_texture, noise_texture_sampler);

        let mesh = Object::plane(
            renderer,
            7,
            vec![Instance::default()
                .rotate(90.0f32.to_radians(), Vec3::X)
                .translate(Vec3::new(-50.0, -1.0, -50.0))
                .scale(200.0)],
        );

        let wave_data = WaveData {
            time: 0.0,
            strength: 1.0,
            direction: 0.0,
            speed: 0.015,
            ocean_size_meters: mesh.instances[0].size().x,
            position_offset: Vec3::ZERO,
            noise_scale: NOISE_SCALE,
            padding: [0.0; 3],
        };
        let wave_data_buffer_handle =
            renderer.create_uniform_buffer(bytemuck::cast_slice(&[wave_data]));
        let wave_data_bind_group = renderer.create_bind_group(BindGroupLayout {
            entries: vec![BindGroupLayoutEntry::Data(wave_data_buffer_handle)],
        });
        Self {
            mesh,
            map,
            noise_texture,
            wave_data,
            wave_data_bind_group,
            wave_data_buffer_handle,
        }
    }

    pub fn update(&mut self, _: f32, renderer: &mut Renderer) {
        renderer.write_buffer(
            self.wave_data_buffer_handle,
            bytemuck::cast_slice(&[self.wave_data]),
        );
    }

    pub fn normal_at(&self, world_position: Vec3) -> Vec3 {
        let epsilon = 0.01;
        // let scale = self.wave_data.strength;
        let height_right = self
            .position_at(world_position + Vec3::new(epsilon, 0.0, 0.0))
            .y;
        let height_left = self
            .position_at(world_position + Vec3::new(-epsilon, 0.0, 0.0))
            .y;
        let height_up = self
            .position_at(world_position + Vec3::new(0.0, 0.0, epsilon))
            .y;
        let height_down = self
            .position_at(world_position + Vec3::new(0.0, 0.0, -epsilon))
            .y;

        let dx = height_right - height_left;
        let dy = height_up - height_down;

        // Vec3::new(-dx / epsilon, scale * 0.5, -dy / epsilon).normalize()
        Vec3::new(-dx / epsilon, 1.0, -dy / epsilon).normalize()

        // let tangent = Vec3::new(2.0, dx, 0.0);
        // let bitangent = Vec3::new(0.0, dy, scale);
        // tangent.cross(bitangent).normalize()

        // -Vec3::new(1.0, height_right / epsilon, 0.0)
        //     .normalize()
        //     .cross(Vec3::new(0.0, height_up / epsilon, 1.0).normalize())
        //     .normalize()

        // let dhdx = (dx) / epsilon;
        // let dhdy = (dy) / epsilon;
        // Vec3::new(dhdx, 1.0, dhdy).normalize()

        // let dhdx = (height_right - height_center) / epsilon;
        // let dhdy = (height_up - height_center) / epsilon;
        // Vec3::new(-dhdx, epsilon, -dhdy).normalize()

        // Vec3::new(
        //     (dx),
        //     epsilon,
        //     (dy),
        // )
        // .normalize()

        // let dhdx = (height_right - height_center) / epsilon;
        // let dhdy = (height_up - height_center) / epsilon;

        // Vec3::new(-dhdx, 1.0, -dhdy).normalize()
    }

    pub fn position_at(&self, world_position: Vec3) -> Vec3 {
        let uv = world_position / NOISE_SCALE + self.wave_data.time;

        let height_center = (0..6).fold(0.0, |final_height, i| {
            let height = self.sample(Vec2::new(uv.x, uv.z) * (i + 1) as f32);

            final_height + height / 1.75f32.powf(i as f32)
        }) * self.wave_data.strength;

        Vec3::new(
            height_center * 0.5,
            height_center * 0.4,
            height_center * 0.5,
        )
    }

    fn sample(&self, mut uv: Vec2) -> f32 {
        uv = uv.fract() * Vec2::new(NOISE_TEXTURE_SIZE as f32, NOISE_TEXTURE_SIZE as f32);

        let x0 = uv.x.floor() as usize;
        let y0 = uv.y.floor() as usize;

        let x1 = (x0 + 1) % NOISE_TEXTURE_SIZE;
        let y1 = (y0 + 1) % NOISE_TEXTURE_SIZE;

        let fx = uv.x - uv.x.floor();
        let fy = uv.y - uv.y.floor();

        let bottomleft = self.map.get_value(x0, y0) as f32;
        let bottomright = self.map.get_value(x1, y0) as f32;
        let topleft = self.map.get_value(x0, y1) as f32;
        let topright = self.map.get_value(x1, y1) as f32;

        let bottom = bottomleft * (1.0 - fx) + bottomright * fx;
        let top = topleft * (1.0 - fx) + topright * fx;
        let value = bottom * (1.0 - fy) + top * fy;

        value
    }
}

pub struct WaveDataBindGroup {}

impl WaveDataBindGroup {
    pub fn get_layout_descriptor() -> jandering_engine::bind_group::BindGroupLayoutDescriptor {
        BindGroupLayoutDescriptor {
            entries: vec![BindGroupLayoutDescriptorEntry::Data { is_uniform: true }],
        }
    }
}
