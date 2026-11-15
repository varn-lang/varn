use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, parenthesized, Expr, Ident, Pat, Token};

pub(super) struct NativesInput {
    pub(super) sections: Vec<Section>,
}

pub(super) struct Section {
    pub(super) class: Option<String>,
    pub(super) members: Vec<UserMember>,
}

pub(super) struct UserMember {
    pub(super) name: Ident,
    pub(super) params: Vec<Pat>,
    pub(super) body: Expr,
}

impl UserMember {
    pub(super) fn key(&self) -> String {
        self.name.unraw().to_string()
    }
}

impl Parse for NativesInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut sections: Vec<Section> = Vec::new();
        while !input.is_empty() {
            if is_class_section(input) {
                let keyword: Ident = input.parse()?;
                debug_assert_eq!(keyword, "class");
                let class: Ident = input.parse()?;
                let inner;
                braced!(inner in input);
                sections.push(Section {
                    class: Some(class.unraw().to_string()),
                    members: members(&inner)?,
                });
            } else {
                let member = member(input)?;
                match sections.iter_mut().find(|s| s.class.is_none()) {
                    Some(section) => section.members.push(member),
                    None => sections.push(Section {
                        class: None,
                        members: vec![member],
                    }),
                }
            }
        }
        Ok(Self { sections })
    }
}

fn is_class_section(input: ParseStream) -> bool {
    let fork = input.fork();
    let Ok(keyword) = fork.call(Ident::parse_any) else {
        return false;
    };
    keyword == "class" && fork.peek(Ident::peek_any) && fork.peek2(syn::token::Brace)
}

fn members(input: ParseStream) -> syn::Result<Vec<UserMember>> {
    let mut out = Vec::new();
    while !input.is_empty() {
        out.push(member(input)?);
    }
    Ok(out)
}

fn member(input: ParseStream) -> syn::Result<UserMember> {
    let name = Ident::parse_any(input)?;
    let params = if input.peek(syn::token::Paren) {
        let inner;
        parenthesized!(inner in input);
        Punctuated::<Pat, Token![,]>::parse_terminated_with(&inner, Pat::parse_single)?
            .into_iter()
            .collect()
    } else {
        Vec::new()
    };
    input.parse::<Token![=>]>()?;
    let body: Expr = input.parse()?;
    let _ = input.parse::<Token![,]>();
    Ok(UserMember { name, params, body })
}
