struct SceneMatrices {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    instances: array<mat4x4<f32>, 16>,
};

@group(0) @binding(0)
var<uniform> scene_matrices: SceneMatrices;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @builtin(instance_index) index: u32
) -> VertexOutput {
    var mvp_matrix: mat4x4<f32> = scene_matrices.projection * scene_matrices.view * scene_matrices.instances[index];
    var result: VertexOutput;
    result.position = mvp_matrix * vec4<f32>(position, 1.0);
    result.color = vec4<f32>(uv, 0.0, 1.0);
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return abs(vertex.color);
}