use crate::tokens::TokenStream;
use std::collections::{HashMap, HashSet};
use vn_core::ast::arena::ExprNode;
use vn_core::ast::{ArrowBody, AstArena, ExprKind};
use vn_core::{BinOp, TokenKind};

#[derive(Clone, Debug)]
pub enum ChainKind {
    Operators(Vec<usize>),
    Pipeline(Vec<usize>),
    Members(Vec<usize>),
    Ternary {
        question: usize,
        colon: usize,
        trailing_question: bool,
    },
}

#[derive(Clone, Debug)]
pub struct Chain {
    pub end: usize,
    pub kind: ChainKind,
}

pub struct Shapes {
    pub chains: HashMap<usize, Vec<Chain>>,
    pub assigns: HashMap<usize, usize>,
    pub ternary: HashSet<usize>,
}

fn class_of(kind: &ExprKind) -> Option<u32> {
    match kind {
        ExprKind::Logical { op, .. } => Some(100 + *op as u32),
        ExprKind::Binary { op, .. } => match op {
            BinOp::Add | BinOp::Sub => Some(1),
            BinOp::Mul | BinOp::Div | BinOp::Mod => Some(2),
            BinOp::BitAnd => Some(10),
            BinOp::BitOr => Some(11),
            BinOp::BitXor => Some(12),
            BinOp::Shl | BinOp::Shr | BinOp::Ushr => Some(13),
            BinOp::Pow
            | BinOp::Eq
            | BinOp::Ne
            | BinOp::Lt
            | BinOp::Le
            | BinOp::Gt
            | BinOp::Ge
            | BinOp::In => Some(20 + *op as u32),
        },
        ExprKind::Pipeline { .. } => Some(200),
        ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BigIntLiteral { .. }
        | ExprKind::DecimalLiteral { .. }
        | ExprKind::StrLiteral { .. }
        | ExprKind::CharLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::NullLiteral
        | ExprKind::Template { .. }
        | ExprKind::TaggedTemplate { .. }
        | ExprKind::Identifier { .. }
        | ExprKind::Missing
        | ExprKind::This
        | ExprKind::Super
        | ExprKind::ImportMeta(_)
        | ExprKind::Array { .. }
        | ExprKind::Object { .. }
        | ExprKind::Tuple { .. }
        | ExprKind::Unary { .. }
        | ExprKind::Update { .. }
        | ExprKind::Assign { .. }
        | ExprKind::Conditional { .. }
        | ExprKind::Member { .. }
        | ExprKind::Call { .. }
        | ExprKind::New { .. }
        | ExprKind::Function { .. }
        | ExprKind::Arrow { .. }
        | ExprKind::Sequence { .. }
        | ExprKind::Paren { .. }
        | ExprKind::Await { .. }
        | ExprKind::Yield { .. }
        | ExprKind::Spread { .. }
        | ExprKind::Range { .. }
        | ExprKind::NonNull { .. }
        | ExprKind::Try { .. }
        | ExprKind::As { .. }
        | ExprKind::Satisfies { .. }
        | ExprKind::ClassExpr { .. }
        | ExprKind::Match { .. }
        | ExprKind::Is { .. }
        | ExprKind::With { .. }
        | ExprKind::MetaAccess { .. } => None,
    }
}

fn chainable(class: u32) -> bool {
    !(20..100).contains(&class)
}

fn left_of(kind: &ExprKind) -> Option<vn_core::ast::ExprId> {
    if let ExprKind::Binary { left, .. }
    | ExprKind::Logical { left, .. }
    | ExprKind::Pipeline { left, .. } = kind
    {
        Some(*left)
    } else {
        None
    }
}

pub fn wraps_after_assign(kind: &ExprKind) -> bool {
    matches!(
        kind,
        ExprKind::Binary { .. }
            | ExprKind::Logical { .. }
            | ExprKind::Conditional { .. }
            | ExprKind::Pipeline { .. }
            | ExprKind::StrLiteral { .. }
    )
}

