
pub mod vs {
    skyshard_shaders::shader! {
        kind: "Vertex",
        src: "
            #version 450
            #extension GL_ARB_separate_shader_objects : enable

            layout(binding = 0) uniform UniformBufferObject {
                mat4 mvp;
            } ubo;

            layout(location = 0) in vec3 inPosition;
            layout(location = 1) in vec3 inColor;
            layout(location = 2) in vec2 inTextCord;

            layout(location = 3) in uint inObjectId;
            layout(location = 4) in mat4 transformation; // consumes location 4, 5, 6, 7

            layout(location = 0) out uint outObjectId;
            layout(location = 1) out vec3 outColor;
            layout(location = 2) out vec2 outTextCord;

            void main() {
                gl_Position = ubo.mvp * transformation * vec4(inPosition, 1.0);
                outObjectId = inObjectId;
                outColor = inColor;
                outTextCord = inTextCord;
            }
        "
    }
}

pub mod fs {
    skyshard_shaders::shader! {
        kind: "Fragment",
        src: "
            #version 450
            #extension GL_ARB_separate_shader_objects : enable

            layout(location = 0) in flat uint inObjectId;
            layout(location = 1) in vec3 inColor;
            layout(location = 2) in vec2 inTextCord;

            layout(set = 1, binding = 0) uniform sampler2D texture_sampler;

            layout(location = 0) out vec4 outColor;
            layout(location = 1) out uint outObjectId;

            void main() {
                vec3 color = texture(texture_sampler, inTextCord).xyz;
                outColor = vec4(color, 1.0f);
                outObjectId = inObjectId;
            }
        "
    }
}
