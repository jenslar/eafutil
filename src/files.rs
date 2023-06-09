use std::{io::Write, path::{Path, PathBuf}, fs::File};

/// Used for any acknowledgement, e.g. overwrite file.
pub fn acknowledge(message: &str) -> std::io::Result<bool> {
    loop {
        print!("(!) {} (y/n): ", message);
        std::io::stdout().flush()?;
        let mut overwrite = String::new();
        let _ = std::io::stdin().read_line(&mut overwrite)?;

        return match overwrite.to_lowercase().trim() {
            "y" | "yes" => Ok(true),
            "n" | "no" => Ok(false),
            _ => {
                println!("Enter y/yes or n/no");
                continue;
            }
        };
    }
}

/// Write file with confirmation if path exists
pub fn writefile(content: &[u8], path: &Path) -> std::io::Result<bool> {
    // TODO return Result<bool> instead with Ok(true) = write success, Ok(false) = user aborted write
    let write = if path.exists() {
        acknowledge(&format!("{} already exists. Overwrite?", path.display()))?
    } else {
        true
    };

    if write {
        let mut outfile = File::create(&path)?;
        outfile.write_all(content)?;
        return Ok(true)
    }

    Ok(false)
}

pub fn is_hidden(path: &Path) -> bool {
    path.file_name().map(|s| s.to_string_lossy().starts_with(".")) == Some(true)
}

/// Adds suffix to existing file stem of path and returns the new path.
/// Returns path untouched if no file stem can be extracted.
pub fn append_file_name(path: &Path, suffix: &str) -> PathBuf {
    let new_path = match path.file_stem().and_then(|s| s.to_str()) {
        Some(stem) => path.with_file_name(format!("{stem}_{suffix}")),
        None => path.to_owned()
    };
    if let Some(ext) = path.extension() {
        return new_path.with_extension(ext)
    }
    new_path
}
