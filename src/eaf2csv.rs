use std::path::PathBuf;

use csv;
use eaf_rs::{Annotation, Eaf};
use time::{Time, ext::NumericalDuration};
// use inquire::{self, validator::Validation, list_option::ListOption, formatter::MultiOptionFormatter, MultiSelect};

use crate::files::{select_tiers, writefile};

// const TIER_ATTRIBUTES: [&'static str; 5] = [
//     "Tier ID",
//     "Tier linguistic type",
//     "Tier annotator",
//     "Tier participant",
//     "Tier parent ID",
// ];
// const ANNOTATION_ATTRIBUTES: [&'static str; 7] = [
//     "Annotation ID",
//     "Annotation value",
//     "Annotation timespan as HH:MM:SS.fff",
//     "Annotation timespan as milliseconds",
//     "Annotation duration",
//     "Annotation parent ID",
//     "Annotation time slot reference",
// ];

// fn create_menu<'a>(options: &[&'a str], min_choices: Option<usize>, max_choices: Option<usize>) {
//     let min = min_choices.unwrap_or(0);
//     let max = max_choices.unwrap_or(0);

//     let validator = |a: &[ListOption<&&str>]| {
//         if a.len() < min && min != 0 {
//             return Ok(Validation::Invalid(format!("At least {min} values must be selected.").into()));
//         }

//         if a.len() > max && max != 0 {
//             return Ok(Validation::Invalid(format!("At most {min} values can be selected.").into()));
//         }

//         Ok(Validation::Valid)

//         // let x = a.iter().any(|o| *o.value == "Pineapple");

//         // match x {
//         //     true => Ok(Validation::Valid),
//         //     false => Ok(Validation::Invalid("Remember to buy pineapples".into())),
//         // }
//     };

//     // fn validator2(a: &[ListOption<&&str>], min: usize, max: usize) -> Validation {
//     //     if a.len() < min && min != 0 {
//     //         return Validation::Invalid(format!("At least {min} values must be selected.").into());
//     //     }

//     //     if a.len() > max && max != 0 {
//     //         return Validation::Invalid(format!("At most {min} values can be selected.").into());
//     //     }

//     //     Validation::Valid
//     // }

//     let formatter: MultiOptionFormatter<'_, &str> = &|a| format!("{} options", a.len());

//     let options = options.iter().cloned().collect_vec();

//     let ans = MultiSelect::new("Select which attributes to include:", options)
//         .with_validator(validator)
//         .with_formatter(formatter)
//         .prompt();

//     match ans {
//         Ok(_) => println!("I'll get right on it"),
//         Err(_) => println!("The shopping list could not be processed"),
//     }

