use std::path::PathBuf;

use textgrid_rs::TextGrid;

// use textgrid_rs::TextGrid;

// Inspect EAF, main
pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let tg_path = args.get_one::<PathBuf>("textgrid").unwrap(); // clap ensures value
    
    let tg = TextGrid::from_path(tg_path)?;
    println!("{:?}", tg);

    Ok(())
}