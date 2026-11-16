use rustc_hash::FxHashMap as HashMap;

#[derive(Debug, Clone, Default)]
pub struct ModuleClasses {
    pub classes: Vec<vn_tir::ClassInfo>,
    pub origins: Vec<Option<String>>,
    pub types: vn_tir::TyTable,
}

fn is_unboxed_kind(field: vn_core::FieldKind) -> bool {
    use vn_core::layout::{ScalarRepr, TypeLayout};
    match TypeLayout::of_field(Some(field)).repr {
        ScalarRepr::Bool
        | ScalarRepr::NullableBool
        | ScalarRepr::Char
        | ScalarRepr::NullableChar
        | ScalarRepr::I64
        | ScalarRepr::F64 => true,
        ScalarRepr::Ref | ScalarRepr::Boxed => false,
    }
}

fn fields_cover(
    candidate_fields: &[vn_tir::FieldInfo],
    candidate_types: &vn_tir::TyTable,
    expected_fields: &[vn_tir::FieldInfo],
    expected_types: &vn_tir::TyTable,
) -> bool {
    if candidate_fields.len() != expected_fields.len() {
        return false;
    }
    candidate_fields
        .iter()
        .zip(expected_fields.iter())
        .all(|(have, want)| {
            if have.name != want.name || have.slot != want.slot {
                return false;
            }
            match (
                have.ty.field_kind(candidate_types),
                want.ty.field_kind(expected_types),
            ) {
                (Some(have_kind), Some(want_kind)) => have_kind == want_kind,
                (None, Some(want_kind)) => !is_unboxed_kind(want_kind),
                (None, None) => true,
                (Some(_), None) => false,
            }
        })
}

pub fn verify_foreign_layouts(tables: &HashMap<String, ModuleClasses>) -> Result<(), String> {
    let mut modules: Vec<&String> = tables.keys().collect();
    modules.sort();
    for module in modules {
        let entry = &tables[module];
        let mut foreign: Vec<(&str, &str)> = entry
            .origins
            .iter()
            .zip(entry.classes.iter())
            .filter_map(|(origin, ci)| origin.as_ref().map(|o| (ci.name.as_ref(), o.as_str())))
            .collect();
        foreign.sort();
        for (name, origin) in foreign {
            let Some(defining) = tables.get(origin) else {
                continue;
            };
            let Some(local) = defining
                .origins
                .iter()
                .zip(defining.classes.iter())
                .find(|(o, ci)| o.is_none() && ci.name.as_ref() == name)
                .map(|(_, ci)| ci)
            else {
                continue;
            };
            let copy = entry
                .origins
                .iter()
                .zip(entry.classes.iter())
                .find(|(o, ci)| o.as_deref() == Some(origin) && ci.name.as_ref() == name)
                .map(|(_, ci)| ci)
                .expect("foreign entry vanished");
            if !fields_cover(&copy.fields, &entry.types, &local.fields, &defining.types) {
                let show = |fields: &[vn_tir::FieldInfo], types: &vn_tir::TyTable| {
                    fields
                        .iter()
                        .map(|f| format!("{}:{:?}#{}", f.name, f.ty.non_nullable(types), f.slot))
                        .collect::<Vec<_>>()
                        .join(",")
                };
                return Err(format!(
                    "E_CLASS_LAYOUT_DIVERGENCE: '{name}' copied into '{module}' from '{origin}' differs from its definition (copy [{}] vs def [{}])",
                    show(&copy.fields, &entry.types),
                    show(&local.fields, &defining.types)
                ));
            }
        }
    }
    Ok(())
}
