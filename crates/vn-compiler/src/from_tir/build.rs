use std::sync::Arc;
use vn_tir::{BackendTy, DynReason};
use vn_types::ssa::SsaOp;

use crate::ssa::ir::Value;
use vn_tir::{TirExpr, TirExprKind};

mod assign;
mod call;
mod checked_values;
mod class_def;
mod classdef_methods;
mod conform;
mod construct;
mod context;
mod exports;
mod expr_access;
mod expr_arith;
mod expr_collect;
mod expr_invoke;
mod expr_lit;
mod functions;
mod globals;
mod infra;
mod inline_body;
mod inline_method;
mod ops;
mod stmt;

pub(super) use context::{Builder, Result};

impl<'m> Builder<'m> {
    fn lower_expr(&mut self, e: &TirExpr) -> Result<Value> {
        let ty = e.ty.representation();
        match &e.kind {
            TirExprKind::IntLit(n) => Ok(self.emit(SsaOp::ConstInt(*n), BackendTy::INT)),
            TirExprKind::FloatLit(f) => Ok(self.emit(SsaOp::ConstFloat(*f), BackendTy::FLOAT)),
            TirExprKind::BoolLit(b) => Ok(self.emit(SsaOp::ConstBool(*b), BackendTy::Bool)),
            TirExprKind::StrLit(s) => Ok(self.emit(SsaOp::ConstStr(s.clone()), BackendTy::Str)),

            TirExprKind::CharLit(c) => Ok(self.emit(SsaOp::ConstChar(*c), BackendTy::Char)),
            TirExprKind::NullLit => {
                Ok(self.emit(SsaOp::ConstNull, BackendTy::Dynamic(DynReason::Boxed)))
            }
            TirExprKind::DecimalLit(s) => {
                let d = s.parse().unwrap_or_default();
                Ok(self.emit(SsaOp::ConstDecimal(d), BackendTy::Dynamic(DynReason::Boxed)))
            }
            TirExprKind::BigIntLit(n) => Ok(self.emit(
                SsaOp::ConstBigInt(n.clone()),
                BackendTy::Dynamic(DynReason::Boxed),
            )),
            TirExprKind::RangeLit {
                start,
                end,
                inclusive,
            } => self.lower_range_lit(start, end, *inclusive),
            TirExprKind::ObjectRest { object, skip_keys } => {
                self.lower_object_rest(object, skip_keys)
            }
            TirExprKind::ExtensionCall { func, recv, args } => {
                self.lower_extension_call(func, recv, args, ty)
            }

            TirExprKind::Var => self.lower_var(&e.res, ty),

            TirExprKind::Binary { op, lhs, rhs } => self.lower_binary(*op, lhs, rhs, ty),
            TirExprKind::Unary { op, operand } => self.lower_unary(*op, operand, ty),
            TirExprKind::Cast { operand } => self.lower_cast(operand, e.ty, ty),
            TirExprKind::Checked { operand, kind } => self.lower_checked(operand, e.ty, ty, *kind),
            TirExprKind::Select {
                cond,
                then_val,
                else_val,
            } => {
                let c = self.lower_expr(cond)?;
                self.lower_select(c, then_val, else_val, ty)
            }
            TirExprKind::Seq { stmts, value } => self.lower_seq(stmts, value),

            TirExprKind::Field { object, name } => self.lower_field(object, name, &e.res, ty),
            TirExprKind::Index { object, index } => self.lower_index(object, index, ty),

            TirExprKind::Call { callee, args } => self.lower_call_expr(callee, args, &e.res, ty),
            TirExprKind::MethodCall { recv, name, args } => {
                self.lower_method_call(recv, name, args, &e.res, ty)
            }
            TirExprKind::New { class, args } => self.lower_new_expr(*class, args, ty),
            TirExprKind::MakeVariant { args } => self.lower_make_variant(args, &e.res, ty),

            TirExprKind::ArrayLit(els) => self.lower_array_lit(els, ty),
            TirExprKind::TupleLit(xs) => self.lower_tuple_lit(xs, ty),
            TirExprKind::ObjectLit { entries } => self.lower_object_lit(entries, ty),

            TirExprKind::Assign { target, value } => self.lower_assign_expr(target, value),

            TirExprKind::Await { future } => {
                let v = self.lower_expr(future)?;
                Ok(self.emit(SsaOp::Await { operand: v }, ty))
            }
            TirExprKind::Yield { value, .. } => {
                let v = match value {
                    Some(e) => self.lower_expr(e)?,
                    None => self.emit(SsaOp::ConstNull, BackendTy::Dynamic(DynReason::Boxed)),
                };
                Ok(self.emit(SsaOp::Yield { operand: v }, ty))
            }

            TirExprKind::Discriminant { value } => {
                let v = self.lower_expr(value)?;
                Ok(self.emit(SsaOp::GetEnumTag { operand: v }, BackendTy::INT))
            }
            TirExprKind::VariantPayload { value, field, .. } => {
                let v = self.lower_expr(value)?;
                Ok(self.emit(
                    SsaOp::GetFixedField {
                        object: v,
                        slot: *field,
                        offset: 0,
                        access: vn_core::FieldAccess::Slot,
                    },
                    ty,
                ))
            }
            TirExprKind::TypeTest { value, class } => {
                let v = self.lower_expr(value)?;
                let cname = self
                    .tir
                    .class(*class)
                    .map(|ci| ci.name.clone())
                    .unwrap_or_else(|| Arc::from("?"));
                let cls = self.emit(
                    self.global_load(&cname),
                    BackendTy::Dynamic(DynReason::Runtime),
                );
                Ok(self.emit(
                    SsaOp::TypeTest {
                        value: v,
                        class: cls,
                    },
                    BackendTy::Bool,
                ))
            }
            TirExprKind::DynamicTypeTest { value, class } => {
                let v = self.lower_expr(value)?;
                let cls = self.lower_expr(class)?;
                Ok(self.emit(
                    SsaOp::TypeTest {
                        value: v,
                        class: cls,
                    },
                    BackendTy::Bool,
                ))
            }

            TirExprKind::Length { operand } => self.lower_length(operand, ty),

            TirExprKind::ObjectKeys { operand } => {
                let o = self.lower_expr(operand)?;
                Ok(self.emit(SsaOp::ObjectKeys { operand: o }, ty))
            }

            TirExprKind::SuperCall { args }
                if args.iter().any(|a| matches!(a, vn_tir::TirArg::Spread(_))) =>
            {
                if let Some(slot) = self.super_method_slot(vn_core::TokenKind::Constructor.as_str())
                {
                    let ctor = self.emit(
                        SsaOp::GetSuperSlot { slot },
                        BackendTy::Dynamic(DynReason::Runtime),
                    );
                    return self.lower_call(ctor, args, ty);
                }
                if self.super_chain_ctorless() {
                    self.lower_effects(args)?;
                    return Ok(self.emit(SsaOp::ConstNull, ty));
                }
                let ctor = self.emit(
                    SsaOp::GetSuper {
                        name: Arc::from(vn_core::TokenKind::Constructor.as_str()),
                    },
                    BackendTy::Dynamic(DynReason::Runtime),
                );
                self.lower_call(ctor, args, ty)
            }
            TirExprKind::SuperCall { args } => {
                if !args.iter().any(|a| matches!(a, vn_tir::TirArg::Spread(_)))
                    && let Some(slot) =
                        self.super_method_slot(vn_core::TokenKind::Constructor.as_str())
                {
                    let argv = self.lower_args(args)?;
                    return Ok(self.emit(SsaOp::SuperInvokeSlot { slot, args: argv }, ty));
                }
                if self.super_chain_ctorless() {
                    self.lower_effects(args)?;
                    return Ok(self.emit(SsaOp::ConstNull, ty));
                }
                let argv = self.lower_args(args)?;
                Ok(self.emit(SsaOp::SuperCall { args: argv }, ty))
            }
            TirExprKind::SuperMethodCall { name, args }
                if args.iter().any(|a| matches!(a, vn_tir::TirArg::Spread(_))) =>
            {
                if let Some(slot) = self.super_method_slot(name) {
                    let method = self.emit(
                        SsaOp::GetSuperSlot { slot },
                        BackendTy::Dynamic(DynReason::Runtime),
                    );
                    return self.lower_call(method, args, ty);
                }
                let method = self.emit(
                    SsaOp::GetSuper { name: name.clone() },
                    BackendTy::Dynamic(DynReason::Runtime),
                );
                self.lower_call(method, args, ty)
            }
            TirExprKind::SuperMethodCall { name, args } => {
                if !args.iter().any(|a| matches!(a, vn_tir::TirArg::Spread(_)))
                    && let Some(slot) = self.super_method_slot(name)
                {
                    let argv = self.lower_args(args)?;
                    return Ok(self.emit(SsaOp::SuperInvokeSlot { slot, args: argv }, ty));
                }
                let argv = self.lower_args(args)?;
                Ok(self.emit(
                    SsaOp::SuperMethodCall {
                        name: name.clone(),
                        args: argv,
                    },
                    ty,
                ))
            }

            TirExprKind::SuperField { name } => {
                if let Some(slot) = self.super_method_slot(name) {
                    return Ok(self.emit(SsaOp::GetSuperSlot { slot }, ty));
                }
                Ok(self.emit(SsaOp::GetSuper { name: name.clone() }, ty))
            }

            TirExprKind::Closure { func, upvalues } => {
                let upvalues = upvalues
                    .iter()
                    .map(|src| match src {
                        vn_tir::TirUpvalue::ParentLocal(id) => {
                            vn_types::ssa::SsaUpvalue::Captured(crate::ssa::ir::SsaVar::Local(*id))
                        }
                        vn_tir::TirUpvalue::ParentParam(i) => {
                            vn_types::ssa::SsaUpvalue::Captured(crate::ssa::ir::SsaVar::Param(*i))
                        }
                        vn_tir::TirUpvalue::ParentUpvalue(idx) => {
                            vn_types::ssa::SsaUpvalue::Inherited(*idx)
                        }
                    })
                    .collect();
                Ok(self.emit(
                    SsaOp::MakeClosure {
                        func: func.0,
                        upvalues,
                    },
                    BackendTy::Dynamic(DynReason::Runtime),
                ))
            }
        }
    }
}

pub(crate) use functions::defaulted_param_mask;
pub use functions::{build_function, build_module, build_top_level};
