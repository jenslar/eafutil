use std::{ffi::OsString, fs::File, io::Write, path::{Path, PathBuf}};

use clap::builder::OsStr;
use eaf_rs::Eaf;
use inquire::{formatter::MultiOptionFormatter, list_option::ListOption, validator::Validation, MultiSelect};
use itertools::Itertools;

/// Used for any confirmation, e.g. overwrite file.
pub fn confirm(message: &str) -> std::io::Result<bool> {
    loop {
        print!("{} (y/n): ", message);
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

/// Returns `true` if `path` has specified extension.
pub fn has_extension(path: &Path, ext: &str) -> bool {
    path.extension().map(|s| s.to_ascii_lowercase()) == Some(OsString::from(&ext.to_lowercase()))
}

/// Write file with confirmation if path exists
pub fn writefile(content: &[u8], path: &Path) -> std::io::Result<bool> {
    // TODO return Result<bool> instead with Ok(true) = write success, Ok(false) = user aborted write
    let write = if path.exists() {
        confirm(&format!("{} already exists. Overwrite?", path.display()))?
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

/// Checks if file name is hidden on Unix/Linux platforms.
/// Does not check directories.
// #[cfg(not(windows))] skipped, since compiler can't find is_hidden for windows compilation...
pub fn is_hidden(path: &Path) -> bool {
    path.file_name().map(|s| s.to_string_lossy().starts_with(".")) == Some(true)
}

// /// Checks if file name is hidden on Windows platforms.
// /// Does not check directories.
// #[cfg(windows)]
// pub fn is_hidden(path: &Path) -> bool {
//     use std::os::windows::MetadataExt;
//     MetadataExt::file_attributes(path)
// }

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

/// Adds suffix to existing file stem of path and returns the new path.
/// Returns path untouched if no file stem can be extracted.
pub fn affix_file_name(path: &Path, prefix: Option<&str>, suffix: Option<&str>, delimiter: Option<&str>) -> PathBuf {
    let delim_prefix = if prefix.is_none() {""} else {delimiter.unwrap_or_default()};
    let prefix = prefix.unwrap_or_default();
    let delim_suffix = if suffix.is_none() {""} else {delimiter.unwrap_or_default()};
    let suffix = suffix.unwrap_or_default();
    let new_path = match path.file_stem().and_then(|s| s.to_str()) {
        Some(stem) => path.with_file_name(format!("{prefix}{delim_prefix}{stem}{delim_suffix}{suffix}")),
        None => path.to_owned()
    };
    if let Some(ext) = path.extension() {
        return new_path.with_extension(ext)
    }
    new_path
}

pub fn file_stem_as_string(path: &Path) -> Option<String>{
    path.file_stem().map(|p| p.to_string_lossy().to_string())
}


pub fn select_tiers(eaf: &Eaf) -> Result<Vec<String>, inquire::InquireError> {
    let options = eaf.tier_ids(); // .iter().map(|s| s.as_str()).collect_vec();

    // let validator = |a: &[ListOption<&&str>]| {
    //     if a.len() < 2 {
    //         return Ok(Validation::Invalid("This list is too small!".into()));
    //     }

    //     let x = a.iter().any(|o| *o.value == "Pineapple");

    //     match x {
    //         true => Ok(Validation::Valid),
    //         false => Ok(Validation::Invalid("Remember to buy pineapples".into())),
    //     }
    // };

    let formatter: MultiOptionFormatter<'_, &str> = &|a| format!("{} tiers", a.len());

    let ans = MultiSelect::new("Select tiers to include:", options.iter().map(|s| s.as_str()).collect_vec())
        // .with_validator(validator)
        .with_formatter(formatter)
        .prompt();

    ans.map(|a| a.iter().cloned().map(|s| s.to_owned()).collect())

    // match ans {
    //     Ok(_) => println!("I'll get right on it"),
    //     // Ok(_) => println!("I'll get right on it"),
    //     Err(_) => println!("The shopping list could not be processed"),
    // }
}
