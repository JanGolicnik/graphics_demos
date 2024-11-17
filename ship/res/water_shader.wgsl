struct Camera {
    up: vec3<f32>,
    right: vec3<f32>,
    position: vec3<f32>,
    direction: vec3<f32>,
    view_proj: mat4x4<f32>,
};

struct WaveData {
    time: f32,
    strength: f32,
    speed: f32,
    direction: f32,
    world_position: vec3<f32>,
    ocean_size_meters: f32,
    noise_scale: f32,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var noise_tex: texture_2d<f32>;
@group(1) @binding(1)
var noise_tex_sampler: sampler;

@group(2) @binding(0)
var<uniform> wave_data: WaveData;

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
    // @location(1) normal: vec3<f32>,
    @location(2) world_position: vec4<f32>,
    @location(3) @interpolate(flat) should_discard: f32,
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

    let epsilon = 0.001;

    var world_position = model_matrix * vec4<f32>(model.position, 1.0);

    // let normalized_world_pos = fract((world_position.xz - wave_data.world_position.xz)  / wave_data.ocean_size_meters + 0.5);
    // world_position.x = (normalized_world_pos.x - 0.5) * wave_data.ocean_size_meters;
    // world_position.z = (normalized_world_pos.y - 0.5) * wave_data.ocean_size_meters;

    let noise_uv = fract(world_position.xz / wave_data.noise_scale) + wave_data.time;

    let normalized_world_pos = fract((world_position.xz - wave_data.world_position.xz) / wave_data.ocean_size_meters + 0.5) - 0.5;
    world_position.x = normalized_world_pos.x * wave_data.ocean_size_meters;
    world_position.z = normalized_world_pos.y * wave_data.ocean_size_meters;

    var heightCenter = 0.0;
    for (var i = 0; i < 5; i++){
        let uv = noise_uv * f32(i + 1);
        let height = textureSampleLevel(noise_tex, noise_tex_sampler, uv, 0.0).r;
        // let heightRight = textureSampleLevel(noise_tex, noise_tex_sampler, uv + vec2<f32>(epsilon, 0.0), 0.0).r;
        // let heightUp = textureSampleLevel(noise_tex, noise_tex_sampler, uv + vec2<f32>(0.0, epsilon), 0.0).r;

        // let dhdx = (heightRight - heightCenter) / epsilon;
        // let dhdy = (heightUp - heightCenter) / epsilon;

        // let normal = normalize(vec3<f32>(-dhdx, -dhdy, 1.0)) * 0.5 + 0.5;

        heightCenter += height / f32(pow(1.75, f32(i)));
    }

    heightCenter *= wave_data.strength;
    
    world_position.y += heightCenter * 0.4;
    world_position.x -= heightCenter * 0.2;
    world_position.z -= heightCenter * 0.2;

    var should_discard = 0.0;
    // let d2_world_position = vec4<f32>(world_position.x, 0.0, world_position.z, 1.0);
    // let d2_clip_position = camera.view_proj * d2_world_position;
    // if abs(d2_clip_position.x / d2_clip_position.w) > 1.0 ||
    //     abs(d2_clip_position.y / d2_clip_position.w) > 1.0 {
    //     should_discard = 1.0;
    // }

    if abs(normalized_world_pos.x * 2.0) > 0.9 || abs(normalized_world_pos.y * 2.0) > 0.9{
        should_discard = 1.0;
    }

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.uv = noise_uv;
    // out.uv = model.uv;
    out.world_position = world_position;
    out.should_discard = should_discard;
    
    return out;
}

@fragment
fn fs_shadow_main(in: VertexOutput) {}

fn light_direction() -> vec3<f32> {
    return -normalize(vec3<f32>(-70.0, 60.0, -70.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>{
    if in.should_discard != 0.0  {
        discard;
    }
    // let warm = vec3<f32>(1.0, 0.77, 0.34);
    let warm = vec3<f32>(1.0, 0.77, 0.34) * 1.5;
    let cool = vec3<f32>(0.25, 0.14, 0.67);

    let faceNormal = -normalize(cross(dpdx(in.world_position.xyz), dpdy(in.world_position.xyz)));

    // Diffuse
    var d = dot(light_direction(), faceNormal) * 0.5 + 0.5;
    d *= 3.05;
    let diffuse = warm * (1.0 - d) + cool * d; 

    // Specular
    let viewDir = normalize(camera.position - in.world_position.xyz);
    let reflectDir = reflect(light_direction(), faceNormal);
    let specular = pow(max(dot(viewDir, reflectDir), 0.0), 500.0);

    // let color = diffuse + warm * specular * 1.0;

    let color = diffuse;

    return vec4<f32>(color, 1.0);
    // return vec4<f32>(d);
    // return vec4<f32>(vec3<f32>(specular), 1.0);
    // return vec4<f32>(in.uv, 0.0, 1.0);
}


