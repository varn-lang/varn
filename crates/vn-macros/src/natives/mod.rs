mod fast;
mod input;
mod mapping;
mod member;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::path::{Path, PathBuf};
use syn::LitStr;

use crate::contract_members::Kind;
use crate::contract_parse::LookupFailure;
use input::{NativesInput, Section};

pub(crate) use mapping::Mapped;

struct Site {
    module: String,
    contract: PathBuf,
}

pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as NativesInput);
    let site = match locate() {
        Ok(site) => site,
        Err(msg) => return err(msg),
    };
    let contract_path = site.contract.to_string_lossy().replace('\\', "/");
    let Ok(source) = std::fs::read_to_string(&site.contract) else {
        return err(format!("cannot read contract `{contract_path}`"));
    };

    let mut sections = Vec::new();
    let mut entries = Vec::new();
    for section in &input.sections {
        match expand_section(&site.module, section, &source, &contract_path) {
            Ok((tokens, idents)) => {
                sections.push(tokens);
                entries.extend(idents);
            }
            Err(msg) => return err(msg),
        }
    }

    let abs_lit = LitStr::new(&contract_path, proc_macro2::Span::call_site());
    let module_id = LitStr::new(&site.module, proc_macro2::Span::call_site());
    quote! {
        const _: &[u8] = include_bytes!(#abs_lit);

        pub const MODULE_ID: &str = #module_id;

        #(#sections)*

        pub static NATIVE_ENTRIES: &[&::vn_types::NativeOpEntry] = &[
            #(&#entries),*
        ];
    }
    .into()
}

fn locate() -> Result<Site, String> {
    let file = proc_macro::Span::call_site()
        .local_file()
        .ok_or("natives! must be invoked from a source file")?;
    let file = absolute(&file);
    let parts: Vec<String> = file
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let root = parts
        .windows(2)
        .position(|w| w[0] == "src" && w[1] == "modules")
        .ok_or("natives! belongs under src/modules/<layer>/<module>/")?;
    let layer = parts
        .get(root + 2)
        .ok_or("natives! needs a layer directory")?;
    let relative = parts[root + 3..parts.len() - 1].join("/");
    let module = match layer.as_str() {
        "core" => "globals".to_string(),
        "runtime" if !relative.is_empty() => format!("runtime:{relative}"),
        other => return Err(format!("`{other}` is not a module layer of src/modules")),
    };
    let dir = file
        .parent()
        .ok_or("natives! has no containing directory")?;
    Ok(Site {
        module,
        contract: sole_contract(dir)?,
    })
}

fn absolute(file: &Path) -> PathBuf {
    if file.is_absolute() {
        return file.to_path_buf();
    }
    let from_cwd = std::env::current_dir().map(|d| d.join(file));
    if let Ok(path) = from_cwd
        && path.exists()
    {
        return path;
    }
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let mut dir = Some(Path::new(&manifest));
    while let Some(d) = dir {
        if d.join(file).exists() {
            return d.join(file);
        }
        dir = d.parent();
    }
    file.to_path_buf()
}

fn sole_contract(dir: &Path) -> Result<PathBuf, String> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("cannot read `{}`: {e}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("vn"))
        .collect();
    found.sort();
    match found.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(format!("`{}` holds no .vn contract", dir.display())),
        _ => Err(format!(
            "`{}` holds more than one .vn contract",
            dir.display()
        )),
    }
}

