@group(0) @binding(0)
var tex: texture_2d<f32>;
@group(0) @binding(1)
var tex_sampler: sampler;

@group(1) @binding(0)
var prev_tex: texture_2d<f32>;
@group(1) @binding(1)
var prev_tex_sampler: sampler;

@group(2) @binding(0)
var world_position_tex: texture_storage_2d<rg32float, read_write>;

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
    @location(1) normal: vec3<f32>,
    @location(0) uv: vec2<f32>,
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
    out.clip_position = world_position;
    out.normal = normalize(normal.xyz);
    out.uv = model.uv;
    
    return out;
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>{
    var color = textureSample(tex, tex_sampler, in.uv);

    //do motion blur
    let world_position_tex_size = textureDimensions(world_position_tex);
    let t = vec2<u32>(u32(in.uv.x * f32(world_position_tex_size.x)), u32(in.uv.y * f32(world_position_tex_size.y)));
    var velocity = textureLoad(world_position_tex, t) * 2.0;
    velocity.y = -velocity.y;

    let n_samples = 10;
    for (var i = 1; i <= n_samples; i += 1){
        let t = f32(i) / f32(n_samples);
        color += textureSample(prev_tex, prev_tex_sampler, in.uv - velocity.xy * t);
    }
    color /= f32(n_samples);

    // return vec4<f32>(velocity, 0.0, 1.0);
    return vec4<f32>(color.rgb, 1.0);
}