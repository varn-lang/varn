use proc_macro2::TokenStream as TS2;
use quote::{format_ident, quote};
use syn::{Ident, Pat};

use super::input::UserMember;
use super::mapping::{
    call_expr, mapped_tag_path, owned_ty, param_ty, receiver_mapped, ret_ty, Mapped,
};
use super::sanitize;
use crate::contract_members::{Kind, Member};

pub(super) struct Cx<'a> {
    pub(super) prefix: &'a str,
    pub(super) module: &'a str,
    pub(super) ns: &'a str,
    pub(super) class_name: &'a str,
}

#[derive(Default)]
pub(super) struct Generated {
    pub(super) bodies: Vec<TS2>,
    pub(super) wrappers: Vec<TS2>,
    pub(super) setup_calls: Vec<TS2>,
    pub(super) method_setup: Vec<TS2>,
    pub(super) accessor_setup: Vec<TS2>,
    pub(super) fn_entries: Vec<TS2>,
    pub(super) method_entries: Vec<TS2>,
    pub(super) generated_entry_idents: Vec<Ident>,
}

pub(super) fn has_receiver(kind: Kind) -> bool {
    matches!(kind, Kind::Method | Kind::Getter | Kind::Constructor)
}

pub(super) fn expected_params(m: &Member) -> usize {
    usize::from(has_receiver(m.kind)) + m.params.len()
}

