use super::{Captured, Site};
use crate::ssa::ir::Inst;
use vn_tir::BackendTy;
use vn_types::ssa::{SsaInst, SsaOp, SsaResume, SsaUpvalue, ValueId};

pub(super) fn project_inst(
    inst: &Inst,
    value_tys: &[BackendTy],
    site: Site,
    captured: &mut Captured,
) -> Option<SsaInst> {
    if let SsaOp::Try { handler } = &inst.kind {
        let (catch_ip, live) = site.landing?;
        let catch_value = inst.dest?;
        let mut live: Vec<ValueId> = live
            .iter()
            .copied()
            .map(ValueId)
            .filter(|v| *v != catch_value)
            .collect();
        live.sort_unstable_by_key(|v| v.0);
        return Some(SsaInst {
            dest: inst.dest,
            op: SsaOp::Try { handler: *handler },
            line: inst.line,
            ip: site.next_ip,
            cache: None,
            resume: Some(SsaResume { ip: catch_ip, live }),
        });
    }
    let mut op = inst.kind.clone();
    remap_captured(&mut op, captured);
    if site.direct_slot.is_none()
        && let SsaOp::CallDirect {
            callee,
            args,
            shape,
            ..
        } = op
    {
        op = SsaOp::Call {
            callee,
            args,
            shape,
        };
    }
    specialize_index(&mut op, value_tys);
    let (cache, resume) = annotate(&op, &site)?;
    Some(SsaInst {
        dest: inst.dest,
        op,
        line: inst.line,
        ip: site.next_ip,
        cache,
        resume,
    })
}

fn remap_captured(op: &mut SsaOp, captured: &mut Captured) {
    if let SsaOp::MakeClosure { upvalues, .. } = op {
        for uv in upvalues.iter_mut() {
            if let SsaUpvalue::Captured(var) = uv {
                *var = captured.var(*var);
            }
        }
    }
    if let SsaOp::LoadCaptured { var } = op {
        *var = captured.var(*var);
    }
    if let SsaOp::StoreCaptured { var, .. } = op {
        *var = captured.var(*var);
    }
    if let SsaOp::CloseUpvalues { vars } = op {
        for var in vars.iter_mut() {
            *var = captured.var(*var);
        }
    }
}

fn specialize_index(op: &mut SsaOp, value_tys: &[BackendTy]) {
    let is_int = |v: ValueId| {
        value_tys.get(v.0 as usize).is_some_and(|t| {
            crate::ssa::emit::slot_kind_of(*t) == vn_types::register_meta::SlotKind::Int
        })
    };
    if let SsaOp::ArrayGetIndex { object, index } = &*op
        && !is_int(*index)
    {
        let (object, index) = (*object, *index);
        *op = SsaOp::GetIndex { object, index };
    }
    if let SsaOp::BytesGetIndex { object, index } = &*op
        && !is_int(*index)
    {
        let (object, index) = (*object, *index);
        *op = SsaOp::GetIndex { object, index };
    }
    if let SsaOp::ArraySetIndex {
        object,
        index,
        value,
    } = &*op
        && !is_int(*index)
    {
        let (object, index, value) = (*object, *index, *value);
        *op = SsaOp::SetIndex {
            object,
            index,
            value,
        };
    }
    if let SsaOp::BytesSetIndex {
        object,
        index,
        value,
    } = &*op
        && !is_int(*index)
    {
        let (object, index, value) = (*object, *index, *value);
        *op = SsaOp::SetIndex {
            object,
            index,
            value,
        };
    }
}

