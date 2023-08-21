#version 450

layout(location = 0) in vec4 in_frag_color;

layout(location = 0) out vec4 out_frag_color;

void main() {
    out_frag_color = in_frag_color;
}