use std::path::PathBuf;

use eaf_rs::Pfsx;

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {

    let pfsx_path = args.get_one::<PathBuf>("pfsx").unwrap(); // already checked

    println!("Parsing pfsx-file...");

    let pfsx = match Pfsx::read(pfsx_path) {
        Ok(p) => p,
        Err(err) => {
            let msg = format!("Failed to parse '{}': {err}", pfsx_path.display());
            return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
        },
    };
    println!("{pfsx:#?}");

    // let pfsx_out = pfsx_path.with_file_name("TESTWRITE.pfsx");

    // println!("{:?}", pfsx.to_string(Some(4)));

    // pfsx.write(&pfsx_out, Some(4))?;

    // println!("Wrote {}", pfsx_out.display());

    Ok(())
}