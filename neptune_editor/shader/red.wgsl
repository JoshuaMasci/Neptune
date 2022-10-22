struct Camera {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Mesh {
    model: mat4x4<f32>,
}

@group(1) @binding(0)
var<uniform> mesh: Mesh;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> VertexOutput {
    var mvp_matrix: mat4x4<f32> = camera.projection * camera.view * mesh.model;
    var result: VertexOutput;
    result.position = mvp_matrix * vec4<f32>(position, 1.0);
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.1, 0.1, 1.0);
}