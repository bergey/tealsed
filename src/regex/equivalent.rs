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
            (Ast::Group(a), Ast::Group(b)) => a.equivalent(b),
            (Ast::Alternation(a), Ast::Alternation(b)) => a.equivalent(b),
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

impl Equivalent for Group {
    fn equivalent(&self, other: &Group) -> bool {
        self.kind == other.kind && self.ast.equivalent(&other.ast)
    }
}

impl Equivalent for Alternation {
    fn equivalent(&self, other: &Alternation) -> bool {
        self.asts.len() == other.asts.len() &&
            self.asts.iter().enumerate().all( |(i, a)| a.equivalent(&other.asts[i]) )
    }
}

impl Equivalent for Concat {
    fn equivalent(&self, other: &Concat) -> bool {
        self.asts.len() == other.asts.len() && std::iter::zip(&self.asts, &other.asts).all(|(a, b)| a.equivalent(&b))
    }
}
