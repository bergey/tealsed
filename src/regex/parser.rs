extern crate nom;

use nom::character::complete::{char, none_of, one_of, u32};
use nom::branch::alt;
use nom::error::{ Error, ErrorKind};
use nom::{
    multi::many1,
    combinator::opt,
    Err, Finish, IResult,
};
use nom_locate::{LocatedSpan};
use regex_syntax::ast::{Ast, Concat, Literal, LiteralKind, Position, Repetition, RepetitionKind, RepetitionOp, RepetitionRange, Span};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtraState {
    pub last_regex: u32,
    pub end_char: char,
}

pub type Input<'a> = LocatedSpan<&'a str, ExtraState>;

pub fn new_regex_input<'a>(s: &'a str) -> Input<'a> {
    LocatedSpan::new_extra(s, ExtraState {
        last_regex: 0,
        end_char: '/',
    })
}

pub type Progress<'a> = IResult<Input<'a>, Ast>;

// Construct a regex::ast Position from a nom_locate LocatedSpan
fn position(s: Input) -> Position {
    Position {
        offset: s.location_offset(),
        line: usize::try_from(s.location_line()).unwrap(),
        column: s.get_column(),
    }
}

// only valid in ()?
fn empty(s: Input<'_>) -> Progress {
    let pos = position(s);
    Ok((s, Ast::Empty(Span{start: pos, end: pos.clone()})))
}

fn dot(s: Input<'_>) -> Progress {
    let start = position(s);
    let (s, _) = char('.')(s)?;
    let end = position(s);
    Ok((s, Ast::Dot(Span{start: start, end: end})))
}

// re_format says these have special meaning if not escaped with \
const SPECIAL_CHARS : &str = "^.[$()|*+?{\\";

fn literal(s: Input<'_>) -> Progress {
    let start = position(s);
    if let Some(c) = s.fragment().chars().next() {
        if c == s.extra.end_char {
            return Err(Err::Error(Error::new(s, ErrorKind::Fail)))
        }
    }
    let (s, lit) = none_of(SPECIAL_CHARS)(s)?;
    let end = position(s);
    Ok((s, Ast::Literal(Literal{
        span: Span{start: start, end: end},
        kind: LiteralKind::Verbatim,
        c: lit
    })))
}

fn escaped_literal(s: Input<'_>) -> Progress {
    let start = position(s);
    let (s, _) = char('\\')(s)?;
    let (s, c) = alt((char(s.extra.end_char), one_of(SPECIAL_CHARS)))(s)?;
    let end = position(s);
    Ok((s, Ast::Literal(Literal{
        span: Span{start: start, end: end},
        kind: if c == s.extra.end_char {
            LiteralKind::Punctuation
        } else {
            LiteralKind::Verbatim
        },
        c: c
    })))
}

fn atom(s: Input<'_>) -> Progress {
    // TODO () ^ $ \^.[$()|*+?{\ \
    alt((literal, escaped_literal, dot))(s)
}

fn char_quantifier(s: Input<'_>) -> IResult<Input, RepetitionOp> {
    let start = position(s);
    let (s, c) = one_of("*+?")(s)?;
    let quantifier = match c {
        '*' => RepetitionKind::ZeroOrMore,
        '+' => RepetitionKind::OneOrMore,
        '?' => RepetitionKind::ZeroOrOne,
        _ => panic!("one_of returned an unexpected character")

    };
    let end = position(s);
    Ok((s, RepetitionOp {
        span: Span{start: start, end: end},
        kind: quantifier
    } ))
}

fn bound(s: Input<'_>) -> IResult<Input, RepetitionOp> {
    let start = position(s);
    let (s, _) = char('{')(s)?;
    let (s, min) = u32(s)?;
    let (s, o_comma) = opt(char(','))(s)?;
    let (s, bound) = match o_comma {
        None => Ok((s, RepetitionKind::Range(RepetitionRange::Exactly(min)))),
        Some(_) => {
            let (s, o_max) = opt(u32)(s)?;
            match o_max {
                None => Ok((s, RepetitionKind::Range(RepetitionRange::AtLeast(min)))),
                // check that max <= 255 to match sed?
                Some(max) => Ok((s, RepetitionKind::Range(RepetitionRange::Bounded(min, max))))
            }
        }
    }?;
    let (s, _) = char('}')(s)?;
    let end = position(s);
    Ok((s, RepetitionOp {
        span: Span{start: start, end: end},
        kind: bound
    } ))
}

fn quantified_piece(s: Input<'_>) -> Progress {
    let start = position(s);
    let (s, atom) = atom(s)?;
    let (s, o_quantifier) = opt(alt((char_quantifier, bound)))(s)?;
    let end = position(s);
    match o_quantifier {
        None => Ok((s, atom)),
        Some(quantifier) => {
            Ok((s, Ast::Repetition(Repetition {
                span: Span{start: start, end: end},
                op: quantifier,
                greedy: true,
                ast: Box::new(atom)
            })))
        }
    }
}

fn branch(s: Input<'_>) -> Progress {
    let start = position(s);
    let (s, atoms) = many1(quantified_piece)(s)?;
    let end = position(s);
    if atoms.len() == 1 { // TODO make this less clunky or define a helper
        Ok((s, atoms.into_iter().nth(0).unwrap()))
    } else {
        Ok((s, Ast::Concat(Concat{
            span: Span{start: start, end: end},
            asts: atoms
        })))
    }
}

pub fn parse(end_char: char, mut s: Input<'_>) -> Progress {
    // TODO posix Extended Regular Expressions
    // according to `man re_format` or IEEE 1003.2
    s.extra.end_char = end_char;
    branch(s)
}

pub fn parse_complete(end_char: char, s: &str) -> Result<Ast, nom::error::Error<Input>> {
    let s = new_regex_input(s);
    let (_, ast) = parse(end_char, s).finish()?;
    Ok(ast)
}

#[cfg(test)]
pub mod tests {
    use crate::regex::equivalent::Equivalent;
    use super::*;
    use assert_ok::assert_ok;
    use regex_syntax::ast::parse::Parser;


    fn match_modern_syntax(pattern: &str) {
        let expected = Parser::new().parse(pattern).unwrap();
        let actual = assert_ok!(parse_complete('/', pattern));
        if !actual.equivalent(&expected) {
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn literals() {
        let input = "this is a valid regex";
        let ast = assert_ok!(parse_complete('/', &input));
        match &ast {
            Ast::Concat(c) => assert_eq!(c.asts.len(), input.len()),
            _ => panic!("unexpected regex parse: {:?}", ast),
        }
    }

    #[test]
    fn wildcard_dot() {
        let ast = assert_ok!(parse_complete('/', "."));
        match &ast {
            Ast::Dot(_) => (),
            _ => panic!("unexpected regex parse: {:?}", ast),
        }
    }

    #[test]
    fn star() {
        match_modern_syntax("foo*");
    }

    #[test]
    fn plus() {
        match_modern_syntax("a+");
    }

    #[test]
    fn question() {
        match_modern_syntax("ab?");
    }

    #[test]
    fn exact_count() {
        match_modern_syntax("o{2}")
    }

    #[test]
    fn min_count() {
        match_modern_syntax("x{2,}")
    }

    #[test]
    fn range() {
        match_modern_syntax("x{2,5}")
    }
    
}