pub(super) fn generate_layout(m: &Member, out: &mut Generated) {
    let sym = &m.symbol;
    let tag = mapped_tag_path(&m.ret);
    out.setup_calls.push(quote! {
        if !cls.extend_layout(&[(::std::sync::Arc::from(#sym), #tag)]) {
            return ::core::result::Result::Err(::vn_types::NativeError::failure("cannot extend frozen class"));
        }
    });
}

pub(super) struct BodyIdents {
    pub(super) body: Ident,
    pub(super) wrap: Ident,
    pub(super) fast_wrap: Ident,
    pub(super) method: Ident,
}

pub(super) fn synthetic_struct_ctor(m: &Member) -> Result<UserMember, String> {
    let mut params: Vec<Pat> = vec![syn::parse_quote!(__s_ctx), syn::parse_quote!(__s_this)];
    let mut sets = Vec::new();
    for (i, p) in m.params.iter().enumerate() {
        if p.is_rest {
            return Err("a native struct cannot have a rest field".to_string());
        }
        let ident = format_ident!("__s_f{}", i);
        let vid = format_ident!("__s_v{}", i);
        let name = &p.name;
        params.push(Pat::Ident(syn::PatIdent {
            attrs: Vec::new(),
            by_ref: None,
            mutability: None,
            ident: ident.clone(),
            subpat: None,
        }));
        sets.push(quote! {
            let #vid = ::vn_types::marshal::IntoVm::into_vm(#ident, __s_ctx);
            __s_ctx.set_field(__s_this, #name, #vid);
        });
    }
    let body: syn::Expr = syn::parse_quote!({
        #(#sets)*
        __s_this
    });
    Ok(UserMember {
        name: format_ident!("constructor"),
        params,
        body,
    })
}

pub(super) fn idents(prefix: &str, sym: &str) -> BodyIdents {
    let rust_sym = sym.trim_end_matches('$');
    BodyIdents {
        body: format_ident!("__vn_body_{}_{}", sanitize(prefix), sanitize(rust_sym)),
        wrap: format_ident!("__vn_wrap_{}_{}", sanitize(prefix), sanitize(rust_sym)),
        fast_wrap: format_ident!("__vn_fast_wrap_{}_{}", sanitize(prefix), sanitize(rust_sym)),
        method: format_ident!("__vn_body_{}_{}", sanitize(prefix), sanitize(rust_sym)),
    }
}

pub(super) fn generate(cx: &Cx, m: &Member, user: &UserMember, out: &mut Generated) {
    let prefix = cx.prefix;
    let module = cx.module;
    let ns = cx.ns;
    let class_name_str = cx.class_name;
    let sym = &m.symbol;
    let ids = idents(prefix, sym);
    let body_ident = &ids.body;
    let wrap_ident = &ids.wrap;

    let expected = expected_params(m);
    let has_ctx = user.params.len() == expected + 1;
    let mut names = user.params.iter();
    let ctx_pat: Pat = if has_ctx {
        names
            .next()
            .cloned()
            .unwrap_or_else(|| syn::parse_quote!(_ctx))
    } else {
        syn::parse_quote!(_ctx)
    };

    let mut sig_params: Vec<TS2> = Vec::new();
    let mut decode: Vec<TS2> = Vec::new();
    let mut call_args: Vec<TS2> = Vec::new();

    let mut arg_base = 0usize;
    let mut arg_base_is_dynamic = false;

    match m.kind {
        Kind::Method | Kind::Getter => {
            let recv = receiver_mapped(prefix);
            let pty = param_ty(&recv);
            let oty = owned_ty(&recv);
            let pat = names.next();
            sig_params.push(quote!(#pat: #pty));
            let b = format_ident!("__this");
            decode.push(quote! {
                let #b = <#oty as ::vn_types::marshal::FromVm>::from_vm(
                    ctx,
                    args.first().copied().unwrap_or(::vn_types::VmValue::null()),
                )?;
            });
            call_args.push(call_expr(&b, &recv));
            arg_base = 1;
        }
        Kind::Constructor => {
            let pat = names.next();
            sig_params.push(quote!(#pat: ::vn_types::VmValue));
            let b = format_ident!("__this");
            decode.push(quote! {
                let #b = args.first().copied().unwrap_or(::vn_types::VmValue::null());
            });
            call_args.push(quote!(#b));
            arg_base = 1;
        }
        Kind::Function => {}
        Kind::StaticMethod | Kind::StaticGetter | Kind::Property => {
            arg_base_is_dynamic = true;
        }
    }

    if arg_base_is_dynamic {
        let expected_len = m.params.len();
        decode.push(quote! {
            let arg_base = if args.len() > #expected_len {
                if let Some(&first) = args.first() {
                    if ctx.is_static_receiver(first) {
                        1usize
                    } else {
                        0usize
                    }
                } else {
                    0usize
                }
            } else {
                0usize
            };
        });
    }

    for (i, p) in m.params.iter().enumerate() {
        let pname = format_ident!("__p{}", i);
        let pat = names.next();
        let arg_idx_expr = if arg_base_is_dynamic {
            quote!(arg_base + #i)
        } else {
            let val = arg_base + i;
            quote!(#val)
        };
        if p.is_rest {
            sig_params.push(quote!(#pat: &[::vn_types::VmValue]));
            decode.push(quote! {
                let #pname: &[::vn_types::VmValue] =
                    if args.len() > #arg_idx_expr { &args[#arg_idx_expr..] } else { &[] };
            });
            call_args.push(quote!(#pname));
        } else {
            let pty = param_ty(&p.mapped);
            let oty = owned_ty(&p.mapped);
            sig_params.push(quote!(#pat: #pty));
            decode.push(quote! {
                let #pname = <#oty as ::vn_types::marshal::FromVm>::from_vm(
                    ctx,
                    args.get(#arg_idx_expr).copied().unwrap_or(::vn_types::VmValue::null()),
                )?;
            });
            call_args.push(call_expr(&pname, &p.mapped));
        }
    }

    let rty = ret_ty(&m.ret);
    let is_void = matches!(m.ret, Mapped::Void);
    let is_fn = m.kind == Kind::Function;
    let fallible = is_fn || m.fallible;

    let body = &user.body;
    let body_ret = if fallible {
        let inner = if is_void { quote!(()) } else { rty.clone() };
        quote!(::core::result::Result<#inner, ::vn_types::NativeError>)
    } else if is_void {
        quote!(())
    } else {
        rty.clone()
    };
    let body_value = if fallible {
        quote!({ ::vn_types::marshal::into_native_ret(#body) })
    } else {
        quote!({ #body })
    };
    out.bodies.push(quote! {
        #[allow(non_snake_case, unused_mut, unused_variables, clippy::all)]
        fn #body_ident(#ctx_pat: &mut dyn ::vn_types::NativeCtx, #(#sig_params),*) -> #body_ret
            #body_value
    });

    let call = quote!(#body_ident(ctx, #(#call_args),*));
    let call = if m.throws.is_empty() {
        call
    } else {
        let declared = &m.throws;
        quote!(#call.map_err(|__e| ::vn_types::NativeError::within(__e, &[#(#declared),*], #sym)))
    };
    let ret_encode = match (fallible, is_void) {
        (true, true) => quote! { #call?; Ok(::vn_types::VmValue::null()) },
        (true, false) => quote! {
            let __ret = #call?;
            Ok(::vn_types::marshal::IntoVm::into_vm(__ret, ctx))
        },
        (false, true) => quote! {
            #call;
            Ok(::vn_types::VmValue::null())
        },
        (false, false) => quote! {
            let __ret = #call;
            Ok(::vn_types::marshal::IntoVm::into_vm(__ret, ctx))
        },
    };

    out.wrappers.push(quote! {
        #[allow(non_snake_case)]
        pub fn #wrap_ident(
            ctx: &mut dyn ::vn_types::NativeCtx,
            args: &[::vn_types::VmValue],
        ) -> ::core::result::Result<::vn_types::VmValue, ::vn_types::NativeError> {
            #(#decode)*
            #ret_encode
        }
    });

    let (fast_wrapper, raw_func_val, sig_val) = super::fast::fast_path(cx, m, &ids);

    out.wrappers.push(fast_wrapper);

    let capability = m.capability.join(", ");
    if m.kind == Kind::Function {
        let fn_entry_ident = format_ident!(
            "__VN_OP_{}_{}",
            sanitize(prefix).to_uppercase(),
            sanitize(sym).to_uppercase()
        );
        out.generated_entry_idents.push(fn_entry_ident.clone());
        out.fn_entries.push(quote! {
            pub static #fn_entry_ident: ::vn_types::NativeOpEntry = ::vn_types::NativeOpEntry {
                module_id: #module,
                namespace_path: #ns,
                symbol_name: #sym,
                func_ptr: #wrap_ident as *const u8,
                raw_func_ptr: #raw_func_val,
                signature: #sig_val,
                entry_kind: ::vn_types::EntryKind::Function,
                flags: 0,
                capability: #capability,
            };
        });
    } else {
        let native = quote!(ctx.alloc_fn(#wrap_ident, #sym));
        let setup = match m.kind {
            Kind::Method => quote!(if !cls.add_method(#sym, #native) {
                return ::core::result::Result::Err(::vn_types::NativeError::failure("cannot define method on frozen class"));
            }),
            Kind::Getter => quote!(if !cls.add_getter(#sym, #native) {
                return ::core::result::Result::Err(::vn_types::NativeError::failure("cannot define getter on frozen class"));
            }),
            Kind::StaticMethod => quote!(if !cls.add_static(#sym, #native) {
                return ::core::result::Result::Err(::vn_types::NativeError::failure("cannot define static on frozen class"));
            }),
            Kind::StaticGetter => quote!(if !cls.add_static_getter(#sym, #native) {
                return ::core::result::Result::Err(::vn_types::NativeError::failure("cannot define static getter on frozen class"));
            }),
            Kind::Constructor => {
                quote!(if !cls.add_method(::vn_core::TokenKind::Constructor.as_str(), ctx.alloc_fn(#wrap_ident, ::vn_core::TokenKind::Constructor.as_str())) {
                    return ::core::result::Result::Err(::vn_types::NativeError::failure("cannot define constructor on frozen class"));
                })
            }
            Kind::Function | Kind::Property => unreachable!(),
        };
        match m.kind {
            Kind::Method | Kind::Constructor => out.method_setup.push(setup),
            Kind::Getter => out.accessor_setup.push(setup),
            Kind::StaticMethod | Kind::StaticGetter => out.setup_calls.push(setup),
            Kind::Function | Kind::Property => unreachable!(),
        }

        let mkind = match m.kind {
            Kind::Method => Some(quote!(InstanceMethod)),
            Kind::StaticMethod => Some(quote!(StaticMethod)),
            Kind::Getter => Some(quote!(InstanceGetter)),
            Kind::StaticGetter => Some(quote!(StaticGetter)),
            Kind::Constructor => Some(quote!(Constructor)),
            Kind::Property | Kind::Function => None,
        };
        if let Some(mkind) = mkind {
            let mentry_ident = format_ident!(
                "__VN_OPM_{}_{}",
                sanitize(prefix).to_uppercase(),
                sanitize(sym).to_uppercase()
            );
            out.generated_entry_idents.push(mentry_ident.clone());
            out.method_entries.push(quote! {
                pub static #mentry_ident: ::vn_types::NativeOpEntry = ::vn_types::NativeOpEntry {
                    module_id: #module,
                    namespace_path: #class_name_str,
                    symbol_name: #sym,
                    func_ptr: #wrap_ident as *const u8,
                    raw_func_ptr: #raw_func_val,
                    signature: #sig_val,
                    entry_kind: ::vn_types::EntryKind::#mkind,
                    flags: 0,
                    capability: #capability,
                };
            });
        }
    }
}
