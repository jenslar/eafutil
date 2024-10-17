//! Find a string in a single ELAN-file or in all ELAN-files found in the specified
//! directory. Lists tier ID, annotation ID, and annotation "index"
//! (the list number in the Grid tab in ELAN) for each match.
//! Currently does not support regular expressions.

use std::{
    path::{Path, PathBuf},
    ffi::OsStr,
    collections::HashMap
};
// use std::io::{self, Write};
// use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use walkdir::WalkDir;
use regex::Regex;
use eaf_rs::eaf::Eaf;

use crate::{text::process_string, files::is_hidden};

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let eaf_path = args.get_one::<PathBuf>("eaf");
    let eaf_dir = args.get_one::<PathBuf>("dir");
    let ignore_case = *args.get_one::<bool>("ignore-case").unwrap(); // ensured by clap
    let full_path = *args.get_one::<bool>("full-path").unwrap(); // ensured by clap
    // let context = *args.get_one::<bool>("context").unwrap(); // ensured by clap
    let pattern = args.get_one::<String>("pattern");
    let regex = match args.get_one::<String>("regex") {
        Some(s) => {
            let p = match ignore_case {
                true => format!(r"(?i){s}"),
                false => s.to_owned()
            };
            match Regex::new(&p) {
                Ok(rx) => Some(rx),
                Err(err) => {
                    let msg = format!("(!) '{s}' is not a valid regular expression: {err}");
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
            }
        }
        },
        None => None
    };
    let verbose = *args.get_one::<bool>("verbose").unwrap(); // ensured by clap
    let context = *args.get_one::<bool>("context").unwrap(); // ensured by clap
    let mut parse_errors: HashMap<String, Vec<PathBuf>> = HashMap::new();

    if let Some(p) = eaf_path {

        if Path::new(p).is_dir() {
            let msg = format!("(!) {} is a directory. Try '--dir <DIR>'.", p.display());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }

        let eaf = match Eaf::read(p) {
            Ok(f) => f,
            Err(err) => {
                let msg = format!("(!) Failed to parse '{}': {err}", p.display());
                return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
            }
        };

        
        let results = match (pattern, &regex) {
            (Some(ptrn), None) => eaf.query(ptrn, ignore_case),
            (None, Some(rx)) => eaf.query_rx(&rx),
            _ => {
                // Should never reach this branch since clap ensures
                // either 'pattern' or 'regex'.
                let msg = format!("(!) No search pattern provided.");
                return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
            }
        };

        let path_display = match full_path {
            true => p.display().to_string(),
            false => p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap()
        };

        if results.is_empty() {
            if verbose {
                println!("╭─[{:4}.]", path_display);
                println!("│ No matches");
                println!("╰────")
            }
        } else {
            println!("╭─[{}]", path_display);
            println!("│      Tier            Index / ID     Value");
            for (i, (a_idx, t_id, a_id, a_val, a_ref_id)) in results.iter().enumerate() {
                if context {
                    let (t_ref_id, a_ref_val) = eaf.parent_tier(t_id)
                        .map(|t| {
                            let a_ref_val = a_ref_id.as_deref()
                                .and_then(|id| t.find(&id))
                                .map(|a| a.to_str())
                                .unwrap_or("None");
                            (t.tier_id.to_owned(), a_ref_val)
                        })
                        .unwrap_or_default();
                    println!("│     {:10} [PARENT] {}",
                        t_ref_id,
                        a_ref_val
                    );
                }
                println!("│ {:2}. {:10}  {:>7} / {:<8} {}",
                    i+1,
                    process_string(&t_id, None, None, None, Some(10)),
                    a_idx,
                    a_id,
                    a_val
                );
            }
            println!("╰────")
        }
    }

    if let Some(d) = eaf_dir {

        if d.is_file() {
            let msg = format!("(!) {} is a file. Try '--eaf'.", d.display());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }

        let mut count = 0;
        let mut total = 0;
        let mut matches = 0;

        for occurence in WalkDir::new(d) {
            let path = match occurence {
                Ok(de) => {
                    let p = de.path().to_owned();
                    if p.extension() == Some(&OsStr::new("eaf")) {
                        // ignore hidden *nix files, e.g. temp eaf-files starting with "."
                        if is_hidden(&p) {
                            continue
                        }
                        p
                    } else {
                        continue
                    }
                },
                Err(_) => continue // perhaps handle error, but would mostly be permission related issues
            };

            total += 1;

            let path_display = match full_path {
                true => path.display().to_string(),
                false => path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap()
            };

            match Eaf::read(&path) {
                Ok(eaf) => {

                    // (Annotation Index, Tier ID, Annotation ID, Annotation value)
                    let results = match (pattern, &regex) {
                        (Some(ptrn), None) => eaf.query(ptrn, ignore_case),
                        (None, Some(rx)) => eaf.query_rx(&rx),
                        _ => {
                            // Should never reach this branch since clap ensures
                            // either 'pattern' or 'regex'.
                            let msg = format!("(!) No search pattern provided.");
                            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
                        }
                    };
                    matches += results.len();
        
                    if results.is_empty() {
                        if verbose {
                            println!("╭─[{:4}. {}]", total, path_display); // file ensuread above
                            println!("│ No matches");
                        } else {
                            continue
                        }
                    } else {
                        count += 1;
                        println!("╭─[{}. {}]", count, path_display); // file ensuread 
                        println!("│     Tier          Index / ID       Value");
                        for (i, (a_idx, t_id, a_id, a_val, _a_ref_id)) in results.iter().enumerate() {
                            if context {
                                let main_val = eaf.main_annotation(a_id).map(|a| a.to_str());
                                println!("│{}MAIN   {}",
                                    " ".repeat(29),
                                    if let Some(val) = main_val {val} else {"None"}
                                );
                            }
                            println!("│ {:2}. {:10}  {:>7} / {:<8} {}",
                                i+1,
                                process_string(&t_id, None, None, None, Some(10)),
                                a_idx,
                                a_id, a_val
                            );
                        }
                        println!("╰────")
                    }
                },
                Err(err) => {
                    if verbose {
                        println!("╭─[{:4}. {}]", count, path_display); // file ensuread above
                        println!("│ (!) Failed to parse '{}': {err}", path.display());
                        println!("╰────")
                    }
                    parse_errors.entry(err.to_string()).or_insert(vec!()).push(path);
                }
            };

        }

        print!("Done. Found {matches} matches in {count} files. Searched {total} files.");
        if parse_errors.is_empty() {
            println!(" No errors.")
        } else {
            println!("\n\nSome files failed to parse due to errors:");
            for (err, paths) in parse_errors.iter() {
                println!("[ERR: {}]", err);
                for path in paths {
                    println!("  {}", path.display())
                }
            }
        }
    }

    if regex.is_some() {
        println!("If '--regex' returned unexpected results, try adding citation marks around the pattern.")
    }

    Ok(())
}

// fn write_green(text: &str) -> io::Result<()> {
//     let mut stdout = StandardStream::stdout(ColorChoice::Always);
//     stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
//     writeln!(&mut stdout, "{}", text)
// }