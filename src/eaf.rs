use std::io::Write;

use eaf_rs::eaf::{
    Eaf,
    Tier,
    Annotation
};

use crate::text::process_string;

pub fn select_tier(eaf: &Eaf, no_tokenized: bool) -> std::io::Result<Tier> {
    println!("Select tier:");
    println!("      ID{}Parent              Tokenized  Annotations  Tokens unique/total  Participant     Annotator       Start of first annotation", " ".repeat(19));
    for (i, tier) in eaf.tiers.iter().enumerate() {
        println!("  {:2}. {:21}{:21}{:5}      {:>9}     {:>6} / {:<6}    {:15} {:15} {}",
            i+1,
            process_string(&tier.tier_id, None, None, None, Some(20)),
            process_string(tier.parent_ref.as_deref().unwrap_or("None"), None, None, None, Some(20)),
            tier.is_tokenized(),
            tier.len(),
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
    
    loop {
        print!("> ");
        std::io::stdout().flush()?;
        let mut buffer = String::new();
        let stdin = std::io::stdin();
        stdin.read_line(&mut buffer)?;
        match buffer.trim_end().parse::<usize>() {
            Ok(i) => {
                match eaf.tiers.get(i-1) {
                    // check if selected tier or any parent tier is tokenized
                    Some(t) => if eaf.is_tokenized(&t.tier_id, true) && no_tokenized {
                        println!("(!) '{}' or one of its parents is tokenized.", t.tier_id);
                    } else {
                        return Ok(t.to_owned())
                    },
                    None => println!("(!) No such tier.")
                }
            },
            Err(_) => println!("(!) Not a number.")
        }
    }
}

pub fn select_annotation(tier: &Tier) -> std::io::Result<Annotation> {
    println!("Select annotation in '{}' ({} annotations):", tier.tier_id, tier.len());

    let mut no_ts: Vec<usize> = Vec::new();
    for (i, annotation) in tier.annotations.iter().enumerate() {
        if let (Some(ts1), Some(ts2)) = annotation.ts_val() {
            println!("{:3}. [{:>8}ms | {ts1:>8}-{ts2:<8}ms] {}",
                i+1,
                ts2-ts1,
                annotation.to_str())
        } else {
            println!("{:3}. [NO TIMESTAMPS] {}",
                i+1,
                annotation.to_str());
            // Add to list of time slots with no values for checking below
            no_ts.push(i)
        }
    }
    
    loop {
        print!("> ");
        std::io::stdout().flush()?;
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer)?;
        match buffer.trim_end().parse::<usize>() {
            Ok(i) => {
                // Ensure ts has time slot values when necessary
                if no_ts.contains(&i) {
                    println!("(!) Annotation has no timestamp.");
                } else {
                    match tier.annotations.get(i-1) {
                        Some(t) => return Ok(t.to_owned()),
                        None => println!("(!) No such tier.")
                    }
                }
            },
            Err(_) => println!("(!) Not a number.")
        }
    }
}