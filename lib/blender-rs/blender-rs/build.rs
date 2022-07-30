use std::error::Error;
use std::result::Result::Ok;
use std::fs;
use glob::glob;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>>{

    for entry in glob("src/blender/blender*.rs")? {
        fs::remove_file(entry?)?
    }

    for entry in glob("resources/*.blend")? {
        generator::generate(&format!("{}", entry?.display()), "src/blender/");
    }

    for entry in glob("src/blender/blender*.rs")? {
        Command::new("rustfmt")
            .args(&["--edition", "2021"])
            .arg(&format!("{}", entry?.display()))
            .status().unwrap();
    }

    Ok(())
}
