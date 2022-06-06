#version 450

layout(location = 0) in vec2 in_uv;

layout(location = 0) out vec4 out_color;

layout(binding = 2) uniform texture2D sampled_textures[];
layout(binding = 3) uniform sampler samplers[];

void main() {
    out_color = vec4(in_uv, 0.0, 0.0);
    out_color = texture(sampler2D(sampled_textures[1], samplers[0]), in_uv);
}