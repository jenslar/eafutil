use std::path::PathBuf;

use eaf_rs::TimeSeries;

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let tsconf_path = args.get_one::<PathBuf>("tsconf").unwrap(); // already checked

    println!("Parsing time series configuration file...");
    let tsconf = match TimeSeries::read(&tsconf_path) {
        Ok(t) => t,
        Err(err) => {
            let msg = format!("Failed to parse '{}': {err}", tsconf_path.display());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        },
    };
    println!("{tsconf:#?}");

    Ok(())
}