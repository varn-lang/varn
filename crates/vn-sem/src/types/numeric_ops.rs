use super::numeric_literal::literal_operand_class;
use super::{CheckerTyTable, Type};
use vn_core::ast::{AstArena, ExprId};
use vn_core::TypeKind;

pub fn numeric_operand(t: &Type, table: &CheckerTyTable) -> Option<vn_core::NumericOperand> {
    use vn_core::NumericOperand;
    match table.get(t.0) {
        TypeKind::Primitive(vn_core::LangPrimitive::Num(k)) => Some(NumericOperand::Num(k)),
        TypeKind::Literal(vn_core::TypeLiteral::Int(_)) => {
            Some(NumericOperand::Num(vn_core::NumKind::INT))
        }
        TypeKind::Primitive(vn_core::LangPrimitive::Decimal) => Some(NumericOperand::Decimal),
        TypeKind::Primitive(vn_core::LangPrimitive::BigInt) => Some(NumericOperand::BigInt),
        TypeKind::Primitive(_)
        | TypeKind::Literal(_)
        | TypeKind::This
        | TypeKind::Array(_)
        | TypeKind::Union(_)
        | TypeKind::Intersection(_)
        | TypeKind::Tuple(_)
        | TypeKind::Named(..)
        | TypeKind::Generic(..)
        | TypeKind::Fn(_)
        | TypeKind::Record(_)
        | TypeKind::Interface(_)
        | TypeKind::EnumVariant { .. }
        | TypeKind::TypePredicate { .. } => None,
    }
}

pub fn numeric_binary_type(l: &Type, r: &Type, table: &CheckerTyTable) -> Option<Type> {
    use vn_core::NumericOperand;
    let kind = vn_core::binary_operand_kind(numeric_operand(l, table), numeric_operand(r, table))?;
    Some(match kind {
        NumericOperand::Num(k) => Type::num(k),
        NumericOperand::Decimal => Type::Decimal,
        NumericOperand::BigInt => Type::BigInt,
    })
}

pub fn integer_binary_type(l: &Type, r: &Type, table: &CheckerTyTable) -> Option<Type> {
    numeric_binary_type(l, r, table).filter(Type::is_integer)
}

pub fn adopt_literal_operands(
    arena: &AstArena,
    left: ExprId,
    right: ExprId,
    l: Type,
    r: Type,
    table: &CheckerTyTable,
) -> (Type, Type) {
    let l2 = literal_operand_class(arena, left, &r, table).unwrap_or(l);
    let r2 = literal_operand_class(arena, right, &l, table).unwrap_or(r);
    (l2, r2)
}

pub fn numeric_operands_compatible(l: &Type, r: &Type, table: &CheckerTyTable) -> bool {
    numeric_binary_type(l, r, table).is_some()
}
