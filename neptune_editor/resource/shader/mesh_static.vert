#version 450

layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec4 tangent;
layout (location = 3) in vec4 uv1_uv2;
layout (location = 4) in vec4 color;

layout (location = 0) out mat3 tangent_space_matrix;
layout (location = 3) out vec2 frag_uv1;
layout (location = 4) out vec2 frag_uv2;
layout (location = 5) out vec4 frag_color;

layout(push_constant) uniform PushConstants
{
    mat4 view_projection_matrix;
    mat4 model_matrix;
} push_constants;

void main() {
    mat4 mvp_matrix = push_constants.view_projection_matrix * push_constants.model_matrix;
    gl_Position = mvp_matrix * vec4(position, 1.0);

    mat3 normal_matrix = mat3(push_constants.model_matrix);
    vec3 world_normal = normalize(normal_matrix * normal);
    vec3 world_tangent = normalize(normal_matrix * tangent.xyz);
    vec3 world_bitangent = cross(  world_normal, world_tangent ) * tangent.w;
    tangent_space_matrix = mat3(world_tangent, world_bitangent, world_normal);

    frag_uv1 = uv1_uv2.xy;
    frag_uv2 = uv1_uv2.zw;
    frag_color = color;
}