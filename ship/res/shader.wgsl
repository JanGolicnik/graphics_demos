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
    @location(2) world_position: vec4<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput
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
    out.clip_position = camera.view_proj * world_position;
    out.normal = normalize(normal.xyz);
    out.uv = model.uv;
    out.world_position = world_position;
    
    return out;
}

@fragment
fn fs_shadow_main(in: VertexOutput) {}

fn light_direction() -> vec3<f32> {
    return -normalize(vec3<f32>(-70.0, 60.0, -70.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>{
    let warm = vec3<f32>(1.0, 0.77, 0.34);
    let cool = vec3<f32>(0.25, 0.14, 0.67);

    let d = dot(light_direction(), in.normal) * 0.5 + 0.5;
    let color = warm * (1.0 - d) + cool * d; 

    return vec4<f32>(color, 1.0);
}

@fragment
fn fs_cube(in: VertexOutput) -> @location(0) vec4<f32>{
    return vec4<f32>(vec3<f32>(0.001, 0.002, 0.003) * in.normal * length(in.uv)* length(in.uv)* length(in.uv)* length(in.uv) * 2.0, 1.0);
}