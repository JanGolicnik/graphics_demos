use jandering_engine::{
    shader::{BufferLayout, BufferLayoutEntry, BufferLayoutEntryDataType},
    types::Mat4,
};

#[repr(C)]
#[derive(Copy, Debug, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HistoryInstance {
    pub model: Mat4,
    pub inv_model: Mat4,
    pub prev_model: Mat4,
}

impl Default for HistoryInstance {
    fn default() -> Self {
        Self {
            model: Mat4::IDENTITY,
            inv_model: Mat4::IDENTITY,
            prev_model: Mat4::IDENTITY,
        }
    }
}

impl HistoryInstance {
    pub fn desc() -> BufferLayout {
        BufferLayout {
            step_mode: jandering_engine::shader::BufferLayoutStepMode::Instance,
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
                BufferLayoutEntry {
                    location: 9,
                    data_type: BufferLayoutEntryDataType::Float32x4,
                },
                BufferLayoutEntry {
                    location: 10,
                    data_type: BufferLayoutEntryDataType::Float32x4,
                },
                BufferLayoutEntry {
                    location: 11,
                    data_type: BufferLayoutEntryDataType::Float32x4,
                },
                BufferLayoutEntry {
                    location: 12,
                    data_type: BufferLayoutEntryDataType::Float32x4,
                },
                BufferLayoutEntry {
                    location: 13,
                    data_type: BufferLayoutEntryDataType::Float32x4,
                },
                BufferLayoutEntry {
                    location: 14,
                    data_type: BufferLayoutEntryDataType::Float32x4,
                },
                BufferLayoutEntry {
                    location: 15,
                    data_type: BufferLayoutEntryDataType::Float32x4,
                },
                BufferLayoutEntry {
                    location: 16,
                    data_type: BufferLayoutEntryDataType::Float32x4,
                },
            ],
        }
    }
}
