//! Generates clips from annotation boundaries in selected tier for linked media files. Requires FFmpeg.
//! Optionally add annotation value (default max length = 20 characters, can be overridden by the user),
//! timestamps, internal annotation ID. An option for ensuring ASCII for the annotation value also exists 
//! (not recommended for non-latin based scripts).

use std::path::{Path, PathBuf};

use regex::Regex;

use eaf_rs::{
    eaf::Eaf,
    ffmpeg::process::extract_timespan
};

use crate::text::process_string;

use super::eaf::{select_tier, select_annotation};
use super::files::acknowledge;

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let eaf_path = args.get_one::<PathBuf>("eaf").unwrap(); // clap ensures value

    let dryrun = *args.get_one::<bool>("dryrun").unwrap();

    let filestem = match eaf_path.file_stem().map(|s| s.to_string_lossy()) {
        Some(stem) => stem.to_string(),
        None => {
            println!("(!) Failed to extract file steam from {}", eaf_path.display());
            std::process::exit(1)
        }
    };

    // Determine output dir, user specified or eaf parent dir.
    let outdir = match args.get_one::<PathBuf>("outdir") {
        Some(d) => d.join(format!("{filestem}_CLIPS")),
        None => match eaf_path.parent() {
            Some(p) => p.join(format!("{filestem}_CLIPS")).to_owned(),
            None => {
                println!("(!) Failed to determine output path.");
                std::process::exit(1)
            }
        }
    };

    if !dryrun {
        if !outdir.exists() {
            match std::fs::create_dir_all(&outdir) {
                Ok(_) => (),
                Err(err) => {
                    println!("(!) Failed to create '{}': {err}", outdir.display());
                    std::process::exit(1)
                }
            };
        }
    }

    let single = *args.get_one::<bool>("single-annotation").unwrap();
    let use_id = *args.get_one::<bool>("annotation-id").unwrap();
    let use_val = *args.get_one::<bool>("annotation-value").unwrap();
    let use_time = *args.get_one::<bool>("annotation-time").unwrap();
    let max_len = *args.get_one::<usize>("max-length").unwrap(); // clap default 20
    // let max_len: usize = match args.value_of("max-length").unwrap().parse() { // clap default 20
    //     Ok(v) => v,
    //     Err(err) => {
    //         println!("(!) '--trunc' must be a positive integer: {err}");
    //         std::process::exit(1)
    //     }
    // };
    let ffmpeg = args.get_one::<String>("ffmpeg").unwrap(); // clap default ffmpeg/ffmpeg.exe
    // let extract_wav = args.is_present("extract-wav"); // clap default ffmpeg/ffmpeg.exe
    let ascii_path = *args.get_one::<bool>("ascii-path").unwrap();

    let eaf = match Eaf::de(eaf_path, true) {
        Ok(f) => f,
        Err(err) => {
            println!("(!) Error parsing '{}': {err}", eaf_path.display());
            std::process::exit(1)
        }
    };

    // Get linked media paths
    let media: Vec<(PathBuf, Option<PathBuf>)> = eaf.media_paths().iter()
        .map(|(p1, p2)| (
            PathBuf::from(p1.trim_start_matches("file://")),
            p2.as_deref().map(PathBuf::from)
        ))
        .collect();

    if media.is_empty() && !dryrun {
       println!("(!) No linked media files in '{}'", eaf_path.display());
       std::process::exit(1)
    }

    // Check whether paths are valid.
    // Use either of media_url or relative_media_url, whichever is set and exists.
    let mut media_process_paths: Vec<PathBuf> = Vec::new();
    for (abs, rel) in media.iter() {
        match (abs.exists(), rel.as_deref().map(|p| p.exists()).unwrap_or(false)) {
            // First check absolute media path...
            (true, _) => media_process_paths.push(abs.to_owned()),
            // ...if no good try the relative media path and make it absolute
            (_, true) => media_process_paths.push(rel.to_owned().unwrap().canonicalize()?),
            // Abort if no media file can be located
            (false, false) => {
                if dryrun {
                    media_process_paths.push(abs.to_owned()) // may be empty, i.e. ""...
                } else {
                    println!("(!) Linked media files could not be located:");
                    for (i, (a, r)) in media.iter().enumerate() {
                        println!("    {:2}. ABS: {}\n        REL: {}", i+1, a.display(), r.as_deref().unwrap_or(Path::new("NONE")).display())
                    }
                    println!("    Re-link them in ELAN and try again.");
                    std::process::exit(1)
                }
            }
        }
    }

    // println!("{:?}", media_process_paths);
    // std::process::exit(0);

    // Select tier to generate clips from
    let tier = match select_tier(&eaf, true) {
        Ok(t) => t,
        Err(err) => {
            println!("(!) Failed to extract tier: {err}");
            std::process::exit(1)
        }
    };

    // Vec containing annotation boundaries in milliseconds, ID, value:
    // (start_ms, end_ms, annotation ID, annotation value)
    let boundaries: Vec<(i64, i64, String, String)> = match single {
        true => {
            let len = tier.len();

            // let user choose whether to list large tiers
            if len > 40 {
                if !acknowledge(&format!("The tier '{}' has {} annotations. List all?", tier.tier_id, len))? {
                    println!("(!) User aborted process.");
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
                vec![(s, e, annotation.id(), annotation.to_str().to_owned())]
            } else {
                println!("(!) Annotation has no time values specified:\n{annotation:?}");
                std::process::exit(1)
            }
        },
        false => {
            tier.annotations.iter()
                .filter_map(|a| {
                    if let (Some(start), Some(end)) = a.ts_val() {
                        Some((start, end, a.id(), a.to_str().to_owned()))
                    } else {
                        None
                    }
                })
                .collect()
                
        }
    };

    // regex compilation run-time checked, not compile time?
    // NOTE '-' can only be literal in regex range if it is the last character as below. issues with raw string (r"") and regex escapes
    let re2remove = Regex::new("[\"\'#*<>{}()\\[\\].,:;!/?=\\\\-]").expect("regex compile error");

    // Process linked media. Generate paths and cut up with ffmpeg.
    for (idx, (start_ms, end_ms, id, val)) in boundaries.iter().enumerate() {
        
        println!("[ {start_ms:6}ms - {end_ms:6}ms... '{val}' ]");
    

        // make part of filestem containing annotation details
        let mut annotstem = format!("annotation_{:04}", idx+1);
        // optionally add internal annotation ID to file name
        if use_id {
            annotstem.push_str(&format!("_{id}"))
        }
        // optionally add annotation value to file name
        if use_val {
            // remove/replace ascii + whitespace
            let processed_val = process_string(
                val,
                if ascii_path {Some(&'_')} else {None},
                Some(&'_'),
                Some(&re2remove),
                Some(max_len));
            annotstem.push_str(&format!("_{processed_val}"))
        }
        // optionally add annotation boundaries in milliseconds file name
        if use_time {
            annotstem.push_str(&format!("_{start_ms}-{end_ms}"))
        }
        
        for media_in in media_process_paths.iter() {
            if let Some(ext) = media_in.extension() {
                // extract filestem for media file
                let mediastem = match media_in.file_stem().and_then(|s| s.to_str()) {
                    Some(s) => s,
                    None => {
                        println!("(!) Could not determine file name for '{}'", media_in.display());
                        std::process::exit(1)
                    }
                };
                // combine media filestem with annotation filestem,
                // set extension according to media file
                let outpath = outdir.join(Path::new(&format!("{mediastem}_{annotstem}"))
                    .with_extension(ext));

                if dryrun {
                    println!("  IN (exists {:5}): {}\n OUT (exists {:5}): {}", media_in.exists(), media_in.display(), outpath.exists(), outpath.display());
                }

                if outpath.exists() {
                    match acknowledge(&format!("'{}' already exists. Overwrite?", outpath.display())) {
                        Ok(false) => {
                            println!("(!) User aborted process.");
                            std::process::exit(1)
                        },
                        Ok(true) => (),
                        Err(err) => {
                            println!("(!) Failed to read input: {err}.");
                            std::process::exit(1)
                        }
                    }
                }

                if !dryrun {
                    let media_out = match extract_timespan(
                        &media_in,
                        *start_ms as u64,
                        *end_ms as u64,
                        Some(&outpath),
                        Some(&Path::new(ffmpeg))) {
                        Ok(p) => p,
                        Err(err) => {
                            println!("(!) Failed to extract\n  '{}'\n  from\n  '{}':\n  {err}", outpath.display(), media_in.display());
                            std::process::exit(1)
                        }
                    };
    
                    println!("Wrote '{}'", media_out.display());
                }

            } else {
                println!("(!) Could not determine file type for '{}'.", media_in.display());
                std::process::exit(1)
            }
        }

    }

    Ok(())
}
