#version 450

layout (location = 0) in vec3 vertex_position;
layout (location = 4) in vec4 vertex_color;

layout(location = 0) out vec4 frag_color;

void main() {
    gl_Position = vec4(vertex_position, 1.0);
    frag_color = vertex_color;
}