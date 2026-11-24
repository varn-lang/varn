use std::sync::Arc;
use vn_types::ssa::SsaOp;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::ssa::ir::{SsaFunc, Terminator, Value};
use crate::ssa::verify::inst_uses;

pub fn run(func: &mut SsaFunc) -> bool {
    let mut const_ints: FxHashMap<u32, i64> = FxHashMap::default();

    let mut obj_literals: FxHashMap<u32, Vec<(Arc<str>, Value)>> = FxHashMap::default();

    let mut tuple_literals: FxHashMap<u32, Vec<Value>> = FxHashMap::default();

    let mut array_literals: FxHashMap<u32, Vec<Value>> = FxHashMap::default();

    for block in &func.blocks {
        for inst in &block.insts {
            if let Some(d) = inst.dest {
                match &inst.kind {
                    SsaOp::ConstInt(n) => {
                        const_ints.insert(d.0, *n);
                    }
                    SsaOp::BuildObject { pairs } => {
                        let mut seen: Vec<_> = pairs.iter().map(|(k, _)| k.clone()).collect();
                        seen.sort();
                        seen.dedup();
                        if seen.len() == pairs.len() {
                            obj_literals.insert(d.0, pairs.clone());
                        }
                    }
                    SsaOp::BuildTuple { elements } => {
                        tuple_literals.insert(d.0, elements.clone());
                    }
                    SsaOp::BuildArray { elements, .. } => {
                        array_literals.insert(d.0, elements.clone());
                    }
                    SsaOp::ConstFloat(_)
                    | SsaOp::ConstBool(_)
                    | SsaOp::ConstStr(_)
                    | SsaOp::ConstChar(_)
                    | SsaOp::ConstDecimal(_)
                    | SsaOp::ConstBigInt(_)
                    | SsaOp::ConstNull
                    | SsaOp::Binary { .. }
                    | SsaOp::Unary { .. }
                    | SsaOp::KindOf { .. }
                    | SsaOp::LoadGlobal(_)
                    | SsaOp::LoadGlobalIdx(_)
                    | SsaOp::LoadNativeGlobalIdx(_)
                    | SsaOp::LoadUpvalue(_)
                    | SsaOp::StoreGlobal { .. }
                    | SsaOp::StoreGlobalIdx { .. }
                    | SsaOp::StoreUpvalue { .. }
                    | SsaOp::Call { .. }
                    | SsaOp::CallDirect { .. }
                    | SsaOp::AllocInstance { .. }
                    | SsaOp::SelfCall { .. }
                    | SsaOp::GetProperty { .. }
                    | SsaOp::GetSlotMethod { .. }
                    | SsaOp::GetFixedField { .. }
                    | SsaOp::GetIndex { .. }
                    | SsaOp::ArrayGetIndex { .. }
                    | SsaOp::BytesGetIndex { .. }
                    | SsaOp::SetProperty { .. }
                    | SsaOp::SetFixedField { .. }
                    | SsaOp::SetIndex { .. }
                    | SsaOp::ArraySetIndex { .. }
                    | SsaOp::BytesSetIndex { .. }
                    | SsaOp::ArrayPush { .. }
                    | SsaOp::ObjectMerge { .. }
                    | SsaOp::MethodCall { .. }
                    | SsaOp::InvokeSlot { .. }
                    | SsaOp::IsNull { .. }
                    | SsaOp::TypeTest { .. }
                    | SsaOp::Cast { .. }
                    | SsaOp::Convert { .. }
                    | SsaOp::BuildMap { .. }
                    | SsaOp::ObjectRest { .. }
                    | SsaOp::ToString { .. }
                    | SsaOp::BuildStr { .. }
                    | SsaOp::MakeClosure { .. }
                    | SsaOp::LoadCaptured { .. }
                    | SsaOp::StoreCaptured { .. }
                    | SsaOp::MakeClass { .. }
                    | SsaOp::DeclareLayout { .. }
                    | SsaOp::FreezeClass { .. }
                    | SsaOp::DefineMember { .. }
                    | SsaOp::MakeEnumVariant { .. }
                    | SsaOp::Try { .. }
                    | SsaOp::PopTry
                    | SsaOp::CatchParam { .. }
                    | SsaOp::CloseUpvalues { .. }
                    | SsaOp::LoadModule { .. }
                    | SsaOp::StoreModuleSlot { .. }
                    | SsaOp::Await { .. }
                    | SsaOp::Spawn { .. }
                    | SsaOp::Yield { .. }
                    | SsaOp::IntrinsicCall { .. }
                    | SsaOp::CallNativeOp { .. }
                    | SsaOp::AssertNotNull { .. }
                    | SsaOp::GetPropertyMaybe { .. }
                    | SsaOp::ModuleSlot { .. }
                    | SsaOp::GetEnumTag { .. }
                    | SsaOp::IsArray { .. }
                    | SsaOp::StrLength { .. }
                    | SsaOp::ArrayLength { .. }
                    | SsaOp::BytesLength { .. }
                    | SsaOp::This
                    | SsaOp::Range { .. }
                    | SsaOp::ObjectKeys { .. }
                    | SsaOp::GetSuper { .. }
                    | SsaOp::GetSuperSlot { .. }
                    | SsaOp::SuperCall { .. }
                    | SsaOp::SuperMethodCall { .. }
                    | SsaOp::SuperInvokeSlot { .. }
                    | SsaOp::ExtensionCall { .. }
                    | SsaOp::CallSpread { .. }
                    | SsaOp::BuildArraySpread { .. }
                    | SsaOp::BuildObjectSpread { .. } => {}
                }
            }
        }
    }

    if obj_literals.is_empty() && tuple_literals.is_empty() && array_literals.is_empty() {
        return false;
    }

    for block in &func.blocks {
        for inst in &block.insts {
            if let SsaOp::GetProperty { object, .. } = &inst.kind
                && obj_literals.contains_key(&object.0)
            {
                continue;
            }
            if let SsaOp::GetIndex { object, index } | SsaOp::ArrayGetIndex { object, index } =
                &inst.kind
            {
                let elems = tuple_literals
                    .get(&object.0)
                    .or_else(|| array_literals.get(&object.0));
                if let (Some(elems), Some(&idx)) = (elems, const_ints.get(&index.0))
                    && idx >= 0
                    && (idx as usize) < elems.len()
                {
                    continue;
                }
            }

            for u in inst_uses(&inst.kind) {
                obj_literals.remove(&u.0);
                tuple_literals.remove(&u.0);
                array_literals.remove(&u.0);
            }
        }
        match &block.term {
            Terminator::Return(Some(v)) | Terminator::Throw(v) => {
                obj_literals.remove(&v.0);
                tuple_literals.remove(&v.0);
                array_literals.remove(&v.0);
            }
            Terminator::Branch {
                cond,
                then_args,
                else_args,
                ..
            } => {
                obj_literals.remove(&cond.0);
                tuple_literals.remove(&cond.0);
                array_literals.remove(&cond.0);
                for a in then_args.iter().chain(else_args) {
                    obj_literals.remove(&a.0);
                    tuple_literals.remove(&a.0);
                    array_literals.remove(&a.0);
                }
            }
            Terminator::Jump { args, .. } => {
                for a in args {
                    obj_literals.remove(&a.0);
                    tuple_literals.remove(&a.0);
                    array_literals.remove(&a.0);
                }
            }
            Terminator::Return(_) | Terminator::Unreachable => {}
        }
    }

    if obj_literals.is_empty() && tuple_literals.is_empty() && array_literals.is_empty() {
        return false;
    }

    let mut forwards: Vec<(Value, Value)> = Vec::new();
    let mut dead_reads: FxHashSet<Value> = FxHashSet::default();
    for block in &func.blocks {
        for inst in &block.insts {
            if let (Some(dest), SsaOp::GetProperty { object, name }) = (inst.dest, &inst.kind)
                && let Some(pairs) = obj_literals.get(&object.0)
                && let Some((_, v)) = pairs.iter().find(|(k, _)| k == name)
            {
                forwards.push((dest, *v));
                dead_reads.insert(dest);
            }
            if let (
                Some(dest),
                SsaOp::GetIndex { object, index } | SsaOp::ArrayGetIndex { object, index },
            ) = (inst.dest, &inst.kind)
            {
                if let Some(elems) = tuple_literals.get(&object.0)
                    && let Some(&idx) = const_ints.get(&index.0)
                    && idx >= 0
                    && (idx as usize) < elems.len()
                {
                    forwards.push((dest, elems[idx as usize]));
                }
                if let Some(elems) = array_literals.get(&object.0)
                    && let Some(&idx) = const_ints.get(&index.0)
                    && idx >= 0
                    && (idx as usize) < elems.len()
                {
                    let elem = elems[idx as usize];
                    if func.value_ty(elem) == func.value_ty(dest) {
                        forwards.push((dest, elem));
                        dead_reads.insert(dest);
                    }
                }
            }
        }
    }

    let changed = !forwards.is_empty();

    let mut fwd_map: FxHashMap<Value, Value> = forwards.into_iter().collect();
    let keys: Vec<Value> = fwd_map.keys().copied().collect();
    for k in keys {
        let mut v = fwd_map[&k];
        loop {
            match fwd_map.get(&v) {
                Some(&w) if w != v => v = w,
                _ => break,
            }
        }
        fwd_map.insert(k, v);
    }
    for (dest, value) in fwd_map {
        func.replace_all_uses(dest, value);
    }
    if !dead_reads.is_empty() {
        for block in &mut func.blocks {
            block
                .insts
                .retain(|inst| inst.dest.is_none_or(|d| !dead_reads.contains(&d)));
        }
    }
    changed
}
