extern crate nom;

use nom::character::complete::{char, none_of, one_of, u32};
use nom::branch::alt;
use nom::{
    multi::many1,
    combinator::opt,
    Finish, IResult,
};
use regex_syntax::ast::{Ast, Concat, Literal, LiteralKind, Position, Repetition, RepetitionKind, RepetitionOp, RepetitionRange, Span};

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

fn char_quantifier(s: &str) -> IResult<&str, RepetitionKind> {
    let (s, c) = one_of("*+?")(s)?;
    let quantifier = match c {
        '*' => RepetitionKind::ZeroOrMore,
        '+' => RepetitionKind::OneOrMore,
        '?' => RepetitionKind::ZeroOrOne,
        _ => panic!("one_of returned an unexpected character")

    };
    Ok((s, quantifier))
}

fn bound(s: &str) -> IResult<&str, RepetitionKind> {
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
    Ok((s, bound))
}

fn quantified_piece(s: &str) -> Progress {
    let (s, atom) = atom(s)?;
    let (s, o_quantifier) = opt(alt((char_quantifier, bound)))(s)?;
    match o_quantifier {
        None => Ok((s, atom)),
        Some(quantifier) => {
            Ok((s, Ast::Repetition(Repetition {
                span: ZERO_SPAN,
                op: RepetitionOp {
                    span: ZERO_SPAN,
                    kind: quantifier
                },
                greedy: true,
                ast: Box::new(atom)
            })))
        }
    }
}

fn branch(s: &str) -> Progress {
    let (s, atoms) = many1(quantified_piece)(s)?;
    if atoms.len() == 1 { // TODO make this less clunky or define a helper
        Ok((s, atoms.into_iter().nth(0).unwrap()))
    } else {
        Ok((s, Ast::Concat(Concat{
            span: ZERO_SPAN,
            asts: atoms
        })))
    }
}

pub fn posix(s: &str) -> Result<Ast, nom::error::Error<&str>> {
    // TODO posix Extended Regular Expressions
    // according to `man re_format` or IEEE 1003.2
    let (_, ast) = branch(s).finish()?;
    Ok(ast)
}

#[cfg(test)]
pub mod tests {
    use crate::equivalent::Equivalent;
    use super::*;
    use assert_ok::assert_ok;
    use regex_syntax::ast::parse::Parser;


    fn match_modern_syntax(pattern: &str) {
        let expected = Parser::new().parse(pattern).unwrap();
        let actual = assert_ok!(posix(pattern));
        if !actual.equivalent(&expected) {
            assert_eq!(actual, expected);
        }
    }

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
