use ::regex::Regex;
use crate::regex;
use std::io;

use nom;
use nom::{Err, IResult};
use nom::branch::alt;
use nom::character::complete::{char, none_of};
use nom::combinator::opt;
use nom::error::{ Error, ErrorKind};
use nom::multi::many0;

pub enum Address {
    LineNumber(u64),
    // LastLine, // TODO how do we detect last line?  From stdin, in particular
    Context(Regex),
}

pub enum Function {
    S(Regex, String),
}

pub struct Command {
    start: Option<Address>,
    end: Option<Address>, // should not be Some if start is None
    function: Function,
}

// TODO better error handling
pub fn parse_function(cmd: &str, syntax: &regex::Syntax) -> Result<Function, std::io::Error> {
    let mut chars = cmd.chars();
    match chars.next().unwrap() {
        's' => {
            let sep = chars.next().unwrap();
            let mut words = split_on(&cmd[2..], &sep);
            if words.len() == 2 {
                regex::parse(syntax, &words[0])
                    .map(|regex| Function::S(regex, words.pop().unwrap()))
            } else {
                Err(io::Error::new(io::ErrorKind::InvalidInput, format!(
                    "unexpected number of function arguments: {}",
                    words.len()
                )))
            }
        }
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "unknown function letter".to_string())),
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

pub fn match_address(addr: &Address, text: &str, line_num: u64) -> bool {
    match addr {
        Address::LineNumber(l) => *l == line_num,
        Address::Context(regex) => regex.is_match(text),
    }
}

pub fn parse_address<'a>(s: &'a str, syntax: &regex::Syntax) -> IResult<&'a str, Address> {
    alt((line_number_addr, context_addr(syntax)))(s)
}

fn line_number_addr(s: &str) -> IResult<&str, Address> {
    let (s, n) = nom::character::complete::u64(s)?;
    Ok((s, Address::LineNumber(n)))
}

fn context_addr(syntax: &regex::Syntax) -> impl Fn(&str) -> IResult<&str, Address> {

    let syntax_ = syntax.clone();
    move |s: &str| {
        // TODO other start chars
        let (s, _) = char('/')(s)?;
        let (s, addr) = many0(none_of("/"))(s)?;
        let (s, _) = char('/')(s)?;
        // TODO \/ or [/] do not end the regex
        let r_regex = regex::parse(&syntax_, String::from_iter(addr).as_ref());
        let regex = match r_regex {
            Err(_) => {
                Err(Err::Failure(Error::new(s, ErrorKind::Fail)))
            },
            Ok(regex) => Ok(regex)
        }?;
        Ok((s, Address::Context(regex)))
    }
}

pub fn parse_command<'a>(s: &'a str, syntax: &regex::Syntax) -> IResult<&'a str, Command> {
    let (s, start) = opt(|s|parse_address(s, syntax))(s)?;
    let (s, end) = match &start {
        None => Ok((s, None)),
        Some(_) => {
            let (s, maybe) = opt(char(','))(s)?;
            match maybe {
                None => Ok((s, None)),
                Some(_) => {
                    let (s, addr) = parse_address(s, syntax)?;
                    Ok((s, Some(addr)))
                } 
            }
        }
    }?;
    let (s, function) = parse_function(s, syntax)?;
    OK((s, Command {
        start:  start,
        end: end,
        function: function
    }))

}
