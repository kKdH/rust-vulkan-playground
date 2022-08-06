use std::error::Error;
use std::result::Result::Ok;
use std::fs;
use glob::glob;
use std::process::Command;

use blend_bindgen_rs::generate;

fn main() -> Result<(), Box<dyn Error>>{

    for entry in glob("src/blend/blender*.rs")? {
        fs::remove_file(entry?)?
    }

    for entry in glob("resources/*.blend")? {
        generate(&format!("{}", entry?.display()), "src/blend/");
    }

    for entry in glob("src/blend/blender*.rs")? {
        Command::new("rustfmt")
            .args(&["--edition", "2021"])
            .arg(&format!("{}", entry?.display()))
            .status().unwrap();
    }

    Ok(())
}
