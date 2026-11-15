use proc_macro2::TokenStream as TS2;
use quote::quote;
use syn::Ident;

use crate::contract_members::{Kind, Member};

#[derive(Clone)]
pub(crate) enum Mapped {
    Int,
    Float,
    Bool,
    Char,
    Str,

    StrRecv,
    Array,
    Dynamic,
    Void,
    Opt(Box<Mapped>),
}

impl Mapped {
    pub(crate) fn of(ty: &vn_contract::ContractTy) -> Self {
        use vn_contract::ContractTy as T;
        match ty {
            T::Num(vn_core::NumKind::Int(_)) => Self::Int,
            T::Num(vn_core::NumKind::Float(_)) => Self::Float,
            T::Bool => Self::Bool,
            T::Char => Self::Char,
            T::Str => Self::Str,
            T::Void => Self::Void,
            T::Opt(inner) => Self::Opt(Box::new(Self::of(inner))),
            T::Array(_) => Self::Array,
            T::Null
            | T::Never
            | T::BigInt
            | T::Decimal
            | T::Dynamic
            | T::Tuple(_)
            | T::Union(_)
            | T::Generic(..)
            | T::Named(_)
            | T::Fn
            | T::Other => Self::Dynamic,
        }
    }
}

fn primitive_mapped(name: &str) -> Option<Mapped> {
    use vn_core::LangPrimitive as P;
    match P::from_str(name)? {
        P::Num(vn_core::NumKind::Int(_)) => Some(Mapped::Int),
        P::Num(vn_core::NumKind::Float(_)) => Some(Mapped::Float),
        P::Bool => Some(Mapped::Bool),
        P::Char => Some(Mapped::Char),
        P::Str => Some(Mapped::Str),
        P::Void => Some(Mapped::Void),
        P::Null | P::BigInt | P::Decimal | P::Never | P::Dynamic | P::Error => None,
    }
}

fn mapped_kind(m: &Mapped) -> Option<TS2> {
    match m {
        Mapped::Int => Some(quote! { Int }),
        Mapped::Float => Some(quote! { Float }),
        Mapped::Bool => Some(quote! { Bool }),
        Mapped::Char => Some(quote! { Char }),
        Mapped::Str | Mapped::StrRecv => Some(quote! { Str }),
        Mapped::Array => Some(quote! { Array }),
        Mapped::Dynamic | Mapped::Void | Mapped::Opt(_) => None,
    }
}

