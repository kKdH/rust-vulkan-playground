#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(binding = 0) uniform UniformBufferObject {
    mat4 mvp;
} ubo;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;

layout(location = 2) in mat4 transformation; // consumes location 2, 3, 4, 5

layout(location = 0) out vec3 outColor;

void main() {

    gl_Position = ubo.mvp * transformation * vec4(inPosition, 1.0);
    outColor = inColor;
}
