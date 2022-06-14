#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 inFragColor;
layout(location = 1) in vec2 inTextCord;

layout(location = 0) out vec4 outColor;

void main() {
    outColor = vec4(inTextCord.x, inTextCord.y, 0.5f, 1.0f);
//    outColor = vec4(inFragColor, 1.0);
}
