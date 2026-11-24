use super::operands::{args_list, binop, unop, val};
use vn_types::ssa::SsaOp;

pub fn inst_kind(kind: &SsaOp) -> String {
    match kind {
        SsaOp::ConstInt(n) => format!("int {n}"),
        SsaOp::ConstFloat(n) => format!("float {n}"),
        SsaOp::ConstBool(b) => format!("bool {b}"),
        SsaOp::ConstStr(s) => format!("str {s:?}"),
        SsaOp::ConstChar(c) => format!("char {c:?}"),
        SsaOp::ConstDecimal(d) => format!("decimal {d}"),
        SsaOp::ConstBigInt(n) => format!("bigint {n}"),
        SsaOp::ConstNull => "null".to_owned(),
        SsaOp::Binary { op, lhs, rhs } => {
            format!("{} {}, {}", binop(*op), val(*lhs), val(*rhs))
        }
        SsaOp::Unary { op, operand } => format!("{} {}", unop(*op), val(*operand)),
        SsaOp::KindOf { operand } => format!("kindof {}", val(*operand)),
        SsaOp::LoadGlobal(name) => format!("global {name}"),
        SsaOp::LoadGlobalIdx(slot) => format!("global @{slot}"),
        SsaOp::LoadNativeGlobalIdx(slot) => format!("native-global @{slot}"),
        SsaOp::LoadUpvalue(uv) => format!("upvalue #{uv}"),
        SsaOp::StoreGlobal { name, value } => format!("storeglobal {name} = {}", val(*value)),
        SsaOp::StoreGlobalIdx { slot, value } => {
            format!("storeglobal @{slot} = {}", val(*value))
        }
        SsaOp::StoreUpvalue { index, value } => {
            format!("storeupvalue #{index} = {}", val(*value))
        }
        SsaOp::Call { callee, args, .. } => format!("call {}{}", val(*callee), args_list(args)),
        SsaOp::CallDirect {
            target,
            slot,
            callee,
            args,
            ..
        } => {
            let via = match slot {
                Some(s) => format!("@{s}"),
                None => String::new(),
            };
            format!(
                "calldirect {target}{via} {} {}",
                val(*callee),
                args_list(args)
            )
        }
        SsaOp::AllocInstance { class, .. } => format!("alloc {}", val(*class)),
        SsaOp::SelfCall { args } => format!("callself{}", args_list(args)),
        SsaOp::GetProperty { object, name } => format!("getprop {}.{name}", val(*object)),
        SsaOp::GetSlotMethod { object, slot } => {
            format!("getslotmethod {}.#{slot}", val(*object))
        }
        SsaOp::GetFixedField { object, slot, .. } => {
            format!("getfixed {}[{slot}]", val(*object))
        }
        SsaOp::GetIndex { object, index } => {
            format!("getindex {}[{}]", val(*object), val(*index))
        }
        SsaOp::ArrayGetIndex { object, index } => {
            format!("arraygetindex {}[{}]", val(*object), val(*index))
        }
        SsaOp::BytesGetIndex { object, index } => {
            format!("bytesgetindex {}[{}]", val(*object), val(*index))
        }
        SsaOp::SetProperty {
            object,
            name,
            value,
        } => {
            format!("setprop {}.{name} = {}", val(*object), val(*value))
        }
        SsaOp::SetFixedField {
            object,
            value,
            slot,
            ..
        } => {
            format!("setfixed {}[{slot}] = {}", val(*object), val(*value))
        }
        SsaOp::SetIndex {
            object,
            index,
            value,
        } => {
            format!(
                "setindex {}[{}] = {}",
                val(*object),
                val(*index),
                val(*value)
            )
        }
        SsaOp::ArrayPush { array, value } => {
            format!("arraypush {}, {}", val(*array), val(*value))
        }
        SsaOp::ArraySetIndex {
            object,
            index,
            value,
        } => {
            format!(
                "arraysetindex {}[{}] = {}",
                val(*object),
                val(*index),
                val(*value)
            )
        }
        SsaOp::BytesSetIndex {
            object,
            index,
            value,
        } => {
            format!(
                "bytessetindex {}[{}] = {}",
                val(*object),
                val(*index),
                val(*value)
            )
        }
        SsaOp::ObjectMerge { target, source } => {
            format!("objectmerge {} <- {}", val(*target), val(*source))
        }
        SsaOp::MethodCall { recv, name, args } => {
            format!("callmethod {}.{name}{}", val(*recv), args_list(args))
        }
        SsaOp::InvokeSlot {
            recv, slot, args, ..
        } => {
            format!("invokeslot {}.#{slot}{}", val(*recv), args_list(args))
        }
        SsaOp::IsNull { operand } => format!("isnull {}", val(*operand)),
        SsaOp::TypeTest { value, class } => {
            format!("typetest {} {}", val(*value), val(*class))
        }
        SsaOp::Cast { operand } => format!("cast {}", val(*operand)),
        SsaOp::Convert { operand, conv } => format!("convert {} {conv:?}", val(*operand)),
        SsaOp::BuildArray { elements, .. } => format!("array{}", args_list(elements)),
        SsaOp::BuildTuple { elements } => format!("tuple{}", args_list(elements)),
        SsaOp::BuildObject { pairs } => {
            let inner = pairs
                .iter()
                .map(|(k, v)| format!("{k}: {}", val(*v)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("object {{{inner}}}")
        }
        SsaOp::BuildMap { pairs } => {
            let inner = pairs
                .iter()
                .map(|(k, v)| format!("{}: {}", val(*k), val(*v)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("map {{{inner}}}")
        }
        SsaOp::ObjectRest { object, skip_keys } => {
            format!("objectrest {} skip={:?}", val(*object), skip_keys)
        }
        SsaOp::ToString { operand } => format!("tostring {}", val(*operand)),
        SsaOp::BuildStr { parts } => format!("buildstr{}", args_list(parts)),
        SsaOp::MakeClosure { func, .. } => format!("closure tir#{func}"),
        SsaOp::IntrinsicCall { object, args, wire } => {
            format!("intrinsic#{wire} {}{}", val(*object), args_list(args))
        }
        SsaOp::CallNativeOp {
            object,
            args,
            op_id,
        } => {
            format!("nativeop#{op_id:#x} {}{}", val(*object), args_list(args))
        }
        SsaOp::AssertNotNull { operand } => format!("assertnotnull {}", val(*operand)),
        SsaOp::GetPropertyMaybe { object, name } => {
            format!("getpropmaybe {}.{name}", val(*object))
        }
        SsaOp::ModuleSlot { object, slot } => format!("moduleslot {}[{slot}]", val(*object)),
        SsaOp::GetEnumTag { operand } => format!("enumtag {}", val(*operand)),
        SsaOp::IsArray { operand } => format!("isarray {}", val(*operand)),
        SsaOp::StrLength { operand } => format!("strlen {}", val(*operand)),
        SsaOp::ArrayLength { operand } => format!("arrlen {}", val(*operand)),
        SsaOp::BytesLength { operand } => format!("byteslen {}", val(*operand)),
        SsaOp::This => "this".to_owned(),
        SsaOp::Range {
            start,
            end,
            inclusive,
        } => {
            let op = if *inclusive { "..=" } else { ".." };
            format!("range {}{op}{}", val(*start), val(*end))
        }
        SsaOp::ObjectKeys { operand } => format!("objectkeys {}", val(*operand)),
        SsaOp::GetSuper { name } => format!("getsuper super.{name}"),
        SsaOp::GetSuperSlot { slot } => format!("getsuperslot super.#{slot}"),
        SsaOp::SuperCall { args } => format!("supercall{}", args_list(args)),
        SsaOp::SuperMethodCall { name, args } => {
            format!("supercall super.{name}{}", args_list(args))
        }
        SsaOp::SuperInvokeSlot { slot, args } => {
            format!("supercall super.#{slot}{}", args_list(args))
        }
        SsaOp::ExtensionCall {
            func, recv, args, ..
        } => {
            format!("extcall {func}({}{})", val(*recv), {
                let a = args.iter().map(|v| val(*v)).collect::<Vec<_>>().join(", ");
                if a.is_empty() {
                    String::new()
                } else {
                    format!(", {a}")
                }
            })
        }
        SsaOp::CallSpread { callee, args } => {
            let a = args
                .iter()
                .map(|a| {
                    if a.spread {
                        format!("...{}", val(a.value))
                    } else {
                        val(a.value)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("callspread {}({a})", val(*callee))
        }
        SsaOp::BuildArraySpread { elements } => {
            let a = elements
                .iter()
                .map(|a| {
                    if a.spread {
                        format!("...{}", val(a.value))
                    } else {
                        val(a.value)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("array[{a}]")
        }
        SsaOp::BuildObjectSpread { parts } => {
            let a = parts
                .iter()
                .map(|p| match &p.key {
                    Some(k) => format!("{k}: {}", val(p.value)),
                    None => format!("...{}", val(p.value)),
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("object {{{a}}}")
        }
        SsaOp::LoadCaptured { var } => format!("loadcaptured {var:?}"),
        SsaOp::StoreCaptured { var, value } => {
            format!("storecaptured {var:?} = {}", val(*value))
        }
        SsaOp::MakeClass { name, super_class } => {
            format!("makeclass {name} super={:?}", super_class.map(val))
        }
        SsaOp::DeclareLayout { class, layout } => {
            format!(
                "declarelayout {} {{{}}}",
                val(*class),
                layout
                    .fields
                    .iter()
                    .map(|f| format!("{}@{}", f.name, f.offset))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        SsaOp::FreezeClass { class } => {
            format!("freezeclass {}", val(*class))
        }
        SsaOp::DefineMember {
            class,
            name,
            member,
            kind,
        } => format!(
            "define {} {}.{name} = {}",
            kind.label(),
            val(*class),
            val(*member)
        ),
        SsaOp::MakeEnumVariant { def } => format!(
            "makeenumvariant {}.{} tag={}",
            def.enum_name, def.variant_name, def.tag
        ),
        SsaOp::Try { handler } => format!("try b{}", handler.0),
        SsaOp::PopTry => "poptry".to_owned(),
        SsaOp::CatchParam { try_val } => format!("catchparam {}", val(*try_val)),
        SsaOp::CloseUpvalues { vars } => format!("closeupvalues {:?}", vars),
        SsaOp::LoadModule { source } => format!("loadmodule {source}"),
        SsaOp::StoreModuleSlot { value, slot } => {
            format!("storemoduleslot {slot} = {}", val(*value))
        }
        SsaOp::Await { operand } => format!("await {}", val(*operand)),
        SsaOp::Spawn { operand } => format!("spawn {}", val(*operand)),
        SsaOp::Yield { operand } => format!("yield {}", val(*operand)),
    }
}
