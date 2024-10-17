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

    let mut eaf = match Eaf::read(path) {
        Ok(f) => f,
        Err(err) => {
            let msg = format!("(!) Failed to parse '{}': {err}", path.display());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }
    };

    match eaf.shift(shift, false) {
        Ok(_) => (),
        Err(err) => {
            let msg = format!("(!) Failed to shift '{}' {shift} ms: {err}", path.display());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }
    }

    let eaf_str = match eaf.to_string(Some(4)) {
        Ok(s) => s,
        Err(err) => {
            let msg = format!("(!) Failed to serialize '{}': {err}", path.display());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }
    };

    let eaf_path = match path.file_stem() {
        Some(name) => {
            let mut file_name = match name.to_str().map(String::from) {
                Some(n) => n,
                None => {
                    let msg = format!("(!) Failed to extract file name from '{}'", path.display());
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                }
            };
            file_name.push_str(&format!("_{shift}.eaf"));
            path.with_file_name(file_name)
        },
        None => {
            let msg = format!("(!) Failed to extract file name from '{}'", path.display());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }
    };
    
    if let Err(err) = writefile(&eaf_str.as_bytes(), &eaf_path) {
        let msg = format!("(!) Failed to write '{}': {err}", eaf_path.display());
        return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
    };

    Ok(())
}
