use crate::types::CheckerTyTable;
use crate::types::TypeContext;
use std::sync::Arc;
use vn_core::ast::QualifiedName;

pub(super) fn resolve_atom_name(
    atom: vn_core::Atom,
    ctx: Option<&dyn TypeContext>,
    interner: &vn_core::AtomInterner,
) -> Arc<str> {
    if let Some(s) = interner.try_resolve(atom) {
        return Arc::from(s);
    }
    ctx.and_then(|c| c.atom_text(atom))
        .map(Arc::from)
        .unwrap_or_default()
}

pub(super) struct WrittenName {
    pub(super) atom: vn_core::Atom,
    pub(super) text: Arc<str>,
    pub(super) segments: Vec<Arc<str>>,
}

pub(super) fn written_name(
    path: &QualifiedName,
    ctx: Option<&dyn TypeContext>,
    interner: &vn_core::AtomInterner,
    table: &mut CheckerTyTable,
) -> WrittenName {
    let segments: Vec<Arc<str>> = path
        .segments
        .iter()
        .map(|segment| resolve_atom_name(segment.name, ctx, interner))
        .collect();
    match path.simple() {
        Some(atom) => WrittenName {
            atom,
            text: segments[0].clone(),
            segments,
        },
        None => {
            let text: Arc<str> = Arc::from(segments.join(".").as_str());
            WrittenName {
                atom: table.intern_name(&text),
                text,
                segments,
            }
        }
    }
}
