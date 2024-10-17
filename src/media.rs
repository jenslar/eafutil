//! Add or remove linked media files. Can scrub absolute paths in batch
//! to prepare for e.g. archiving/sharing, since an absolute path may
//! contain personal information, such as user name.

use std::{path::{Path, PathBuf}, ffi::OsStr};

use eaf_rs::eaf::Eaf;
use walkdir::WalkDir;

use crate::files::{is_hidden, append_file_name, writefile};

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let eaf_path = args.get_one::<PathBuf>("eaf"); // clap ensures value
    let eaf_dir = args.get_one::<PathBuf>("dir");
    let media_path = args.get_one::<PathBuf>("media");
    let remove = *args.get_one::<bool>("remove").unwrap(); // abs, all, conflicts with "add"
    let add = *args.get_one::<bool>("add").unwrap(); // abs, all, conflict with "remove"
    let scrub = *args.get_one::<bool>("scrub").unwrap(); // abs, all
    let filename_only = *args.get_one::<bool>("filename-only").unwrap(); // abs, all

    // Collect EAF paths.
    let paths = match (eaf_path, eaf_dir) {
        (Some(path), None) => vec!(path.to_owned()),
        (None, Some(dir)) => {
            if Path::new(dir).is_file() {
                let msg = format!("(!) {} is a file.", dir.display());
                return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
            }
            let mut dirs: Vec<PathBuf> = Vec::new();
            for occurence in WalkDir::new(dir) {
                match occurence {
                    Ok(de) => {
                        let p = de.path().to_owned();
                        if p.extension() == Some(&OsStr::new("eaf")) {
                            // ignore hidden *nix files
                            if is_hidden(&p) {
                                continue
                            }
                            dirs.push(p)
                        } else {
                            continue
                        }
                    },
                    Err(_) => continue // perhaps handle error, but would mostly be permission issues
                }
            }

            dirs
        },
        // clap ensures only one, so _ should never match
        _ => {
            let msg = format!("(!) Only one of 'eaf' and 'dir' can be specified.");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }
    };

    // Process media in EAF paths.
    for path in paths.iter() {
        let mut eaf = match Eaf::read(path) {
            Ok(f) => f,
            Err(err) => {
                let msg = format!("(!) Failed to parse '{}': {err}", path.display());
                return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
            }
        };

        let mut filename_suffix = "";

        if let Some(mpath) = media_path {
            match (add, remove) {
                (true, false) => {
                    eaf.add_media(mpath, None);
                    filename_suffix = "ADDMEDIA";
                },
                (false, true) => {
                    eaf.remove_media(mpath);
                    filename_suffix = "REMMEDIA";
                },
                // clap ensures only one, so _ should never match
                _ => {
                    let msg = format!("(!) Only one of 'add', 'remove' can be specified.");
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                }
            }
        } else {
            match (scrub, filename_only) {
                (true, false) => {
                    eaf.scrub_media(false);
                    filename_suffix = "SCRMEDIA";
                },
                (false, true) => {
                    eaf.scrub_media(true);
                    filename_suffix = "FNMEDIA";
                },
                // clap ensures only one, so _ should never match
                _ => {
                    let msg = format!("(!) Only one of 'scrub', 'filename-only' can be specified.");
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                }
            }
        }


        let eaf_str = match eaf.to_string(Some(4)) {
            Ok(s) => s,
            Err(err) => {
                let msg = format!("(!) Failed to serialize {}: {err}", path.display());
                return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
            }
        };
        let eaf_outpath = append_file_name(path, filename_suffix);
        
        if let Err(err) = writefile(&eaf_str.as_bytes(), &eaf_outpath) {
            let msg = format!("(!) Failed to write '{}': {err}", eaf_outpath.display());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }

        println!("Resulting media paths in '{}':", path.display());
        for (i, paths) in eaf.media_paths().iter().enumerate() {
            println!("{:2}.  Media URL:          {}", i+1, paths.0.display());
            println!("     Relative media URL: {}", paths.1.as_deref().unwrap_or(Path::new("")).display());
        }
    }

    Ok(())
}