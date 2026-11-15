#![allow(clippy::disallowed_methods)]
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/modules");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("registry.generated.rs");
    let mut out = fs::File::create(&out_path).expect("failed to create registry.generated.rs");

    writeln!(out, "pub static MODULE_REGISTRY: &[ModuleSpec] = &[").unwrap();
    for (layer, kind) in [
        ("core", "ModuleKind::Core"),
        ("runtime", "ModuleKind::Runtime"),
    ] {
        let root = Path::new("src/modules").join(layer);
        collect_modules(&root, &root, layer, kind, &mut out);
    }
    writeln!(out, "];").unwrap();
}

fn collect_modules(root: &Path, dir: &Path, layer: &str, kind: &str, out: &mut impl Write) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));
    let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();

    let contracts: Vec<&Path> = paths
        .iter()
        .filter(|p| p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("vn"))
        .map(|p| p.as_path())
        .collect();
    let templates: Vec<&Path> = paths
        .iter()
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|e| e.to_str()) == Some(vn_contract::template::EXTENSION)
        })
        .map(|p| p.as_path())
        .collect();
    for template in templates {
        emit_template_entries(out, layer, kind, template);
    }
    match contracts.as_slice() {
        [] => {}
        [contract] => {
            let rel = dir
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            assert!(
                !rel.is_empty(),
                "{} is a layer root, not a module",
                dir.display()
            );
            emit_spec_entry(out, &format!("{layer}:{rel}"), kind, contract);
        }
        many => panic!(
            "{} holds {} .vn contracts; a module is exactly one",
            dir.display(),
            many.len()
        ),
    }

    for sub in paths.iter().filter(|p| p.is_dir()) {
        collect_modules(root, sub, layer, kind, out);
    }
}

fn emit_template_entries(out: &mut impl Write, layer: &str, kind_expr: &str, template: &Path) {
    let shown = template.to_string_lossy().replace('\\', "/");
    let text = fs::read_to_string(template).unwrap_or_else(|e| panic!("cannot read {shown}: {e}"));
    let kinds = vn_contract::template::kinds(&text).unwrap_or_else(|e| panic!("{shown}: {e}"));
    for kind in kinds {
        let source = vn_contract::template::instantiate(&text, kind);
        let name = format!("{shown}#{}", kind.name());
        let surface =
            vn_contract::parse(&source, &name).unwrap_or_else(|e| panic!("template {name}: {e}"));
        let exports: String = surface
            .exports
            .iter()
            .map(|e| format!(r#""{e}","#))
            .collect();
        let code = if surface.has_code { ".with_code()" } else { "" };
        writeln!(
            out,
            r#"    ModuleSpec::new("{layer}:types/{id}", {kind_expr}, "crates/vn-builtins/{shown}").with_source({source:?}).with_exports(&[{exports}]){code},"#,
            id = kind.name(),
        )
        .unwrap();
    }
}

fn emit_spec_entry(out: &mut impl Write, id: &str, kind_expr: &str, contract: &Path) {
    let include_path = contract.to_string_lossy().replace('\\', "/");
    let source =
        fs::read_to_string(contract).unwrap_or_else(|e| panic!("cannot read {include_path}: {e}"));
    let surface = vn_contract::parse(&source, &include_path)
        .unwrap_or_else(|e| panic!("contract {include_path}: {e}"));
    let exports: String = surface
        .exports
        .iter()
        .map(|e| format!(r#""{e}","#))
        .collect();
    let code = if surface.has_code { ".with_code()" } else { "" };
    writeln!(
        out,
        r#"    ModuleSpec::new("{id}", {kind_expr}, "crates/vn-builtins/{include_path}").with_source(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/{include_path}"))).with_exports(&[{exports}]){code},"#,
    )
    .unwrap();
}
