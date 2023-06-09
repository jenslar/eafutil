//! Generate an ELAN-file from a CSV-file with annotations.
//! Delimiter options are: comma (default), tab, or semi-colon.
//! One row per annotation. Requires a column denoting start time, end time,
//! and annotation value in chronological order (not checked).
//! Time stamps can be either millseconds as a positive integer,
//! or HH:MM:SS.fff - sub-seconds `fff` are optional.

use std::{path::PathBuf, collections::HashMap, error::Error};

use csv::{self, Trim};
use time::Duration;

use eaf_rs::eaf::{Eaf, Tier, StereoType};

/// Takes a string representing a duration in the form `HH:MM:SS` or `HH:MM:SS.fff` (hours:minutes:seconds.sub-seconds) and returns `chrono::Duration`. E.g. `00:03:54` or `00:03:54.234`.
/// Sub-seconds are optional.
fn hms2duration(hmsf: &str) -> Result<Duration, Box<dyn Error>> { // Box<dyn Error> for parse int + float errors...
    let mut duration = Duration::hours(0);
    // split should have len 3 or 4 (.count() consumes iterator)
    for (i, value) in hmsf.split(":").enumerate() {
        match i {
            0 => duration = duration + Duration::hours(value.parse()?),
            1 => duration = duration + Duration::minutes(value.parse()?),
            2 => {
                // Parse to float and add as milliseconds
                // to catch sec + sub-second value if '.fff' present.
                let t: f64 = value.parse()?;
                duration = duration + Duration::milliseconds((t * 1000.0) as i64);
            },
            _ => break
        }
    }

    Ok(duration)
}

