use std::path::PathBuf;

pub mod eaf;
pub mod pfsx;
pub mod tsconf;
pub mod textgrid;

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    if args.get_one::<PathBuf>("eaf").is_some() {
        return eaf::run(args)
    };

    if args.get_one::<PathBuf>("pfsx").is_some() {
        return pfsx::run(args)
    };

    if args.get_one::<PathBuf>("tsconf").is_some() {
        return tsconf::run(args)
    };

    if args.get_one::<PathBuf>("textgrid").is_some() {
        return textgrid::run(args)
    };

    Ok(())
}