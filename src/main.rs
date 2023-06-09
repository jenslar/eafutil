use std::path::PathBuf;

use clap::{Arg, Command, ArgAction};

mod csv2eaf;
mod shift;
mod filter;
mod merge;
mod search;
mod tokens;
mod ngram;
mod compare;
mod media;
mod clips;
// mod inspect_old;
mod inspect;
mod tree;
mod eaf;
mod files;
mod text;
mod json;
mod color;


fn main() -> std::io::Result<()> {
    let args = Command::new("eafutil")
        .version("0.5")
        .author("Jens Larsson")
        .arg_required_else_help(true)
        .term_width(90)
        .about("Utility for processing ELAN-files in various ways. Some commands require FFmpeg to be installed.

For help on a specific sub-command, use:
eafutil <SUB-COMMAND> --help
eafutil csv2eaf --help

- csv2eaf: Convert a CSV-file to an ELAN-file.
- shift:   Shift all annotations in an ELAN-file in time.
- filter:  Filter a time span of an ELAN-file into a new ELAN-file.
- tokens:  Extract the word/token distribution from all annotations.
- search:  Search annotations in one or more ELAN-files.
- media:   Add or remove media paths.
- inspect: Get an overview of an ELAN-file. Number of tiers, annotations...
- TODO merge: Merge ELAN-files.")
        // .setting(AppSettings::ArgRequiredElseHelp)

        // CSV2EAF: Generate an EAF from a CSV-file.
        .subcommand(Command::new("csv2eaf")
            .about("Convert a CSV-file containing annotation values and time stamps (annotation boundaries) to an ELAN-file. Requires headers and columns representing start, end, and annotation value, respectively. Annotations and time stamps should be listed in chronological order.

Accepted time formats in CSV-file:
- HH:MM:SS        hours:minutes:seconds                 00:02:54
- HH:MM:SS.fff    hours:minutes:seconds.sub-seconds     00:02:54.456
- milliseconds    unsigned integer                      174456

ELAN defults to milliseconds internally.")
            .visible_alias("c2e")
            
            .arg(Arg::new("csv")
                .help("Path to CSV-file.")
                .long("csv")
                .short('c')
                .required(true)
                .value_parser(clap::value_parser!(PathBuf))
            )
            .arg(Arg::new("delimiter")
                .help("Delimiter used in CSV-file.")
                .long("delimiter")
                .short('d')
                // .possible_values(&["tab", "comma", "semicolon"])
                .value_parser(["tab", "comma", "semicolon"])
                .default_value("comma")
            )
            .arg(Arg::new("start")
                .help("Name of column containing annotation start time stamps as 'HH:MM:SS', 'HH:MM:SS.fff', or a millisecond value.")
                .long("start")
                .short('s')
                .required_unless_present("debug")
                .default_value("start") // deafult csv header for annotation start time
            )
            .arg(Arg::new("end")
                .help("Name of column containing annotation end time stamps as 'HH:MM:SS', 'HH:MM:SS.fff', or a millisecond value.")
                .long("end")
                .short('e')
                .required_unless_present("debug")
                .default_value("end") // deafult csv header for annotation end time
            )
            .arg(Arg::new("offset")
                .help("Offset time in +/- milliseconds for start/end time stamps for imported main tier. Resulting start time stamps < 0ms will be set to 0ms, but if the end time stamp < 0ms the annotation will be discarded.")
                .long("offset")
                .short('o')
                .default_value("0")
            )
            .arg(Arg::new("values")
                .help("Name of column containing annotation values. Empty cells will generate empty annotations, with the specified time stamps in 'start' and 'end' columns.")
                .long("values")
                .short('v')
                // .takes_value(true)
                .required_unless_present("debug")
                .default_value("values") // deafult csv header for annotation value
            )
            .arg(Arg::new("ref-values")
                .help("Name of columns containing referred tier annotation values. These must not exceed the number of rows in the 'values', 'start', 'end' columns. Multiple values can be specified, e.g.: '--refs col1 col2 col3', resulting in multiple referred tiers.")
                .long("refs")
                .short('r')
                // .action(clap::ArgAction::)
                .num_args(0..) // TODO test that this actually works
                // .multiple_values(true)
                // .takes_value(true)
            )
            // .arg(Arg::new("no-headers")
            //     .help("Use if the CSV-file has no headers on the first row.")
            //     .long("no-headers")
            //     .short('n')
            // )
            .arg(Arg::new("debug")
                .help("Prints the all content in the CSV-file in debug form to track down e.g. columns with missing headers.")
                .long("debug")
                .action(ArgAction::SetTrue)
            )
            .arg(Arg::new("video")
                .help("Video file to link in ELAN-file. Optional.")
                .long("video")
                .value_parser(clap::value_parser!(PathBuf))
                // .takes_value(true)
            )
            .arg(Arg::new("eaf")
                .help("If specified, an existing ELAN-file will be modified with new tiers from the CSV-file.")
                .long("eaf")
                .value_parser(clap::value_parser!(PathBuf))
                // .takes_value(true)
            ) 
        )

        // FILTER: Filter time span in EAF.
        .subcommand(Command::new("filter")
            .about("Filters a section of the ELAN-file via start/end time values in milliseconds. Optionally use the time span of an annotation in a tier. If no start and end time is ")
            .visible_alias("f")
            .arg(Arg::new("eaf")
                .help("ELAN-file to process.")
                .long("eaf")
                .short('e')
                .value_parser(clap::value_parser!(PathBuf))
                .required(true)
            )
            .arg(Arg::new("start")
                .help("Start time of time span to extract in milliseconds. Must be a positive integer.")
                .long("start")
                .value_parser(clap::value_parser!(i64))
            )
            .arg(Arg::new("end")
                .help("End time of time span to extract in milliseconds. Must be a positive integer.")
                .long("end")
                .value_parser(clap::value_parser!(i64))
            )
            .arg(Arg::new("process-media")
                .help("Extract and link corresponding media clips. Requires FFmpeg.")
                .long("media")
                .short('m')
                .action(ArgAction::SetTrue)
            )
            .arg(Arg::new("ffmpeg")
                .help("Custom ffmpeg path.")
                .long("ffmpeg")
                .short('f')
                .default_value(if cfg!(windows) {"ffmpeg.exe"} else {"ffmpeg"})
            )
        )   

        // SHIFT: Shift all annotations according to specified millisecond value.
        .subcommand(Command::new("shift")
            .about("Shifts all annotations forward or backward (use a negative millisecond value) in an ELAN-file according to the specified millisecond value.")
            .visible_alias("sh")
            .arg(Arg::new("eaf")
                .help("ELAN-file to process.")
                .long("eaf")
                .short('e')
                .value_parser(clap::value_parser!(PathBuf))
                .required(true)
            )
            .arg(Arg::new("shift-value")
                .help("Positive or negative millisecond value. Must be an integer.")
                .long("shift")
                .short('s')
                .value_parser(clap::value_parser!(i64))
                .allow_hyphen_values(true)
                .required(true)
            )
        )

        // MERGE: Merge two ELAN-files.
        .subcommand(Command::new("merge")
            .hide(true) // not yet ready
            .about("Merges two ELAN-files, presuming both link the same media source. This is done with a few caveats. Time slots with no time value set will be discarded. If merging causes annotations to overlap on the same tier, the process will be aborted by default, but can be overriden via the 'join' flag. This will instead join overlapping annotations.

Example:
  eafutil merge --eaf ELANFILE_1.eaf --eaf ELANFILE_2.eaf")
            .visible_alias("mg")
            .arg(Arg::new("eaf")
                .help("ELAN-file to merge. This option can be used multiple times.")
                .long("eaf")
                .short('e')
                // .takes_value(true)
                .action(clap::ArgAction::Append) // TODO ensure to use get_many
                // .multiple_occurrences(true)
            )
            .arg(Arg::new("join-overlaps")
                .help("Joins overlapping annotations instead of aborting.")
                .long("join")
                .short('j')
                .action(clap::ArgAction::Append) // TODO ensure to use get_many
                // .multiple_occurrences(true)
            )
        )

        // FIND: Find annotation value. Regex possible
        .subcommand(Command::new("search")
            .about("Search for a pattern in annotation values (regular expressions possible). Specify either in a single file, or a directory for multi-file search.")
            .visible_alias("s")
            .arg(Arg::new("eaf")
                .help("Single ELAN-file to search.")
                .long("eaf")
                .short('e')
                .value_parser(clap::value_parser!(PathBuf))
                .required_unless_present("dir")
            )
            .arg(Arg::new("dir")
                .help("Directory of ELAN-files to search.")
                .long("dir")
                .short('d')
                .value_parser(clap::value_parser!(PathBuf))
                .required_unless_present("eaf")
            )
            .arg(Arg::new("pattern")
                .help("Search pattern. Simple string match. Any string containing the pattern is regarded a match.")
                .long("pattern")
                .short('p')
                .required_unless_present("regex")
            )
            .arg(Arg::new("regex")
                .help(r#"Regular expression. Note that special characters such as '$' must be escaped, e.g. '\$'. Obeys the 'ignore-case' flag, but only for the whole pattern."#)
                .long("regex")
                .short('r')
                .required_unless_present("pattern")
            )
            .arg(Arg::new("ignore-case")
                .help("Ignore case for the entire pattern.")
                .long("ignore-case")
                .short('i')
                .action(ArgAction::SetTrue)
            )
            .arg(Arg::new("context")
                .help("Show annotation/s in parent and referred tiers. Only valid for single-file search.")
                .long("context")
                .short('c')
                .requires("eaf")
                .action(ArgAction::SetTrue)
            )
            .arg(Arg::new("full-path")
                .help("Show full path to files with matches.")
                .long("full-path")
                .short('f')
                .action(ArgAction::SetTrue)
            )
            .arg(Arg::new("verbose")
                .help("Prints all found ELAN-files, including those with no matches or could not be parsed.")
                .long("verbose")
                .short('v')
                .action(ArgAction::SetTrue)
            )
        )

        // TOKENS: Extract all tokens/words
        .subcommand(Command::new("tokens")
            .about("Extract all words/tokens in annnotation values. Only works on whitespace delimited scripts.")
            .visible_alias("t")
            .arg(Arg::new("eaf")
                .help("ELAN-file to process.")
                .long("eaf")
                .short('e')
                .value_parser(clap::value_parser!(PathBuf))
                // .takes_value(true)
                // .required_unless_present("dir")
            )
            .arg(Arg::new("prefix")
                .help("Single-character prefix pattern to strip, so that for e.g. '<:', '<:hi', '<hi' and 'hi' are considered equal. Single-character o")
                .long("prefix")
                .short('p')
                // .takes_value(true)
            )
            .arg(Arg::new("suffix")
                .help("Single-character suffix pattern to strip, so that for e.g. '>:', 'hi:>', 'hi:' and 'hi' are considered equal.")
                .long("suffix")
                .short('s')
                // .takes_value(true)
            )
            .arg(Arg::new("strip-common")
                .help("Strip common characters, such as '(', ')', '-'")
                .long("strip")
                .action(clap::ArgAction::SetTrue)
            )
            .arg(Arg::new("unique")
                .help("List unique words only.")
                .long("unique")
                .short('u')
                .action(clap::ArgAction::SetTrue)
            )
            .arg(Arg::new("ignore-case")
                .help("Ignore case.")
                .long("case")
                .short('c')
                .action(clap::ArgAction::SetTrue)
            )
            .arg(Arg::new("distribution")
                .help("Distribution of unique words.")
                .long("distribution")
                .short('d')
                .action(clap::ArgAction::SetTrue)
            )
            .arg(Arg::new("sort-alphabetically")
                .help("Sorts distribution alphabetially, rather than on commonality.")
                .long("alpha")
                .short('a')
                .action(clap::ArgAction::SetTrue)
                .requires("distribution")
            )
            .arg(Arg::new("sort-reverse")
                .help("Sorts distribution in reversed order.")
                .long("reverse")
                .short('r')
                .action(clap::ArgAction::SetTrue)
                .requires("distribution")
            )
            .arg(Arg::new("select-tier")
                .help("Words for selected tier only.")
                .long("tier")
                .short('t')
                .action(clap::ArgAction::SetTrue)
            )
        )

        // WORDS: Extract all words
        .subcommand(Command::new("ngram")
            .about("Simple n-gram distribution.")
            .visible_alias("n")
            .arg(Arg::new("eaf")
                .help("ELAN-file to process.")
                .long("eaf")
                .short('e')
                .value_parser(clap::value_parser!(PathBuf))
                // .takes_value(true)
                // .required_unless_present("dir")
            )
            .arg(Arg::new("ngram-size")
                .help("Ngram size")
                .long("ngram")
                .short('n')
                .value_parser(clap::value_parser!(usize))
                // .takes_value(true)
                .default_value("2")
            )
            .arg(Arg::new("scope")
                .help("Scope of ngram extraction. 'annotation' does not cross annotation boundaries. 'tier' does not cross tier boundaries. 'file' combines all annotations before generation ngrams.")
                .long("scope")
                .value_parser(["annotation", "tier", "file"])
                .default_value("annotation")
                .short('s')
            )
            .arg(Arg::new("ignore-case")
                .help("Ignore case.")
                .long("case")
                .short('c')
                .action(clap::ArgAction::SetTrue)
            )
            .arg(Arg::new("remove-common")
                .help("Remove characters, such as '(', ')', '.'")
                .long("remove")
                .action(clap::ArgAction::SetTrue)
            )
            .arg(Arg::new("remove-custom")
                .help("Remove custom characters. Specify as string, e.g. '.-='")
                .long("custom")
                .value_parser(clap::value_parser!(String))
            )
        )

        // MEDIA
        .subcommand(Command::new("media")
            .about("Add or remove media to or from the specified ELAN-file. Lists linked media files if a single ELAN-file is specified. Alternatively remove specific media file or all linked media.")
            .visible_alias("md")
            .arg(Arg::new("eaf")
                .help("ELAN-file to process.")
                .long("eaf")
                .short('e')
                .value_parser(clap::value_parser!(PathBuf))
                // .takes_value(true)
                .required_unless_present("dir")
            )
            .arg(Arg::new("dir")
                .help("Path to dir with ELAN-files to process. Recursive. Only valid for scrubbing paths.")
                .long("dir")
                .short('d')
                .value_parser(clap::value_parser!(PathBuf))
                // .takes_value(true)
                .required_unless_present("eaf")
            )
            .arg(Arg::new("media")
                .help("Media file to add or remove.")
                .long("media")
                .short('m')
                .value_parser(clap::value_parser!(PathBuf))
                // .takes_value(true)
                .requires("eaf")
            )
            .arg(Arg::new("remove")
                .help("Removes specified '--media' from ELAN-file/s. Matches file name, not full path.")
                .long("remove")
                .short('r')
                .action(ArgAction::SetTrue)
                .conflicts_with_all(&["add", "scrub", "filename-only"])
                .requires_all(&["eaf", "media"])
            )
            .arg(Arg::new("add")
                .help("Adds specified '--media' to ELAN-file/s. Matches file name, not full path.")
                .long("add")
                .short('a')
                .action(ArgAction::SetTrue)
                .conflicts_with_all(&["remove", "scrub", "filename-only"])
                .requires_all(&["eaf", "media"])
            )
            .arg(Arg::new("scrub")
                .help("Scrubs all linked media files from ELAN-file/s.")
                .long("scrub")
                .short('s')
                .action(ArgAction::SetTrue)
                .conflicts_with_all(&["add", "remove", "media"])
            )
            .arg(Arg::new("filename-only")
                .help("Replaces all full paths to linked media files with file names only. E.g. 'path/to/video.mp4' becomes 'video.mp4'.")
                .long("filename")
                .short('f')
                .action(ArgAction::SetTrue)
                .conflicts_with_all(&["add", "remove", "media", "scrub"])
            )
        )

        // CLIPS
        .subcommand(Command::new("clips")
            .about(r"Generate media clips from annotation boundaries in selected tier. Requires ffmpeg.")
            .visible_alias("c")
            .arg(Arg::new("eaf")
                .help("ELAN-file to process.")
                .long("eaf")
                .short('e')
                .value_parser(clap::value_parser!(PathBuf))
                // .takes_value(true)
                .required(true)
            )
            // defaults to "clips" dir in eaf parent dir
            .arg(Arg::new("outdir")
                .help("Output directory.")
                .long("outdir")
                .short('o')
                .value_parser(clap::value_parser!(PathBuf))
                // .takes_value(true)
            )
            .arg(Arg::new("single-annotation")
                .help("Only extract clips for a single, selected annotation in selected tier.")
                .long("single")
                .short('s')
                .action(clap::ArgAction::SetTrue)
            )
            .arg(Arg::new("annotation-id")
                .help("Include internal annotation ID in output filename, e.g. 'a43'.")
                .long("id")
                .short('i')
                .action(clap::ArgAction::SetTrue)
            )
            .arg(Arg::new("annotation-value")
                .help(r"Include annotation value in output filename. The following characters will always be removed, regardless of 'ascii' setting: #*<>{}()[]-.,:;!/\?=")
                .long("value")
                .short('v')
                .action(clap::ArgAction::SetTrue)
            )
            .arg(Arg::new("annotation-time")
                .help("Include annotation value in output filename.")
                .long("time")
                .short('t')
                .action(clap::ArgAction::SetTrue)
            )
            .arg(Arg::new("max-length")
                .help("Max annotation length if annotation value is used in output filename.")
                .long("length")
                .short('l')
                .requires("annotation-value")
                .value_parser(clap::value_parser!(usize))
                .default_value("20")
            )
            .arg(Arg::new("ascii-path")
                .help("Replace non-ASCII characters with '_' in output filename.")
                .long("ascii")
                .action(clap::ArgAction::SetTrue)
            )
            .arg(Arg::new("ffmpeg")
                .help("Custom FFmpeg path if not in system path.")
                .long("ffmpeg")
                .short('f')
                .default_value(if cfg!(windows) {"ffmpeg.exe"} else {"ffmpeg"})
            )
            .arg(Arg::new("dryrun")
                .help("Show output paths, but do not extract any clips.")
                .long("dryrun")
                .short('d')
                .action(clap::ArgAction::SetTrue)
            )
            // .arg(Arg::new("extract-wav")
            //     .help("Extract WAV for each clip.")
            //     .long("wav")
            //     .short('w')
            // )
        )

        // INSPECT
        .subcommand(Command::new("inspect")
            .about("Print an overview of the ELAN-file, with the option to print annotations in a specific tier.")
            .visible_alias("i")
            .arg(Arg::new("eaf")
                .help("ELAN-file.")
                .long("eaf")
                .short('e')
                .value_parser(clap::value_parser!(PathBuf))
                // .takes_value(true)
                .required_unless_present("pfsx")
            )
            .arg(Arg::new("pfsx")
                .help("ELAN-file.")
                .long("pfsx")
                .short('p')
                .value_parser(clap::value_parser!(PathBuf))
            )
            .arg(Arg::new("annotations")
                .help("List annotations in selected tier.")
                .long("annotations")
                .short('a')
                .action(clap::ArgAction::SetTrue)
            )
            .arg(Arg::new("verbose")
                .help("List linguistic types, controlled vocabulary etc.")
                .long("verbose")
                .short('v')
                .action(clap::ArgAction::SetTrue)
            )
            .arg(Arg::new("debug")
                .help("Print internal representation of EAF..")
                .long("debug")
                .short('d')
                .action(clap::ArgAction::SetTrue)
            )
        )

        // COMPARE
        .subcommand(Command::new("compare")
            .about("Compare annotation values of two tiers.")
            .visible_alias("cmp")
            .arg(Arg::new("eaf")
                .help("ELAN-file.")
                .long("eaf")
                .short('e')
                .value_parser(clap::value_parser!(PathBuf))
                // .takes_value(true)
                .required(true)
            )
            .arg(Arg::new("timeline")
                .help("Select and compare two tiers visually. Timeline based on start time of annotation. Requires all time slots to have values.")
                .long("timeline")
                .short('t')
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("compact")
            )
            .arg(Arg::new("compact")
                .help("Select and compare two tiers lengths visually. Compact. Default.")
                .long("compact")
                .short('c')
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("timeline")
            )
            .arg(Arg::new("max-length")
                .help("Max length of annotation values listed.")
                .long("len")
                .short('l')
                .value_parser(clap::value_parser!(usize))
                .default_value("50")
                // .takes_value(true)
            )
        )

        // JSON
        .subcommand(Command::new("json")
            .about("Generate a JSON-file from the specified ELAN-file. If the 'simple' flag is set, the ELAN-file will be exported as JSON in a simplified form, containing only tiers and their annotation values with start and end time stamps.")
            .visible_alias("j")
            .arg(Arg::new("eaf")
                .help("ELAN-file.")
                .long("eaf")
                .short('e')
                .value_parser(clap::value_parser!(PathBuf))
                // .takes_value(true)
                .required(true)
            )
            .arg(Arg::new("simple")
                .help("Generate a simplified EAF structure as JSON.")
                .long("simple")
                .short('s')
                .action(clap::ArgAction::SetTrue)
            )
        )

        // TIER TREE
        .subcommand(Command::new("tree")
            .about("Show tier tree structure.")
            .arg(Arg::new("eaf")
                .help("ELAN-file.")
                .long("eaf")
                .short('e')
                .value_parser(clap::value_parser!(PathBuf))
                // .takes_value(true)
                .required(true)
            )
        )

        .get_matches();

    // 
    // CSV2EAF, generate eaf from csv
    // 
    if let Some(arg_matches) = args.subcommand_matches("csv2eaf") {
        csv2eaf::run(&arg_matches)?;
    }

    // 
    // SHIFT, shift eaf specified milliseconds
    // 
    if let Some(arg_matches) = args.subcommand_matches("shift") {
        shift::run(&arg_matches)?;
    }

    // 
    // FILTER, filter eaf time span, generate new eaf
    // 
    if let Some(arg_matches) = args.subcommand_matches("filter") {
        filter::run(&arg_matches)?;
    }

    // 
    // MERGE, merge two eaf-files
    // NOT IMPLEMENTED
    // 
    if let Some(arg_matches) = args.subcommand_matches("merge") {
        merge::run(&arg_matches)?;
    }

    // 
    // SEARCH, search for annotation values in one or more eaf-file.
    // 
    if let Some(arg_matches) = args.subcommand_matches("search") {
        search::run(&arg_matches)?;
    }

    // 
    // TOKENS/WORDS, extracts/lists all tokens/words present in the annotations
    // 
    if let Some(arg_matches) = args.subcommand_matches("tokens") {
        tokens::run(&arg_matches)?;
    }

    // 
    // NGRAM
    // 
    if let Some(arg_matches) = args.subcommand_matches("ngram") {
        ngram::run(&arg_matches)?;
    }

    // 
    // MEDIA, add or remove linked media files
    // 
    if let Some(arg_matches) = args.subcommand_matches("media") {
        media::run(&arg_matches)?;
    }

    // 
    // CLIPS, extract media clips from annotation boundaries
    // 
    if let Some(arg_matches) = args.subcommand_matches("clips") {
        clips::run(&arg_matches)?;
    }

    // 
    // INSPECT, general overview of EAF
    // 
    if let Some(arg_matches) = args.subcommand_matches("inspect") {
        // inspect_old::run(&arg_matches)?;
        inspect::eaf::run(&arg_matches)?;
    }

    // 
    // COMPARE, compare two tiers in EAF
    // 
    if let Some(arg_matches) = args.subcommand_matches("compare") {
        compare::run(&arg_matches)?;
    }

    // 
    // JSON, generate JSON from EAF
    // 
    if let Some(arg_matches) = args.subcommand_matches("json") {
        json::run(&arg_matches)?;
    }

    // 
    // TREE, VISUALIZE TIER HIERARCHY
    // 
    if let Some(arg_matches) = args.subcommand_matches("tree") {
        tree::run(&arg_matches)?;
    }

    Ok(())
}
