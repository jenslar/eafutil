//! Generates clips from annotation boundaries in selected tier for linked media files. Requires FFmpeg.
//! Optionally add annotation value (default max length = 20 characters, can be overridden by the user),
//! timestamps, internal annotation ID. An option for ensuring ASCII for the annotation value also exists 
//! (not recommended for non-latin based scripts).

use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use regex::Regex;

use eaf_rs::{
    eaf::Eaf,
    ffmpeg::FFmpeg,
};
use serde::{Deserialize, Serialize};

use crate::text::process_string;

use super::eaf::{select_tier, select_annotation};
use super::files::confirm;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Clips {
    original_media: Vec<PathBuf>,
    clips: Vec<Clip>,
}

impl Clips {
    pub fn read(path: &Path) -> std::io::Result<Self> {
        let string = read_to_string(path)?;
        let clips: Self = serde_json::from_str(&string)?;
    
        Ok(clips)
    }

    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        let string = serde_json::to_string(&self)?;
        let mut file = File::create(path)?;

        file.write_all(string.as_bytes())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Clip> {
        self.clips.iter()
    }

    pub fn with_media(media: Vec<PathBuf>) -> Self {
        Self {
            original_media: media.to_owned(),
            ..Self::default()
        }
    }

    pub fn len(&self) -> usize {
        self.clips.len()
    }

    pub fn add(&mut self, clip: &Clip) {
        self.clips.push(clip.to_owned())
    }

