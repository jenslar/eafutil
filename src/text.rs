use std::collections::HashMap;

use regex::Regex;

/// Replaces non-ASCII and whitespace with specified `char` for each,
/// truncates to specified `len`, and removes anything captured by `regex`. Trims leading and trailing whitespace,
/// after truncation, meaning the string may be shorter than the specified `len`.
/// If no optional value is specified, a copy of `value` is returned untouched.
/// - `ascii`: character to substitute non-ASCII character
/// - `whitespace`: character to substitute whitespace character
/// - `regex`: regular expression pattern to remove
/// - `len`: maxumim character length of output string
pub fn process_string(value: &str, ascii: Option<&char>, whitespace: Option<&char>, regex: Option<&Regex>, len: Option<usize>) -> String {

    // Truncate
    let mut string = match len {
        Some(l) => {
            value.chars()
                .enumerate()
                .filter(|(i, _)| &l > i)
                .map(|(_, c)| c)
                .collect()
        },
        None => value.to_owned()
    };

    // Remove regex captures
    if let Some(rx) = regex {
        string = rx.replace_all(&string, "").to_string()
    }

    // Replace ascii, whitespace. Prio on ascii.
    // Whitespace as specified in https://www.unicode.org/reports/tr44/
    match (ascii, whitespace) {
        (Some(a), Some(w)) => {
            string
                .trim()
                .replace(|c: char| c.is_whitespace(), &w.to_string())
                .replace(|c: char| !c.is_ascii(), &a.to_string())
        },
        // prioritise ascii before whitespace
        (Some(a), None) => {
            string
                .trim()
                .replace(|c: char| !c.is_ascii(), &a.to_string())
        },
        (None, Some(w)) => {
            string
                .trim()
                .replace(|c: char| c.is_whitespace(), &w.to_string())
        },
        _ => string.to_owned()
    }
}


/// Attempts to return an ASCII alternative for IPA characters.
/// Note that IPA isn't fully covered. This is only intended for
/// creating ASCII file names, and is not sufficient
/// for unambiguously representing IPA as ASCII.
/// 
/// `substitute` can be optionally set for characters
/// 
/// See <http://www.unicode.org/reports/tr15/> for actual normalisation standards.
pub fn ipa2ascii(token: &char, substitute: Option<&char>) -> char {
    let i2a: HashMap<char, char> = HashMap::from([
        ('a', 'b')
    ]);
    let sub = substitute.unwrap_or(&'_');
    i2a.get(token).unwrap_or(&sub).to_owned()
}