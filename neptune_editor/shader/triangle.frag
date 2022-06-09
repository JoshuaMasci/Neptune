#version 450
#extension GL_EXT_nonuniform_qualifier : enable

layout(location = 0) in vec2 in_uv;

layout(location = 0) out vec4 out_color;

layout(binding = 2) uniform texture2D sampled_textures[];
layout(binding = 3) uniform sampler samplers[];

layout(push_constant) uniform PushData
{
    uint texture;
} push_data;

void main() {
    out_color = vec4(in_uv, 0.0, 0.0);
    out_color = texture(sampler2D(sampled_textures[push_data.texture], samplers[0]), in_uv);
}