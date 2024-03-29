extern crate nom;
use nom::character::complete::{anychar, char, none_of, one_of, u32};
use nom::branch::alt;
use nom::error::{ Error, ErrorKind};
use nom::{
    multi::{many0, many1},
    combinator::{not, opt, peek},
    Err, Finish, IResult,
};
use nom_locate::{LocatedSpan};
use regex_syntax::ast::{Alternation, Assertion, AssertionKind, Ast, CaptureName, Class, ClassBracketed, ClassSet, ClassSetItem, ClassSetRange, ClassSetUnion, Concat, Flags, Group, GroupKind, Literal, LiteralKind, Position, Repetition, RepetitionKind, RepetitionOp, RepetitionRange, Span};

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

pub fn new_regex_input<'a>(s: &'a str) -> Input<'a> {
    LocatedSpan::new_extra(s, ExtraState {
        last_regex: 0,
        end_char: '/',
        syntax: Syntax::Teal,
        gnu: false
    })
}

// Construct a regex::ast Position from a nom_locate LocatedSpan
fn position(s: Input) -> Position {
    Position {
        offset: s.location_offset(),
        line: usize::try_from(s.location_line()).unwrap(),
        column: s.get_column(),
    }
}

// only valid in ()?
fn empty(s: Input) -> Progress {
    let pos = position(s);
    Ok((s, Ast::Empty(Span{start: pos, end: pos.clone()})))
}

fn dot(s: Input) -> Progress {
    let start = position(s);
    let (s, _) = char('.')(s)?;
    let end = position(s);
    Ok((s, Ast::Dot(Span{start: start, end: end})))
}

// re_format says these have special meaning if not escaped with \, { is handled extra-specially
const SPECIAL_CHARS : &str = "^.[$()|*+?\\";

fn literal(s: Input) -> Progress {
    let start = position(s);
    let (s, lit) = none_of(SPECIAL_CHARS)(s)?;
    if lit == s.extra.end_char {
        return Err(Err::Error(Error::new(s, ErrorKind::Fail)))
    }
    if lit == '{' { // taken as a literal if it cannot be a bound
        // TODO support this in Teal syntax?
        peek(not(one_of("0123456789")))(s)?;
    }
    let end = position(s);
    Ok((s, Ast::Literal(Literal{
        span: Span{start: start, end: end},
        kind: LiteralKind::Verbatim,
        c: lit
    })))
}

fn escaped_literal(s: Input) -> Progress {
    use LiteralKind::*;
    use regex_syntax::ast::SpecialLiteralKind::*;

    let start = position(s);
    let (s, _) = char('\\')(s)?;
    let (s, c) = anychar(s)?;
    let end = position(s);
    Ok((s, Ast::Literal(Literal{
        span: Span{start: start, end: end},
        kind: match c {
            '^' | '.' | '[' | '$' | '(' | ')' | '|' | '*' | '+' | '?' | '{' | '\\' => Punctuation,
            'n' => Special(LineFeed),
            'r' => Special(CarriageReturn),
            't' => Special(Tab),
            _ => Verbatim,
        },
        c: c
    })))
}

fn named_group_intro(s: Input) -> Progress<GroupKind> {
    let start = position(s);
    let (s, _) = char('P')(s)?;
    let (mut s, v) = nom::sequence::delimited( char('<'), many1(none_of(">")), char('>'))(s)?;
    let end = position(s);
    s.extra.last_regex += 1;
    Ok((s, GroupKind::CaptureName( CaptureName {
        span: Span { start, end },
        name: v.into_iter().collect(),
        index: s.extra.last_regex,
    })))
}

fn non_capture_group_intro(s: Input) -> Progress<GroupKind> {
    let start = position(s);
    let (s, _) = char(':')(s)?;
    let end = position(s);
    Ok((s, GroupKind::NonCapturing(Flags {
        span: Span { start, end },
        items: Vec::new(),
    })))
}

fn group(s: Input) -> Progress {
    use nom::sequence::preceded;
    let start = position(s);
    let (s, _) = char( '(' )(s)?;
    let (s, group_kind) = match s.extra.syntax {
        Syntax::Basic => (s, None),
        Syntax::Extended => opt(preceded(char('?'), non_capture_group_intro))(s)?,
        Syntax::Teal => opt(preceded(char('?'), alt((named_group_intro, non_capture_group_intro))))(s)?
    };
    let (s, ast) = alt((alternation, empty))(s)?;
    let (mut s, _) = char( ')' )(s)?;
    let end = position(s);

    Ok((s, Ast::Group( Group {
        span: Span{ start: start, end: end},
        kind: match group_kind {
            Some(k) => k,
            None => {
                s.extra.last_regex += 1;
                GroupKind::CaptureIndex(s.extra.last_regex)
            }
        },
        ast: Box::new(ast),
    })))
}

