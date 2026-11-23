use vn_tir::BackendTy;

pub(super) fn is_free_fn_name(name: &str) -> bool {
    name.split("::").all(|part| {
        let mut cs = part.chars();
        matches!(cs.next(), Some(c) if c == '_' || c.is_ascii_alphabetic())
            && cs.all(|c| c == '_' || c.is_ascii_alphanumeric())
    })
}

pub(super) fn numeric_domain(bt: BackendTy) -> Option<vn_core::NumericDomain> {
    use vn_core::NumericDomain as D;
    Some(match bt {
        BackendTy::Literal(_) => return numeric_domain(bt.representation()),
        BackendTy::Num(vn_core::NumKind::Int(_)) => D::Int,
        BackendTy::Num(vn_core::NumKind::Float(_)) => D::Float,
        BackendTy::BigInt => D::BigInt,
        BackendTy::Decimal => D::Decimal,
        BackendTy::Bool
        | BackendTy::Char
        | BackendTy::Str
        | BackendTy::Bytes
        | BackendTy::Array(_)
        | BackendTy::Map(..)
        | BackendTy::Set(_)
        | BackendTy::Tuple(_)
        | BackendTy::Class(_)
        | BackendTy::Enum(_)
        | BackendTy::Fn(_)
        | BackendTy::Nullable(_)
        | BackendTy::Void
        | BackendTy::Never
        | BackendTy::Dynamic(_) => return None,
    })
}
