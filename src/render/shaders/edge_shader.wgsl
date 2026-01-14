// Vertex shader for outline rendering
struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(1) @binding(0) // 1.
var<uniform> camera: CameraUniform;

// Uniform for controlling edge thickness
struct EdgeParams {
    thickness: f32,
};
@group(2) @binding(0)
var<uniform> edge_params: EdgeParams;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
};

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) world_normal: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.world_pos = (model_matrix * vec4<f32>(model.position, 1.0)).xyz;
    out.world_normal = (model_matrix * vec4<f32>(model.normal, 0.0)).xyz;

    // Apply view-projection transformation
    let world_pos_homogeneous = vec4<f32>(out.world_pos, 1.0);
    out.clip_position = camera.view_proj * world_pos_homogeneous;

    // Slightly offset to avoid z-fighting with the main model
    out.world_pos.z += 0.00001;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // black
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
