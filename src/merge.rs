//! Merges two ELAN-files. If annotations overlap the merge will be aborted by default.
//! Optionally prioritise to preserve timestamps in one of the files, or join the overlapping
//! annotation.
//! Important: Time slots with no time value set will be discarded.

use std::path::PathBuf;

use eaf_rs::eaf::Eaf;

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {

    // Assert two files are specified
    if args.get_count("eaf") != 2 {
        println!("(!) Please specify two ELAN-files.");
        std::process::exit(1)
    }

    // Get eaf paths
    let paths: Vec<PathBuf> = args.get_many::<PathBuf>("eaf").unwrap().cloned().collect();
        // .unwrap() // ensured by clap
        // .map(PathBuf::from)
        // .collect();

    println!("{paths:?}");

    unimplemented!("Merge not yet implemented");

    // TODO re-implement merge
    // let merged = match Eaf::merge(&[paths[0].to_owned(), paths[1].to_owned()]) {
    //     Ok(e) => e,
    //     Err(err) => {
    //         println!("(!) Failed to merge ELAN-files: {err}");
    //         std::process::exit(1)
    //     }
    // };

    // println!("{:?}", merged.tiers);


    // Parse into Eaf
    // let eafs: Vec<_> = paths.iter().map(|path| {
    //     match Eaf::deserialize(path, true) {
    //         Ok(e) => e,
    //         Err(err) => {
    //             println!("(!) Failed to parse ELAN-file: {err}");
    //             std::process::exit(1)
    //         }
    //     }
    // })
    // .collect();

    Ok(())
}
