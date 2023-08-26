#version 450

layout (location = 0) in mat3 tangent_space_matrix;
layout (location = 3) in vec2 frag_uv1;
layout (location = 4) in vec2 frag_uv2;
layout (location = 5) in vec4 frag_color;

layout(location = 0) out vec4 out_frag_color;

void main() {
    out_frag_color = frag_color;
}