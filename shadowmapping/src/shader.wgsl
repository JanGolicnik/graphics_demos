struct Camera {
    up: vec3<f32>,
    right: vec3<f32>,
    position: vec3<f32>,
    direction: vec3<f32>,
    view_proj: mat4x4<f32>,
};

struct Light {
    up: vec3<f32>,
    right: vec3<f32>,
    position: vec3<f32>,
    direction: vec3<f32>,
    view_proj: mat4x4<f32>,
    texture_size: vec2<u32>,
    fov: f32
};

@group(0) @binding(0)
var<uniform> light: Light;
@group(0) @binding(1)
var light_map_tex: texture_depth_2d;
@group(0) @binding(2)
var light_tex_sampler: sampler;

@group(1) @binding(0)
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
    @location(3) light_space_position: vec4<f32>,
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
    out.light_space_position = light.view_proj * world_position;
    
    return out;
}

@fragment
fn fs_shadow_main(in: VertexOutput) {}

fn light_direction() -> vec3<f32> {
    return -normalize(light.position);
}

fn is_in_light(world_position: vec4<f32>) -> bool {
    let to_light = world_position.xyz - light.position;
    
    let fov_rad = light.fov * 0.0174532925;
    let ddd = dot(normalize(to_light), normalize(light.direction));
    return acos(ddd) < fov_rad / 2.0;
}

fn get_shadow_amount(world_position: vec4<f32>, position: vec4<f32>, normal: vec3<f32>) -> f32 {
    let camera_space_pos = position;
    var clip_pos = camera_space_pos.xyz / camera_space_pos.w;
    let normalized_clip_pos = clip_pos.xy * 0.5 + 0.5;

    var avg_sampled_depth = 0.0;
    let tex_d = vec2<f32>(1.0) / vec2<f32>(f32(light.texture_size.x), f32(light.texture_size.y));
    for (var i = -1; i <= 1; i++) {
        for (var j = -1; j <= 1; j++) {
            let uv = vec2<f32>(normalized_clip_pos.x, 1.0 - normalized_clip_pos.y);
            let diff = vec2<f32>(f32(i), f32(j)) * tex_d * 2.0;
            avg_sampled_depth += textureSample(light_map_tex, light_tex_sampler, uv + diff);
        }
    }
    let sampled_depth = avg_sampled_depth / 9.0;
    // let sampled_depth = textureSample(light_map_tex, light_tex_sampler, vec2<f32>(normalized_clip_pos.x, 1.0 - normalized_clip_pos.y));

    let actual_depth = clip_pos.z;

    let constant_bias = 0.00000015;

    let slope_bias_factor = 0.0000002;
    let d = max(dot(light_direction(), normal), 0.0);
    let slope_bias = slope_bias_factor * d;

    // let bias = constant_bias;
    let bias = constant_bias + slope_bias;
    if actual_depth - bias > sampled_depth {
        return 1.0;
    }

    return 0.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>{
    if !is_in_light(in.world_position){
        discard;
    }

    let shadow = get_shadow_amount(in.world_position, in.light_space_position, in.normal);

    let warm = vec3<f32>(1.0, 0.77, 0.34);
    let cool = vec3<f32>(0.25, 0.14, 0.67);

    let d = dot(light_direction(), in.normal) * 0.5 + 0.5;
    let weight_with_shadow = min(d + shadow, 1.0);
    let color = warm * (1.0 - weight_with_shadow) + cool * weight_with_shadow; 

    return vec4<f32>(color, 1.0);
}


