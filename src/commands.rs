use ::regex::Regex;
use crate::regex;
use crate::regex::parser::{Input};
use crate::regex::equivalent::Equivalent;
use std::io;

use nom;
use nom::{Err, Finish, IResult};
use nom::branch::alt;
use nom::character::complete::{anychar, char, none_of};
use nom::combinator::{fail, opt, rest};
use nom::error::{ Error, ErrorKind};
use nom::multi::many0;

#[derive(Debug)]
pub enum Address {
    LineNumber(u64),
    // LastLine, // TODO how do we detect last line?  From stdin, in particular
    Context(Regex), // TODO case-insensitive
}

// single letter for uppercase function names
// F followed by a letter for lowercase function names
#[derive(Clone, Debug)]
pub enum Function {
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

type Progress<'a, T> = IResult<Input<'a>, T>;

pub struct Command {
    pub start: Option<Address>,
    pub end: Option<Address>, // should not be Some if start is None
    pub function: Function,
}

fn take_until<'a>(sep: char, s: Input) -> Progress<String> {
    let string: String = sep.to_string();
    let str: &str = string.as_ref();
    let (s, vec) = many0(none_of(str))(s)?;
    Ok((s, vec.into_iter().collect()))
}

pub fn parse_function<'a>(cmd: Input<'a>) -> Progress<Function> {
    let (s, function) = anychar(cmd)?;
    use Function::{*};
    match function {
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
            Ok((s, Fs(regex, replacement)))
        },
        'x' => Ok((s, Fx)),
        _ => fail(cmd)
    }
}

fn split_on(s: &str, sep: &char) -> Vec<String> {
    let mut ret = Vec::new();
    let mut backslash = false;
    let mut begin = 0;
    for (i, c) in s.char_indices() {
        if c == *sep && !backslash {
            // let mut part = String::new();
            // part.push_str(&s[begin..i]);
            // ret.push(part);
            ret.push(s[begin..i].to_string());
            begin = i + 1;
        }
        backslash = c == '\\';
    }
    ret
}

// handles only a single address
// caller must maintain state between calls, decide whether to pass start or end pattern
pub fn match_address(addr: &Address, text: &str, line_num: u64) -> bool {
    match addr {
        Address::LineNumber(l) => *l == line_num,
        Address::Context(regex) => regex.is_match(text),
    }
}

pub fn parse_address<'a>(s: Input) -> Progress<Address> {
    alt((line_number_addr, context_addr))(s)
}

fn line_number_addr(s: Input) -> Progress<Address> {
    let (s, n) = nom::character::complete::u64(s)?;
    Ok((s, Address::LineNumber(n)))
}

fn context_addr<'a>(s: Input) -> Progress<Address> {
    // TODO other start chars
    let (s, _) = char('/')(s)?;
    let (s, addr) = many0(none_of("/"))(s)?;
    let (s, _) = char('/')(s)?;
    // TODO \/ or [/] do not end the regex
    let r_regex = regex::parse('/', String::from_iter(addr).as_ref());
    let regex = match r_regex {
        Err(_) => {
            Err(Err::Failure(Error::new(s, ErrorKind::Fail)))
        },
        Ok(regex) => Ok(regex)
    }?;
    Ok((s, Address::Context(regex)))
}

pub fn parse_command<'a>(s: Input) -> Progress<Command> {
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

pub fn parse_command_finish<'a>(s: Input) -> io::Result<Command> {
    match parse_command(s).finish() {
        Ok((_, cmd)) => Ok(cmd),
        Err(e) => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("{}", e)))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use super::Function::*;
    use crate::new_regex_input;
    use assert_ok::assert_ok;

    #[test]
    fn fun_d() {
        let p_f = parse_function(new_regex_input("d"));
        assert_ok!(&p_f);
        if let Ok((rest, f)) = p_f {
            assert_eq!(rest.fragment(), &"");
            assert!(f.equivalent(&Fd), "unexpected function constructor {:?}", f);
        }
    }

    #[test]
    fn s_slash() {
        let p_f = parse_function(new_regex_input("s/a/b/"));
        assert_ok!(&p_f);
        if let Ok((rest, f)) = p_f {
            match f {
                Fs(_, replacement) => assert_eq!(replacement, "b"),
                _ => panic!("failed to parse s")
            }
            assert_eq!(rest.fragment(), &"");
        }
    }
}
