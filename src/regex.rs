mod equivalent;

pub mod posix;

use regex_syntax::ast::Ast;

pub enum Syntax {
    Rust,
    PosixExtended
}

pub fn parse(syntax: Syntax, s: &str) -> Result<Ast, String> {
    match syntax {
        Syntax::PosixExtended => posix::parse(s).map_err(|e| e.to_string()),
        Syntax::Rust => regex_syntax::ast::parse::Parser::new().parse(s).map_err(|e| e.to_string())
    }
}
