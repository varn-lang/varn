use vn_sem::resolver::ImportResolver;
use vn_types::FunctionProto;

pub fn compile_source(
    source: &str,
    path: &str,
    session: &crate::ModuleResolver,
) -> Result<FunctionProto, String> {
    compile_source_inner(source, path, false, session).map(|(proto, _)| proto)
}

pub fn compile_source_checked(
    source: &str,
    path: &str,
    session: &crate::ModuleResolver,
) -> Result<FunctionProto, String> {
    compile_source_inner(source, path, true, session).map(|(proto, _)| proto)
}

fn compile_source_inner(
    source: &str,
    path: &str,
    reject_type_errors: bool,
    session: &crate::ModuleResolver,
) -> Result<(FunctionProto, vn_tir::package::ModuleTemplates), String> {
    let (program, arena, interner) =
        crate::quiet_parse::parse_module(source, path).map_err(|f| f.to_string())?;
    let check = vn_checker::Checker::check(&program, &arena, interner, session);
    if reject_type_errors && check.diagnostics.has_errors() {
        let mut msg = String::new();
        for d in check.diagnostics.errors() {
            msg.push_str(&format!(
                "\n  {path}:{}:{}: {}",
                d.range.start.line, d.range.start.column, d.message
            ));
        }
        return Err(format!("type errors in stdlib module:{msg}"));
    }
    crate::lower::lower_module(crate::lower::LowerInput {
        program: &program,
        arena: &arena,
        check: &check,
        resolver: session,
        source,
        library: &crate::lower::TemplateLibrary::default(),
        measure: false,
    })
    .map(|lowered| (lowered.proto, lowered.templates))
    .map_err(|e| e.to_string())
}

fn std_imports_are_pure(id: &str, source: &str) -> Result<bool, String> {
    let (program, arena, interner) =
        crate::quiet_parse::parse_module(source, id).map_err(|f| f.to_string())?;
    let mut pure = true;
    for spec in crate::import_collector::collect_imports(&program, &arena, &interner) {
        vn_modules::layer::check_import(vn_modules::layer::Layer::Std, &spec)
            .map_err(|message| format!("{id}: {message}"))?;
        pure &= !vn_core::ModuleKind::Runtime.owns(&spec);
    }
    Ok(pure)
}

struct ImageUnit {
    id: String,
    path: String,
    source: String,
    pure: Option<bool>,
}

fn core_units() -> Result<Vec<ImageUnit>, String> {
    let provider =
        vn_modules::provider::get().ok_or_else(|| "stdlib provider not registered".to_owned())?;
    vn_modules::core_module_ids()
        .into_iter()
        .map(|id| {
            let spec = provider
                .spec_for(id)
                .ok_or_else(|| format!("core module without spec: {id}"))?;
            let source = provider
                .module(id)
                .map(|m| m.source)
                .ok_or_else(|| format!("core module without source: {id}"))?;
            Ok(ImageUnit {
                id: id.to_owned(),
                path: spec.vn_source.to_owned(),
                source: source.to_owned(),
                pure: Some(spec.pure),
            })
        })
        .collect()
}

fn std_units(std_dir: &std::path::Path) -> Result<Vec<ImageUnit>, String> {
    vn_modules::std_root::tree_modules(std_dir)
        .into_iter()
        .map(|m| {
            let file = std_dir.join(&m.path);
            let source = std::fs::read_to_string(&file)
                .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
            Ok(ImageUnit {
                id: m.id,
                path: m.path,
                source,
                pure: None,
            })
        })
        .collect()
}

pub fn compile_stdlib_image(
    std_dir: &std::path::Path,
    session: &crate::ModuleResolver,
) -> Result<Vec<u8>, String> {
    struct Compiled {
        unit: ImageUnit,
        pure: bool,
        interface: Vec<u8>,
        bytecode: Vec<u8>,
        templates: Vec<u8>,
    }

    let mut units = core_units()?;
    units.extend(std_units(std_dir)?);

    let mut compiled = Vec::new();
    let mut failures = String::new();
    for unit in units {
        let id = unit.id.as_str();
        let pure = match unit.pure {
            Some(pure) => pure,
            None => match std_imports_are_pure(id, &unit.source) {
                Ok(pure) => pure,
                Err(e) => {
                    failures.push_str(&format!("\n{e}"));
                    continue;
                }
            },
        };

        let resolver = session;
        let interface = if vn_sem::codec::has_portable_interface(id) {
            let exports = resolver.stdlib_exports(id);
            let Some(bind) = resolver.stdlib_bind(id) else {
                let err_msg = match compile_source_checked(&unit.source, id, session) {
                    Ok(_) => "unknown bind failure".to_string(),
                    Err(e) => e,
                };
                return Err(format!("cannot bind {id}: {err_msg}"));
            };
            vn_sem::codec::serialize_module_interface(&exports, &bind)
                .map_err(|e| format!("interface serialization failed for {id}: {e}"))?
        } else {
            Vec::new()
        };

        let (proto, templates) = match compile_source_inner(&unit.source, id, true, session) {
            Ok(done) => done,
            Err(e) => {
                failures.push_str(&format!("\n{e}"));
                continue;
            }
        };
        let bytecode = postcard::to_allocvec(&proto)
            .map_err(|e| format!("bytecode serialization failed for {id}: {e}"))?;

        let templates = postcard::to_allocvec(&templates)
            .map_err(|e| format!("template serialization failed for {id}: {e}"))?;

        compiled.push(Compiled {
            unit,
            pure,
            interface,
            bytecode,
            templates,
        });
    }
    if !failures.is_empty() {
        return Err(failures);
    }

    let modules = compiled
        .iter()
        .map(|m| vn_modules::std_image::ImageModule {
            id: &m.unit.id,
            path: &m.unit.path,
            pure: m.pure,
            interface: &m.interface,
            bytecode: &m.bytecode,
            templates: &m.templates,
            source: &m.unit.source,
        })
        .collect();
    Ok(vn_modules::std_image::write_image(
        env!("CARGO_PKG_VERSION"),
        vn_core::HOST_API_VERSION,
        modules,
    ))
}
