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
            (Ast::Assertion(a), Ast::Assertion(b)) => a.equivalent(b),
            (Ast::Class(a), Ast::Class(b)) => a.equivalent(b),
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

impl Equivalent for Assertion {
    fn equivalent(&self, other: &Assertion) -> bool {
        self.kind == other.kind
    }
}

impl Equivalent for Class {
    fn equivalent(&self, other: &Class) -> bool {
        use Class::*;
        match (self, other) {
            (Bracketed(a), Bracketed(b)) => a.equivalent(b),
            (Unicode(_), Unicode(_)) => panic!("not implemented"),
            (Perl(_), Perl(_)) => panic!("not implemented"),
            _ => false
        }
    }
}

impl Equivalent for ClassBracketed {
    fn equivalent(&self, other: &ClassBracketed) -> bool {
        self.negated == other.negated && self.kind.equivalent(&other.kind)
    }
}

impl Equivalent for ClassSet {
    fn equivalent(&self, other: &ClassSet) -> bool {
        use ClassSet::*;
        match (self, other) {
            (Item(a), Item(b)) => a.equivalent(b),
            (BinaryOp(_), BinaryOp(_)) => panic!("not implemented"),
            _ => false
        }
    }
}

impl Equivalent for ClassSetItem {
    fn equivalent(&self, other: &ClassSetItem) -> bool {
        use ClassSetItem::*;
        match (self, other) {
            (Empty(_), Empty(_)) => true,
            (Literal(a), Literal(b)) => a.equivalent(b),
            (Range(a), Range(b)) => a.start.equivalent(&b.start) && a.end.equivalent(&b.end),
            (Ascii(_), Ascii(_)) => panic!("not implemented"),
            (Unicode(_), Unicode(_)) => panic!("not implemented"),
            (Perl(_), Perl(_)) => panic!("not implemented"),
            (Bracketed(_), Bracketed(_)) => panic!("not implemented"),
            (Union(a), Union(b)) => a.items.len() == b.items.len() &&
                a.items.iter().enumerate().all( |(i, item)| item.equivalent(&b.items[i])),
            _ => false
        }
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
        self.kind.equivalent(&other.kind) && self.ast.equivalent(&other.ast)
    }
}

impl Equivalent for GroupKind {
    fn equivalent(&self, other: &GroupKind) -> bool {
        use GroupKind::*;
        match (self, other) {
            (CaptureIndex(a), CaptureIndex(b)) => a == b,
            (CaptureName(a), CaptureName(b)) => a.name == b.name  && a.index == b.index,
            (NonCapturing(_), NonCapturing(_)) => true, // TODO flags?
            _ => false
        }
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
