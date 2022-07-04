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
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOutput {
    var result: VertexOutput;
    result.position = position;
    result.color = color;
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vertex.color;
}