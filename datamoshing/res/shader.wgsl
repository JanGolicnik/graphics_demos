struct Camera {
    up: vec3<f32>,
    right: vec3<f32>,
    position: vec3<f32>,
    direction: vec3<f32>,
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var world_position_tex: texture_storage_2d<rgba32float, read_write>;

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
    @location(3) position: vec3<f32>,
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
    out.clip_position_raw = camera.view_proj * world_position;
    out.clip_position = out.clip_position_raw;
    out.normal = normalize(normal.xyz);
    out.uv = model.uv;
    out.position = world_position.xyz;
    
    return out;
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>{
    {
        let texture_size = textureDimensions(world_position_tex);
        var fbc = (in.clip_position_raw.xy / in.clip_position_raw.w) * 0.5 + 0.5;
        fbc.y = 1.0 - fbc.y;
        let depth = in.clip_position_raw.z / in.clip_position_raw.w;
        let t = fbc * vec2<f32>(f32(texture_size.x), f32(texture_size.y));
        
        // zkj rabmo to ?
        let current_value = textureLoad(world_position_tex, vec2<u32>(u32(t.x), u32(t.y)));
        if (current_value.x == 0.0 && current_value.y == 0.0 && current_value.z == 0.0 ) || current_value.w > depth 
        {
            textureStore(world_position_tex, vec2<u32>(u32(t.x), u32(t.y)), vec4<f32>(in.position.xyz, depth));
        }
    }

    let light_dir = vec3<f32>(-1.0);

    let warm = vec3<f32>(1.0, 0.8, 0.5);
    let cool = vec3<f32>(0.2, 0.5, 1.0);

    let d = dot(light_dir, in.normal) * 0.5 + 0.5;
    let color = warm * (1.0 - d) + cool * d; 

    return vec4<f32>(color, 1.0);
}