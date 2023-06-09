//! Filters a timespan in an ELAN-file and generates a new ELAN-file, with
//! all tiers and annotations intact within that timespan.
//! Optionally process and and re-link corresponding cuts
//! of the original linked media files (requires FFmpeg) .

use std::path::{Path, PathBuf};

use eaf_rs::eaf::Eaf;

use crate::{
    eaf::{
        select_tier,
        select_annotation
    },
    files::{
        acknowledge,
        append_file_name, writefile
    }
};

/// filter main
pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let path = args.get_one::<PathBuf>("eaf").unwrap(); // ensured by clap
    let start = args.get_one::<i64>("start").cloned();
    let end = args.get_one::<i64>("end").cloned();
    let process = *args.get_one::<bool>("process-media").unwrap();
    let ffmpeg = args.get_one::<String>("ffmpeg").unwrap(); // ensured by clap

    let eaf = match Eaf::de(path, true) {
        Ok(f) => f,
        Err(err) => {
            println!("(!) Failed to parse '{}': {err}", path.display());
            std::process::exit(1)
        }
    };

    let (start_ms, end_ms): (i64, i64) = match (start, end) {
        // check if start, end values have been set first...
        (Some(s), Some(e)) => (s, e),

        // ...or select tier, then annotation to use as boundary for extraction
        _ => {
            let tier = match select_tier(&eaf, true) {
                Ok(t) => t,
                Err(err) => {
                    println!("(!) Failed to extract tier: {err}");
                    std::process::exit(1)
                }
            };

            // let user choose whether to list large tiers
            if tier.len() > 40 {
                if !acknowledge(&format!("The tier '{}' has {} annotations. List all?", tier.tier_id, tier.len()))? {
                    println!("(!) Aborted process.");
                    std::process::exit(1)
                }
            }

            let annotation = match select_annotation(&tier) {
                Ok(a) => a,
                Err(err) => {
                    println!("(!) Failed to extract annotation: {err}");
                    std::process::exit(1)
                }
            };
            if let (Some(s), Some(e)) = annotation.ts_val() {
                (s, e)
            } else {
                println!("(!) Annotation has no time values specified.");
                std::process::exit(1)
            }
        }
    };

    let timespan_str = format!("{start_ms}-{end_ms}");
    println!("{start_ms}-{end_ms}");

    // Filter eaf, optionally cut media and re-link new media file.
    let eaf_out = match eaf.filter(
        start_ms,
        end_ms,
        None, // require valid media paths
        Some(&Path::new(ffmpeg)),
        process
    ) {
        Ok(e) => e,
        Err(err) => {
            println!("(!) Failed to filter ELAN-file: {err}");
            std::process::exit(1)
        }
    };

    let eaf_str = match eaf_out.se() {
        Ok(s) => s,
        Err(err) => {
            println!("(!) Failed to serialize ELAN-file: {err}");
            std::process::exit(1)
        }
    };

    // TODO generate filename (with timestamps), process media option, write file to same dir as source
    // TODO see clips.rs for filename generation, add similar options
    let eaf_outpath = append_file_name(path, &timespan_str);
    // if let Err(err) = writefile(&eaf_str.as_bytes(), &eaf_outpath) {
    //     println!("(!) Failed to write '{}': {err}", eaf_outpath.display());
    //     std::process::exit(1)
    // };

    match writefile(&eaf_str.as_bytes(), &eaf_outpath) {
        Ok(true) => println!("Wrote '{}'", eaf_outpath.display()),
        Ok(false) => println!("Write to file aborted by user"),
        Err(err) => println!("(!) Failed to write '{}': {err}", eaf_outpath.display()),
    }

    println!("EAF IN:  {} annotations", eaf.a_len());
    println!("EAF OUT: {} annotations", eaf_out.a_len());

    Ok(())
}