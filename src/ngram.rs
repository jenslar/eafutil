use std::path::PathBuf;

use eaf_rs::eaf::{
    Eaf,
    Scope
};
use regex::Regex;

use crate::eaf::select_tier;

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let eaf_path = args.get_one::<PathBuf>("eaf").unwrap(); // clap ensures val
    let scope = args.get_one::<String>("scope").unwrap(); // clap default: annotation
    let size = *args.get_one::<usize>("ngram-size").unwrap();
    let ignore_case = *args.get_one::<bool>("ignore-case").unwrap();
    let delete_custom_string = args.get_one::<String>("remove-custom");
    let mut delete_string = match *args.get_one::<bool>("remove-common").unwrap() {
        true => Some("[\"\'#*<>{}()\\[\\].,:;!/?=\\\\_]".to_owned()), // include '-'?
        false => None
    };
    if let Some(custom) = &delete_custom_string {
        delete_string = delete_string.map(|c| format!("{c}{custom}"));
    }
    // NOTE '-' can only be literal in regex range if it is the last character.
    // Issues with raw string (r"") and regex escapes
    let delete_regex: Option<Regex> = match delete_string {
        Some(s) => match Regex::new(&s) {
            Ok(rx) => Some(rx),
            Err(err) => {
                println!("(!) Regex error: {err}");
                std::process::exit(1)
            }
        }
        None => None
    };

    let eaf = match Eaf::read(eaf_path) {
        Ok(f) => f,
        Err(err) => {
            println!("(!) Failed to parse '{}': {err}", eaf_path.display());
            std::process::exit(1)
        }
    };

    let ngrams_map = match scope.as_str() {
        "annotation" => {
            let tier = match select_tier(&eaf, false) {
                Ok(t) => t,
                Err(err) => {
                    println!("(!) Failed to extract tier: {err}");
                    std::process::exit(1)
                }
            };
            eaf.ngram(size, delete_regex.as_ref(), Scope::Annotation(Some(tier.tier_id)))
        },
        "tier" => {
            let tier = match select_tier(&eaf, false) {
                Ok(t) => t,
                Err(err) => {
                    println!("(!) Failed to extract tier: {err}");
                    std::process::exit(1)
                }
            };
            eaf.ngram(size, delete_regex.as_ref(), Scope::Tier(Some(tier.tier_id)))
        },
        "file" => {
            eaf.ngram(size, delete_regex.as_ref(), Scope::File)
        },
        s => {
            println!("(!) '{s}' is not a valid scope. Choose one of 'annotation', 'tier', 'file'.");
            std::process::exit(1)
        }
    };

    let mut ngrams: Vec<(String, usize)> = ngrams_map.iter()
        .map(|(k,v)| (k.to_owned(), v.to_owned()))
        .collect();
        
    ngrams.sort_by_key(|(_, v)| *v);

    if ngrams.is_empty() {
        println!("No annotation of length {size} or greater for selected context.");
    }
    for (i, (ngram, count)) in ngrams.iter().enumerate() {
        println!("{:4}. {ngram:>40}: {count}", i+1)
    }

    Ok(())
}