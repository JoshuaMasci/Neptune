#version 450
#extension GL_EXT_nonuniform_qualifier : require

layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec4 tangent;
layout (location = 3) in vec4 uv1_uv2;
layout (location = 4) in vec4 color;

layout (location = 0) out mat3 tangent_space_matrix;
layout (location = 3) out vec2 frag_uv1;
layout (location = 4) out vec2 frag_uv2;
layout (location = 5) out vec4 frag_color;

layout(std140, set = 0, binding = 0) readonly buffer Some{
	mat4 view_projection_matrix;
} Matrices[];

layout(push_constant) uniform PushConstants
{
    uint view_projection_matrix_index;
} push_constants;

void main() {
    mat4 model_matrix = mat4(1.0);
    mat4 mvp_matrix = Matrices[push_constants.view_projection_matrix_index].view_projection_matrix * model_matrix;
    gl_Position = mvp_matrix * vec4(position, 1.0);

    mat3 normal_matrix = mat3(model_matrix);
    vec3 world_normal = normalize(normal_matrix * normal);
    vec3 world_tangent = normalize(normal_matrix * tangent.xyz);
    vec3 world_bitangent = cross(  world_normal, world_tangent ) * tangent.w;
    tangent_space_matrix = mat3(world_tangent, world_bitangent, world_normal);

    frag_uv1 = uv1_uv2.xy;
    frag_uv2 = uv1_uv2.zw;
    frag_color = color;
}