use crate::regex;
use crate::regex::parser::{Input, Progress, Syntax};
use crate::regex::equivalent::Equivalent;

use ::regex::Regex;
use std::io;
use lazy_static::lazy_static;

use nom;
use nom::{Finish};
use nom::branch::alt;
use nom::character::complete::{anychar, char, none_of};
use nom::combinator::{fail, opt, rest};
use nom::multi::many0;

#[derive(Clone, Debug)]
pub enum Address {
    LineNumber(u64),
    // LastLine, // TODO how do we detect last line?  From stdin, in particular
    Context(Regex), // TODO case-insensitive
}

impl Equivalent for Address {
    fn equivalent(&self, other: &Address) -> bool {
        use Address::*;
        match (self, other) {
            (LineNumber(n), LineNumber(m)) => n == m,
            (Context(_), Context(_)) => true,
            _ => false
        }
    }
}

// single letter for uppercase function names
// F followed by a letter for lowercase function names
#[derive(Clone, Debug)]
pub enum Function {
    Equals,
    D, Fd,
    G, Fg,
    H, Fh,
    Fi(String),
    Fp,
    Fs(Regex, String),
    Fx
}

impl Equivalent for Function {
    fn equivalent(&self, other: &Function) -> bool {
        use Function::*;
        match (self, other) {
            (Equals, Equals) => true,
            (D, D) | (Fd, Fd) => true,
            (G, G) | (Fg, Fg) => true,
            (H, H) | (Fh, Fh) => true,
            (Fi(s), Fi(t)) => s == t,
            (Fp, Fp) => true,
            (Fs(_, s), Fs(_, t)) => s == t,
            (Fx, Fx) => true,
            _ => false
        }
    }
}

#[derive(Clone, Debug)]
pub struct Command {
    pub start: Option<Address>,
    pub end: Option<Address>, // should not be Some if start is None
    pub function: Function,
}

fn take_until(sep: char, s: Input) -> Progress<String> {
    let string: String = sep.to_string();
    let str: &str = string.as_ref();
    let (s, vec) = many0(none_of(str))(s)?;
    Ok((s, vec.into_iter().collect()))
}

lazy_static! {
    static ref DOLLAR: Regex = Regex::new(r"\$").unwrap();
    static ref BACKSLASH_DIGITS: Regex = Regex::new(r"\\([0-9]+)").unwrap();
}

// convert sed \1 syntax to regex crate $1 and escape $
pub fn clean_replacement(syntax: &Syntax, mut s: String) -> String {
    if syntax == &Syntax::Teal {
        return s
    }

    let mut dest = String::new();

    let changed = regex::replace_all(&*DOLLAR, &s, &mut dest, "$$$$");
    if changed {std::mem::swap(&mut s, &mut dest)}

    let changed = regex::replace_all(&*BACKSLASH_DIGITS, &s, &mut dest, r"$${$1}");
    if changed {std::mem::swap(&mut s, &mut dest)}

    s
}

pub fn parse_function(cmd: Input) -> Progress<Function> {
    let (s, function) = anychar(cmd)?;
    use Function::{*};
    match function {
        '=' => Ok((s, Equals)), // spec says only allows one addr, not a 2-addr range 🤷
        'd' => Ok((s, Fd)),
        'D' => Ok((s, D)),
        'g' => Ok((s, Fg)),
        'G' => Ok((s, G)),
        'h' => Ok((s, Fh)),
        'H' => Ok((s, H)),
        'i' => rest(s).map(|(s, i)| (s, Fi(i.to_string()))),
        'p' => Ok((s, Fp)),
        's' => {
            let (s, sep) = anychar(s)?;
            let (s, ast) = regex::parser::parse(sep, s)?;
            let (s, _) = char(sep)(s)?;
            let regex = Regex::new(&format!("{}", ast)).unwrap();
            let (s, replacement) = take_until(sep, s)?;
            let (s, _) = char(sep)(s)?;
            Ok((s, Fs(regex, clean_replacement(&s.extra.syntax, replacement))))
        },
        'x' => Ok((s, Fx)),
        _ => fail(cmd)
    }
}

