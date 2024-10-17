//! Generate an ELAN-file from a CSV-file with annotations.
//! Delimiter options are: comma (default), tab, or semi-colon.
//! One row per annotation. Requires a column denoting start time, end time,
//! and annotation value in chronological order (not checked).
//! Time stamps can be either millseconds as a positive integer,
//! or HH:MM:SS.fff - sub-seconds `fff` are optional.

use std::{path::PathBuf, collections::{HashMap, HashSet}, error::Error};

use csv::{self, Trim};
use itertools::Itertools;
use time::Duration;

use eaf_rs::{eaf::{Eaf, Tier, StereoType}, EafError, LinguisticType};
use rttm_rs::{Rttm, RttmSegment};

use crate::files::writefile;

/// Takes a string representing a duration in the form
/// `HH:MM:SS` (hours:minutes:seconds),
/// `HH:MM:SS.fff` (hours:minutes:seconds.sub-seconds),
/// `HH:MM:SS,fff` (hours:minutes:seconds.sub-seconds)
/// and returns `chrono::Duration`. E.g. `00:03:54` or `00:03:54.234`.
/// Sub-seconds are optional.
fn hms2duration(hmsf: &str) -> Result<Duration, Box<dyn Error>> { // Box<dyn Error> for parse int + float errors...
    let mut duration = Duration::hours(0);
    // split should have len 3 or 4 (.count() consumes iterator)
    // TODO perhaps check len of .split(":") and if 2 assume MM:SS, rather than HH:MM?
    for (i, value) in hmsf.split(":").enumerate() {
        match i {
            0 => duration = duration + Duration::hours(value.parse()?),
            1 => duration = duration + Duration::minutes(value.parse()?),
            2 => {
                // Parse to float and add as milliseconds
                // to catch sec + sub-second value if '.fff' or ',fff' present
                let t: f64 = value.trim().replace(",", ".").parse()?;
                duration = duration + Duration::milliseconds((t * 1000.0) as i64);
            },
            _ => break
        }
    }

    Ok(duration)
}

fn string2ms(value: &str) -> Option<i64> {
    // First try parsing to i64, from e.g. "112300"...
    if let Ok(num) = value.parse::<i64>() {
        Some(num)
    // ... then try SS.fff as float in seconds and sub-seconds, from e.g. 13.454 added 230815
    } else if let Ok(num) = value.parse::<f64>() {
        Some((num * 1000.).round() as i64)
    // ... then try parsing HH:MM:SS.fff ...
    } else if let Ok(dur) = hms2duration(value) {
        // i128 -> i64 cast for ms should be "safe" for video clip durations
        Some(dur.whole_milliseconds() as i64)
    // ...or fail.
    } else {
        None
    }
}

