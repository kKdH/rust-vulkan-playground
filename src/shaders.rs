
pub mod vs {
    vulkano_shaders::shader! {
            ty: "vertex",
            src: "
                #version 450
                layout(location = 0) in vec3 position;
                layout(location = 1) in vec2 tex_coord;
                layout(location = 2) in vec3 position_offset;
                layout(location = 3) in float scale;

                layout(location = 0) out vec2 tex_coords;

                layout(set = 0, binding = 0) uniform Data {
                    mat4 world;
                    mat4 view;
                    mat4 proj;
                } uniforms;

                void main() {
                   mat4 worldview = uniforms.view * uniforms.world;
                   gl_Position = uniforms.proj * worldview *  vec4(position, 1.0);
                   tex_coords = tex_coord;
                }
			"
        }
}

pub mod fs {
    vulkano_shaders::shader! {
            ty: "fragment",
            src: "
				#version 450
				layout(location = 0) out vec4 f_color;
                layout(location = 0) in vec2 tex_coords;

                layout(set = 0, binding = 1) uniform sampler2D tex;

				void main() {
                    f_color = texture(tex, tex_coords);
				}
			"
        }
}
