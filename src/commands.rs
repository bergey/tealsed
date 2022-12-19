use ::regex::Regex;
use crate::regex;
use std::io;

use nom;
use nom::{Err, Finish, IResult};
use nom::branch::alt;
use nom::character::complete::{anychar, char, none_of};
use nom::combinator::{eof, fail, opt};
use nom::error::{ Error, ErrorKind};
use nom::multi::many0;

#[derive(Debug)]
pub enum Address {
    LineNumber(u64),
    // LastLine, // TODO how do we detect last line?  From stdin, in particular
    Context(Regex), // TODO case-insensitive
}

pub enum Function {
    D,
    DD,
    P,
    S(Regex, String),
}

pub struct Command {
    pub start: Option<Address>,
    pub end: Option<Address>, // should not be Some if start is None
    pub function: Function,
}

type Progress<'a, T> = IResult<&'a str, T>;

fn take_until<'a>(sep: char, s: &'a str) -> Progress<&'a str> {
    let o_split = s.split_once(sep);
    match o_split {
        None => Err(Err::Error(Error::new(s, ErrorKind::SeparatedNonEmptyList))),
        Some((before, after)) => Ok((after, before))
    }
}

pub fn parse_function<'a>(cmd: &'a str) -> Progress<Function> {
    let (s, function) = anychar(cmd)?;
    match function {
        'd' => Ok((s, Function::D)),
        'D' => Ok((s, Function::DD)),
        'p' => Ok((s, Function::P)),
        's' => {
            let (s, sep) = anychar(s)?;
            let (s, pattern) = take_until(sep, s)?;
            let (unused, ast) = regex::posix::parse(pattern)?;
            let _ = eof(unused)?;
            let regex = Regex::new(&format!("{}", ast)).unwrap();
            let (s, replacement) = take_until(sep, s)?;
            Ok((s, Function::S(regex, String::from(replacement))))
        }
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

pub fn parse_address<'a>(s: &'a str) -> IResult<&'a str, Address> {
    alt((line_number_addr, context_addr))(s)
}

fn line_number_addr(s: &str) -> IResult<&str, Address> {
    let (s, n) = nom::character::complete::u64(s)?;
    Ok((s, Address::LineNumber(n)))
}

fn context_addr<'a>(s: &'a str) -> IResult<&'a str, Address> {
    // TODO other start chars
    let (s, _) = char('/')(s)?;
    let (s, addr) = many0(none_of("/"))(s)?;
    let (s, _) = char('/')(s)?;
    // TODO \/ or [/] do not end the regex
    let r_regex = regex::parse(String::from_iter(addr).as_ref());
    let regex = match r_regex {
        Err(_) => {
            Err(Err::Failure(Error::new(s, ErrorKind::Fail)))
        },
        Ok(regex) => Ok(regex)
    }?;
    Ok((s, Address::Context(regex)))
}

pub fn parse_command<'a>(s: &'a str) -> IResult<&'a str, Command> {
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

pub fn parse_command_finish<'a>(s: &'a str) -> io::Result<Command> {
    match parse_command(s).finish() {
        Ok((_, cmd)) => Ok(cmd),
        Err(e) => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("{}", e)))
    }
}
