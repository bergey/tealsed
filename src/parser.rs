extern crate nom;

use nom::bytes::complete::{is_not, take_while};
use nom::character::complete::char;
use nom::combinator::success;
use nom::branch::alt;
use nom::{
    bytes::complete::{tag, take_while_m_n},
    combinator::map_res,
    sequence::tuple,
    Finish, IResult,
};
use regex_syntax::ast::{Ast, Literal, LiteralKind, Position, Span};

type Progress<'a> = IResult<&'a str, Ast>;

// probably regexen are short enough that position isn't that important
// could pass Position around, make it part of Input, but need to implement ParseError trait
const ZERO_POSITION: Position = Position {
    offset: 0,
    line: 0,
    column: 0,
};

const ZERO_SPAN: Span = Span {
    start: ZERO_POSITION,
    end: ZERO_POSITION,
};

fn empty(s: &str) -> Progress {
    Ok((s, Ast::Empty(ZERO_SPAN.clone())))
}

fn dot(s: &str) -> Progress {
    let (s, _) = char('.')(s)?;
    Ok((s, Ast::Dot(ZERO_SPAN.clone())))
}

// TODO figure out which are actually special
const SPECIAL_CHARS : &str = ".\\[]{}^$";

fn literal(s: &str) -> Progress {
    let (s, lit) = is_not(SPECIAL_CHARS)(s)?;
    Ok((s, Ast::Literal(Literal{
        span: ZERO_SPAN.clone(),
        kind: LiteralKind::Verbatim,
        c: lit.chars().nth(0).unwrap()
    })))
}

pub fn posix(s: &str) -> Result<Ast, nom::error::Error<&str>> {
    // TODO posix Extended Regular Expressions
    // according to `man re_format` or IEEE 1003.2
    let (_, ast) = alt((literal, dot, empty))(s).finish()?;
    Ok(ast)
}

#[cfg(test)]
pub mod tests {
    use crate::*;
    use assert_ok::assert_ok;

    #[test]
    fn empty() {
        assert_ok!(posix(""));
    }

    #[test]
    fn literals() {
        assert_ok!(posix("this is a valid regex"));
    }

    #[test]
    fn wildcard_dot() {
        let ast = assert_ok!(posix("."));
        match ast {
            Ast::Dot(_) => (),
            _ => panic!("unexpected regex parse: {:?}", ast),
        }
    }
}
