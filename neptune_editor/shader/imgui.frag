#version 460

layout(location = 0) in vec2 in_uv;
layout(location = 1) in vec4 in_color;

layout(location = 0) out vec4 out_color;

//TODO: bindless textures

void main()
{
    out_color = in_color; //* vec4(1.0, 1.0, 1.0, texture(sampled_texture[push_data.texture_index], in_uv).r);
}