// handles only a single address
// caller must maintain state between calls, decide whether to pass start or end pattern
pub fn match_address(addr: &Address, text: &str, line_num: u64) -> bool {
    match addr {
        Address::LineNumber(l) => *l == line_num,
        Address::Context(regex) => regex.is_match(text),
    }
}

pub fn parse_address(s: Input) -> Progress<Address> {
    alt((line_number_addr, context_addr))(s)
}

fn line_number_addr(s: Input) -> Progress<Address> {
    let (s, n) = nom::character::complete::u64(s)?;
    Ok((s, Address::LineNumber(n)))
}

fn backslash_char(s: Input) -> Progress<char> {
    let (s, _) = char('\\')(s)?;
    anychar(s)
}

fn context_addr(s: Input) -> Progress<Address> {
    let (s, sep) = alt((char('/'), backslash_char))(s)?;
    let (s, ast) = regex::parser::parse(sep, s)?;
    let regex = Regex::new(&format!("{}", ast)).unwrap();
    let (s, _) = char(sep)(s)?;
    Ok((s, Address::Context(regex)))
}

pub fn parse_command(s: Input) -> Progress<Command> {
    let (s, start) = opt(|s|parse_address(s))(s)?;
    let (s, end) = match &start {
        None => Ok((s, None)),
        Some(_) => {
            let (s, maybe) = opt(char(','))(s)?;
            match maybe {
                None => Ok((s, None)),
                Some(_) => {
                    let (s, addr) = parse_address(s)?;
                    Ok((s, Some(addr)))
                }
            }
        }
    }?;
    let (s, function) = parse_function(s)?;
    Ok((s, Command {
        start:  start,
        end: end,
        function: function
    }))
}

pub fn parse_command_finish(s: Input) -> io::Result<Command> {
    match parse_command(s).finish() {
        Ok((_, cmd)) => Ok(cmd),
        Err(e) => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("{}", e)))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use super::Address::*;
    use super::Function::*;
    use crate::new_regex_input;
    use assert_ok::assert_ok;

    fn function_equivalent(input: &str, expected: &Function, complete: bool) {
        let p_f = parse_function(new_regex_input(input));
        let (rest, f) = assert_ok!(&p_f);
        assert!(f.equivalent(expected), "unexpected function constructor {:?}", f);
        if complete {
            assert_eq!(rest.fragment(), &"");
        }
    }

    fn dummy_regex() -> Regex {
        Regex::new(".").unwrap() // ignored in equivalence
    }

    #[test]
    fn fun_d() {
        function_equivalent("d", &Fd, true);
    }

    #[test]
    fn s_slash() {
        function_equivalent("s/a/b/", &Fs(dummy_regex(), String::from("b")), true);
    }

    #[test]
    fn s_comma() {
        function_equivalent("s,a,b,", &Fs(dummy_regex(), String::from("b")), true);
    }

    fn address_equivalent(input: &str, expected: &Address) {
        let p_addr = parse_address(new_regex_input(input));
        let (rest, addr) = assert_ok!(&p_addr);
        assert!(addr.equivalent(expected), "unexpected Address constructor {:?}", addr);
        assert_eq!(rest.fragment(), &"");
    }

    #[test]
    fn addr_slash() {
        address_equivalent("/foo/", &Context(dummy_regex()))
    }

    #[test]
    fn addr_comma() {
        address_equivalent("\\,foo,", &Context(dummy_regex()))
    }

    #[test]
    fn clean_noop() {
        assert_eq!(clean_replacement(&Syntax::Extended, "foo".to_string()), "foo")
    }

    #[test]
    fn clean_ref() {
        assert_eq!(clean_replacement(&Syntax::Extended, r"foo\1".to_string()), "foo${1}")
    }

    #[test]
    fn clean_dollar() {
        assert_eq!(clean_replacement(&Syntax::Extended, "$foo".to_string()), "$$foo")
    }
}
