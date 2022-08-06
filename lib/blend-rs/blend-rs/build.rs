use std::error::Error;
use std::result::Result::Ok;
use std::{env, fs};
use glob::glob;
use std::process::Command;

use blend_bindgen_rs::generate;

fn main() -> Result<(), Box<dyn Error>>{

    let inputs: Vec<String> = glob("resources/*.blend")?
        .map(|result| {
            format!("{}", result.unwrap().display())
        })
        .collect();

    let out_dir = "src/blend/";

    let mut generated_files: Vec<String> = Vec::new();
    for file_path in &inputs {
        let file = generate(&file_path, out_dir);
        generated_files.push(file);
    }

    generated_files.iter().for_each(|generated_file| {
        Command::new("rustfmt")
            .args(&["--edition", "2021"])
            .arg(format!("{}", generated_file))
            .status().unwrap();
    });

    inputs.iter().for_each(|file_path| {
        println!("cargo:rerun-if-changed={}", file_path);
    });

    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}
