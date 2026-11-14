use vn_core::layout::{ClassLayout, ScalarRepr};
use vn_core::term::chalk::chalk;
use vn_core::term::terminal;
use vn_core::term::terminal::{Align, Section};
use vn_tir::TirModule;

use crate::flags::DebugFlags;

fn repr_name(repr: ScalarRepr) -> &'static str {
    match repr {
        ScalarRepr::Bool => "bool",
        ScalarRepr::NullableBool => "bool?",
        ScalarRepr::Char => "char",
        ScalarRepr::NullableChar => "char?",
        ScalarRepr::I64 => "i64",
        ScalarRepr::F64 => "f64",
        ScalarRepr::Ref => "ref",
        ScalarRepr::Boxed => "boxed",
    }
}

fn summary(layout: &ClassLayout) -> String {
    let shape = if layout.is_value {
        "valor"
    } else {
        "referencia"
    };
    format!(
        "{} B · alineación {} · {} slot(s) que traza el GC · {shape}",
        layout.payload_size,
        layout.alignment,
        layout.gc.slots.len()
    )
}

pub fn debug_layout(module: &TirModule, flags: &DebugFlags) {
    Section::new("layout")
        .subtitle(module.source_file.clone())
        .color(|c| c.cyan())
        .print();

    let mut shown = 0usize;
    for class in &module.classes {
        if flags
            .fn_filter
            .as_ref()
            .is_some_and(|needle| !class.name.contains(needle.as_str()))
        {
            continue;
        }
        shown += 1;
        terminal::tagged(class.name.as_ref(), summary(&class.layout));
        if class.layout.fields.is_empty() {
            continue;
        }
        let mut table = terminal::Table::new(["campo", "tipo", "offset", "bytes", "repr"]).align([
            Align::Left,
            Align::Left,
            Align::Right,
            Align::Right,
            Align::Left,
        ]);
        for field in &class.layout.fields {
            table.row([
                field.name.to_string(),
                field
                    .kind
                    .map_or_else(|| "—".to_string(), |kind| kind.to_string()),
                field.offset.to_string(),
                field.layout.size.to_string(),
                repr_name(field.layout.repr).to_string(),
            ]);
        }
        table.print();
    }

    if shown == 0 {
        terminal::info("el módulo no declara clases");
    } else {
        terminal::log(format!(
            "  {}",
            chalk("`boxed` es un VmValue de 16 bytes; `—` es un tipo sin representación propia.")
                .dim()
        ));
    }
    Section::new("layout").close();
}