fn descend(kind: &ExprKind) -> Option<vn_core::ast::ExprId> {
    match kind {
        ExprKind::Call { callee, .. } => Some(*callee),
        ExprKind::Member { object, .. } => Some(*object),
        ExprKind::NonNull { expression } => Some(*expression),
        ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BigIntLiteral { .. }
        | ExprKind::DecimalLiteral { .. }
        | ExprKind::StrLiteral { .. }
        | ExprKind::CharLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::NullLiteral
        | ExprKind::Template { .. }
        | ExprKind::TaggedTemplate { .. }
        | ExprKind::Identifier { .. }
        | ExprKind::Missing
        | ExprKind::This
        | ExprKind::Super
        | ExprKind::ImportMeta(_)
        | ExprKind::Array { .. }
        | ExprKind::Object { .. }
        | ExprKind::Tuple { .. }
        | ExprKind::Unary { .. }
        | ExprKind::Update { .. }
        | ExprKind::Binary { .. }
        | ExprKind::Logical { .. }
        | ExprKind::Assign { .. }
        | ExprKind::Conditional { .. }
        | ExprKind::New { .. }
        | ExprKind::Function { .. }
        | ExprKind::Arrow { .. }
        | ExprKind::Sequence { .. }
        | ExprKind::Paren { .. }
        | ExprKind::Await { .. }
        | ExprKind::Yield { .. }
        | ExprKind::Spread { .. }
        | ExprKind::Pipeline { .. }
        | ExprKind::Range { .. }
        | ExprKind::Try { .. }
        | ExprKind::As { .. }
        | ExprKind::Satisfies { .. }
        | ExprKind::ClassExpr { .. }
        | ExprKind::Match { .. }
        | ExprKind::Is { .. }
        | ExprKind::With { .. }
        | ExprKind::MetaAccess { .. } => None,
    }
}

pub fn wraps_after_arrow(kind: &ExprKind) -> bool {
    !matches!(
        kind,
        ExprKind::Object { .. }
            | ExprKind::Array { .. }
            | ExprKind::Tuple { .. }
            | ExprKind::Template { .. }
            | ExprKind::TaggedTemplate { .. }
            | ExprKind::Match { .. }
            | ExprKind::ClassExpr { .. }
            | ExprKind::Arrow { .. }
            | ExprKind::Function { .. }
            | ExprKind::Paren { .. }
    )
}

pub struct Scanner<'a> {
    pub stream: &'a TokenStream,
    pub arena: &'a AstArena,
}

