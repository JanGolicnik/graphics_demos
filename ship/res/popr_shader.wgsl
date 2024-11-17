struct PoprConfig {
    resolution: vec2<f32>,
    screenspace_cube_position: vec2<f32>,
    time: f32
}

@group(0) @binding(0)
var tex: texture_2d<f32>;
@group(0) @binding(1)
var tex_sampler: sampler;

@group(1) @binding(0)
var<uniform> popr_config: PoprConfig;

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
    // return vec4<f32>(popr_config.screenspace_cube_position, 0.0, 1.0);

    var uv = in.uv * 2.0 - 1.0;
    let aspect = popr_config.resolution.x / popr_config.resolution.y;
    uv = vec2<f32>(uv.x * aspect, uv.y);

    var cube_effect = length(uv - popr_config.screenspace_cube_position * 2.0) * 100.0;
    cube_effect = 1.0 / cube_effect;

    let color_aberration = 0.01 * min(cube_effect * cube_effect * 5000.0, 1.0) * (sin(cos(sin(cos(popr_config.time * 5.0) * 3.0) * 5.0) * 4.0) * 0.3 + 1.0);
    var color = vec3<f32>(textureSample(tex, tex_sampler, in.uv - color_aberration).r, 
                          textureSample(tex, tex_sampler, in.uv).g,
                          textureSample(tex, tex_sampler, in.uv + color_aberration).b);
    // var color = textureSample(tex, tex_sampler, in.uv);

    // color *= clamp(cube_effect, 0.0, 1.0);

    return vec4<f32>(color, 1.0);
    // return vec4<f32>(vec3<f32>(cube_effect), 1.0);
}