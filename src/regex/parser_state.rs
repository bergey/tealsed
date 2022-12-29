use nom::IResult;
use nom_locate::{LocatedSpan};
use regex_syntax::ast::Ast;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Syntax {
    Basic, // POSIX basic, according man re_syntax
    Extended,  // POSIX Extended, like egrep
    Teal, // probably the syntax of regex crate except substitutions, TBD
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtraState {
    pub last_regex: u32,
    // below are not state; they are set once at start of parsing
    pub end_char: char,
    pub syntax: Syntax,
    pub gnu: bool,
}

pub type Input<'a> = LocatedSpan<&'a str, ExtraState>;

pub type Progress<'a, T = Ast> = IResult<Input<'a>, T>;

pub fn new_parser_input<'a>(s: &'a str) -> Input<'a> {
    LocatedSpan::new_extra(s, ExtraState {
        last_regex: 0,
        end_char: '/',
        syntax: Syntax::Teal,
        gnu: false
    })
}

