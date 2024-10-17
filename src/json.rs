use std::path::PathBuf;

use eaf_rs::eaf::Eaf;

use crate::files::writefile;

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let eaf_path = args.get_one::<PathBuf>("eaf").unwrap(); // value ensured by clap

    let simple = args.contains_id("simple");
    
    let eaf = match Eaf::read(eaf_path) {
        Ok(f) => f,
        Err(err) => {
            let msg = format!("(!) Failed to parse '{}': {err}", eaf_path.display());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }
    };
    
    let json = match eaf.to_json(simple) {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("(!) Failed to export to JSON: {e}");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }
    };
    
    let json_path = eaf_path.with_extension("json");
    if let Err(err) = writefile(&json.as_bytes(), &json_path) {
        let msg = format!("(!) Failed to write '{}': {err}", eaf_path.display());
        return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
    };

    Ok(())
}