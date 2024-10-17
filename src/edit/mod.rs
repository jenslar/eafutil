use std::path::PathBuf;

use eaf_rs::Eaf;

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let path = args.get_one::<PathBuf>("eaf"); // required unless 'dir'
    let dir = args.get_one::<PathBuf>("dir"); // required unless ''af'
    let tier_id = args.get_one::<String>("tier-id");
    let strip_referred = *args.get_one::<bool>("strip-referred").unwrap();
    let annotator = args.get_one::<String>("set-annotator");
    let prefix = args.get_one::<String>("prefix");
    let suffix = args.get_one::<String>("suffix");
    let save_etf = *args.get_one::<bool>("etf").unwrap();

    let mut eafs = match (path, dir) {
        (None, Some(d)) => {
            d.read_dir()?
                .filter_map(|e| Eaf::read(&e.ok()?.path()).ok())
                .collect::<Vec<_>>()
        },
        (Some(p), None) => vec![Eaf::read(p)?],
        (..) => {
            let msg = format!("Must choose one of 'eaf', 'dir'");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        },
    };

    for eaf in eafs.iter_mut() {

    }

    Ok(())
}