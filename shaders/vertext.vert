#version 450
layout(location = 0) in vec3 position;
layout(location = 1) in vec2 tex_coord;
layout(location = 2) in vec3 position_offset;
layout(location = 3) in float scale;

layout(location = 0) out vec2 tex_coords;

layout(set = 0, binding = 0) uniform Data {
    mat4 mvp;
} data;

layout(set = 0, binding = 0) uniform Model {
    mat4 translation;
} model;

void main() {
    gl_Position = data.mvp * vec4(position, 1.0);
    tex_coords = tex_coord;
}
