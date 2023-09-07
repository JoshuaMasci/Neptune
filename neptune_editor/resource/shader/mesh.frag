#version 450
#extension GL_EXT_nonuniform_qualifier : require

layout (location = 0) in mat3 tangent_space_matrix;
layout (location = 3) in vec2 frag_uv1;
layout (location = 4) in vec2 frag_uv2;
layout (location = 5) in vec4 frag_color;

layout(location = 0) out vec4 out_frag_color;

struct SampledImageBinding {
    uint binding_index;
};

uint get_index(SampledImageBinding binding) {
    return binding.binding_index & 0xFFFF;
}

layout(set = 0, binding = 1) uniform sampler2D combined_image_samplers[];
vec4 sample_texture(SampledImageBinding binding, vec2 uv) {
    uint index = get_index(binding);
    return texture(combined_image_samplers[index], uv);
}

layout(push_constant) uniform PushConstants
{
    mat4 view_projection_matrix;
    mat4 model_matrix;
    SampledImageBinding albedo_texture;
} push_constants;

void main() {
    out_frag_color = frag_color * sample_texture(push_constants.albedo_texture, frag_uv1);
}