fn expand_section(
    module: &str,
    section: &Section,
    source: &str,
    contract_path: &str,
) -> Result<(proc_macro2::TokenStream, Vec<syn::Ident>), String> {
    let contract =
        match crate::contract_parse::lookup(source, contract_path, section.class.as_deref()) {
            Ok(contract) => contract,
            Err(LookupFailure::Unparsable(reason)) | Err(LookupFailure::BadDecorator(reason)) => {
                return Err(reason)
            }
            Err(LookupFailure::ClassNotFound) => {
                let class = section.class.clone().unwrap_or_default();
                return Err(format!(
                    "class `{class}` not found in contract `{contract_path}`"
                ));
            }
            Err(LookupFailure::NoFunctions) => {
                return Err(format!(
                    "no `declare function`s found in contract `{contract_path}`"
                ))
            }
        };

    let prefix = section.class.clone().unwrap_or_else(|| module.to_string());
    let class_name_str: String = section.class.clone().unwrap_or_default();

    let cx = member::Cx {
        prefix: &prefix,
        module,
        ns: "",
        class_name: &class_name_str,
    };
    let mut generated = member::Generated::default();
    let mut claimed = vec![false; section.members.len()];
    for m in &contract.members {
        if m.kind == Kind::Property {
            member::generate_layout(m, &mut generated);
            continue;
        }
        let key = m.symbol.trim_end_matches('$');
        let Some(at) = section.members.iter().position(|u| u.key() == key) else {
            return Err(format!(
                "`{prefix}::{}` is in the contract `{contract_path}` but has no binding",
                m.symbol
            ));
        };
        claimed[at] = true;
        let user = &section.members[at];
        let expected = member::expected_params(m);
        if user.params.len() != expected && user.params.len() != expected + 1 {
            return Err(format!(
                "`{prefix}::{key}` takes {expected} parameters ({} with the context first), got {}",
                expected + 1,
                user.params.len()
            ));
        }
        member::generate(&cx, m, user, &mut generated);
    }
    if let Some(at) = claimed.iter().position(|c| !c) {
        return Err(format!(
            "`{prefix}::{}` is not in the contract `{contract_path}`",
            section.members[at].key()
        ));
    }
    let member::Generated {
        bodies,
        wrappers,
        setup_calls,
        fn_entries,
        method_entries,
        mut generated_entry_idents,
    } = generated;

    let registration = if let Some(class) = &section.class {
        let builder_ident = format_ident!("__vn_build_{}", sanitize(class));
        let entry_ident = format_ident!("__VN_OP_{}", sanitize(class).to_uppercase());
        generated_entry_idents.push(entry_ident.clone());
        let superclass_setup = if let Some(parent) = &contract.extends {
            quote! {
                if let Some(parent) = ctx.get_class(#parent) {
                    *cls.superclass.borrow_mut() = Some(parent.clone());
                    cls.set_layout(parent.layout());
                }
            }
        } else {
            quote! {}
        };
        quote! {
            pub fn #builder_ident(
                ctx: &mut dyn ::vn_types::NativeCtx,
                _args: &[::vn_types::VmValue],
            ) -> ::core::result::Result<::vn_types::VmValue, ::vn_types::NativeError> {
                let cls = ctx
                    .get_class(#class)
                    .unwrap_or_else(|| ::vn_types::value::ClassObj::new_native_rc(#class));
                #superclass_setup
                #(#setup_calls)*
                ctx.register_class(#class, cls.clone());
                Ok(ctx.alloc_class(cls.clone()))
            }

            static #entry_ident: ::vn_types::NativeOpEntry = ::vn_types::NativeOpEntry {
                module_id: #module,
                namespace_path: "",
                symbol_name: #class,
                func_ptr: #builder_ident as *const u8,
                raw_func_ptr: ::core::ptr::null(),
                signature: ::vn_types::SignatureDescriptor::empty(),
                entry_kind: ::vn_types::EntryKind::ClassDef,
                flags: 0,
                capability: "",
            };

            #(#method_entries)*
        }
    } else {
        quote! { #(#fn_entries)* }
    };

    let tokens = quote! {
        #(#bodies)*

        #(#wrappers)*

        #registration
    };
    Ok((tokens, generated_entry_idents))
}

pub(super) fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

fn err(msg: String) -> TokenStream {
    let lit = LitStr::new(&msg, proc_macro2::Span::call_site());
    TokenStream::from(quote! { compile_error!(#lit); })
}
