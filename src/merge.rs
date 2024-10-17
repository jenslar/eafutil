//! NOT YET IMPLEMENTED
//! 
//! Merges two ELAN-files. If annotations overlap the merge will be aborted by default.
//! Optionally prioritise to preserve timestamps in one of the files, or join the overlapping
//! annotation.
//! Important: Time slots with no time value set will be discarded.

use std::path::{Path, PathBuf};

use eaf_rs::{Eaf, OverlapStrategy, EafError};

use crate::files::file_stem_as_string;

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let paths = args.get_many::<PathBuf>("eaf");
    let dir = args.get_one::<PathBuf>("dir");
    // optionally prefix tier IDs with filestem to avoid
    // combining tiers with same ID but distinct content
    let prefix_tiers = *args.get_one::<bool>("prefix-tiers").unwrap();
    let media_paths: Vec<PathBuf> = args.get_many::<PathBuf>("media")
        .map(|m| m.into_iter().map(|p| p.into()).collect())
        .unwrap_or_default();

    // Get EAFs.
    let eafs = match (paths, dir) {
        (None, Some(d)) => {
            d.read_dir()?
                .filter_map(|e| {
                    let p = e.ok()?.path();
                    let mut eaf = Eaf::read(&p).ok()?;
                    if prefix_tiers {
                        let stem = file_stem_as_string(&p)?;
                        eaf.prefix_tier_all_mut(&stem).ok()?;
                    }
                    Some(eaf)
                })
                .collect::<Vec<_>>()
        },
        (Some(ps), None) => ps.into_iter()
            .filter_map(|p| Eaf::read(p).ok())
            .collect::<Vec<_>>(),
        (..) => {
            let msg = format!("Must choose one of 'eaf', 'dir'");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        },
    };

    let mut eaf = Eaf::merge(&eafs)?;

    // Add optional media files
    media_paths.iter()
        .try_for_each(|p| eaf.add_media(p, None))?;

    let outpath = Path::new("merged_eaf.eaf");
    eaf.write(outpath, Some(4))?;
    println!("Wrote {}", outpath.display());

    Ok(())
}
