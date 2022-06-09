#version 460
#extension GL_EXT_nonuniform_qualifier : enable

layout(location = 0) in vec2 in_uv;
layout(location = 1) in vec4 in_color;

layout(location = 0) out vec4 out_color;

layout(binding = 2) uniform texture2D sampled_textures[];
layout(binding = 3) uniform sampler samplers[];

layout(push_constant) uniform PushData
{
    vec4 scale_translate;
    uint texture;
} push_data;

void main()
{
    out_color = in_color * vec4(1.0, 1.0, 1.0, texture(sampler2D(sampled_textures[push_data.texture], samplers[0]), in_uv).r);
}