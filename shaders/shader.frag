#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 inFragColor;
layout(location = 1) in vec2 inTextCord;

layout(set = 1, binding = 0) uniform sampler2D texture_sampler;

layout(location = 0) out vec4 outColor;

void main() {
    vec3 color = texture(texture_sampler, inTextCord).xyz;
    outColor = vec4(color, 1.0f);
//    outColor = vec4(inTextCord.x, inTextCord.y, 0.15f, 1.0f);
//    outColor = vec4(inFragColor, 1.0);
}