fn assertion(s: Input) -> Progress {
    let start = position(s);
    let (s, c) = one_of("^$")(s)?;
    let kind = match c {
        '^' => AssertionKind::StartLine,
        '$' => AssertionKind::EndLine,
        _ => panic!("impossible assertion char")
    };
    let end = position(s);
    Ok((s, Ast::Assertion(Assertion {
        span: Span { start, end },
        kind: kind
    })))
}

// also known as "bracket expression"
fn class(s: Input) -> Progress {
    let start = position(s);
    let (s, _) = char('[')(s)?;
    let (s, negated) = opt(char('^'))(s)?;
    let (s, items) = many1(alt((class_range, class_literal)))(s)?;
    let (s, _) = char(']')(s)?;
    let end = position(s);
    Ok((s, Ast::Class(Class::Bracketed( ClassBracketed {
        span: Span { start, end },
        negated: negated.is_some(),
        // DESIGN: do I want to support Rust-regex set operations, in Syntax::Teal?
        kind: ClassSet::Item(
            if items.len() == 1 {
                items.into_iter().next().unwrap()
            } else {
                ClassSetItem::Union( ClassSetUnion {
                    span: Span { start, end },
                    items
                })
            }
        )
    }) )))
}

fn class_literal(s: Input) -> Progress<ClassSetItem> {
    let start = position(s);
    let (s, c) = none_of("]")(s)?;
    let end = position(s);
    Ok((s, ClassSetItem::Literal(Literal {
        span: Span { start, end },
        kind: LiteralKind::Verbatim,
        c: c
    })))
}

fn class_range(s: Input) -> Progress<ClassSetItem> {
    let start = position(s);
    let (s, a) = none_of("-]")(s)?;
    let (s, _) = char('-')(s)?;
    let (s, z) = none_of("-]")(s)?;
    let end = position(s);
    let span = Span { start, end };  // I'm lazy, don't care where each char is
    Ok((s, ClassSetItem::Range(ClassSetRange {
        span: Span { start, end },
        start: Literal {
            span, kind: LiteralKind::Verbatim,
            c: a
        },
        end: Literal {
            span, kind: LiteralKind::Verbatim,
            c: z
        }
    })))
}

fn atom(s: Input) -> Progress {
    alt((group, class, literal, escaped_literal, dot, assertion))(s)
}

fn char_quantifier(s: Input) -> Progress<RepetitionOp> {
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

fn bound(s: Input) -> Progress<RepetitionOp> {
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

fn quantified_piece(s: Input) -> Progress {
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

fn branch(s: Input) -> Progress {
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

fn bar_branch(s: Input) -> Progress {
    let (s, _) = char('|')(s)?;
    branch(s)
}

fn alternation(s: Input) -> Progress {
    let start = position(s);
    let (s, first) = branch(s)?;
    let (s, mut rest) = many0(bar_branch)(s)?;
    let end = position(s);
    if rest.len() == 0 {
        Ok((s, first))   
    } else {
        rest.insert(0, first);
        Ok((s, Ast::Alternation(Alternation {
            span: Span { start, end },
            asts: rest
        })))
    }
}

pub fn parse(end_char: char, mut s: Input) -> Progress {
    // TODO posix Extended Regular Expressions
    // according to `man re_format` or IEEE 1003.2
    s.extra.end_char = end_char;
    alternation(s)
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
    use ::regex::Regex;

    fn match_modern_syntax(pattern: &str) {
        let expected = Parser::new().parse(pattern).unwrap();
        let actual = assert_ok!(parse_complete('/', pattern));
        if !actual.equivalent(&expected) {
            assert_eq!(actual, expected);
        }
    }

    fn matches(pattern: &str, input: &str) {
        let ast = assert_ok!(parse_complete('/', pattern));
        let regex = assert_ok!(Regex::new(&format!("{}", ast)));
        assert!(regex.is_match(input))
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

    #[test]
    fn group() {
        match_modern_syntax("(a*)")
    }

    #[test]
    fn non_capturing_group() {
        match_modern_syntax("(?:a*)")
    }

    #[test]
    fn named_group() {
        match_modern_syntax("(?P<n>a*)")
    }

    #[test]
    fn empty_group() {
        match_modern_syntax("()")
    }

    #[test]
    fn alternation1() {
        match_modern_syntax("a|b")
    }
    
    #[test]
    fn alternation2() {
        match_modern_syntax("a|b|c")
    }

    #[test]
    fn end() {
        match_modern_syntax("a$")
    }

    #[test]
    fn matches_end() {
        matches("$", "")
    }

    #[test]
    fn class_one() {
        match_modern_syntax("[a]")
    }

    #[test]
    fn class_two() {
        match_modern_syntax("[ab]")
    }

    #[test]
    fn class_negated() {
        match_modern_syntax("[^a]")
    }

    #[test]
    fn class_range() {
        match_modern_syntax("[a-z]")
    }

}
