use std::{path::{Path, PathBuf}, process::Command, io::{stdout, Write}};

use eaf_rs::EafError;

pub struct FFmpeg {}

impl FFmpeg {
    /// Returns media duration in milliseconds.
    pub fn get_duration(media_file: &Path, ffprobe_path: &Path) -> Result<u64, EafError> {
        // ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 FILE
        let args = [
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            &media_file.display().to_string(),
        ];
    
        // let ffprobe = if cfg!(windows) {"ffprobe.exe"} else {"ffprobe"};
    
        let output = Command::new(ffprobe_path).args(&args).output()?.stdout; // or ::new().spawn() ?
        let duration: f64 = std::str::from_utf8(&output).unwrap().trim().parse()?;
    
        Ok((duration * 1000.0) as u64)
    }
    
    pub fn extract_wav(video_path: &Path, ffmpeg_path: &Path) -> std::io::Result<PathBuf> {
        let wav = video_path.with_extension("wav");
        if wav.exists() {
            println!("      Audio target already exists.")
        } else {
            print!("      Extracting wav to {}... ", wav.display());
            stdout().flush()?;
            Command::new(&ffmpeg_path)
                .args(&[
                    "-i",
                    &video_path.display().to_string(),
                    "-vn", &wav.display().to_string()
                ])
                .output()?;
            println!("Done");
        }
    
        Ok(wav)
    }

    /// Extract section of a media file with `start_ms` and `end_ms` timespan
    /// in milliseconds (ELAN's default time value).
    /// If succesful the path to the extracted media file is returned.
    pub fn extract_timespan(media_path: &Path, start_ms: u64, end_ms: u64, custom_outpath: Option<&Path>, ffmpeg_path: Option<&Path>) -> Result<PathBuf, EafError> {
        let ffmpeg = if let Some(p) = ffmpeg_path {
            p.to_owned()
        } else if cfg!(windows) {
            PathBuf::from("ffmpeg.exe")
        } else {
            PathBuf::from("ffmpeg")
        };
    
        let outpath = match custom_outpath {
            Some(p) => p.to_owned(),
            None => {// e.g. path/to/infile.mp4 -> path/to/infile_1000-14000.mp4
                media_path.with_file_name(format!("{}_{}-{}.{}",
                    media_path.file_stem()
                        .ok_or(EafError::FileNameMissing(media_path.display().to_string()))?
                        .to_string_lossy()
                        .to_string(),
                    start_ms,
                    end_ms,
                    media_path.extension()
                        .ok_or(EafError::FileExtensionMissing(media_path.display().to_string()))?
                        .to_string_lossy()
                        .to_string()
                ))
            }
        };
    
        Command::new(&ffmpeg)
            .args(&[
                "-loglevel", "fatal",
                "-guess_layout_max", "0", // ffmpeg does not guess channel layout
                "-bitexact", // ffmpeg does not include LIST metadata
                "-i", &media_path.display().to_string(),
                "-ss", &format!("{}", start_ms as f64/1000.0), // start point in ms
                "-t", &format!("{}", (end_ms - start_ms) as f64/1000.0), // duration from start point in ms
                "-c", "copy", // copy streams, no recoding, faster and preserves input quality
                &outpath.display().to_string()
            ])
            .output()?;
        
        Ok(outpath)
    }
}

// pub fn extract_clip(video_path: &Path) -> std::io::Result<()> {
//     // RUN FFMPEG
//     let key_start = key.timeslot1 as f64 / 1000.0; // decimal seconds seem ok for ffmpeg? or HH:MM:SS.sss
//     let key_duration = (key.timeslot2 - key.timeslot1) as f64 / 1000.0 + wav_buffer as f64; // ok?
//     // println!("start:    {}\nduration: {}", key_start, key_duration);
//     let ffmpeg_arguments = ["-loglevel", "error", "-i", &format!("{:?}", wav).replace("\"", ""), "-ss", &format!("{}", key_start), "-t", &format!("{}", key_duration), &format!("{:?}", split_wav).replace("\"", "")];
    
//     println!("[{}/{}] Generating new EAF and WAV files from {}-{}s for key \"{}\"", splits, key_annotations.len(), key.timeslot1 as f64 / 1000.0, key.timeslot2 as f64 / 1000.0, key.text);
//     print!("  Extracting new WAV to {:?}... ", &split_wav);
//     std::io::stdout().flush().unwrap();
//     Command::new("ffmpeg")
//             .args(&ffmpeg_arguments)
//             .output()
//             .expect("(!) Failed to execute process. Is ffmpeg in PATH?");
//     println!("Done");

//     Ok(())
// }
