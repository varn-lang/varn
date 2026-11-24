use rustc_hash::FxHashMap;
use vn_types::ssa::SsaOp;

use crate::ssa::ir::{SsaFunc, Value};

use std::sync::Arc;

#[derive(PartialEq, Eq, Hash)]
enum Key {
    Int(i64),
    FloatBits(u64),
    Bool(bool),
    Str(Arc<str>),
    Char(char),
    BigInt(Arc<str>),
    Null,
    Binary(u32, u32, u32),
    Unary(u32, u32),
    KindOf(u32),
    IsNull(u32),
    IsArray(u32),
    TypeTest(u32, u32),
    StrLength(u32),
    EnumTag(u32),

    FixedField(u32, u16),
    ArrayElem(u32, u32),
    ModuleSlot(u32, u16),
    Global(Arc<str>),
    GlobalIdx(u32),
    NativeGlobalIdx(u32),
    Upvalue(u32),
}

impl Key {
    fn reads_memory(&self) -> bool {
        matches!(
            self,
            Key::FixedField(..)
                | Key::ArrayElem(..)
                | Key::ModuleSlot(..)
                | Key::Global(_)
                | Key::GlobalIdx(_)
                | Key::NativeGlobalIdx(_)
                | Key::Upvalue(_)
        )
    }
}

pub fn run(func: &mut SsaFunc) -> bool {
    crate::ssa::verify::recompute_preds(func);
    let mut rewrites: FxHashMap<Value, Value> = FxHashMap::default();

    for block in &func.blocks {
        let mut table: FxHashMap<Key, Value> = FxHashMap::default();

        for inst in &block.insts {
            if !super::dce::is_pure(&inst.kind) {
                table.retain(|k, _| !k.reads_memory());
                continue;
            }
            let Some(dest) = inst.dest else { continue };

            let Some(key) = key_of(&inst.kind, &|v| resolve(&rewrites, v)) else {
                continue;
            };

            match table.get(&key) {
                Some(&existing) => {
                    rewrites.insert(dest, existing);
                }
                None => {
                    table.insert(key, dest);
                }
            }
        }
    }

    func.replace_uses_with_map(&rewrites)
}

pub fn run_global(func: &mut SsaFunc) -> bool {
    crate::ssa::verify::recompute_preds(func);
    let Ok(dom) = crate::ssa::dom::DomTree::compute(func) else {
        return false;
    };
    let mut uses: Vec<Vec<u32>> = vec![Vec::new(); func.values.len()];
    for (b, block) in func.blocks.iter().enumerate() {
        let mut push = |v: crate::ssa::ir::Value| {
            if let Some(slot) = uses.get_mut(v.0 as usize)
                && slot.last() != Some(&(b as u32))
            {
                slot.push(b as u32);
            }
        };
        for inst in &block.insts {
            inst.kind.for_each_use(&mut push);
        }
        block.term.for_each_use(&mut push);
    }
    let mut seen: FxHashMap<Key, (Value, usize)> = FxHashMap::default();
    let mut rewrites: FxHashMap<Value, Value> = FxHashMap::default();
    for b in 0..func.blocks.len() {
        for inst in &func.blocks[b].insts {
            let Some(dest) = inst.dest else { continue };
            if !super::dce::is_pure(&inst.kind) {
                continue;
            }
            let Some(key) = key_of(&inst.kind, &|v| resolve(&rewrites, v)) else {
                continue;
            };
            if key.reads_memory() {
                continue;
            }
            if !matches!(key, Key::Binary(..) | Key::Unary(..)) {
                continue;
            }
            match seen.get(&key) {
                Some((existing, def)) => {
                    let dominated = uses
                        .get(dest.0 as usize)
                        .is_some_and(|bs| bs.iter().all(|u| dom.dominates(*def, *u as usize)));
                    if dominated {
                        rewrites.insert(dest, *existing);
                    }
                }
                None => {
                    seen.insert(key, (dest, b));
                }
            }
        }
    }
    if rewrites.is_empty() {
        return false;
    }
    func.replace_uses_with_map(&rewrites)
}

fn resolve(rewrites: &FxHashMap<Value, Value>, v: Value) -> u32 {
    let mut cur = v;

    for _ in 0..rewrites.len() + 1 {
        match rewrites.get(&cur) {
            Some(&next) if next != cur => cur = next,
            _ => break,
        }
    }
    cur.0
}

