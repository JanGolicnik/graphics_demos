use std::collections::HashSet;

use jandering_engine::{
    bind_group::{
        BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutDescriptorEntry,
        BindGroupLayoutEntry,
    },
    object::Renderable,
    renderer::{BindGroupHandle, BufferHandle, Janderer, Renderer},
    shader::{BufferLayout, BufferLayoutEntry, BufferLayoutEntryDataType, BufferLayoutStepMode},
    types::{Mat4, Qua, UVec4, Vec2, Vec3, Vec4},
    utils::load_binary,
};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug, Default)]
pub struct AnimatedVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub weights: Vec4,
    pub joints: UVec4,
}

impl AnimatedVertex {
    pub fn desc() -> BufferLayout {
        BufferLayout {
            step_mode: BufferLayoutStepMode::Vertex,
            entries: &[
                BufferLayoutEntry {
                    location: 0,
                    data_type: BufferLayoutEntryDataType::Float32x3,
                },
                BufferLayoutEntry {
                    location: 1,
                    data_type: BufferLayoutEntryDataType::Float32x3,
                },
                BufferLayoutEntry {
                    location: 2,
                    data_type: BufferLayoutEntryDataType::Float32x2,
                },
                BufferLayoutEntry {
                    location: 3,
                    data_type: BufferLayoutEntryDataType::Float32x4,
                },
                BufferLayoutEntry {
                    location: 4,
                    data_type: BufferLayoutEntryDataType::U32x4,
                },
            ],
        }
    }
}

#[derive(Debug)]
pub enum Keyframes {
    Rotations(Vec<Qua>),
    Other,
}

#[derive(Debug)]
pub struct Animation {
    pub timestamps: Vec<f32>,
    pub target: usize,
    pub keyframes: Keyframes,
}

#[derive(Debug)]
pub struct AnimatedObjectRenderData {
    pub vertex_buffer: BufferHandle,
    pub index_buffer: BufferHandle,
    pub instance_buffer: BufferHandle,
}

#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<AnimatedVertex>,
    pub indices: Vec<u32>,
    pub render_data: AnimatedObjectRenderData,
}

#[derive(Debug)]
pub enum NodeType {
    Mesh { mesh: usize },
    Generic,
}

#[derive(Debug)]
pub struct Node {
    node_type: NodeType,
    transform: Mat4,
    world_transform: Mat4,
    children: Vec<usize>,
}

#[derive(Debug)]
pub struct AnimatedObject {
    pub meshes: Vec<Mesh>,
    pub animations: Vec<Animation>,
    pub nodes: Vec<Node>,

    joints: Vec<usize>,

    animation_start_time: std::time::Instant,

    joint_buffer: BufferHandle,
    pub joint_data_bind_group: BindGroupHandle,

    parentless_nodes: HashSet<usize>
}

