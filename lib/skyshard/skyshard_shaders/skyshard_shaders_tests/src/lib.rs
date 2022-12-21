
#[cfg(test)]
mod test {

    mod vs {
        skyshard_shaders::shader! {
            kind: "Vertex",
            src: "
                #version 450
                void main() {
                }
            "
        }
    }

    mod fs {
        skyshard_shaders::shader! {
            kind: "Fragment",
            src: "
                #version 450
                void main() {
                }
            "
        }
    }

    #[test]
    fn test() {
        let vertex_shader = vs::shader();
        let fragment_shader = fs::shader();
    }
}