/// csv2eaf main
pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    // CSV-file path
    let csv_path = args.get_one::<PathBuf>("csv").unwrap(); // clap ensures value

    // Debug, verbose print of CSV-file
    let debug = *args.get_one::<bool>("debug").unwrap();

    // Media files to link.
    let media: Vec<PathBuf> = args.get_many::<PathBuf>("media")
        .map(|m| m.into_iter().map(|p| p.into()).collect())
        .unwrap_or_default();
    
    // Set delimiter. Default: comma.
    let mut delimiter_string = args.get_one::<String>("delimiter").cloned().unwrap_or_default(); // clap ensures value
    let mut delimiter = match delimiter_string.as_str() { // clap ensures value
        "comma" => b',',
        "semicolon" => b';',
        "tab" => b'\t',
        d => {
            let msg = format!("(!) Invalid delimiter '{d}'.");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
        }
    };
    let mut has_headers = true;

    let rttm_mode = *args.get_one::<bool>("rttm").unwrap();

    if rttm_mode {
        println!("RTTM mode: Setting delimiter to single space and no column headers.");
        delimiter_string = String::from("space");
        delimiter = b' ';
        has_headers = false;
    }

    // default value "0"
    // currently unimplemented
    let _offset: i64 = match args.get_one::<String>("offset").unwrap().parse() {
        Ok(num) => num,
        Err(err) => {
            let msg = format!("(!) 'offset' must be an integer: {err}");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
        }
    };

    // Read csv file
    let reader_builder = csv::ReaderBuilder::new()
        .has_headers(has_headers)
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
            let msg = format!("(!) Error parsing '{}': {err}", csv_path.display());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
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
        println!("Delimiter: {delimiter_string}");
        println!("Headers:   {has_headers}");
        println!("RTTM mode: {rttm_mode}");
        println!("Media:     {media:?}");
        
        return Ok(())
    }

    // Annotations in main tier
    // (annotation value, time value ms start, time value ms end, ref annotation value)
    let mut main_annotations: Vec<(String, i64, i64, String)> = Vec::new();
    // Annotations in referred tier, if specified.
    // Must equal in length to 'annotations'
    // Will use the annotation ID as value with same vec index
    // in 'annotations'.
    // Hashmap: key: (ref_tier ID, parent tier ID) value: ref_tier annotation values
    let mut ref_annotations: HashMap<(String, String), Vec<String>> = HashMap::new();
    let mut ref_col_missing: HashSet<String> = HashSet::new();

    // Parse as RTTM file
    if rttm_mode {
        let rttm = Rttm::read(csv_path, false)?;
        rttm.iter()
            .for_each(|seg| {
                let (t1, t2) = seg.timespan_ms();
                main_annotations.push((String::from(""), t1, t2, seg.speaker_name.to_owned()))
            });

    // Parse as CSV file with headers
    } else {

        // Column headers. Defaults: start, end, values.
        let start_col = args.get_one::<String>("start").unwrap(); // clap ensures value
        let end_col = args.get_one::<String>("end").unwrap(); // clap ensures value
        let values_col = args.get_one::<String>("values").unwrap(); // clap ensures value
        // column with tier ID if multiple main tiers in one sheet
        let tier_id_col = args.get_one::<String>("tier-id");
        // column headers for values that go into referred tiers, one for each ref col
        let ref_values: Vec<String> = args.get_many::<String>("ref-values").unwrap_or_default().cloned().collect();
        if let Some(_) = args.get_many::<String>("ref-values") {
            println!("(!) Ref values will be ignored for now");
        }

        // If columns with referred annotation values are specified,
        // column with tier ID must also be specified (since this will be the parent tier)
        if !ref_values.is_empty() && tier_id_col.is_none() {
            let msg = format!("Tier ID column not specifified, required for referred annotaions.");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }
    
        // hashmap example from
        // https://docs.rs/csv/latest/csv/tutorial/index.html#delimiters-quotes-and-variable-length-records
        // 'Record' derived via serde support: each row becomes a hashmap if parse successful.
        type HeaderRecord = HashMap<String, String>;
        for result in reader.deserialize() {
            
            let record: HeaderRecord = result?; // one row
            
            let value = match record.get(values_col) {
                Some(val) => val,
                None => {
                    let msg = format!("(!) No column named '{values_col}'");
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
                }
            };
            let t1 =  match record.get(start_col) {
                Some(t) => {
                    if let Some(num) = string2ms(t) {
                        num
                    } else {
                        let msg = format!("(!) Start time: Failed to convert '{t}' to milliseconds.");
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
                    }
                },
                None => {
                    let msg = format!("(!) No column named '{start_col}'");
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
                }
            };
            let t2 =  match record.get(end_col) {
                Some(t) => {
                    if let Some(num) = string2ms(t) {
                        num
                    } else {
                        let msg = format!("(!) End time: Failed to convert '{t}' to milliseconds.");
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
                    }
                },
                None => {
                    let msg = format!("(!) No column named '{end_col}'");
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
                }
            };
            let tier_id = match tier_id_col {
                Some(id) => match record.get(id) {
                    Some(val) => val.to_owned(),
                    None => {
                        let msg = format!("(!) No column named '{end_col}'");
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
                    }
                },
                None => String::from("default")
            };
    
            println!("{t1:>10} - {t2:<10} | {} ", value.to_owned());
    
            main_annotations.push((value.to_owned(), t1, t2, tier_id.to_owned()));
    
            // Since ref value correspond to a single row, it is always known which the parent tier is
            // build ref tiers content
            for ref_col in ref_values.iter() {
                if let Some(ref_annotation) = record.get(ref_col.as_str()) {
                    ref_annotations.entry((ref_col.to_owned(), tier_id.to_owned()))
                        .or_insert(Vec::new())
                        .push(ref_annotation.to_owned());
                } else {
                    println!("(!) No column named '{ref_col}'. Ignoring.");
                    ref_col_missing.insert(ref_col.to_owned());
                }
            }
        }
    }




    // // Map optional video path to vec
    // let media_paths = match media {
    //     Some(p) => vec!(PathBuf::from(p)),
    //     None => Vec::new()
    // };

    let mut eaf = match Eaf::from_values_multi(&main_annotations) {
        Ok(mut e) => {
            let main_a_count = e.a_len();
            let mut ref_count = 0;
            if !ref_annotations.is_empty() {
                let lt_type_ref = "ref-tier-symbolic-association";
                let st = StereoType::SymbolicAssociation;
                // let lt = LinguisticType::new(lt_type_ref, Some(&st));
                // e.add_linguistic_type(&lt, true);
                
                for ((ref_id, parent_id), annotations) in ref_annotations.iter() {
                    if let Some(parent) = e.get_tier(&parent_id) {
                        if let Ok(t) = Tier::ref_from_values(&annotations, ref_id, parent, lt_type_ref, Some(main_a_count + ref_count)) {
                            ref_count += t.len();
                            e.add_tier(Some(t), Some(&st))?;
                        }
                    }
                }
            }
            e
        },
        Err(err) => {
            let msg = format!("(!) Failed to generate ELAN-file: {err}");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
        }
    };

    // Link media files
    eaf.with_media_mut(&media);

    println!("Generated the following tiers:");
    for (i, tier) in eaf.tiers.iter().enumerate() {
        println!("{}. {} ({} annotations)", i+1, tier.tier_id, tier.len());
    }

    if !ref_col_missing.is_empty() {
        println!("Missing referred tier column headers specified by user:");
        for (i, tier_id) in ref_col_missing.iter().enumerate() {
            println!("{}. {}", i+1, tier_id);
        }
    }

    let eaf_path = csv_path.with_extension("eaf");
    let eaf_string = match eaf.to_string(Some(4)) {
        Ok(s) => s,
        Err(err) => {
            let msg = format!("(!) Failed to generate EAF:: {err}");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }
    };
    // Not using the Eaf::write() method, as it does not return a Result<bool, EafError>
    match writefile(eaf_string.as_bytes(), &eaf_path) {
        Ok(true) => println!("Wrote {}", eaf_path.display()),
        Ok(false) => println!("User aborted writing ELAN-file"),
        Err(err) => {
            let msg = format!("(!) Failed to write '{}': {err}", eaf_path.display());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        },
    }
    // match eaf.write(&eaf_path, Some(4)) {
    //     Ok(true) => println!("Wrote {}", eaf_path.display()),
    //     Ok(false) => println!("User aborted writing ELAN-file"),
    //     Err(err) => {
    //         let msg = format!("(!) Failed to write '{}': {err}", eaf_path.display());
    //         return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
    //     },
    // }
    
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