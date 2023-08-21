#version 450

layout (location = 0) in vec3 position;

layout (location = 1) in vec3 normal;
layout (location = 2) in vec4 tangent;
layout (location = 3) in vec4 uv1_uv2;
layout (location = 4) in vec4 color;

layout(location = 0) out vec4 frag_color;

void main() {
    gl_Position = vec4(position, 1.0);
    frag_color = color;
}