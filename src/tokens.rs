//! Prints all words/tokens in an ELAN-file. It is possible to remove prefixes and suffixes
//! such as brackets, punctuation etc with the `strip` flag. If these remove too much, or are
//! not sufficient, user the `prefix` and/or `suffix` options to customize.
//! Possible to list words/tokens in a specific tier, including word distribution/frequency.
//! 
//! The `strip` flag removes the following:
//! - prefixes: `#*_<{([-"'=`
//! - suffixes: `#*_>})]-"'=.,:;!?`

use std::{path::{Path, PathBuf}, collections::HashMap};

use super::eaf;

use eaf_rs::eaf::Eaf;

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let path = args.get_one::<PathBuf>("eaf");
    let ignore_case = *args.get_one::<bool>("ignore-case").unwrap(); // ensured by clap
    let mut prefix = args.get_one::<String>("prefix").cloned();
    let mut suffix = args.get_one::<String>("suffix").cloned();
    if *args.get_one::<bool>("strip-common").unwrap() {
        let common_pre = "#*_<{([-\"\'=";
        let common_suf = "#*_>})]-\"\'=.,:;!?";
        prefix = prefix
            .map(|s| format!("{s}{common_pre}"))
            .or(Some(common_pre.to_owned()));
        suffix = suffix
            .map(|s| format!("{s}{common_suf}"))
            .or(Some(common_suf.to_owned()));
    };
    let distribution = *args.get_one::<bool>("distribution").unwrap();
    let select_tier = *args.get_one::<bool>("select-tier").unwrap();
    let alphaorder = *args.get_one::<bool>("sort-alphabetically").unwrap();
    let reverse = *args.get_one::<bool>("sort-reverse").unwrap();
    // distribution = count instances of each words so set unique to false
    let unique = match distribution {
        true => false,
        false => *args.get_one::<bool>("unique").unwrap()
    };

    let mut tokens: Vec<String> = Vec::new();

    if let Some(p) = path {
        if Path::new(p).is_dir() {
            println!("(!) {} is a directory.", p.display());
            std::process::exit(1)
        }

        let eaf = match Eaf::de(p, true) {
            Ok(f) => f,
            Err(err) => {
                println!("(!) Failed to parse '{}': {err}", p.display());
                std::process::exit(1)
            }
        };

        if select_tier {
            let tier = match eaf::select_tier(&eaf, false) {
                Ok(t) => t,
                Err(err) => {
                    println!("(!) Failed to extract tier: {err}");
                    std::process::exit(1)
                }
            };
            tokens = tier.tokens(prefix.as_deref(), suffix.as_deref(), unique, ignore_case);
        } else {
            tokens = eaf.tokens(prefix.as_deref(), suffix.as_deref(), unique, ignore_case);
        }
    }

    // if let Some(d) = dir {
    //     if Path::new(d).is_file() {
    //         println!("(!) {d} is a file.");
    //         std::process::exit(1)
    //     }

    //     unimplemented!()
    // }

    if distribution {
        let mut count: HashMap<String, usize> = HashMap::new();

        tokens.iter()
            .for_each(|w| {
                *count.entry(w.to_owned()).or_insert(0) += 1;
            });

        let mut sorted: Vec<(String, usize)> = count.iter()
            .map(|(k, v)| (k.to_owned(), *v))
            .collect();

        if alphaorder {
            sorted.sort_by_key(|(w, _)| w.to_owned());
        } else {
            sorted.sort_by_key(|(_, c)| *c);
        }

        if reverse {
            sorted.reverse()
        }

        for (word, count) in sorted.iter() {
            println!("{count:>6}: {word}")
        }

    } else {
        println!("{}", tokens.join(", "));
    }

    println!("---");
    println!("Count:          {} tokens", tokens.len());
    println!("Unique only:    {}", unique || distribution);
    println!("Ignore case:    {ignore_case}");
    println!("Strip prefixes: {}", prefix.as_deref().unwrap_or("None"));
    println!("Strip suffixes: {}", suffix.as_deref().unwrap_or("None"));

    Ok(())
}