use vn_tir::BackendTy;
mod calls;
mod composite;
mod composite_class;
mod composite_object;
mod consts;
mod control;

use super::super::ir::{Inst, InstKind};
use super::{Captured, Site};
use vn_types::ssa::{SsaInst, SsaOp, SsaResume, ValueId};

pub(super) fn project_inst(
    inst: &Inst,
    value_tys: &[BackendTy],
    site: Site,
    captured: &mut Captured,
) -> Option<SsaInst> {
    if let InstKind::Try { .. } = &inst.kind {
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
            dest: None,
            op: SsaOp::Try { catch_value },
            line: inst.line,
            ip: site.next_ip,
            cache: None,
            resume: Some(SsaResume { ip: catch_ip, live }),
        });
    }
    let kind = &inst.kind;
    let op = consts::try_project(kind)
        .or_else(|| calls::try_project(kind, &site, captured))
        .or_else(|| composite::try_project(kind, value_tys))
        .or_else(|| control::try_project(kind, value_tys))?;
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
        | SsaOp::FreezeClass { .. }
        | SsaOp::DeclareLayout { .. }
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
        | SsaOp::BindMethod { .. }
        | SsaOp::ArrayExtend { .. }
        | SsaOp::WrapSpread { .. }
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
