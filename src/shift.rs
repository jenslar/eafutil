//! Shift all annotations backwards or forwards with the specified time.
//! By default, shifts that result in negative time values are set to zero
//! so that ELAN's timelines always aligns with the media start time.

use std::path::PathBuf;

use eaf_rs::eaf::Eaf;

use crate::files::writefile;

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {

    // EAF-file path
    let path = args.get_one::<PathBuf>("eaf").unwrap(); // clap ensures value

    // Shift value, milliseconds.
    let shift = *args.get_one::<i64>("shift-value").unwrap(); // clap ensures value

    let mut eaf = match Eaf::de(path, true) {
        Ok(f) => f,
        Err(err) => {
            println!("(!) Failed to parse '{}': {err}", path.display());
            std::process::exit(1)
        }
    };

    match eaf.shift(shift, false) {
        Ok(_) => (),
        Err(err) => {
            println!("(!) Failed to shift '{}' {shift} ms: {err}", path.display());
            std::process::exit(1)
        }
    }

    let eaf_str = match eaf.se() {
        Ok(s) => s,
        Err(err) => {
            println!("(!) Failed to serialize '{}': {err}", path.display());
            std::process::exit(1)
        }
    };

    let eaf_path = match path.file_stem() {
        Some(name) => {
            let mut file_name = match name.to_str().map(String::from) {
                Some(n) => n,
                None => {
                    println!("(!) Failed to extract file name from '{}'", path.display());
                    std::process::exit(1)
                }
            };
            file_name.push_str(&format!("_{shift}.eaf"));
            path.with_file_name(file_name)
        },
        None => {
            println!("(!) Failed to extract file name from '{}'", path.display());
            std::process::exit(1)
        }
    };
    
    if let Err(err) = writefile(&eaf_str.as_bytes(), &eaf_path) {
        println!("(!) Failed to write '{}': {err}", eaf_path.display());
        std::process::exit(1)
    };

    Ok(())
}
