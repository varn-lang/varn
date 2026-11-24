use rustc_hash::FxHashMap;
use vn_types::ssa::SsaOp;

use super::ir::{SsaFunc, Value};

macro_rules! by_value {
    ($f:ident, $v:expr) => {
        $f(*$v)
    };
}

macro_rules! by_ref {
    ($f:ident, $v:expr) => {
        $f($v)
    };
}

macro_rules! shared {
    ($c:expr) => {
        $c.iter()
    };
}

macro_rules! exclusive {
    ($c:expr) => {
        $c.iter_mut()
    };
}

macro_rules! inst_uses {
    ($kind:expr, $f:ident, $use:ident, $iter:ident) => {{
        use SsaOp::*;
        match $kind {
            ConstInt(_)
            | ConstFloat(_)
            | ConstBool(_)
            | ConstStr(_)
            | ConstChar(_)
            | ConstDecimal(_)
            | ConstBigInt(_)
            | ConstNull
            | LoadGlobal(_)
            | LoadGlobalIdx(_)
            | LoadNativeGlobalIdx(_)
            | LoadUpvalue(_)
            | LoadCaptured { .. }
            | MakeClosure { .. }
            | MakeEnumVariant { .. }
            | Try { .. }
            | PopTry
            | CloseUpvalues { .. }
            | LoadModule { .. }
            | This
            | GetSuper { .. }
            | GetSuperSlot { .. } => {}

            Unary { operand, .. }
            | KindOf { operand }
            | IsNull { operand }
            | Cast { operand, .. }
            | Convert { operand, .. }
            | ToString { operand }
            | AssertNotNull { operand }
            | GetEnumTag { operand }
            | IsArray { operand }
            | StrLength { operand }
            | ArrayLength { operand }
            | BytesLength { operand }
            | ObjectKeys { operand }
            | Await { operand }
            | Spawn { operand }
            | Yield { operand } => $use!($f, operand),

            GetProperty { object, .. }
            | GetSlotMethod { object, .. }
            | ObjectRest { object, .. }
            | GetFixedField { object, .. }
            | GetPropertyMaybe { object, .. }
            | ModuleSlot { object, .. } => $use!($f, object),

            StoreGlobal { value, .. }
            | StoreGlobalIdx { value, .. }
            | StoreUpvalue { value, .. }
            | StoreCaptured { value, .. }
            | StoreModuleSlot { value, .. } => $use!($f, value),

            DeclareLayout { class, .. } => $use!($f, class),
            FreezeClass { class } => $use!($f, class),
            CatchParam { try_val } => $use!($f, try_val),
            MakeClass { super_class, .. } => {
                if let Some(sc) = super_class {
                    $use!($f, sc);
                }
            }

            Binary { lhs, rhs, .. } => {
                $use!($f, lhs);
                $use!($f, rhs);
            }
            TypeTest { value, class } => {
                $use!($f, value);
                $use!($f, class);
            }
            GetIndex { object, index }
            | ArrayGetIndex { object, index }
            | BytesGetIndex { object, index } => {
                $use!($f, object);
                $use!($f, index);
            }
            SetProperty { object, value, .. } | SetFixedField { object, value, .. } => {
                $use!($f, object);
                $use!($f, value);
            }
            ObjectMerge { target, source } => {
                $use!($f, target);
                $use!($f, source);
            }
            ArrayPush { array, value } => {
                $use!($f, array);
                $use!($f, value);
            }
            Range { start, end, .. } => {
                $use!($f, start);
                $use!($f, end);
            }
            DefineMember { class, member, .. } => {
                $use!($f, class);
                $use!($f, member);
            }

            SetIndex {
                object,
                index,
                value,
            }
            | ArraySetIndex {
                object,
                index,
                value,
            }
            | BytesSetIndex {
                object,
                index,
                value,
            } => {
                $use!($f, object);
                $use!($f, index);
                $use!($f, value);
            }

            SelfCall { args }
            | SuperCall { args }
            | SuperMethodCall { args, .. }
            | SuperInvokeSlot { args, .. } => $iter!(args).for_each(|v| $use!($f, v)),
            Call { callee, args, .. } => {
                $use!($f, callee);
                $iter!(args).for_each(|v| $use!($f, v));
            }
            CallDirect { callee, args, .. } => {
                $use!($f, callee);
                $iter!(args).for_each(|v| $use!($f, v));
            }
            AllocInstance { class, .. } => $use!($f, class),
            MethodCall { recv, args, .. }
            | InvokeSlot { recv, args, .. }
            | ExtensionCall { recv, args, .. } => {
                $use!($f, recv);
                $iter!(args).for_each(|v| $use!($f, v));
            }
            IntrinsicCall { object, args, .. } | CallNativeOp { object, args, .. } => {
                $use!($f, object);
                $iter!(args).for_each(|v| $use!($f, v));
            }
            BuildArray { elements, .. }
            | BuildTuple { elements }
            | BuildStr { parts: elements } => $iter!(elements).for_each(|v| $use!($f, v)),
            BuildObject { pairs } => $iter!(pairs).for_each(|(_, v)| $use!($f, v)),
            BuildMap { pairs } => $iter!(pairs).for_each(|(k, v)| {
                $use!($f, k);
                $use!($f, v);
            }),
            CallSpread { callee, args } => {
                $use!($f, callee);
                $iter!(args).for_each(|vn_types::ssa::SsaSpread { value, .. }| $use!($f, value));
            }
            BuildArraySpread { elements } => {
                $iter!(elements).for_each(|vn_types::ssa::SsaSpread { value, .. }| $use!($f, value))
            }
            BuildObjectSpread { parts } => $iter!(parts)
                .for_each(|vn_types::ssa::SsaObjectSpreadPart { value, .. }| $use!($f, value)),
        }
    }};
}

pub fn visit_uses(kind: &SsaOp, f: &mut impl FnMut(Value)) {
    inst_uses!(kind, f, by_value, shared)
}

pub fn visit_uses_mut(kind: &mut SsaOp, f: &mut impl FnMut(&mut Value)) {
    inst_uses!(kind, f, by_ref, exclusive)
}

pub fn replace_uses_with_map(func: &mut SsaFunc, map: &FxHashMap<Value, Value>) -> bool {
    if map.is_empty() {
        return false;
    }
    let mut changed = false;
    let mut sub = |v: &mut Value| {
        let mut hops = 0;
        while let Some(&next) = map.get(v) {
            if next == *v || hops > map.len() {
                break;
            }
            *v = next;
            changed = true;
            hops += 1;
        }
    };
    for block in &mut func.blocks {
        for inst in &mut block.insts {
            visit_uses_mut(&mut inst.kind, &mut sub);
        }
        block.term.for_each_use_mut(&mut sub);
    }
    changed
}
