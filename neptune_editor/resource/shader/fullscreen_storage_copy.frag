#version 460
#extension GL_EXT_nonuniform_qualifier : require

layout(location = 0) out vec4 out_frag_color;

layout(set = 0, binding = 1, rgba8) uniform readonly image2D storage_images[];
struct StorageImageBinding {
    uint binding_index;
};
uint get_image_index(StorageImageBinding binding) {
    return binding.binding_index & 0xFFFF;
}

layout(push_constant) uniform PushConstants
{
    StorageImageBinding input_image_binding;
} push_constants;

void main() {
    vec4 color = imageLoad(storage_images[get_image_index(push_constants.input_image_binding)],  ivec2(gl_FragCoord.xy));
    float grey_scale = (0.299 * color.x) + (0.587 * color.y) + (0.114 * color.z);
    out_frag_color = vec4(grey_scale, grey_scale, grey_scale, color.w);
}