pub(super) fn mapped_tag_path(m: &Mapped) -> TS2 {
    let (inner, nullable) = match m {
        Mapped::Opt(inner) => (inner.as_ref(), true),
        Mapped::Int
        | Mapped::Float
        | Mapped::Bool
        | Mapped::Char
        | Mapped::Str
        | Mapped::StrRecv
        | Mapped::Array
        | Mapped::Dynamic
        | Mapped::Void => (m, false),
    };
    match (mapped_kind(inner), nullable) {
        (Some(name), false) => {
            quote! { Some(::vn_core::FieldKind::exact(::vn_core::RuntimeKind::#name)) }
        }
        (Some(name), true) => {
            quote! { Some(::vn_core::FieldKind::nullable(::vn_core::RuntimeKind::#name)) }
        }
        (None, false | true) => quote! { None },
    }
}

pub(super) fn receiver_mapped(class: &str) -> Mapped {
    if class == vn_core::BuiltinType::Array.name() {
        return Mapped::Array;
    }
    match primitive_mapped(class) {
        Some(Mapped::Str) => Mapped::StrRecv,
        Some(m) => m,
        None => Mapped::Dynamic,
    }
}

pub(super) fn param_ty(m: &Mapped) -> TS2 {
    match m {
        Mapped::Int => quote!(i64),
        Mapped::Float => quote!(f64),
        Mapped::Bool => quote!(bool),
        Mapped::Char => quote!(char),
        Mapped::Str => quote!(&str),

        Mapped::StrRecv => quote!(&::vn_types::VnStr),
        Mapped::Array => quote!(::vn_types::VnArray),
        Mapped::Dynamic => quote!(::vn_types::VmValue),
        Mapped::Void => quote!(()),
        Mapped::Opt(inner) => {
            let i = param_ty(inner);
            quote!(::core::option::Option<#i>)
        }
    }
}

pub(super) fn owned_ty(m: &Mapped) -> TS2 {
    match m {
        Mapped::Str | Mapped::StrRecv => quote!(::vn_types::VnStr),
        Mapped::Opt(inner) => {
            let i = owned_ty(inner);
            quote!(::core::option::Option<#i>)
        }
        Mapped::Int
        | Mapped::Float
        | Mapped::Bool
        | Mapped::Char
        | Mapped::Array
        | Mapped::Dynamic
        | Mapped::Void => param_ty(m),
    }
}

pub(super) fn ret_ty(m: &Mapped) -> TS2 {
    match m {
        Mapped::Str => quote!(impl ::vn_types::marshal::IntoVm + use<Self>),
        Mapped::Array => quote!(::std::vec::Vec<::vn_types::VmValue>),
        Mapped::Opt(inner) => {
            let i = ret_ty(inner);
            quote!(::core::option::Option<#i>)
        }
        Mapped::Int
        | Mapped::Float
        | Mapped::Bool
        | Mapped::Char
        | Mapped::StrRecv
        | Mapped::Dynamic
        | Mapped::Void => param_ty(m),
    }
}

pub(super) fn call_expr(binding: &Ident, m: &Mapped) -> TS2 {
    match m {
        Mapped::Str => quote!(#binding.as_str()),

        Mapped::StrRecv => quote!(&#binding),
        Mapped::Opt(inner) if matches!(**inner, Mapped::Str) => {
            quote!(#binding.as_ref().map(|s| s.as_str()))
        }
        Mapped::Int
        | Mapped::Float
        | Mapped::Bool
        | Mapped::Char
        | Mapped::Array
        | Mapped::Dynamic
        | Mapped::Void
        | Mapped::Opt(_) => quote!(#binding),
    }
}

pub(super) fn map_to_arg_type_token(m: &Mapped) -> TS2 {
    match m {
        Mapped::Int => quote!(::vn_types::ArgType::Int),
        Mapped::Float => quote!(::vn_types::ArgType::Float),
        Mapped::Bool => quote!(::vn_types::ArgType::Bool),
        Mapped::Char => quote!(::vn_types::ArgType::Char),
        Mapped::Str | Mapped::StrRecv => quote!(::vn_types::ArgType::Str),
        Mapped::Array => quote!(::vn_types::ArgType::Generic),
        Mapped::Dynamic => quote!(::vn_types::ArgType::Generic),
        Mapped::Void => quote!(::vn_types::ArgType::Void),
        Mapped::Opt(_) => quote!(::vn_types::ArgType::Generic),
    }
}

pub(super) fn is_scalar(m: &Mapped) -> bool {
    matches!(
        m,
        Mapped::Int | Mapped::Float | Mapped::Bool | Mapped::Char | Mapped::Void
    )
}

pub(super) fn is_fast_eligible(m: &Member) -> bool {
    if m.fallible {
        return false;
    }
    if !matches!(m.kind, Kind::Function | Kind::StaticMethod) {
        return false;
    }
    if !is_scalar(&m.ret) {
        return false;
    }
    for p in &m.params {
        if p.is_rest || !is_scalar(&p.mapped) {
            return false;
        }
    }
    true
}

pub(super) fn signature_token(m: &Member) -> TS2 {
    let ret_token = map_to_arg_type_token(&m.ret);
    let mut param_tokens = Vec::new();

    if m.kind == Kind::Method {
        param_tokens.push(quote!(::vn_types::ArgType::Generic));
    }

    for p in m.params.iter().take(7) {
        param_tokens.push(map_to_arg_type_token(&p.mapped));
    }
    while param_tokens.len() < 7 {
        param_tokens.push(quote!(::vn_types::ArgType::Void));
    }
    let count = (m.params.len() + if m.kind == Kind::Method { 1 } else { 0 }) as u8;
    quote! {
        ::vn_types::SignatureDescriptor {
            return_type: #ret_token,
            param_count: #count,
            param_types: [ #(#param_tokens),* ],
        }
    }
}

pub(super) fn default_value_token(m: &Mapped) -> TS2 {
    match m {
        Mapped::Dynamic => quote!(::vn_types::VmValue::null()),
        Mapped::Int
        | Mapped::Float
        | Mapped::Bool
        | Mapped::Char
        | Mapped::Str
        | Mapped::StrRecv
        | Mapped::Array
        | Mapped::Void
        | Mapped::Opt(_) => quote!(Default::default()),
    }
}