fn annotate(op: &SsaOp, site: &Site) -> Option<(Option<u16>, Option<SsaResume>)> {
    let resume = |ip: u32| SsaResume {
        ip,
        live: site.resume_live.clone(),
    };
    Some(match op {
        SsaOp::GetProperty { .. }
        | SsaOp::SetProperty { .. }
        | SsaOp::MethodCall { .. }
        | SsaOp::InvokeSlot { .. }
        | SsaOp::BuildObjectSpread { .. } => (Some(u16::from(site.ic_slot?)), None),
        SsaOp::CallDirect { .. } => (Some(u16::from(site.direct_slot?)), None),
        SsaOp::Await { .. } | SsaOp::Yield { .. } => (None, Some(resume(site.next_ip))),
        SsaOp::LoadModule { .. } => (None, Some(resume(site.own_ip))),
        SsaOp::Try { .. } => (None, None),
        SsaOp::ConstInt(..)
        | SsaOp::ConstFloat(..)
        | SsaOp::ConstBool(..)
        | SsaOp::ConstNull
        | SsaOp::ConstStr(..)
        | SsaOp::Binary { .. }
        | SsaOp::Unary { .. }
        | SsaOp::SelfCall { .. }
        | SsaOp::Call { .. }
        | SsaOp::AllocInstance { .. }
        | SsaOp::LoadGlobalIdx(..)
        | SsaOp::ArrayGetIndex { .. }
        | SsaOp::BytesGetIndex { .. }
        | SsaOp::ArraySetIndex { .. }
        | SsaOp::BytesSetIndex { .. }
        | SsaOp::ConstChar(..)
        | SsaOp::ConstBigInt(..)
        | SsaOp::ConstDecimal(..)
        | SsaOp::MakeEnumVariant { .. }
        | SsaOp::IntrinsicCall { .. }
        | SsaOp::LoadNativeGlobalIdx(..)
        | SsaOp::PopTry
        | SsaOp::CatchParam { .. }
        | SsaOp::StoreGlobalIdx { .. }
        | SsaOp::MakeClosure { .. }
        | SsaOp::LoadCaptured { .. }
        | SsaOp::StoreCaptured { .. }
        | SsaOp::LoadUpvalue(..)
        | SsaOp::StoreUpvalue { .. }
        | SsaOp::CloseUpvalues { .. }
        | SsaOp::Cast { .. }
        | SsaOp::Convert { .. }
        | SsaOp::IsNull { .. }
        | SsaOp::TypeTest { .. }
        | SsaOp::KindOf { .. }
        | SsaOp::ToString { .. }
        | SsaOp::IsArray { .. }
        | SsaOp::GetEnumTag { .. }
        | SsaOp::ObjectKeys { .. }
        | SsaOp::BuildStr { .. }
        | SsaOp::BuildArray { .. }
        | SsaOp::BuildMap { .. }
        | SsaOp::BuildObject { .. }
        | SsaOp::GetSlotMethod { .. }
        | SsaOp::GetIndex { .. }
        | SsaOp::SetIndex { .. }
        | SsaOp::ArrayLength { .. }
        | SsaOp::StrLength { .. }
        | SsaOp::BytesLength { .. }
        | SsaOp::CallNativeOp { .. }
        | SsaOp::ArrayPush { .. }
        | SsaOp::This
        | SsaOp::GetFixedField { .. }
        | SsaOp::SetFixedField { .. }
        | SsaOp::MakeClass { .. }
        | SsaOp::DeclareLayout { .. }
        | SsaOp::FreezeClass { .. }
        | SsaOp::DefineMember { .. }
        | SsaOp::GetSuper { .. }
        | SsaOp::GetSuperSlot { .. }
        | SsaOp::LoadGlobal(..)
        | SsaOp::StoreGlobal { .. }
        | SsaOp::BuildTuple { .. }
        | SsaOp::BuildArraySpread { .. }
        | SsaOp::ObjectMerge { .. }
        | SsaOp::ObjectRest { .. }
        | SsaOp::GetPropertyMaybe { .. }
        | SsaOp::AssertNotNull { .. }
        | SsaOp::Range { .. }
        | SsaOp::SuperCall { .. }
        | SsaOp::SuperMethodCall { .. }
        | SsaOp::SuperInvokeSlot { .. }
        | SsaOp::ExtensionCall { .. }
        | SsaOp::CallSpread { .. }
        | SsaOp::ModuleSlot { .. }
        | SsaOp::StoreModuleSlot { .. }
        | SsaOp::Spawn { .. } => (None, None),
    })
}
