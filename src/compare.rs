
use std::path::PathBuf;

use itertools::Itertools;
use itertools::EitherOrBoth::{Both, Left, Right};

use eaf_rs::eaf::Eaf;

use crate::{
    eaf::select_tier,
    text::process_string,
};

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {

    let eaf_path = args.get_one::<PathBuf>("eaf").unwrap(); // clap ensures value

    let mut compact = *args.get_one::<bool>("compact").unwrap();
    let timeline = *args.get_one::<bool>("timeline").unwrap();
    let max_len = *args.get_one::<usize>("max-length").unwrap(); // clap default 50

    if !compact && !timeline {
        println!("SETTING COMPACT TO TRUE");
        compact = true;
    }

    let eaf = match Eaf::de(eaf_path, true) {
        Ok(f) => f,
        Err(err) => {
            println!("(!) Error parsing '{}': {err}", eaf_path.display());
            std::process::exit(1)
        }
    };

    let tier1 = match select_tier(&eaf, false) {
        Ok(t) => t,
        Err(err) => {
            println!("(!) Failed to extract tier: {err}");
            std::process::exit(1)
        }
    };

    let tier2 = match select_tier(&eaf, false) {
        Ok(t) => t,
        Err(err) => {
            println!("(!) Failed to extract tier: {err}");
            std::process::exit(1)
        }
    };

    let line_len = if compact {max_len * 2 + 48} else {max_len * 2 + 30};
    let len = if compact {max_len + 18} else {max_len + 5};
    println!("{}", ".".repeat(line_len));
    let pad = if compact {8} else {23};
    println!("  {:>len$}{}{}", tier1.tier_id, " ".repeat(pad), tier2.tier_id);
    println!("{}", ".".repeat(line_len));

    if timeline {

        let mut all_annotations: Vec<_> = tier1.iter().chain(tier2.iter())
            .map(|a| {
                let tier_id = match a.tier_id() {
                    Some(id) => id,
                    _ => {
                        println!("(!) Missing tier ID for annotation with ID '{}'", a.id());
                        std::process::exit(1)
                    }
                };
                let (ts1, ts2) = match a.ts_val() {
                    (Some(t1), Some(t2)) => (t1, t2),
                    _ => {
                        println!("(!) Missing time value for annotation with ID '{}'", a.id());
                        std::process::exit(1)
                    }
                };
                // (tier_id, ts1, ts2, a.value())
                (tier_id, ts1, ts2, a.to_str())
            })
            .collect();

        // Sort annotations on start time value
        all_annotations.sort_by_key(|a| a.1);

        for (i, a) in all_annotations.iter().enumerate() {
            // match on tier id to print to the left or right
            // (tier ID, t1, t2, value)
            match &a.0 {
                i1 if i1 == &tier1.tier_id => {
                    println!("{:04} | {:>max_len$} |{:8} - {:<8}|",
                        i + 1,
                        process_string(&a.3, None, None, None, Some(max_len)),
                        a.1,
                        a.2,
                    );
                },
                i2 if i2 == &tier2.tier_id => {
                    println!("{:04} | {} |{:8} - {:<8}| {:<max_len$}",
                        i + 1,
                        " ".repeat(max_len),
                        a.1,
                        a.2,
                        process_string(&a.3, None, None, None, Some(max_len)),
                    );
                },
                _ => println!("(!) Unknown tier ID")
            }
        };
    }

    if compact {
        for (i, annots) in tier1.iter().zip_longest(tier2.iter()).enumerate() {
            match annots {
                Both(a1, a2) => {
                    let (t1_1, t2_1) = a1.ts_val();
                    let (t1_2, t2_2) = a2.ts_val();
                    println!("{:>8} - {:<8} {:>max_len$} |{:04}| {:<max_len$} {:>8} - {:<8}",
                        t1_1.map(|n| n.to_string()).unwrap_or("NONE".to_owned()),
                        t2_1.map(|n| n.to_string()).unwrap_or("NONE".to_owned()),
                        // process_string(&a1.value(), None, None, None, Some(max_len)),
                        process_string(a1.to_str(), None, None, None, Some(max_len)),
                        i + 1,
                        // process_string(&a2.value(), None, None, None, Some(max_len)),
                        process_string(a2.to_str(), None, None, None, Some(max_len)),
                        t1_2.map(|n| n.to_string()).unwrap_or("NONE".to_owned()),
                        t2_2.map(|n| n.to_string()).unwrap_or("NONE".to_owned()),
                    );
                }
                Left(a1) => {
                    let (t1_1, t2_1) = a1.ts_val();
                    println!("{:>8} - {:<8} {:>max_len$} |{:04}|",
                        t1_1.map(|n| n.to_string()).unwrap_or("NONE".to_owned()),
                        t2_1.map(|n| n.to_string()).unwrap_or("NONE".to_owned()),
                        // process_string(&a1.value(), None, None, None, Some(max_len)),
                        process_string(a1.to_str(), None, None, None, Some(max_len)),
                        i + 1,
                    );
                }
                Right(a2) => {
                    let (t1_2, t2_2) = a2.ts_val();
                    println!("{} |{:04}| {:<max_len$} {:>8} - {:<8}",
                        " ".repeat(max_len + 21),
                        i + 1,
                        // process_string(&a2.value(), None, None, None, Some(max_len)),
                        process_string(a2.to_str(), None, None, None, Some(max_len)),
                        t1_2.map(|n| n.to_string()).unwrap_or("NONE".to_owned()),
                        t2_2.map(|n| n.to_string()).unwrap_or("NONE".to_owned()),
                    );
                }
            }
        }
    }

    Ok(())
}