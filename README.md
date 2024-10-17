# eafutil

An experimental tool for inspecting, and processing ELAN-files, including extracting media samples corresponding to annotation spans. This tool is developed mostly as a personal aid and is provided as-is. Some commands may currently be broken and/or suddenly change.

Sample extraction, and media processing requires FFmpeg (specify custom path via `--ffmpeg` if not in `$PATH`).

Compile and install (requires installing [Rust](https://www.rust-lang.org)):
```sh
git clone https://github.com/jenslar/eafutil
cd eafutil
cargo install --path .
```

Usage:
```sh
# list tiers with some stats:
eafutil inspect --eaf MYEAF.eaf

# search for a regex pattern in all EAF-files under ~/Desktop:
eafutil search --dir ~/Desktop/ --regex "mo\wn"

# print a word/token distribution with common affixes removed:
eafutil tokens --eaf MYEAF.eaf --tier --distribution --case --strip

# convert word timestamed whisper JSON to eaf with linked media:
eafutil whisper2eaf --json MYWHISPER.json --media VIDEO.MP4 AUDIO.WAV
```