/// csv2eaf main
pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    // CSV-file path
    let csv_path = args.get_one::<PathBuf>("csv").unwrap(); // clap ensures value

    // Debug, verbose print of CSV-file
    let debug = *args.get_one::<bool>("debug").unwrap();
    
    // Set delimiter. Default: comma.
    let delimiter = match args.get_one::<String>("delimiter").cloned().unwrap_or_default().as_str() { // clap ensures value
        "comma" => b',',
        "semicolon" => b';',
        "tab" => b'\t',
        d => {
            println!("(!) Invalid delimiter '{d}'.");
            std::process::exit(1)
        }
    };

    // default value "0"
    // currently unimplemented
    let _offset: i64 = match args.get_one::<String>("offset").unwrap().parse() {
        Ok(num) => num,
        Err(err) => {
            println!("(!) 'offset' must be an integer: {err}");
            std::process::exit(1)
        }
    };

    // Read csv file
    let reader_builder = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(Trim::All)
        .delimiter(delimiter)
        // .double_quote(false)
        // .escape(Some(b'\\'))
        // .flexible(true)
        // .comment(Some(b'#'))
        .from_path(csv_path);

    let mut reader = match reader_builder {
        Ok(rdr) => rdr,
        Err(err) => {
            println!("(!) Error parsing '{}': {err}", csv_path.display());
            std::process::exit(1)
        }
    };

    if debug {
        println!("-- FILE START --");
        match reader.headers() {
            Ok(hdr) => println!("[HDR] LEN: {:3} | {hdr:?}", hdr.len()),
            Err(err) => println!("[HDR] FAILED TO READ HEADERS: {err}")
        }
        for result in reader.records() {
            match result {
                Ok(record) => println!("[REC] LEN: {:3} | {record:?}", record.len()),
                Err(err) => println!("[REC] FAILED TO READ RECORD: {err}")
            }
        }
        println!("--- FILE END ---");
        std::process::exit(0)
    }

    // Column headers. Defaults: start, end, values.
    let start_col = args.get_one::<String>("start").unwrap(); // clap ensures value
    let end_col = args.get_one::<String>("end").unwrap(); // clap ensures value
    let values_col = args.get_one::<String>("values").unwrap(); // clap ensures value
    // column headers for values that go into referred tiers, one for each ref col
    let ref_values: Vec<String> = args.get_many::<String>("ref-values").unwrap_or_default().cloned().collect();

    let video = args.get_one::<PathBuf>("video");

    // Annotations in main tier
    // (annotation value, time value start, time value end, ref annotation value)
    let mut main_annotations: Vec<(String, i64, i64)> = Vec::new();
    // Annotations in referred tier, is specified.
    // Must equal in length to 'annotations'
    // Will use the annotation ID as value with same vec index
    // in 'annotations'.
    // Hashmap: key: ref_tier id, value: ref_tier annotation values
    let mut ref_annotations: HashMap<String, Vec<String>> = HashMap::new();

    // hashmap example from
    // https://docs.rs/csv/latest/csv/tutorial/index.html#delimiters-quotes-and-variable-length-records
    // 'Record' derived via serde support: each row becomes a hashmap if parse successful.
    type Record = HashMap<String, String>;
    for result in reader.deserialize() {
        
        let record: Record = result?;
        
        let value = match record.get(values_col) {
            Some(val) => val,
            None => {
                println!("(!) No column named '{values_col}'");
                std::process::exit(1)
            }
        };
        let t1 =  match record.get(start_col) {
            Some(t) => {
                // First try parsing to i64, from e.g. "112300"...
                if let Ok(num) = t.parse::<i64>() {
                    num
                // ... then try parsing HH:MM:SS.fff ...
                } else if let Ok(dur) = hms2duration(t) {
                    // i128 -> i64 cast for ms should be "safe" for video clip durations
                    dur.whole_milliseconds() as i64
                // ...or fail.
                } else {
                    println!("(!) Failed to convert '{t}' to milliseconds.");
                    std::process::exit(1)
                }
            },
            None => {
                println!("(!) No column named '{start_col}'");
                std::process::exit(1)
            }
        };
        let t2 =  match record.get(end_col) {
            Some(t) => {
                // First try parse to i64, from e.g. "112300"...
                if let Ok(num) = t.parse::<i64>() {
                    num
                // ... then try parsing HH:MM:SS.fff ...
                } else if let Ok(dur) = hms2duration(t) {
                    dur.whole_milliseconds() as i64 // i128 -> i64 cast should be "ok" for video clips
                // ...or fail.
                } else {
                    println!("(!) Failed to convert '{t}' to milliseconds.");
                    std::process::exit(1)
                }
            },
            None => {
                println!("(!) No column named '{end_col}'");
                std::process::exit(1)
            }
        };

        println!("{t1:>10} - {t2:<10} | {} ", value.to_owned());

        main_annotations.push((value.to_owned(), t1, t2));

        // build ref tiers content
        for ref_col in ref_values.iter() {
            if let Some(ref_annotation) = record.get(ref_col.as_str()) {
                ref_annotations.entry(ref_col.to_owned())
                    .or_insert(Vec::new())
                    .push(ref_annotation.to_owned());
            } else {
                println!("(!) No column named '{ref_col}'. Ignoring.");
                // std::process::exit(1)
            }
        }
    }

    // Map optional video path to vec
    let media_paths = match video {
        Some(p) => vec!(PathBuf::from(p)),
        None => Vec::new()
    };

    let mut eaf = match Eaf::from_values(&main_annotations,Some(values_col)) {
        Ok(e) => e,
        Err(err) => {
            println!("(!) Failed to generate ELAN-file: {err}");
            std::process::exit(1)
        }
    };

    // Link media files
    eaf.with_media_mut(&media_paths);

    let main_tier = match eaf.get_tier(&values_col) {
        Some(t) => t.to_owned(),
        None => {
            println!("(!) Error retrieving tier '{values_col}'");
            std::process::exit(1)
        }
    };

    println!("Created main tier '{}'", main_tier.tier_id);

    // add tier
    // add ling type
    // add constraint
    for (k, v) in ref_annotations.iter() {
        let idx = eaf.index.a2idx.len();
        println!("{}", idx);
        // TODO wrong stereotype/constraints in ling type, but constraint added...
        let tier = match Tier::ref_from_values(v, k, &main_tier, k, Some(idx+1)) {
            Ok(t) => t,
            Err(err) => {
                println!("(!) Error creating tier: {err}");
                std::process::exit(1)
            }
        };

        let tier_id = tier.tier_id.to_owned();
        
        // TODO wrong stereotype/constraints in ling type, but constraint added...
        if let Err(err) = eaf.add_tier(Some(tier), Some(&StereoType::SymbolicAssociation)) {
            println!("(!) Error adding tier: {err}");
            std::process::exit(1)
        };
        println!("Added referred tier '{}'", tier_id);
    
        eaf.index();
    }


    // let eaf_str = eaf.serialize()?;
    let eaf_path = csv_path.with_extension("eaf");
    match eaf.write(&eaf_path) {
        Ok(true) => println!("Wrote {}", eaf_path.display()),
        Ok(false) => println!("User aborted writing ELAN-file"),
        Err(err) => {
            println!("(!) Failed to write '{}': {err}", eaf_path.display());
            std::process::exit(1)
        },
    }
    
    // // TODO change to same writefile as in eaf-rs, that returns Result<bool, EafError>
    // if let Err(err) = writefile(&eaf_str.as_bytes(), &eaf_path) {
    //     println!("(!) Failed to write '{}': {err}", eaf_path.display());
    //     std::process::exit(1)
    // };

    // match writefile(&eaf_str.as_bytes(), &eaf_path) {
    //     Ok(wrote_file) => match wrote_file {
    //         true => println!("Wrote {}", eaf_path.display()),
    //         false => println!("Aborted writing {}", eaf_path.display()),
    //     },
    //     Err(err) => println!("(!) Failed to write '{}': {err}", eaf_path.display())
    // }

    Ok(())
}