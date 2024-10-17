//! Print an overview of an ELAN-file. The `verbose` flag also prints
//! properties (in the header), controlled vocabularies etc.
//! It is also possible to list all annotations in the selected tier.

use std::path::PathBuf;

use eaf_rs::eaf::{Eaf, controlled_vocabulary::CVType};

use crate::{
    eaf::select_tier,
    text::process_string,
    files::confirm
};

// Inspect EAF, main
pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let eaf_path = args.get_one::<PathBuf>("eaf"); // clap ensures value
    let list_annotations = *args.get_one::<bool>("annotations").unwrap();
    let verbose = *args.get_one::<bool>("verbose").unwrap();
    let debug = *args.get_one::<bool>("debug").unwrap();

    let eaf_path = match eaf_path {
        Some(p) => p,
        None => {
            let msg = "No EAF file specified.";
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
            
        }
    };

    let eaf = match Eaf::read(eaf_path) {
        Ok(f) => f,
        Err(err) => {
            let msg = format!("Failed to parse '{}': {err}", eaf_path.display());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        }
    };

    println!("[{}]", eaf_path.display());

    if debug {
        println!("{eaf:#?}");
        return Ok(())
    }

    if list_annotations {
        let tier = match select_tier(&eaf, false) {
            Ok(t) => t,
            Err(err) => {
                let msg = format!("Failed to extract tier: {err}");
                return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
            }
        };

        // let user choose wheter to list large tiers
        if tier.len() > 40 {
            if !confirm(&format!("The tier '{}' has {} annotations. List all?", tier.tier_id, tier.len()))? {
                let msg = "Aborted process.";
                return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
            }
        }

        for (i, annotation) in tier.annotations.iter().enumerate() {
            if let (Some(ts1), Some(ts2)) = annotation.ts_val() {
                println!("{:5}. {:8} ms - {:8} ms '{}'", i+1, ts1, ts2, annotation.to_str())
            } else {
                println!("{:5}.{}'{}'", i+1, " ".repeat(27), annotation.to_str())
            }
        }

        return Ok(())
    }

    if verbose {
        println!("[ General ]");
        println!("   Author:      {}", eaf.author);
        println!("   Date:        {}", eaf.date);
        println!("   EAF version: {}", eaf.version);
        
        println!("[ Media ]");
        
        for (i, media) in eaf.header.media_descriptor.iter().enumerate() {
            println!("  {:2}. {}\n      {}\n      {}\n      {}",
                i+1,
                media.media_url(),
                media.relative_media_url().unwrap_or("None"),
                media.extracted_from.as_deref().unwrap_or("None"),
                media.mime_type,
            )
        }

        println!("[ Linked files ]");

        for (i, file) in eaf.header.linked_file_descriptor.iter().enumerate() {
            println!("  {:2}. {}\n      {}\n      {}\n      {}",
                i+1,
                file.link_url,
                file.relative_link_url.as_deref().unwrap_or("None"),
                file.associated_with.as_deref().unwrap_or("None"),
                file.mime_type,
            )
        }

        println!("[ Properties ]");
    
        for (i, (name, value)) in eaf.properties().iter().enumerate() {
            println!("  {:2}. {name:20}: {value}", i+1);
        }
    }

    println!("[ Tiers ]");
    println!("      Tier ID{}Parent tier         Tokenized  Annotations  Tokens unique/total  Participant     Annotator       Start of first annotation", " ".repeat(14));
    for (i, tier) in eaf.tiers.iter().enumerate() {
        let len = tier.len();
        println!("  {:2}. {:21}{:21}{:5}      {:>9}     {:>6} / {:<6}    {:15} {:15} {}",
            i+1,
            process_string(&tier.tier_id, None, None, None, Some(20)),
            process_string(tier.parent_ref.as_deref().unwrap_or("None"), None, None, None, Some(20)),
            tier.is_tokenized(),
            len,
            tier.tokens(None, None, true, true).len(),
            tier.tokens(None, None, false, false).len(),
            process_string(tier.participant.as_deref().unwrap_or("None"), None, None, None, Some(15)),
            process_string(tier.annotator.as_deref().unwrap_or("None"), None, None, None, Some(15)),
            tier.annotations
                .first()
                .map(|a| {
                    format!("'{} ...'", process_string(a.to_str(), None, None, None, Some(30)))
                })
                .unwrap_or("[empty]".to_owned())
        );
    }

    if verbose {
        println!("[ Linguistic Types ]");
        for (i, ltype) in eaf.linguistic_types.iter().enumerate() {
            println!("  {:2}. '{}'\n      Constraints:           {}\n      Controlled vocabulary: {}\n      Graphic references:    {}\n      Time alignable:        {}",
                i+1,
                ltype.linguistic_type_id,
                ltype.constraints.as_deref().unwrap_or("NONE"),
                ltype.controlled_vocabulary.as_deref().unwrap_or("NONE"),
                ltype.graphic_references.as_ref().unwrap_or(&false),
                ltype.time_alignable.as_ref().unwrap_or(&false),
            )
        }

        println!("[ Locales ]");
        for (i, locale) in eaf.locales.iter().enumerate() {
            println!("  {:2}. Country:  {}\n      Language: {}",
                i+1,
                locale.country_code.as_deref().unwrap_or("NONE"),
                locale.language_code,  
            )
        }

        println!("[ Languages ]");
        for (i, language) in eaf.languages.iter().enumerate() {
            println!("  {:2}. Definition: {}\n      ID:         {}\n      Label:      {}",
                i+1,
                language.lang_def.as_deref().unwrap_or("NONE"),
                language.lang_id,
                language.lang_label.as_deref().unwrap_or("NONE"),
            )
        }

        println!("[ Constraints ]");
        for (i, constraint) in eaf.constraints.iter().enumerate() {
            println!("  {:2}. Description: {}\n      Stereotype:  {}",
                i+1,
                constraint.description,
                constraint.stereotype.to_string(),
            )
        }

        println!("[ Controlled vocabulary ]");
        for (i1, cv) in eaf.controlled_vocabularies.iter().enumerate() {
            println!("  {:2}. '{}'",
                i1+1,
                cv.cv_id,
            );
            if let Some(descr) = &cv.description {
                println!("      {descr}")
            }
            if let Some(ext_ref) = &cv.ext_ref {
                println!("      {ext_ref}")
            }
            for entry in cv.entry.iter() {
                match entry {
                    CVType::Description(d) => println!("      Description: {} ({})", d.value.as_deref().unwrap_or("None"), d.lang_ref.as_deref().unwrap_or("None")),
                    CVType::CvEntry(cve) => {
                        println!("        Value: {}", cve.value);
                        println!("          Description:  {}", cve.description.as_deref().unwrap_or("None"));
                        println!("          Language ref: {}", cve.ext_ref.as_deref().unwrap_or("None"));
                    },
                    CVType::CvEntryMl(cveml) => {
                        println!("      Entry:\n  CVE Id:         {}\n  External ref.: {}", cveml.cve_id, cveml.ext_ref.as_deref().unwrap_or("None"));
                        println!("      Values:");
                        for cve_value in cveml.cve_values.iter() {
                            println!("        Value: {}", cve_value.value);
                            println!("          Description:  {}", cve_value.description.as_deref().unwrap_or("None"));
                            println!("          Language ref: {}", cve_value.lang_ref);
                        }
                    },
                }
            }
        }
    }

    println!("---");
    println!("  Tiers             | total:   {}", eaf.t_len());
    println!("  Annotations       | total:   {}", eaf.a_len());
    println!("  Annotations/tier  | average: {:.2}", eaf.t_avr_len());
    let first_a = eaf.first_annotation();
    if let Some(a1) = first_a {
        print!("  First annotation  | ");
        let val = a1.value().to_string();
        let (ts1, ts2) = a1.ts_val();
        let tier = a1.tier_id().unwrap_or("<Tier not set>".to_owned());
        println!("value:   {val}");
        println!("                    | time:    {} - {} ms",
            ts1.map(|t| t.to_string()).unwrap_or("<Timeslot not set>".to_owned()),
            ts2.map(|t| t.to_string()).unwrap_or("<Timeslot not set>".to_owned())
        );
        println!("                    | tier:    {tier}");
    }
    let last_a = eaf.last_annotation();
    if let Some(a2) = last_a {
        print!("  Last annotation   | ");
        let val = a2.value().to_string();
        let (ts1, ts2) = a2.ts_val();
        let tier = a2.tier_id().unwrap_or("<Tier not set>".to_owned());
        println!("value:   {val}");
        println!("                    | time:    {} - {} ms",
            ts1.map(|t| t.to_string()).unwrap_or("<Timeslot not set>".to_owned()),
            ts2.map(|t| t.to_string()).unwrap_or("<Timeslot not set>".to_owned())
        );
        println!("                    | tier:    {tier}");
    }
    println!("  Words/tokens      | total:   {}", eaf.tkn_len());
    println!("  Word/token length | average: {:.2}", eaf.tkn_avr_len());

    Ok(())
}