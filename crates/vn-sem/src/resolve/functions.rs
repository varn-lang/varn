use crate::types::{CheckerTyTable, FunctionType, Type, TypeContext};
use vn_core::ast::{FnTypeParam, TypeNode};

use super::names::resolve_atom_name;
use super::resolve_type_node;

pub(super) fn resolve_fn_type(
    params: &[FnTypeParam],
    ret: &TypeNode,
    ctx: Option<&dyn TypeContext>,
    interner: &vn_core::AtomInterner,
    table: &mut CheckerTyTable,
) -> Type {
    let resolved_params = params
        .iter()
        .map(|p| {
            let ty = resolve_type_node(&p.ty, ctx, table);
            crate::types::FunctionParam {
                name: p.name.map(|name| resolve_atom_name(name, ctx, interner)),
                ty: ty.0,
                optional: false,
                is_rest: p.is_rest,
            }
        })
        .collect();
    let ret_ty = resolve_type_node(ret, ctx, table);
    Type::fn_(
        FunctionType {
            params: resolved_params,
            return_type: ret_ty.0,
            is_arrow: false,
            type_params: vec![],
            type_param_defaults: Vec::new(),
        },
        table,
    )
}
