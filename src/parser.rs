extern crate nom;

use nom::character::complete::{char, none_of, one_of};
use nom::branch::alt;
use nom::{
    multi::many1,
    Finish, IResult,
};
use regex_syntax::ast::{Ast, Concat, Literal, LiteralKind, Position, Span};

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

// only valid in ()?
fn empty(s: &str) -> Progress {
    Ok((s, Ast::Empty(ZERO_SPAN.clone())))
}

fn dot(s: &str) -> Progress {
    let (s, _) = char('.')(s)?;
    Ok((s, Ast::Dot(ZERO_SPAN.clone())))
}

// re_format says these have special meaning if not escaped with \
const SPECIAL_CHARS : &str = "^.[$()|*+?{\\";

fn literal(s: &str) -> Progress {
    let (s, lit) = none_of(SPECIAL_CHARS)(s)?;
    Ok((s, Ast::Literal(Literal{
        span: ZERO_SPAN.clone(),
        kind: LiteralKind::Verbatim,
        c: lit
    })))
}

fn escaped_literal(s: &str) -> Progress {
    let (s, _) = char('\\')(s)?;
    let (s, c) = one_of(SPECIAL_CHARS)(s)?;
    Ok((s, Ast::Literal(Literal{
        span: ZERO_SPAN.clone(),
        kind: LiteralKind::Punctuation,
        c: c
    })))
}

fn atom(s: &str) -> Progress {
    // TODO () ^ $ \^.[$()|*+?{\ \
    alt((literal, escaped_literal, dot))(s)
}

fn branch(s: &str) -> Progress {
    let (s, atoms) = many1(atom)(s)?;
    Ok((s, Ast::Concat(Concat{
        span: ZERO_SPAN,
        asts: atoms
    })))
}

pub fn posix(s: &str) -> Result<Ast, nom::error::Error<&str>> {
    // TODO posix Extended Regular Expressions
    // according to `man re_format` or IEEE 1003.2
    let (_, ast) = branch(s).finish()?;
    Ok(ast)
}

#[cfg(test)]
pub mod tests {
    use crate::*;
    use assert_ok::assert_ok;

    #[test]
    fn literals() {
        let input = "this is a valid regex";
        let ast = assert_ok!(posix(&input));
        match &ast {
            Ast::Concat(c) => assert_eq!(c.asts.len(), input.len()),
            _ => panic!("unexpected regex parse: {:?}", ast),
        }
    }

    #[test]
    fn wildcard_dot() {
        let ast = assert_ok!(posix("."));
        match &ast {
            Ast::Concat(c) => {
                assert_eq!(c.asts.len(), 1);
                match c.asts[0] {
                    Ast::Dot(_) => (),
                    _ => panic!("unexpected regex parse: {:?}", ast),
                }
            },
            _ => panic!("unexpected regex parse: {:?}", ast),
        }
    }
}
