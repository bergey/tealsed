mod equivalent;

pub mod parser;

use ::regex::{Regex, Replacer};
use std::io::{Error, ErrorKind};

pub fn parse(s: &str) -> Result<Regex, Error> {
    let ast = parser::parse_complete(s).map_err(|e| Error::new(ErrorKind::InvalidInput, e.to_string()))?;
    Regex::new(&format!("{}", ast)) // TODO panic on err
        .map_err(|e| Error::new(ErrorKind::InvalidInput, e))
}

// return true if any replacement was made
// this is regex::Regex::replacen, except using DoubleString instead of Cow to make applying multiple replacements to a single string efficient
pub fn replacen<R: Replacer>(regex: &Regex, source: &str, destination: &mut String, limit: usize, mut rep: R) -> bool {
    // If we know that the replacement doesn't have any capture expansions,
    // then we can use the fast path.
    if let Some(rep) = rep.no_expansion() {
        let mut it = regex.find_iter(source).enumerate().peekable();
        if it.peek().is_none() {
            return false; // no change to buffers
        }
        let mut last_match = 0;
        for (i, m) in it {
            if limit > 0 && i >= limit {
                break
            }
            destination.push_str(&source[last_match..m.start()]);
            destination.push_str(&rep);
            last_match = m.end();
        }
        destination.push_str(&source[last_match..]);
        return true;
    }
    // The slower path, if the replacement needs access to capture groups.
    let mut it = regex.captures_iter(source).enumerate().peekable();
    if it.peek().is_none() {
        return false;
    }
    let mut last_match = 0;
    for (i, cap) in it {
        if limit > 0 && i >= limit {
            break;
        }
        // unwrap on 0 is OK because captures only reports matches
        let m = cap.get(0).unwrap();
        destination.push_str(&source[last_match..m.start()]);
        rep.replace_append(&cap, destination);
        last_match = m.end();
    }
    destination.push_str(&source[last_match..]);
    return true;
}

pub fn replace<R: Replacer>(regex: &Regex, source: &str, destination: &mut String, rep: R) -> bool {
    replacen(&regex, source, destination, 1, rep)
}

pub fn replace_all<R: Replacer>(regex: &Regex, source: &str, destination: &mut String, rep: R) -> bool {
    replacen(&regex, source, destination, 0, rep)
}