impl Scanner<'_> {
    fn first(&self, offset: u32) -> usize {
        self.stream.index_at_or_after(offset)
    }

    fn last(&self, end_offset: u32) -> usize {
        self.stream.index_at_or_after(end_offset).saturating_sub(1)
    }

    pub fn operator_after(&self, offset: u32) -> Option<usize> {
        let from = self.first(offset);
        (from..self.stream.toks.len())
            .take(3)
            .find(|index| self.stream.toks[*index].kind != TokenKind::RParen)
    }

    pub fn assignment(&self, rhs: &ExprNode, op: usize, shapes: &mut Shapes) {
        let end = self.last(rhs.range.end.offset);
        if wraps_after_assign(&rhs.kind) && end > op {
            shapes.assigns.insert(op, end);
        }
    }

    pub fn scan(&self) -> Shapes {
        let mut shapes = Shapes {
            chains: HashMap::new(),
            assigns: HashMap::new(),
            ternary: HashSet::new(),
        };
        let mut absorbed: HashSet<(u32, u32)> = HashSet::new();
        for node in self.arena.exprs() {
            let Some(parent) = class_of(&node.kind) else {
                continue;
            };
            if let Some(left) = left_of(&node.kind) {
                let child = self.arena.expr(left);
                if class_of(&child.kind) == Some(parent) && chainable(parent) {
                    absorbed.insert((child.range.start.offset, child.range.end.offset));
                }
            }
        }
        let mut inner_calls: HashSet<(u32, u32)> = HashSet::new();
        for node in self.arena.exprs() {
            if matches!(node.kind, ExprKind::Call { .. }) {
                self.inner_calls(node, &mut inner_calls);
            }
        }
        for node in self.arena.exprs() {
            match &node.kind {
                ExprKind::Call { .. } => {
                    if !inner_calls.contains(&(node.range.start.offset, node.range.end.offset)) {
                        self.members(node, &mut shapes);
                    }
                }
                ExprKind::Arrow { body, .. } => self.arrow(node, body, &mut shapes),
                ExprKind::Conditional {
                    test, consequent, ..
                } => self.ternary(node, *test, *consequent, &mut shapes),
                ExprKind::Assign { target, value, .. } => {
                    if let Some(op) = self.operator_after(self.arena.expr(*target).range.end.offset)
                    {
                        self.assignment(self.arena.expr(*value), op, &mut shapes);
                    }
                }
                ExprKind::Binary { .. } | ExprKind::Logical { .. } | ExprKind::Pipeline { .. } => {
                    let Some(class) = class_of(&node.kind) else {
                        continue;
                    };
                    if !chainable(class)
                        || absorbed.contains(&(node.range.start.offset, node.range.end.offset))
                    {
                        continue;
                    }
                    self.operators(node, class, &mut shapes);
                }
                ExprKind::IntLiteral { .. }
                | ExprKind::FloatLiteral { .. }
                | ExprKind::BigIntLiteral { .. }
                | ExprKind::DecimalLiteral { .. }
                | ExprKind::StrLiteral { .. }
                | ExprKind::CharLiteral { .. }
                | ExprKind::BoolLiteral { .. }
                | ExprKind::NullLiteral
                | ExprKind::Template { .. }
                | ExprKind::TaggedTemplate { .. }
                | ExprKind::Identifier { .. }
                | ExprKind::Missing
                | ExprKind::This
                | ExprKind::Super
                | ExprKind::ImportMeta(_)
                | ExprKind::Array { .. }
                | ExprKind::Object { .. }
                | ExprKind::Tuple { .. }
                | ExprKind::Unary { .. }
                | ExprKind::Update { .. }
                | ExprKind::Member { .. }
                | ExprKind::New { .. }
                | ExprKind::Function { .. }
                | ExprKind::Sequence { .. }
                | ExprKind::Paren { .. }
                | ExprKind::Await { .. }
                | ExprKind::Yield { .. }
                | ExprKind::Spread { .. }
                | ExprKind::Range { .. }
                | ExprKind::NonNull { .. }
                | ExprKind::Try { .. }
                | ExprKind::As { .. }
                | ExprKind::Satisfies { .. }
                | ExprKind::ClassExpr { .. }
                | ExprKind::Match { .. }
                | ExprKind::Is { .. }
                | ExprKind::With { .. }
                | ExprKind::MetaAccess { .. } => {}
            }
        }
        for list in shapes.chains.values_mut() {
            list.sort_by_key(|chain| std::cmp::Reverse(chain.end));
            list.dedup_by(|a, b| a.end == b.end);
        }
        shapes
    }

    fn strip_non_null(&self, mut id: vn_core::ast::ExprId) -> vn_core::ast::ExprId {
        while let ExprKind::NonNull { expression } = &self.arena.expr(id).kind {
            id = *expression;
        }
        id
    }

    fn inner_calls(&self, root: &ExprNode, inner: &mut HashSet<(u32, u32)>) {
        let mut cursor = root;
        loop {
            let Some(next) = descend(&cursor.kind) else {
                return;
            };
            cursor = self.arena.expr(next);
            if matches!(cursor.kind, ExprKind::Call { .. }) {
                inner.insert((cursor.range.start.offset, cursor.range.end.offset));
            }
        }
    }

    fn members(&self, root: &ExprNode, shapes: &mut Shapes) {
        let toks = &self.stream.toks;
        let mut dots = Vec::new();
        let mut calls = 0usize;
        let mut cursor = root;
        loop {
            if let ExprKind::Call { callee, .. } = &cursor.kind
                && matches!(
                    self.arena.expr(self.strip_non_null(*callee)).kind,
                    ExprKind::Member { .. }
                )
            {
                calls += 1;
            }
            if let ExprKind::Member {
                property,
                computed: false,
                ..
            } = &cursor.kind
            {
                let at = self.first(self.arena.expr(*property).range.start.offset);
                let Some(dot) = at.checked_sub(1) else {
                    return;
                };
                if !matches!(toks[dot].kind, TokenKind::Dot | TokenKind::QuestionDot) {
                    return;
                }
                dots.push(dot);
            }
            let Some(next) = descend(&cursor.kind) else {
                break;
            };
            cursor = self.arena.expr(next);
        }
        if calls < 3 || dots.len() < 2 {
            return;
        }
        dots.reverse();
        let start = self.first(root.range.start.offset);
        let end = self.last(root.range.end.offset);
        if dots[0] <= start || dots.iter().any(|dot| *dot >= end) {
            return;
        }
        shapes.chains.entry(start).or_default().push(Chain {
            end,
            kind: ChainKind::Members(dots),
        });
    }

    fn arrow(&self, node: &ExprNode, body: &ArrowBody, shapes: &mut Shapes) {
        let ArrowBody::Expr(body) = body else {
            return;
        };
        let body = self.arena.expr(*body);
        if !wraps_after_arrow(&body.kind) {
            return;
        }
        let toks = &self.stream.toks;
        let mut index = self.first(node.range.start.offset);
        let limit = self.first(body.range.start.offset);
        while index < limit {
            if toks[index].kind == TokenKind::FatArrow {
                let end = self.last(body.range.end.offset);
                if end > index {
                    shapes.assigns.insert(index, end);
                }
                return;
            }
            index = match self.stream.close_of[index] {
                Some(close) if crate::tokens::is_open(toks[index].kind) => close + 1,
                _ => index + 1,
            };
        }
    }

    fn operators(&self, root: &ExprNode, class: u32, shapes: &mut Shapes) {
        let mut operators = Vec::new();
        let mut cursor = root;
        while let Some(left) = left_of(&cursor.kind) {
            let child = self.arena.expr(left);
            if let Some(index) = self.operator_after(child.range.end.offset) {
                operators.push(index);
            }
            if class_of(&child.kind) == Some(class) {
                cursor = child;
            } else {
                break;
            }
        }
        operators.reverse();
        let start = self.first(root.range.start.offset);
        let end = self.last(root.range.end.offset);
        if operators.is_empty() || operators.iter().any(|op| *op <= start || *op >= end) {
            return;
        }
        let kind = if matches!(root.kind, ExprKind::Pipeline { .. }) {
            ChainKind::Pipeline(operators)
        } else {
            ChainKind::Operators(operators)
        };
        shapes
            .chains
            .entry(start)
            .or_default()
            .push(Chain { end, kind });
    }

    fn ends_with_type(&self, id: vn_core::ast::ExprId) -> bool {
        match &self.arena.expr(id).kind {
            ExprKind::Is { .. } | ExprKind::As { .. } | ExprKind::Satisfies { .. } => true,
            ExprKind::Binary { right, .. }
            | ExprKind::Logical { right, .. }
            | ExprKind::Pipeline { right, .. } => self.ends_with_type(*right),
            ExprKind::IntLiteral { .. }
            | ExprKind::FloatLiteral { .. }
            | ExprKind::BigIntLiteral { .. }
            | ExprKind::DecimalLiteral { .. }
            | ExprKind::StrLiteral { .. }
            | ExprKind::CharLiteral { .. }
            | ExprKind::BoolLiteral { .. }
            | ExprKind::NullLiteral
            | ExprKind::Template { .. }
            | ExprKind::TaggedTemplate { .. }
            | ExprKind::Identifier { .. }
            | ExprKind::Missing
            | ExprKind::This
            | ExprKind::Super
            | ExprKind::ImportMeta(_)
            | ExprKind::Array { .. }
            | ExprKind::Object { .. }
            | ExprKind::Tuple { .. }
            | ExprKind::Unary { .. }
            | ExprKind::Update { .. }
            | ExprKind::Assign { .. }
            | ExprKind::Conditional { .. }
            | ExprKind::Member { .. }
            | ExprKind::Call { .. }
            | ExprKind::New { .. }
            | ExprKind::Function { .. }
            | ExprKind::Arrow { .. }
            | ExprKind::Sequence { .. }
            | ExprKind::Paren { .. }
            | ExprKind::Await { .. }
            | ExprKind::Yield { .. }
            | ExprKind::Spread { .. }
            | ExprKind::Range { .. }
            | ExprKind::NonNull { .. }
            | ExprKind::Try { .. }
            | ExprKind::ClassExpr { .. }
            | ExprKind::Match { .. }
            | ExprKind::With { .. }
            | ExprKind::MetaAccess { .. } => false,
        }
    }

    fn ternary(
        &self,
        node: &ExprNode,
        test: vn_core::ast::ExprId,
        consequent: vn_core::ast::ExprId,
        shapes: &mut Shapes,
    ) {
        let toks = &self.stream.toks;
        let mut found = Vec::new();
        for (offset, kind) in [
            (self.arena.expr(test).range.end.offset, TokenKind::Question),
            (
                self.arena.expr(consequent).range.end.offset,
                TokenKind::Colon,
            ),
        ] {
            let from = self.first(offset);
            if let Some(index) = (from..toks.len()).take(4).find(|i| toks[*i].kind == kind) {
                shapes.ternary.insert(index);
                found.push(index);
            }
        }
        let start = self.first(node.range.start.offset);
        let end = self.last(node.range.end.offset);
        if let [question, colon] = found[..]
            && start < question
            && question < colon
            && colon < end
        {
            shapes.chains.entry(start).or_default().push(Chain {
                end,
                kind: ChainKind::Ternary {
                    question,
                    colon,
                    trailing_question: self.ends_with_type(test),
                },
            });
        }
    }
}
