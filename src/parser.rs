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
    match o_comma {
        None => Ok((s, RepetitionKind::Range(RepetitionRange::Exactly(min)))),
        Some(_) => {
            let (s, o_max) = opt(u32)(s)?;
            match o_max {
                None => Ok((s, RepetitionKind::Range(RepetitionRange::AtLeast(min)))),
                // check that max <= 255 to match sed?
                Some(max) => Ok((s, RepetitionKind::Range(RepetitionRange::Bounded(min, max))))
            }
        }
    }
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

mod equivalent {
  use regex_syntax::ast::*;
  // compare regex ASTs, ignoring Span & Position
  pub trait Equivalent {
      fn equivalent(&self, _: &Self) -> bool;
  }

  impl Equivalent for Ast {
      fn equivalent(&self, other: &Ast) -> bool {
          match (self, other) {
              (Ast::Empty(_), Ast::Empty(_)) => true,
              (Ast::Flags(_), Ast::Flags(_)) => panic!("not implemented"),
              (Ast::Literal(a), Ast::Literal(b)) => a.equivalent(b),
              (Ast::Dot(_), Ast::Dot(_)) => true,
              (Ast::Assertion(_), Ast::Assertion(_)) => panic!("not implemented"),
              (Ast::Class(_), Ast::Class(_)) => panic!("not implemented"),
              (Ast::Repetition(a), Ast::Repetition(b)) => a.equivalent(b),
              (Ast::Group(_), Ast::Group(_)) => panic!("not implemented"),
              (Ast::Alternation(_), Ast::Alternation(_)) => panic!("not implemented"),
              (Ast::Concat(a), Ast::Concat(b)) => a.equivalent(b),
              _ => false
          }
      }
  }

  impl Equivalent for Literal {
      fn equivalent(&self, other: &Literal) -> bool {
          self.c == other.c && self.kind == other.kind
      }
  }

  impl Equivalent for Repetition {
      fn equivalent(&self, other: &Repetition) -> bool {
          self.op.equivalent(&other.op) && self.greedy == other.greedy && self.ast.equivalent(&other.ast)
      }
  }

  impl Equivalent for RepetitionOp {
      fn equivalent(&self, other: &RepetitionOp) -> bool {
          self.kind == other.kind
      }
  }

  impl Equivalent for Concat {
      fn equivalent(&self, other: &Concat) -> bool {
          self.asts.len() == other.asts.len() && std::iter::zip(&self.asts, &other.asts).all(|(a, b)| a.equivalent(&b))
      }
  }
}

#[cfg(test)]
pub mod tests {
    use super::equivalent::Equivalent;
    use super::*;
    use assert_ok::assert_ok;
    use regex_syntax::ast::parse::Parser;


    fn match_modern_syntax(pattern: &str) {
        let expected = Parser::new().parse(pattern).unwrap();
        let actual = assert_ok!(posix(pattern));
        assert!(actual.equivalent(&expected));
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

    #[test]
    fn star() {
        match_modern_syntax("foo*");
    }
}
