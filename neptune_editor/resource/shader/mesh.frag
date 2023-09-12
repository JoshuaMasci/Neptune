#version 460
#extension GL_EXT_nonuniform_qualifier : require

layout (location = 0) in mat3 tangent_space_matrix;
layout (location = 3) in vec2 frag_uv1;
layout (location = 4) in vec2 frag_uv2;
layout (location = 5) in vec4 frag_color;

layout(location = 0) out vec4 out_frag_color;

layout(set = 0, binding = 2) uniform texture2D sampled_images[];
struct SampledImageBinding {
    uint binding_index;
};
uint get_image_index(SampledImageBinding binding) {
    return binding.binding_index & 0xFFFF;
}

layout(set = 0, binding = 3) uniform sampler samplers[];
struct SamplerBinding {
    uint binding_index;
};
uint get_sampler_index(SamplerBinding binding) {
    return binding.binding_index & 0xFFFF;
}

vec4 sample_image(SampledImageBinding image_binding, SamplerBinding sampler_binding, vec2 uv) {
    uint image_index = get_image_index(image_binding);
    uint sampler_index = get_sampler_index(sampler_binding);
    return texture(sampler2D(sampled_images[image_index], samplers[sampler_index]), uv);
}

layout(push_constant) uniform PushConstants
{
    mat4 view_projection_matrix;
    mat4 model_matrix;
    SamplerBinding image_sampler;
    SampledImageBinding albedo_texture;
} push_constants;

void main() {
    out_frag_color = frag_color * sample_image(push_constants.albedo_texture, push_constants.image_sampler, frag_uv1);
}