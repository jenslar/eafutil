use std::path::PathBuf;

// use textgrid_rs::TextGrid;

// Inspect EAF, main
pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    unimplemented!("Parsing textgrid files not yet implemented");
    let tg_path = args.get_one::<PathBuf>("eaf").unwrap(); // clap ensures value
    
    Ok(())
}