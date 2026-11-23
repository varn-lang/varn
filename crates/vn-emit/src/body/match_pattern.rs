use super::context::FnEmitter;
use super::small_utils::bool_lit;
use crate::ty::NameResolver;
use std::sync::Arc;
use vn_core::ast::pattern::{MatchField, MatchPattern, SequenceRest, VariantPayload};
use vn_core::BinOp;
use vn_tir::{BackendTy, DynReason, LocalId, Resolution, TirExpr, TirExprKind, TirStmt, TirUnOp};

#[derive(Clone, Copy)]
pub(super) enum MatchDest {
    Statement,
    Return,
    Assign(LocalId),
}

impl<'a> FnEmitter<'a> {
    pub(super) fn match_pattern(
        &mut self,
        s: &TirExpr,
        pat: &MatchPattern,
    ) -> (TirExpr, Vec<TirStmt>) {
        match pat {
            MatchPattern::Wildcard => (bool_lit(true), vec![]),
            MatchPattern::Identifier(name) => {
                let name_str = self.m.interner.resolve(*name);
                if let BackendTy::Enum(eid) = s.ty.non_nullable(self.tt)
                    && let Some(info) = self.m.enums.get(eid.0 as usize)
                    && info.variants.iter().any(|v| v.name.as_ref() == name_str)
                {
                    return self.match_enum_variant(
                        s,
                        None,
                        name_str,
                        &VariantPayload::Positional(Vec::new()),
                    );
                }
                let local = self.bind_local(Arc::from(name_str), s.ty);
                (
                    bool_lit(true),
                    vec![TirStmt::Let {
                        local,
                        ty: s.ty,
                        init: Some(s.clone()),
                    }],
                )
            }
            MatchPattern::Literal(lit) => {
                let l = self.lower_expr(*lit);
                (self.equality(BinOp::Eq, s.clone(), l, s.span), vec![])
            }
            MatchPattern::Tuple(items) => {
                let mut cond = self.present_guard(s);
                let mut binds = Vec::new();
                for (at, item) in items.iter().enumerate() {
                    let element = self.element_read(s, at);
                    let (test, mut bound) = self.match_pattern(&element, item);
                    cond = self.and_cond(cond, test);
                    binds.append(&mut bound);
                }
                (cond, binds)
            }
            MatchPattern::Array { elements, rest } => {
                let mut cond = self.present_guard(s);
                let length = self.field_access(
                    s.clone(),
                    Arc::from(vn_core::MemberKey::Length.name()),
                    BackendTy::INT,
                    s.span,
                );
                let op = match rest {
                    SequenceRest::Closed => BinOp::Eq,
                    SequenceRest::Open | SequenceRest::Bind(_) => BinOp::Ge,
                };
                let length_test = TirExpr {
                    kind: TirExprKind::Binary {
                        op,
                        lhs: Box::new(length),
                        rhs: Box::new(super::small_utils::int_lit(elements.len() as i64)),
                    },
                    ty: BackendTy::Bool,
                    res: Resolution::None,
                    span: s.span,
                };
                cond = self.and_cond(cond, length_test);
                let mut binds = Vec::new();
                for (at, item) in elements.iter().enumerate() {
                    let element = self.element_read(s, at);
                    let (test, mut bound) = self.match_pattern(&element, item);
                    cond = self.and_cond(cond, test);
                    binds.append(&mut bound);
                }
                if let SequenceRest::Bind(name) = rest
                    && self.m.interner.resolve(*name) != vn_core::TokenKind::Placeholder.as_str()
                {
                    let tail = self.tail_read(s, elements.len());
                    let local = self.bind_local(Arc::from(self.m.interner.resolve(*name)), tail.ty);
                    binds.push(TirStmt::Let {
                        local,
                        ty: tail.ty,
                        init: Some(tail),
                    });
                }
                (cond, binds)
            }
            MatchPattern::Record { fields, .. } => {
                let mut cond = self.present_guard(s);
                let mut binds = Vec::new();
                for field in fields {
                    let value = self.field_read(s, Arc::from(self.m.interner.resolve(field.name)));
                    let (test, mut bound) = self.match_field(&value, field);
                    cond = self.and_cond(cond, test);
                    binds.append(&mut bound);
                }
                (cond, binds)
            }
            MatchPattern::EnumVariant {
                enum_name,
                variant_name,
                payload,
            } => {
                let enum_name = enum_name.map(|n| self.m.interner.resolve(n));
                let variant_name = self.m.interner.resolve(*variant_name);
                self.match_enum_variant(s, enum_name, variant_name, payload)
            }
        }
    }

    fn match_field(&mut self, value: &TirExpr, field: &MatchField) -> (TirExpr, Vec<TirStmt>) {
        match &field.pattern {
            Some(sub) => self.match_pattern(value, sub),
            None => {
                let name = self.m.interner.resolve(field.name);
                if name == vn_core::TokenKind::Placeholder.as_str() {
                    return (bool_lit(true), vec![]);
                }
                let local = self.bind_local(Arc::from(name), value.ty);
                (
                    bool_lit(true),
                    vec![TirStmt::Let {
                        local,
                        ty: value.ty,
                        init: Some(value.clone()),
                    }],
                )
            }
        }
    }

