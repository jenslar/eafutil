# eafutil

An experimental tool for inspecting, and processing ELAN-files, including extracting media samples corresponding to annotation spans. This tool is developed as a personal help and is provided as is with no support. Some commands may currently be broken and/or suddenly change.

Sample extraction, and media processing requires FFmpeg (specify custom path via `--ffmpeg` if not in `$PATH`).

Compile and install (requires installing [Rust](https://www.rust-lang.org)):
```sh
git clone https://github.com/jenslar/eafutil
cd eafutil
cargo install --path .
```

Usage:
```
eafutil inspect --eaf MYEAF.eaf                  # list tiers with some stats
eafutil search --dir ~/Desktop/ --regex "mo\wn"  # search for a regex pattern in all EAF-files under ~/Desktop
```