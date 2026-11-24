use super::context::{Builder, Result};
use crate::ssa::ir::Value;
use std::sync::Arc;
use vn_tir::TirClassDef;
use vn_tir::{BackendTy, DynReason};
use vn_types::ssa::SsaOp;

impl<'m> Builder<'m> {
    fn apply_member_decorator(
        &mut self,
        mv: Value,
        deco: &vn_tir::TirExpr,
        key: &Arc<str>,
        is_static: bool,
        is_private: bool,
    ) -> Result<Value> {
        let deco_v = self.lower_expr(deco)?;
        let n = self.emit(SsaOp::ConstStr(key.clone()), BackendTy::Str);
        let st = self.emit(SsaOp::ConstBool(is_static), BackendTy::Bool);
        let pv = self.emit(SsaOp::ConstBool(is_private), BackendTy::Bool);
        let ctx = self.emit(
            SsaOp::BuildObject {
                pairs: vec![
                    (Arc::from(vn_core::MemberKey::Name.name()), n),
                    (Arc::from(vn_core::MemberKey::IsStatic.name()), st),
                    (Arc::from(vn_core::MemberKey::IsPrivate.name()), pv),
                ],
            },
            BackendTy::Dynamic(DynReason::Runtime),
        );
        let result = self.emit(
            SsaOp::Call {
                callee: deco_v,
                args: vec![mv, ctx],
                shape: None,
            },
            BackendTy::Dynamic(DynReason::Runtime),
        );
        let isnull = self.emit(SsaOp::IsNull { operand: result }, BackendTy::Bool);
        self.select_value(isnull, mv, result, BackendTy::Dynamic(DynReason::Runtime))
    }

    pub(super) fn build_class_methods(&mut self, def: &TirClassDef, class_v: Value) -> Result<()> {
        for m in &def.methods {
            let mut mv = self.emit(
                SsaOp::MakeClosure {
                    func: m.func.0,
                    upvalues: vec![],
                },
                BackendTy::Dynamic(DynReason::Runtime),
            );
            for deco in m.decorators.iter().rev() {
                mv = self.apply_member_decorator(mv, deco, &m.key, m.is_static, m.is_private)?;
            }
            self.emit_effect(SsaOp::DefineMember {
                class: class_v,
                name: m.key.clone(),
                member: mv,
                kind: vn_core::MemberDef::Method {
                    is_static: m.is_static,
                },
            });
        }
        Ok(())
    }

    pub(super) fn build_class_decorators(
        &mut self,
        def: &TirClassDef,
        class_v: Value,
    ) -> Result<Value> {
        let mut class_v = class_v;
        for deco in def.decorators.iter().rev() {
            let deco_v = self.lower_expr(deco)?;
            let result = self.emit(
                SsaOp::Call {
                    callee: deco_v,
                    args: vec![class_v],
                    shape: None,
                },
                BackendTy::Dynamic(DynReason::Runtime),
            );
            let isnull = self.emit(SsaOp::IsNull { operand: result }, BackendTy::Bool);
            class_v = self.select_value(
                isnull,
                class_v,
                result,
                BackendTy::Dynamic(DynReason::Runtime),
            )?;
        }
        Ok(class_v)
    }

    pub(super) fn build_class_accessors(
        &mut self,
        def: &TirClassDef,
        class_v: Value,
    ) -> Result<()> {
        for a in &def.accessors {
            let mut av = self.emit(
                SsaOp::MakeClosure {
                    func: a.func.0,
                    upvalues: vec![],
                },
                BackendTy::Dynamic(DynReason::Runtime),
            );
            for deco in a.decorators.iter().rev() {
                av = self.apply_member_decorator(av, deco, &a.key, a.is_static, false)?;
            }
            self.emit_effect(SsaOp::DefineMember {
                class: class_v,
                name: a.key.clone(),
                member: av,
                kind: vn_core::MemberDef::Accessor {
                    kind: if a.is_getter {
                        vn_core::AccessorKind::Get
                    } else {
                        vn_core::AccessorKind::Set
                    },
                    is_static: a.is_static,
                },
            });
        }
        Ok(())
    }

    pub(super) fn build_property_decorators(
        &mut self,
        def: &TirClassDef,
        class_v: Value,
    ) -> Result<()> {
        for prop in &def.property_decorators {
            for deco in prop.decorators.iter().rev() {
                let deco_v = self.lower_expr(deco)?;
                let n = self.emit(SsaOp::ConstStr(prop.key.clone()), BackendTy::Str);
                let st = self.emit(SsaOp::ConstBool(prop.is_static), BackendTy::Bool);
                let pv = self.emit(SsaOp::ConstBool(false), BackendTy::Bool);
                let ctx = self.emit(
                    SsaOp::BuildObject {
                        pairs: vec![
                            (Arc::from(vn_core::MemberKey::Name.name()), n),
                            (Arc::from(vn_core::MemberKey::IsStatic.name()), st),
                            (Arc::from(vn_core::MemberKey::IsPrivate.name()), pv),
                        ],
                    },
                    BackendTy::Dynamic(DynReason::Runtime),
                );
                self.emit(
                    SsaOp::Call {
                        callee: deco_v,
                        args: vec![class_v, ctx],
                        shape: None,
                    },
                    BackendTy::Dynamic(DynReason::Runtime),
                );
            }
        }
        Ok(())
    }
}
