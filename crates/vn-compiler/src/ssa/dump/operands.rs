use super::super::ir::Value;
use vn_tir::{BackendTy, DynReason};

pub(super) fn args_list(args: &[Value]) -> String {
    if args.is_empty() {
        String::new()
    } else {
        let inner = args.iter().map(|v| val(*v)).collect::<Vec<_>>().join(", ");
        format!("({inner})")
    }
}

pub(super) fn val(v: Value) -> String {
    format!("v{}", v.0)
}

pub(super) fn ty(t: BackendTy) -> &'static str {
    match t {
        BackendTy::Literal(_) => ty(t.representation()),
        BackendTy::Num(vn_core::NumKind::Int(_)) => vn_core::LangPrimitive::INT.name(),
        BackendTy::Num(vn_core::NumKind::Float(_)) => vn_core::LangPrimitive::FLOAT.name(),
        BackendTy::Bool => vn_core::LangPrimitive::Bool.name(),
        BackendTy::Str => vn_core::LangPrimitive::Str.name(),
        BackendTy::Dynamic(DynReason::Runtime) => "ref",
        BackendTy::Dynamic(_) => "dyn",
        BackendTy::Char => "char",
        BackendTy::Bytes => "bytes",
        BackendTy::Decimal => "decimal",
        BackendTy::BigInt => "bigint",
        BackendTy::Enum(_) => "enum",
        BackendTy::Fn(_) => "fn",
        BackendTy::Void => "void",
        BackendTy::Never => "never",

        BackendTy::Array(_) => "array",
        BackendTy::Map(..) => "map",
        BackendTy::Set(_) => "set",
        BackendTy::Tuple(_) => "tuple",
        BackendTy::Class(_) => "class",
        BackendTy::Nullable(_) => "nullable",
    }
}

pub(super) fn binop(op: vn_types::ssa::SsaBinOp) -> &'static str {
    use vn_types::ssa::SsaBinOp as B;
    match op {
        B::IntAdd => "iadd",
        B::IntSub => "isub",
        B::IntMul => "imul",
        B::IntDiv => "idiv",
        B::IntMod => "imod",
        B::IntPow => "ipow",
        B::IntEq => "ieq",
        B::IntNe => "ine",
        B::IntLt => "ilt",
        B::IntLe => "ile",
        B::IntGt => "igt",
        B::IntGe => "ige",
        B::IntAnd => "iand",
        B::IntOr => "ior",
        B::IntXor => "ixor",
        B::IntShl => "ishl",
        B::IntShr => "ishr",
        B::IntUshr => "iushr",
        B::FloatAdd => "fadd",
        B::FloatSub => "fsub",
        B::FloatMul => "fmul",
        B::FloatDiv => "fdiv",
        B::FloatMod => "fmod",
        B::FloatPow => "fpow",
        B::FloatEq => "feq",
        B::FloatNe => "fne",
        B::FloatLt => "flt",
        B::FloatLe => "fle",
        B::FloatGt => "fgt",
        B::FloatGe => "fge",
        B::StrConcat => "strconcat",
        B::StrEq => "streq",
        B::StrNe => "strne",
        B::BitsEq => "bitseq",
        B::BitsNe => "bitsne",
        B::Dyn(inner) => match inner {
            vn_core::BinOp::Add => "add",
            vn_core::BinOp::Sub => "sub",
            vn_core::BinOp::Mul => "mul",
            vn_core::BinOp::Div => "div",
            vn_core::BinOp::Mod => "mod",
            vn_core::BinOp::Pow => "pow",
            vn_core::BinOp::Eq => "eq",
            vn_core::BinOp::Ne => "ne",
            vn_core::BinOp::Lt => "lt",
            vn_core::BinOp::Le => "le",
            vn_core::BinOp::Gt => "gt",
            vn_core::BinOp::Ge => "ge",
            vn_core::BinOp::BitAnd => "band",
            vn_core::BinOp::BitOr => "bor",
            vn_core::BinOp::BitXor => "bxor",
            vn_core::BinOp::Shl => "shl",
            vn_core::BinOp::Shr => "shr",
            vn_core::BinOp::Ushr => "ushr",
            vn_core::BinOp::In => "in",
        },
    }
}

pub(super) fn unop(op: vn_types::ssa::SsaUnOp) -> &'static str {
    use vn_types::ssa::{DynUnOp, SsaUnOp as U};
    match op {
        U::NegInt => "ineg",
        U::NegFloat => "fneg",
        U::Not => "not",
        U::BitNotInt => "ibnot",
        U::Dyn(DynUnOp::Neg) => "neg",
        U::Dyn(DynUnOp::Not) => "not",
        U::Dyn(DynUnOp::BitNot) => "bnot",
    }
}
