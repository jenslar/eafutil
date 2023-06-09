use std::path::PathBuf;

use eaf_rs::eaf::Eaf;

use crate::files::writefile;

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let eaf_path = args.get_one::<PathBuf>("eaf").unwrap(); // value ensured by clap

    let simple = args.contains_id("simple");
    
    let eaf = match Eaf::de(eaf_path, true) {
        Ok(f) => f,
        Err(err) => {
            println!("(!) Failed to parse '{}': {err}", eaf_path.display());
            std::process::exit(1)
        }
    };
    
    let json = match eaf.to_json(simple) {
        Ok(s) => s,
        Err(e) => {
            println!("(!) Failed to export to JSON: {e}");
            std::process::exit(1)
        }
    };
    
    let json_path = eaf_path.with_extension("json");
    if let Err(err) = writefile(&json.as_bytes(), &json_path) {
        println!("(!) Failed to write '{}': {err}", eaf_path.display());
        std::process::exit(1)
    };

    Ok(())
}