impl AnimatedObject {
    pub async fn from_gltf(renderer: &mut Renderer, path: &'static str) -> Self {
        let data = load_binary(jandering_engine::utils::FilePath::FileName(path))
            .await
            .unwrap();
        let gltf = gltf::Gltf::from_slice(&data).unwrap();

        let mut buffers = Vec::new();
        for buffer in gltf.buffers() {
            match buffer.source() {
                gltf::buffer::Source::Uri(filename) => {
                    buffers.push(
                        load_binary(jandering_engine::utils::FilePath::FileName(unsafe {
                            std::mem::transmute::<&str, &'static str>(filename)
                        }))
                        .await
                        .unwrap(),
                    );
                }
                gltf::buffer::Source::Bin => {
                    buffers.push(gltf.blob.as_deref().unwrap().into());
                }
            }
        }

        let mut meshes = Vec::new();

        for mesh in gltf.meshes() {
            let primitives = mesh.primitives();

            let mut vertices = Vec::new();
            let mut indices = Vec::new();

            primitives.for_each(|primitive| {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                if let Some(positions) = reader.read_positions() {
                    for position in positions {
                        vertices.push(AnimatedVertex {
                            position: Vec3::from_array(position),
                            ..Default::default()
                        })
                    }
                }

                if let Some(normals) = reader.read_normals() {
                    for (i, normal) in normals.into_iter().enumerate() {
                        vertices[i].normal = Vec3::from_array(normal);
                    }
                }

                if let Some(uvs) = reader.read_tex_coords(0).map(|v| v.into_f32()) {
                    for (i, uv) in uvs.into_iter().enumerate() {
                        vertices[i].uv = uv.into();
                    }
                }

                if let Some(weights) = reader.read_weights(0) {
                    for (i, weight) in weights.into_f32().into_iter().enumerate() {
                        vertices[i].weights = weight.into();
                    }
                }

                if let Some(joints) = reader.read_joints(0) {
                    for (i, joint) in joints.into_u16().into_iter().enumerate() {
                        vertices[i].joints = [
                                joint[0] as u32,
                                joint[1] as u32,
                                joint[2] as u32,
                                joint[3] as u32,
                        ].into();
                    }
                }

                if let Some(indices_raw) = reader.read_indices() {
                    indices.append(&mut indices_raw.into_u32().collect::<Vec<u32>>());
                }
            });

            let render_data = AnimatedObjectRenderData {
                vertex_buffer: renderer.create_vertex_buffer(bytemuck::cast_slice(&vertices)),
                instance_buffer: renderer
                    .create_vertex_buffer(bytemuck::cast_slice(&[Mat4::IDENTITY])),
                index_buffer: renderer.create_index_buffer(bytemuck::cast_slice(&indices)),
            };
            meshes.push(Mesh {
                vertices,
                indices,
                render_data,
            })
        }

        let mut animations = Vec::new();
        for animation in gltf.animations() {
            for channel in animation.channels() {
                let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));
                let timestamps =
                    if let Some(gltf::accessor::Iter::Standard(times)) = reader.read_inputs() {
                        times.collect::<Vec<_>>()
                    } else {
                        panic!("unsupported {:?}", reader.read_inputs());
                    };

                let keyframes = if let Some(outputs) = reader.read_outputs() {
                    match outputs {
                        gltf::animation::util::ReadOutputs::Rotations(rotations) => {
                            let rotations = rotations
                                .into_f32()
                                .map(|rotation| {
                                    Qua::from_xyzw(
                                        rotation[0],
                                        rotation[1],
                                        rotation[2],
                                        rotation[3],
                                    )
                                })
                                .collect::<Vec<_>>();
                            dbg!(rotations.len());
                            Keyframes::Rotations(rotations)
                        }
                        gltf::animation::util::ReadOutputs::Translations(_) => Keyframes::Other,
                        gltf::animation::util::ReadOutputs::Scales(_) => Keyframes::Other,
                        gltf::animation::util::ReadOutputs::MorphTargetWeights(_) => {
                            Keyframes::Other
                        }
                    }
                } else {
                    panic!()
                };

                animations.push(Animation {
                    timestamps,
                    target: channel.target().node().index(),
                    keyframes,
                })
            }
        }

        let mut nodes = Vec::new();
        let mut parentless_nodes = HashSet::new();
        for node in gltf.nodes() {
            let node_type = if let Some(mesh) = node.mesh() {
                NodeType::Mesh { mesh: mesh.index() }
            } else {
                NodeType::Generic
            };

            let transform = node.transform().matrix();

            let children = node.children().map(|n| n.index()).collect();

            parentless_nodes.insert(nodes.len());
            nodes.push(Node {
                node_type,
                transform: Mat4::from_cols_array_2d(&transform),
                world_transform: Mat4::IDENTITY,
                children,
            });
        }

        for node in nodes.iter() {
            for child in node.children.iter(){
                parentless_nodes.remove(child);
            }
        }

        let mut joints = Vec::new();
        for skin in gltf.skins() {
            joints.append(&mut skin.joints().map(|n| n.index()).collect::<Vec<_>>());
            break;
        }

        let joint_matrices = joints
            .iter()
            .map(|joint| nodes[*joint].transform)
            .collect::<Vec<_>>();
        let joint_buffer = renderer.create_storage_buffer(bytemuck::cast_slice(&joint_matrices));
        let joint_data_bind_group = renderer.create_bind_group(BindGroupLayout {
            entries: vec![BindGroupLayoutEntry::Data(joint_buffer)],
        });
