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

void main() {
    //TODO: transform both pos and tsm to world space
    gl_Position = vec4(position, 1.0);

    vec3 bitangent = cross( normal, tangent.xyz ) * tangent.w;
    tangent_space_matrix = mat3(tangent.xyz, bitangent, normal);

    frag_uv1 = uv1_uv2.xy;
    frag_uv2 = uv1_uv2.zw;
    frag_color = color;
}