    /// Returns timestamps in original media for specified clip path.
    /// Compares file stems only, i.e. clip names are assumed to be unique,
    /// but any kind of file can be provided.
    pub fn get_timestamps(&self, path: &Path) -> Option<(i64, i64)> {
        let mut filestem = path.file_stem()?; // need to strip all .wav.word. once not enough
        // Attempt to gradually strip away all multi-dot, "extension-like" components
        // in e.g. MYJSON.words.wav.json
        loop {
            let p = Path::new(filestem);
            if p.extension().is_none() {
                break
            }
            filestem = &p.file_stem()?;
        }
        // dbg!(&filestem);
        self.clips.iter().find(|c| c.media.iter().find(|m| m.file_stem() == Some(filestem)).is_some())
            .map(|c| (c.start, c.end))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Clip {
    media: Vec<PathBuf>,
    /// Start position of clip in original media file.
    start: i64,
    /// End position of clip in original media file.
    end: i64,
}

impl Clip {
    pub fn new(media: &[PathBuf], start: i64, end: i64) -> Self {
        Self {
            media: media.to_owned(),
            start,
            end
        }
    }

    /// Add media path.
    pub fn add(&mut self, path: &Path) {
        self.media.push(path.to_owned())
    }

    /// Set start/end timestamps in milliseconds.
    pub fn ts(&mut self, start: i64, end: i64) {
        self.start = start;
        self.end = end;
    }
}

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let eaf_path = args.get_one::<PathBuf>("eaf").unwrap(); // clap ensures value
    let filestem = match eaf_path.file_stem().map(|s| s.to_string_lossy()) {
        Some(stem) => stem.to_string(),
        None => {
            println!("Failed to extract file steam from {}", eaf_path.display());
            std::process::exit(1)
        }
    };

    // Determine output dir, user specified or eaf parent dir.
    let outdir = match args.get_one::<PathBuf>("outdir") {
        Some(d) => d.join(format!("{filestem}_CLIPS")),
        None => match eaf_path.parent() {
            Some(p) => p.join(format!("{filestem}_CLIPS")).to_owned(),
            None => {
                println!("Failed to determine output path.");
                std::process::exit(1)
            }
        }
    };

    let dryrun = *args.get_one::<bool>("dryrun").unwrap();
    if !dryrun {
        if !outdir.exists() {
            match std::fs::create_dir_all(&outdir) {
                Ok(_) => (),
                Err(err) => {
                    let msg = format!("Failed to create '{}': {err}", outdir.display());
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                }
            };
        }
    }

    let single = *args.get_one::<bool>("single-annotation").unwrap();
    let use_a_id = *args.get_one::<bool>("annotation-id").unwrap();
    let use_t_id = *args.get_one::<bool>("tier-id").unwrap();
    let use_val = *args.get_one::<bool>("annotation-value").unwrap();
    let use_time = *args.get_one::<bool>("annotation-time").unwrap();
    let extract_all = *args.get_one::<bool>("all").unwrap();
    let max_len = *args.get_one::<usize>("max-length").unwrap(); // clap default 20
    let min_dur = args.get_one::<i64>("min-duration"); // clap default 20
    let ffmpeg = args.get_one::<String>("ffmpeg").unwrap(); // clap default ffmpeg/ffmpeg.exe
    // let extract_wav = args.is_present("extract-wav"); // clap default ffmpeg/ffmpeg.exe
    let ascii_path = *args.get_one::<bool>("ascii-path").unwrap();

    let eaf = match Eaf::read(eaf_path) {
        Ok(f) => f,
        Err(err) => {
            let msg = format!("Error parsing '{}': {err}", eaf_path.display());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }
    };

    // Get linked media paths
    let media = eaf.media_paths();

    if media.is_empty() && !dryrun {
        let msg = format!("No linked media files in '{}'", eaf_path.display());
        return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
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
                    println!("Linked media files could not be located:");
                    for (i, (a, r)) in media.iter().enumerate() {
                        println!("    {:2}. ABS: {}\n        REL: {}", i+1, a.display(), r.as_deref().unwrap_or(Path::new("NONE")).display())
                    }
                    println!("    Re-link them in ELAN and try again.");
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to locate linked files."))
                }
            }
        }
    }

    // Select tier to generate clips from
    let tiers = if extract_all {
        eaf.tiers
    } else {
        match select_tier(&eaf, true) {
            Ok(t) => vec![t],
            Err(err) => {
                let msg = format!("Failed to extract tier: {err}");
                return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
            }
        }
    };

    let mut durations: Vec<i64> = Vec::new();
    // Will be cloned and added to for each tier, since each tiers generates a JSON-file
    // with clip positions
    let clips = Clips::with_media(media.iter().map(|m| m.0.to_owned()).collect());

    for tier in tiers.iter() {
        // create sub-dir named after tier ID
        let tier_outdir = outdir.join(&tier.tier_id);
        if !dryrun {
            if !tier_outdir.exists() {
                match std::fs::create_dir_all(&tier_outdir) {
                    Ok(_) => (),
                    Err(err) => {
                        let msg = format!("Failed to create '{}': {err}", tier_outdir.display());
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                    }
                };
            }
        }
        // Vec containing annotation boundaries in milliseconds, ID, value:
        // (start_ms, end_ms, annotation ID, annotation value)
        let boundaries: Vec<(i64, i64, String, String)> = match single {
            true => {
                let len = tier.len();

                // let user choose whether to list large tiers
                if len > 40 {
                    if !confirm(&format!("The tier '{}' contains {} annotations. List all?", tier.tier_id, len))? {
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, "User aborted process."))
                    }
                }

                let annotation = match select_annotation(&tier) {
                    Ok(a) => a,
                    Err(err) => {
                        let msg = format!("Failed to extract annotation: {err}");
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                    }
                };

                if let (Some(s), Some(e)) = annotation.ts_val() {
                    vec![(s, e, annotation.id().to_owned(), annotation.to_string())]
                } else {
                    let msg = format!("Annotation has no time values specified:\n{annotation:?}");
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                }
            },
            false => {
                tier.annotations.iter()
                    .filter_map(|a| {
                        // Annotation time values must be set
                        if let (Some(start), Some(end)) = a.ts_val() {
                            let len = end - start;
                            // Check if annotation duration is below threshold
                            if let Some(min) = min_dur {
                                if min > &len {
                                    None
                                } else {
                                    durations.push(len);
                                    Some((start, end, a.id().to_owned(), a.to_string()))
                                }
                            } else {
                                durations.push(len);
                                Some((start, end, a.id().to_owned(), a.to_string()))
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
                    
            }
        };

        // regex compilation run-time checked, not compile time?
        // NOTE '-' can only be literal in regex range if it is the last character as below. issues with raw string (r"") and regex escapes
        let re2remove = Regex::new("[\"\'#*<>{}()\\[\\].,:;!/?=\\\\-]")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let mut tier_clips = clips.to_owned();

        // Process linked media. Generate paths and cut up with ffmpeg.
        for (idx, (start_ms, end_ms, a_id, val)) in boundaries.iter().enumerate() {
            
            println!("[ {start_ms:6}ms - {end_ms:6}ms... '{val}' ]");

            // make part of filestem containing annotation details
            let mut annotstem = format!("annotation_{:04}", idx+1);
            // optionally add internal annotation ID to file name
            if use_t_id {
                annotstem.push_str(&format!("_{}", tier.tier_id))
            }
            if use_a_id {
                annotstem.push_str(&format!("_{a_id}"))
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

            // dbg!(&annotstem);

            let mut clip = Clip::default();
            clip.ts(*start_ms, *end_ms);
            
            for media_in in media_process_paths.iter() {
                if let Some(ext) = media_in.extension() {
                    // extract filestem for media file
                    let mediastem = match media_in.file_stem().and_then(|s| s.to_str()) {
                        Some(s) => s,
                        None => {
                            println!("Could not determine file name for '{}'", media_in.display());
                            std::process::exit(1)
                        }
                    };

                    // dbg!(&format!("{mediastem}_{annotstem}"));

                    // combine media filestem with annotation filestem,
                    // set extension according to media file
                    // in cases where the file has multiple dots, e.g. audio.wav.wav using `filestem()`
                    // correctly returns audio.wav, but this stem can't be edited then set extension
                    // via `with_extension` since this will yield "audio.wav_ADDED_SUFFIX".with_extention("wav") -> "audio.wav"
                    let outpath = tier_outdir.join(Path::new(&format!("{mediastem}_{annotstem}.{}", ext.to_string_lossy())));
                        // .with_extension(ext));
                    clip.add(&outpath);

                    if dryrun {
                        println!("  IN (exists {:5}): {}\n OUT (exists {:5}): {}", media_in.exists(), media_in.display(), outpath.exists(), outpath.display());
                    } else {
                        if outpath.exists() {
                            match confirm(&format!("'{}' already exists. Overwrite?", outpath.display())) {
                                Ok(false) => {
                                    return Err(std::io::Error::new(std::io::ErrorKind::Other, "User aborted process."))
                                },
                                Ok(true) => (),
                                Err(err) => {
                                    let msg = format!("Failed to read input: {err}.");
                                    return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                                }
                            }
                        }
                        let media_out = match FFmpeg::extract_timespan(
                            &media_in,
                            *start_ms as u64,
                            *end_ms as u64,
                            Some(&outpath),
                            Some(&Path::new(ffmpeg))) {
                            Ok(p) => p,
                            Err(err) => {
                                let msg = format!("Failed to extract\n  '{}'\n  from\n  '{}':\n  {err}", outpath.display(), media_in.display());
                                return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                            }
                        };
        
                        println!("Wrote '{}'", media_out.display());
                    }

                } else {
                    let msg = format!("Could not determine file type for '{}'.", media_in.display());
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                }
            }

            tier_clips.add(&clip);
        }

        let tier_clips_path = tier_outdir.join(Path::new(&tier.tier_id).with_extension("json")); 
        tier_clips.write(&tier_clips_path)?;
        println!("Wrote {}", tier_clips_path.display());
    }

    println!("Longest clip:  {} ms", durations.iter().max().unwrap_or(&0));
    println!("Shortest clip: {} ms", durations.iter().min().unwrap_or(&0));

    Ok(())
}