dbg!(&parentless_nodes);
        Self {
            nodes,
            meshes,
            animations,
            joints,
            animation_start_time: std::time::Instant::now(),
            joint_buffer,
            joint_data_bind_group,
            parentless_nodes
        }
    }

    pub fn update(&mut self, renderer: &mut Renderer, dt: f32) {
        let animation_time = self.animation_start_time.elapsed().as_secs_f32();
        if animation_time > 2.0 {
            self.animation_start_time = std::time::Instant::now();
        }
        for animation in self.animations.iter() {
            let mut current_keyframe = 0;
            for (i, timestamp) in animation.timestamps.iter().enumerate() {
                if *timestamp > animation_time {
                    break;
                }
                current_keyframe = i;
            }
            match &animation.keyframes {
                Keyframes::Rotations(rotations) => {
                    let rotation = rotations[current_keyframe];
                    let node = &mut self.nodes[animation.target];
                    let (scale, _, translation) = node.transform.to_scale_rotation_translation();
                    node.transform =
                        Mat4::from_scale_rotation_translation(scale, rotation, translation)
                }
                _ => {}
            }
        }

        for node in self.parentless_nodes.clone().iter() {
            self.recurse_world_matrix(*node, Mat4::IDENTITY);
        }

        for node in self.nodes.iter() {
            if let NodeType::Mesh { mesh } = node.node_type {
                renderer.write_buffer(
                    self.meshes[mesh].render_data.instance_buffer,
                    bytemuck::cast_slice(&[node.world_transform]),
                );
            }
        }

        // fill up joints array
        let joint_matrices = self
            .joints
            .iter()
            .map(|joint| self.nodes[*joint].world_transform)
            .collect::<Vec<_>>();
        //dbg!(&joint_matrices[1]);
        renderer.write_buffer(self.joint_buffer, bytemuck::cast_slice(&joint_matrices));
    }

    pub fn joint_data_bind_group_layout_descriptor() -> BindGroupLayoutDescriptor {
        BindGroupLayoutDescriptor {
            entries: vec![BindGroupLayoutDescriptorEntry::Data { is_uniform: false }],
        }
    }

    pub fn instance_desc() -> BufferLayout {
        BufferLayout {
            step_mode: BufferLayoutStepMode::Instance,
            entries: &[
                BufferLayoutEntry {
                    location: 5,
                    data_type: BufferLayoutEntryDataType::Float32x4,
                },
                BufferLayoutEntry {
                    location: 6,
                    data_type: BufferLayoutEntryDataType::Float32x4,
                },
                BufferLayoutEntry {
                    location: 7,
                    data_type: BufferLayoutEntryDataType::Float32x4,
                },
                BufferLayoutEntry {
                    location: 8,
                    data_type: BufferLayoutEntryDataType::Float32x4,
                },
            ],
        }
    }

    fn recurse_world_matrix(&mut self, node: usize, parent_matrix: Mat4){
        let node = &mut self.nodes[node];
        node.world_transform = parent_matrix * node.transform;
        let world = node.world_transform;
        for child in node.children.clone().iter() {
            self.recurse_world_matrix(*child, world);
        }
    }
}

impl Renderable for Mesh {
    fn num_instances(&self) -> u32 {
        1
    }

    fn num_indices(&self) -> u32 {
        self.indices.len() as u32
    }

    fn get_buffers(&self) -> (BufferHandle, BufferHandle, Option<BufferHandle>) {
        (
            self.render_data.vertex_buffer,
            self.render_data.index_buffer,
            Some(self.render_data.instance_buffer),
        )
    }
}