fn key_of(kind: &SsaOp, id: &impl Fn(Value) -> u32) -> Option<Key> {
    Some(match kind {
        SsaOp::ConstInt(i) => Key::Int(*i),
        SsaOp::ConstFloat(f) => Key::FloatBits(f.to_bits()),
        SsaOp::ConstBool(b) => Key::Bool(*b),
        SsaOp::ConstStr(s) => Key::Str(s.clone()),
        SsaOp::ConstChar(c) => Key::Char(*c),
        SsaOp::ConstBigInt(i) => Key::BigInt(i.clone()),
        SsaOp::ConstNull => Key::Null,

        SsaOp::Binary { op, lhs, rhs } => Key::Binary(bin_tag(*op), id(*lhs), id(*rhs)),
        SsaOp::Unary { op, operand } => Key::Unary(un_tag(*op), id(*operand)),
        SsaOp::KindOf { operand } => Key::KindOf(id(*operand)),
        SsaOp::IsNull { operand } => Key::IsNull(id(*operand)),
        SsaOp::TypeTest { value, class } => Key::TypeTest(id(*value), id(*class)),
        SsaOp::IsArray { operand } => Key::IsArray(id(*operand)),
        SsaOp::StrLength { operand } => Key::StrLength(id(*operand)),
        SsaOp::GetEnumTag { operand } => Key::EnumTag(id(*operand)),

        SsaOp::GetFixedField { object, slot, .. } => Key::FixedField(id(*object), *slot),
        SsaOp::ArrayGetIndex { object, index } => Key::ArrayElem(id(*object), id(*index)),
        SsaOp::BytesGetIndex { object, index } => Key::ArrayElem(id(*object), id(*index)),
        SsaOp::ModuleSlot { object, slot } => Key::ModuleSlot(id(*object), *slot),
        SsaOp::LoadGlobal(name) => Key::Global(name.clone()),
        SsaOp::LoadGlobalIdx(slot) => Key::GlobalIdx(*slot),
        SsaOp::LoadNativeGlobalIdx(slot) => Key::NativeGlobalIdx(*slot),
        SsaOp::LoadUpvalue(i) => Key::Upvalue(*i),

        SsaOp::ConstDecimal(_)
        | SsaOp::StoreGlobal { .. }
        | SsaOp::StoreGlobalIdx { .. }
        | SsaOp::StoreUpvalue { .. }
        | SsaOp::Call { .. }
        | SsaOp::CallDirect { .. }
        | SsaOp::AllocInstance { .. }
        | SsaOp::SelfCall { .. }
        | SsaOp::GetProperty { .. }
        | SsaOp::GetSlotMethod { .. }
        | SsaOp::GetIndex { .. }
        | SsaOp::SetProperty { .. }
        | SsaOp::SetFixedField { .. }
        | SsaOp::SetIndex { .. }
        | SsaOp::ArraySetIndex { .. }
        | SsaOp::BytesSetIndex { .. }
        | SsaOp::ArrayPush { .. }
        | SsaOp::ObjectMerge { .. }
        | SsaOp::MethodCall { .. }
        | SsaOp::InvokeSlot { .. }
        | SsaOp::Cast { .. }
        | SsaOp::Convert { .. }
        | SsaOp::BuildArray { .. }
        | SsaOp::BuildTuple { .. }
        | SsaOp::BuildObject { .. }
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
        | SsaOp::BuildObjectSpread { .. } => return None,
    })
}

fn bin_tag(op: vn_types::ssa::SsaBinOp) -> u32 {
    use vn_types::ssa::SsaBinOp::*;
    match op {
        IntAdd => 0,
        IntSub => 1,
        IntMul => 2,
        IntDiv => 3,
        IntMod => 4,
        IntPow => 5,
        IntEq => 6,
        IntNe => 7,
        IntLt => 8,
        IntLe => 9,
        IntGt => 10,
        IntGe => 11,
        IntAnd => 12,
        IntOr => 13,
        IntXor => 14,
        IntShl => 15,
        IntShr => 16,
        IntUshr => 17,
        FloatAdd => 18,
        FloatSub => 19,
        FloatMul => 20,
        FloatDiv => 21,
        FloatMod => 22,
        FloatPow => 23,
        FloatEq => 24,
        FloatNe => 25,
        FloatLt => 26,
        FloatLe => 27,
        FloatGt => 28,
        FloatGe => 29,
        StrConcat => 30,
        StrEq => 31,
        StrNe => 32,
        BitsEq => 33,
        BitsNe => 34,
        Dyn(inner) => 100 + dyn_bin_tag(inner),
    }
}

fn dyn_bin_tag(op: vn_core::BinOp) -> u32 {
    use vn_core::BinOp::*;
    match op {
        Add => 0,
        Sub => 1,
        Mul => 2,
        Div => 3,
        Mod => 4,
        Pow => 5,
        Eq => 6,
        Ne => 7,
        Lt => 8,
        Le => 9,
        Gt => 10,
        Ge => 11,
        BitAnd => 12,
        BitOr => 13,
        BitXor => 14,
        Shl => 15,
        Shr => 16,
        Ushr => 17,
        In => 18,
    }
}

fn un_tag(op: vn_types::ssa::SsaUnOp) -> u32 {
    use vn_types::ssa::SsaUnOp::*;
    match op {
        NegInt => 0,
        NegFloat => 1,
        Not => 2,
        BitNotInt => 3,
        Dyn(inner) => {
            100 + match inner {
                vn_types::ssa::DynUnOp::Neg => 0,
                vn_types::ssa::DynUnOp::Not => 1,
                vn_types::ssa::DynUnOp::BitNot => 2,
            }
        }
    }
}
