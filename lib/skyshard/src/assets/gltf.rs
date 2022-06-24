

pub fn load_buffers(gltf: &gltf::Gltf) -> Vec<Vec<u8>> {
    const OCTET_STREAM_URI: &str = "data:application/octet-stream;base64,";

    let mut buffers = Vec::new();

    for buffer in gltf.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Uri(uri) => {
                if uri.starts_with(OCTET_STREAM_URI) {
                    buffers.push(::base64::decode(&uri[OCTET_STREAM_URI.len()..]).expect("Failed to decode data"));
                } else {
                    todo!()
                }
            }
            gltf::buffer::Source::Bin => {
                if let Some(blob) = gltf.blob.as_deref() {
                    buffers.push(blob.into());
                } else {
                    todo!();
                }
            }
        }
    }

    buffers
}
