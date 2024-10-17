//! Extracts a section an ELAN-file from specified timespan and generates a new ELAN-file,
//! with all tiers and annotations intact within that timespan.
//! Optionally process and and re-link corresponding cuts
//! of the original linked media files (requires FFmpeg) .

use std::{env::current_dir, path::{Path, PathBuf}};

use eaf_rs::{eaf::Eaf, ffmpeg::FFmpeg, EafError};
use itertools::join;
use mp4iter::Mp4;

use crate::{
    eaf::{
        select_annotation, select_tier
    },
    files::{
        affix_file_name, append_file_name, confirm, writefile
    }
};

/// extract main
pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let eaf_inpath = args.get_one::<PathBuf>("eaf").unwrap(); // ensured by clap
    let start = args.get_one::<i64>("start").cloned();
    let end = args.get_one::<i64>("end").cloned();
    let tier_prefix = args.get_one::<String>("tier-prefix");
    let process = *args.get_one::<bool>("process-media").unwrap();
    let ffmpeg = args.get_one::<String>("ffmpeg").unwrap(); // ensured by clap

    let eaf = match Eaf::read(&eaf_inpath) {
        Ok(f) => f,
        Err(err) => {
            let msg = format!("(!) Failed to parse '{}': {err}", eaf_inpath.display());
        return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
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
                    let msg = format!("(!) Failed to extract tier: {err}");
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                }
            };

            // let user choose whether to list large tiers
            if tier.len() > 40 {
                if !confirm(&format!("The tier '{}' has {} annotations. List all?", tier.tier_id, tier.len()))? {
                    let msg = format!("(!) Aborted process.");
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                }
            }

            let annotation = match select_annotation(&tier) {
                Ok(a) => a,
                Err(err) => {
                    let msg = format!("(!) Failed to extract annotation: {err}");
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                }
            };
            if let (Some(s), Some(e)) = annotation.ts_val() {
                (s, e)
            } else {
                let msg = format!("(!) Annotation has no time values specified.");
                return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
            }
        }
    };

    let timespan_str = format!("{start_ms}-{end_ms}");
    let eaf_infilestem = eaf_inpath
        .file_stem()
        .ok_or(std::io::Error::new(std::io::ErrorKind::Other, "Failed to extract file stem from EAF input path"))?;
    let eaf_outdir = eaf_inpath
        .parent()
        .ok_or(std::io::Error::new(std::io::ErrorKind::Other, "Failed to extract parent"))?
        .join(eaf_infilestem);
    if !eaf_outdir.exists() {
        std::fs::create_dir_all(&eaf_outdir)?
    }
    let eaf_outpath = append_file_name(&eaf_outdir.join(eaf_infilestem).with_extension("eaf"), &timespan_str);

    // Cut eaf, optionally cut media and re-link new media file.
    let mut eaf_out = match eaf.extract(
        start_ms,
        end_ms,
        &[] // add media extracts later
    ) {
        Ok(e) => e,
        Err(err) => {
            let msg = format!("(!) Failed to cut ELAN-file: {err}");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }
    };

    if let Some(prefix) = tier_prefix {
        println!("Prefixing all tier IDs with '{prefix}'");
        eaf_out.affix_tier_id_mut(None, Some(prefix), None)?;
    }

    if process {
        // Get existing linked media paths...
        let media_paths = eaf.media_paths();
        // ...remove them
        eaf_out.scrub_media(false);
        // The extract clips and re-add these
        for (media_in_abs, media_in_rel) in media_paths {
            let start = start_ms;
            let mut end = end_ms;
            let mpath_abs = PathBuf::from(media_in_abs);
            let mpath_rel = media_in_rel.map(|p| PathBuf::from(p));

            // Ensure paths exist
            let (mut mpath_out, mpath_in) = match (mpath_abs.exists(), mpath_rel) {
                (true, _) => (affix_file_name(&mpath_abs, None, Some(&timespan_str), Some("_")), mpath_abs),
                (false, Some(p)) => {
                    let relpath = current_dir()?.join(p);
                    if !relpath.exists() {
                        let msg = format!("Media path {} is not a valid", relpath.display());
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                    }
                    (affix_file_name(&relpath, None, Some(&timespan_str), Some("_")), relpath)
                },
                _ => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Linked media paths are note valid"))
            };

            let msg = format!("Failed to extract file name from path '{}'", mpath_out.display());
            let err = std::io::Error::new(std::io::ErrorKind::Other, msg);
            let mpath_filename = mpath_out
                .file_name()
                .ok_or_else(|| err)?;
            mpath_out = eaf_outpath
                .with_file_name(mpath_filename);

            // Bounds checking video duration with extraction time span
            // But only MP4-files... which means wav files outside timespan
            // will still be processed...
            let mut mp4 = Mp4::new(&mpath_in)?;
            if let Ok(duration) = mp4.duration(false) {
                let duration_ms = duration.whole_milliseconds() as i64;
                // casting to i64 should be ok for video durations in this case
                if start > duration_ms {
                    println!("Skipping media: Timespan is outside that of '{}'", mpath_in.display());
                    continue;
                }
                if end > duration_ms {
                    println!("Setting end of timespan to max ({duration_ms} ms) for media '{}'", mpath_in.display());
                    end = duration_ms;
                }
            };
            
            // let ext = mpath
            //     .extension()
            //     .ok_or(std::io::Error::new(std::io::ErrorKind::Other, "Failed to extract file extension from media path"))?;
            
            let media_out = match FFmpeg::extract_timespan(
                &mpath_in,
                // start_ms as u64,
                // end_ms as u64,
                start as u64,
                end as u64,
                // Some(&eaf_outpath.with_extension(ext)),
                Some(&mpath_out),
                Some(&Path::new(ffmpeg))
            ) {
                Ok(p) => p,
                Err(err) => {
                    let msg = format!("Failed to extract section from '{}': {err}", mpath_in.display());
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                }
            };
            eaf_out.add_media(&media_out, None);
            println!("Wrote and linked media extract '{}'", media_out.display());
        }
    }

    let eaf_str = match eaf_out.to_string(Some(4)) {
        Ok(s) => s,
        Err(err) => {
            let msg = format!("(!) Failed to serialize ELAN-file: {err}");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }
    };

    match writefile(&eaf_str.as_bytes(), &eaf_outpath) {
        Ok(true) => println!("Wrote '{}'", eaf_outpath.display()),
        Ok(false) => println!("Write to file aborted by user"),
        Err(err) => println!("(!) Failed to write '{}': {err}", eaf_outpath.display()),
    }
    // if let Some(outpath) = eaf_out.path() {
    //     match writefile(&eaf_str.as_bytes(), outpath) {
    //         Ok(true) => println!("Wrote '{}'", outpath.display()),
    //         Ok(false) => println!("Write to file aborted by user"),
    //         Err(err) => println!("(!) Failed to write '{}': {err}", outpath.display()),
    //     }
    // }

    println!("EAF IN:  {} annotations", eaf.a_len());
    println!("EAF OUT: {} annotations", eaf_out.a_len());

    Ok(())
}