struct Camera {
    up: vec3<f32>,
    right: vec3<f32>,
    position: vec3<f32>,
    direction: vec3<f32>,
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput{
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct InstanceInput{
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,

    @location(9)  inv_model_matrix_0: vec4<f32>,
    @location(10) inv_model_matrix_1: vec4<f32>,
    @location(11) inv_model_matrix_2: vec4<f32>,
    @location(12) inv_model_matrix_3: vec4<f32>,
}

struct VertexOutput{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) clip_position_raw: vec4<f32>,
    @location(3) @interpolate(flat) random_value: u32,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
    @builtin(vertex_index) index: u32
) -> VertexOutput{

    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    let inv_model_matrix = mat4x4<f32>(
        instance.inv_model_matrix_0,
        instance.inv_model_matrix_1,
        instance.inv_model_matrix_2,
        instance.inv_model_matrix_3,
    );

    let world_position = model_matrix * vec4<f32>(model.position, 1.0);
    let normal = transpose(inv_model_matrix) * vec4<f32>(model.normal, 1.0);    

    var out: VertexOutput;
    out.clip_position_raw = camera.view_proj * world_position;
    out.clip_position = out.clip_position_raw;
    out.normal = normalize(normal.xyz);
    out.uv = model.uv;

    out.random_value = pcg_hash(index);
    
    return out;
}

const COLORS: array<vec3<f32>, 10> = array<vec3<f32>, 10>(
    vec3<f32>(0.984, 0.592, 0.794),
    vec3<f32>(0.727, 0.941, 0.557),
    vec3<f32>(0.692, 0.778, 0.945),
    vec3<f32>(0.957, 0.771, 0.502),
    vec3<f32>(0.973, 0.616, 0.775),
    vec3<f32>(0.790, 0.941, 0.588),
    vec3<f32>(0.782, 0.696, 0.957),
    vec3<f32>(0.996, 0.837, 0.498),
    vec3<f32>(0.653, 0.751, 0.941),
    vec3<f32>(0.786, 0.696, 0.649) 
);

fn pcg_hash(s: u32) -> u32
{
    var state = s * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    state = (word >> 22u) ^ word; 
    return state;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>{
    let light_dir = vec3<f32>(-1.0);

    var warm = vec3<f32>(1.0);
    switch(in.random_value % 10u){
        case 0u: {
            warm = COLORS[0];
        }
        case 1u: {
            warm = COLORS[1];
        }
        case 2u: {
            warm = COLORS[2];
        }
        case 3u: {
            warm = COLORS[3];
        }
        case 4u: {
            warm = COLORS[4];
        }
        case 5u: {
            warm = COLORS[5];
        }
        case 6u: {
            warm = COLORS[6];
        }
        case 7u: {
            warm = COLORS[7];
        }
        case 8u: {
            warm = COLORS[8];
        }
        case 9u: {
            warm = COLORS[9];
        }
        default: {
            warm = vec3<f32>(1.0); // Fallback to white if needed
        }
    }

    let cool = vec3<f32>(0.2, 0.5, 1.0);

    let d = dot(light_dir, in.normal) * 0.5 + 0.5;
    let color = warm * (1.0 - d) + cool * d; 

    return vec4<f32>(color, 1.0);
}