// }

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {

    let eaf_path = args.get_one::<PathBuf>("eaf").unwrap(); // clap required arg
    // let interactive = *args.get_one::<bool>("interactive").unwrap();

    let delimiter_string = args.get_one::<String>("delimiter").cloned().unwrap_or_default(); // clap ensures value
    let delimiter = match delimiter_string.as_str() { // clap ensures value, deafult to tab
        "comma" => b',',
        "semicolon" => b';',
        "tab" => b'\t',
        d => {
            let msg = format!("(!) Invalid delimiter '{d}'.");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
        }
    };
    let timeline = *args.get_one::<bool>("timeline").unwrap();

    let csv_path = eaf_path.with_extension("csv");

    let eaf = Eaf::read(eaf_path)?;

    // let mut values = args.get_many::<String>("values")
    //     .map(|v| v.collect::<Vec<_>>())
    //     .unwrap_or(vec![""]);

    // let time_format = format_description::parse(
    //     "[hour]:[minute]:[second].[subsecond]",
    // )
    // .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    if timeline {
        let tier_ids = select_tiers(&eaf).unwrap();
        let tiers: Vec<_> = tier_ids.iter()
            .filter_map(|id| eaf.get_tier(id))
            .cloned()
            .collect();
        
        if tier_ids.len() != tiers.len() {
            let tier_ids_out: Vec<&str> = tiers.iter()
                .map(|t| t.tier_id.as_str())
                .collect();
            let missing: Vec<_> = tier_ids.iter()
                .filter(|id| !tier_ids_out.contains(&id.as_str()))
                .map(|s| s.as_str())
                .collect();
            let msg = format!("Failed to retreive tiers {}", missing.join(", "));
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg));
        }

        let len = tiers.len() + 1; // one column per tier + timestamp column

        let mut annotations: Vec<(String, Annotation)> = tiers.iter()
            .flat_map(|t| t.annotations.as_slice())
            .map(|a| (a.tier_id().unwrap().to_owned(), a.to_owned()))
            .collect();
        annotations.sort_by_key(|(_, a)| a.ts_val().0.expect("Annotation level timestamps not set"));
        for (id, a) in annotations.iter() {
            let (ts1, ts2) = a.ts_val();
            println!("{} {:10?} - {:10?} {}", id, ts1, ts2, a.value())
        }

        // for id in tier_ids.iter() {
        //     let tier = eaf.get_tier(id).unwrap();
        //     let a = tier.first().unwrap();
        //     let (start, end) = a.ts_val();
        //     println!("{} {} {} - {} ms", tier.tier_id, a.value(), start.unwrap(), end.unwrap());
        // }

        return Ok(())
    }
    
    let mut builder = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .from_writer(vec![]);

    let headers = vec![
        "TIER_ID",
        "TIER_SIZE",
        "PARENT_TIER",
        "TIER_TYPE",
        "PARTICIPANTS",
        "ANNOTATOR",
        "ANNOTATION_VALUE",
        "ANNOTATION_START_HHMMSS",
        "ANNOTATION_START_MS",
        "ANNOTTION_END_HHMMSS",
        "ANNOTTION_END_MS"
    ];

    builder.write_record(&headers)?;

    for tier in eaf.tiers.iter() {
        if tier.is_empty() {
            let row = vec![
                tier.tier_id.to_owned(),
                tier.len().to_string(),
                tier.parent_ref.as_deref().unwrap_or("<NONE>").to_owned(),
                tier.linguistic_type_ref.to_owned(),
                tier.participant.as_deref().unwrap_or("<NONE>").to_owned(),
                tier.annotator.as_deref().unwrap_or("<NONE>").to_owned(),
                "<EMPTY TIER>".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            ];
            builder.write_record(row)?;
        }
        for annotation in tier.annotations.iter() {
            let (ts1, ts2) = annotation.ts_val();
            // let midnight = Time::MIDNIGHT;
            // let t_hms1 = (midnight + ts1.unwrap_or_default().milliseconds()).as_hms_milli();
            // let t_hms2 = (midnight + ts2.unwrap_or_default().milliseconds()).as_hms_milli();
            let t_hms1 = ms2string(ts1.unwrap_or_default());
            let t_hms2 = ms2string(ts2.unwrap_or_default());
            let row = vec![
                tier.tier_id.to_owned(),
                tier.len().to_string(),
                tier.parent_ref.as_deref().unwrap_or("<NONE>").to_owned(),
                tier.linguistic_type_ref.to_owned(),
                tier.participant.as_deref().unwrap_or("<NONE>").to_owned(),
                tier.annotator.as_deref().unwrap_or("<NONE>").to_owned(),
                annotation.value().to_string(),
                // format!("{:02}:{:02}:{:02}.{:03}", t_hms1.0, t_hms1.1, t_hms1.2, t_hms1.3),
                t_hms1,
                ts1.unwrap_or_default().to_string(),
                // format!("{:02}:{:02}:{:02}.{:03}", t_hms2.0, t_hms2.1, t_hms2.2, t_hms2.3),
                t_hms2,
                ts2.unwrap_or_default().to_string(),
            ];
            builder.write_record(row)?;
        }
    }

    let builder_inner = builder.into_inner().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let tsv = String::from_utf8(builder_inner).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    match writefile(tsv.as_bytes(), &csv_path) {
        Ok(true) => println!("Wrote {}", csv_path.display()),
        Ok(false) => println!("User aborted writing file"),
        Err(e) => return Err(e)
    }

    // println!("Exported {} tiers:");

    Ok(())
}

fn ms2string(ms: i64) -> String {
    hms_milli_to_string(hms_milli(ms))
}

fn hms_milli(ms: i64) -> (u8, u8, u8, u16) {
    let midnight = Time::MIDNIGHT;
    (midnight + ms.milliseconds()).as_hms_milli()
}

fn hms_milli_to_string(hms_ms: (u8, u8, u8, u16)) -> String {
    format!("{:02}:{:02}:{:02}.{:03}", hms_ms.0, hms_ms.1, hms_ms.2, hms_ms.3)
}