    fn present_guard(&mut self, s: &TirExpr) -> TirExpr {
        if !matches!(s.ty, BackendTy::Nullable(_)) {
            return bool_lit(true);
        }
        let is_null = TirExpr {
            kind: TirExprKind::Unary {
                op: TirUnOp::IsNull,
                operand: Box::new(s.clone()),
            },
            ty: BackendTy::Bool,
            res: Resolution::None,
            span: s.span,
        };
        TirExpr {
            kind: TirExprKind::Unary {
                op: TirUnOp::Scalar(vn_core::UnOp::Not),
                operand: Box::new(is_null),
            },
            ty: BackendTy::Bool,
            res: Resolution::None,
            span: s.span,
        }
    }

    fn and_cond(&mut self, left: TirExpr, right: TirExpr) -> TirExpr {
        if matches!(left.kind, TirExprKind::BoolLit(true)) {
            return right;
        }
        if matches!(right.kind, TirExprKind::BoolLit(true)) {
            return left;
        }
        let span = left.span;
        TirExpr {
            kind: TirExprKind::Select {
                cond: Box::new(left),
                then_val: Box::new(right),
                else_val: Box::new(bool_lit(false)),
            },
            ty: BackendTy::Bool,
            res: Resolution::None,
            span,
        }
    }

    pub(super) fn match_enum_variant(
        &mut self,
        s: &TirExpr,
        enum_name: Option<&str>,
        variant_name: &str,
        payload_pattern: &VariantPayload,
    ) -> (TirExpr, Vec<TirStmt>) {
        let eid = enum_name.and_then(|n| self.m.names.enum_id(n)).or_else(|| {
            match s.ty.non_nullable(self.tt) {
                BackendTy::Enum(e) => Some(e),
                BackendTy::Literal(_)
                | BackendTy::Num(_)
                | BackendTy::Bool
                | BackendTy::Char
                | BackendTy::Str
                | BackendTy::Bytes
                | BackendTy::Decimal
                | BackendTy::BigInt
                | BackendTy::Array(_)
                | BackendTy::Map(..)
                | BackendTy::Set(_)
                | BackendTy::Tuple(_)
                | BackendTy::Class(_)
                | BackendTy::Fn(_)
                | BackendTy::Nullable(_)
                | BackendTy::Void
                | BackendTy::Never
                | BackendTy::Dynamic(_) => None,
            }
        });

        let Some((eid, variant)) = eid.and_then(|eid| {
            self.m.enums.get(eid.0 as usize).and_then(|info| {
                info.variants
                    .iter()
                    .find(|v| v.name.as_ref() == variant_name)
                    .map(|v| (eid, v.clone()))
            })
        }) else {
            return (bool_lit(false), Vec::new());
        };
        let typed_subject = matches!(s.ty.non_nullable(self.tt), BackendTy::Enum(e) if e == eid);
        let (guard, subject) = match (typed_subject, enum_name) {
            (true, _) | (false, None) => (bool_lit(true), s.clone()),
            (false, Some(name)) => {
                let test = self.type_test_of_name(s.clone(), name);
                (test, self.cast_to(s.clone(), BackendTy::Enum(eid)))
            }
        };
        let s = &subject;
        let tag = variant.tag;
        let payload: Vec<BackendTy> = variant.payload.clone();
        let field_names: Vec<Arc<str>> = variant.field_names.clone();

        let disc = TirExpr {
            kind: TirExprKind::Discriminant {
                value: Box::new(s.clone()),
            },
            ty: BackendTy::INT,
            res: Resolution::None,
            span: s.span,
        };
        let cond = TirExpr {
            kind: TirExprKind::Binary {
                op: BinOp::Eq,
                lhs: Box::new(disc),
                rhs: Box::new(TirExpr {
                    kind: TirExprKind::IntLit(tag),
                    ty: BackendTy::INT,
                    res: Resolution::None,
                    span: s.span,
                }),
            },
            ty: BackendTy::Bool,
            res: Resolution::None,
            span: s.span,
        };

        let mut cond = self.and_cond(guard, cond);
        let mut binds = Vec::new();
        let read = |at: usize| {
            let fty = payload
                .get(at)
                .copied()
                .unwrap_or(BackendTy::Dynamic(DynReason::Unrecorded));
            TirExpr {
                kind: TirExprKind::VariantPayload {
                    value: Box::new(s.clone()),
                    tag,
                    field: at as u16,
                },
                ty: fty,
                res: Resolution::EnumVariant { enum_id: eid, tag },
                span: s.span,
            }
        };
        match payload_pattern {
            VariantPayload::Positional(items) => {
                for (at, item) in items.iter().enumerate() {
                    let field = read(at);
                    let (test, mut bound) = self.match_pattern(&field, item);
                    cond = self.and_cond(cond, test);
                    binds.append(&mut bound);
                }
            }
            VariantPayload::Named { fields, .. } => {
                for field in fields {
                    let name = self.m.interner.resolve(field.name);
                    let Some(at) = field_names.iter().position(|n| n.as_ref() == name) else {
                        continue;
                    };
                    let value = read(at);
                    let (test, mut bound) = self.match_field(&value, field);
                    cond = self.and_cond(cond, test);
                    binds.append(&mut bound);
                }
            }
        }
        (cond, binds)
    }
}
