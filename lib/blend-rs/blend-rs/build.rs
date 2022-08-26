use std::error::Error;
use std::result::Result::Ok;
use std::env;
use std::process::Command;


fn main() -> Result<(), Box<dyn Error>>{

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_dir = format!("{}", out_dir.to_str().unwrap());

    let generated: Vec<(String, String)> = vec![
        #[cfg(feature = "blender2_7")]
        generate("gen/blender2_7.blend", &out_dir),
        #[cfg(feature = "blender2_8")]
        generate("gen/blender2_8.blend", &out_dir),
        #[cfg(feature = "blender2_8x86")]
        generate("gen/blender2_8x86.blend", &out_dir),
        #[cfg(feature = "blender2_9")]
        generate("gen/blender2_9.blend", &out_dir),
        #[cfg(feature = "blender3_0")]
        generate("gen/blender3_0.blend", &out_dir),
    ];

    generated.iter().for_each(|(src, _)| {
        println!("cargo:rerun-if-changed={}", src);
    });

    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}

fn generate(src: &str, out: &str) -> (String, String) {
    let generated = blend_bindgen_rs::generate(&src, &out);
    Command::new("rustfmt")
        .args(&["--edition", "2021"])
        .arg(format!("{}", &generated))
        .status().unwrap();

    (String::from(src), generated)
}
