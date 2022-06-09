#version 460

layout(location = 0) in vec2 in_position;
layout(location = 1) in vec2 in_uv;
layout(location = 2) in vec4 in_color;

layout(location = 0) out vec2 out_uv;
layout(location = 1) out vec4 out_color;

layout(push_constant) uniform PushData
{
	vec4 scale_translate;
	uint texture;
} push_data;

void main()
{
	vec2 scale = push_data.scale_translate.xy;
	vec2 translate = push_data.scale_translate.zw;

	gl_Position = vec4((in_position  * scale) + translate, 0.0, 1.0);
	out_uv = in_uv;
	out_color = in_color;
}