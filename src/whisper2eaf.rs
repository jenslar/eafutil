//! Generates an EAF-file from one or more Whisper Timestamped JSON-files.
//! If multiple JSON-files are to be combined it is assumed that these are clips
//! of a longer recording. In this case another JSON-file specifying path and position
//! for each clip in the original recording must be provided.
//! 
//! Running `eafutil clips --eaf MYEDAF.eaf` will provide this.

use std::path::PathBuf;

use eaf_rs::Eaf;

use crate::{clips::Clips, files::has_extension, whisper::{WhisperJson, WhisperTsJson}};

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let json_path = args.get_one::<PathBuf>("json"); // required unless "dir"
    let json_clips = args.get_one::<PathBuf>("clips"); // required for "dir"
    let json_dir = args.get_one::<PathBuf>("dir"); // required unless "json"
    let join_json = *args.get_one::<bool>("join").unwrap(); // requires "dir"
    let prefix_tiers = *args.get_one::<bool>("prefix-tiers").unwrap(); // requires "dir"
    let media_paths: Vec<PathBuf> = args.get_many::<PathBuf>("media")
        .map(|m| m.into_iter().map(|p| p.into()).collect())
        .unwrap_or_default();

    let no_speech_threshold = args.get_one::<f64>("no-speech").unwrap();

    let json_paths = if let Some(dir) = json_dir {
        dir.read_dir()?
            .filter_map(|entry| {
                let p = entry.ok()?.path();
                match has_extension(&p, "json") {
                    true => {
                        // dbg!(&p);
                        if p.file_name() == json_clips.and_then(|p| p.file_name()) {
                            None
                        } else {
                            Some(p)
                        }
                    },
                    false => None,
                }
            })
            .collect()
    } else {
        vec![json_path
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Must choose one of 'json' or 'dir'"))?
            .to_owned()
        ]
    };
    
    if join_json {
        if let Some(p) = json_clips {
            let clips = Clips::read(&p)?;
            // let whisper = WhisperTsJson::from_paths(&json_paths, &clips)?;
            // let whisper = WhisperJson::from_paths(&json_paths, &clips)?.filter_no_speech(*no_speech_threshold);
            // let mut eaf = whisper.to_eaf()?;

            let mut eaf = match WhisperJson::from_paths(&json_paths, &clips) {
                Ok(w) => w.filter_no_speech(*no_speech_threshold).to_eaf()?,
                Err(e) => {
                    println!("Failed to read JSON as standard Whisper: {e}\nTrying to read as Whisper Timestamped instead...");
                    WhisperTsJson::from_paths(&json_paths, &clips)?.to_eaf()?
                },
            };

            if prefix_tiers {
                let prefix = p.file_stem()
                    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Failed to derive tier ID prefix"))?
                    .to_string_lossy()
                    .to_string();
                eaf.prefix_tier_all_mut(&prefix)?;
            }
            let path = json_dir
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "No value set for 'dir'"))?
                .canonicalize()?
                .with_extension("eaf");

            // Add optional media files
            media_paths.iter()
                .try_for_each(|p| eaf.add_media(p, None))?;

            eaf.write(&path, Some(4))?;
            println!("Wrote {}", path.display());
        } else {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "No 'clips' file provided"))
        }
    } else {
        for path in json_paths.iter() {
            let mut eaf = match WhisperJson::read(path) {
                Ok(w) => w.filter_no_speech(*no_speech_threshold).to_eaf()?,
                Err(e) => {
                    println!("Failed to read JSON as standard Whisper: {e}\nTrying to read as Whisper Timestamped instead...");
                    WhisperTsJson::read(path)?.to_eaf()?
                },
            };

            let p = path.with_extension("eaf");

            // Add optional media files
            media_paths.iter()
                .try_for_each(|p| eaf.add_media(p, None))?;

            eaf.write(&p, Some(4))?;
            println!("Wrote {}", p.display());
        }
    }

    Ok(())
}