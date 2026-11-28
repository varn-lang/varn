use super::class_member::parse_class_member;
use crate::error::{PResult, ParseError};
use crate::stream::TokenStream;
use crate::types::{parse_type, parse_type_params};
use vn_core::ast::operators::Modifiers;
use vn_core::ast::{ClassDecl, ClassMember, Decorator};
use vn_core::TokenKind;

pub fn parse_struct_decl(s: &mut TokenStream, decorators: Vec<Decorator>) -> PResult<ClassDecl> {
    let range = s.range();
    s.expect(TokenKind::Struct)?;

    let id_offset = s.token().range.start.offset;
    let id = s.expect_id()?;
    let type_params = if s.check(TokenKind::LAngle) {
        parse_type_params(s)?
    } else {
        vec![]
    };

    if s.check(TokenKind::Extends) {
        return Err(ParseError::invalid(
            "a struct cannot extend another type: it is a value, not a hierarchy",
        ));
    }
    let implements = if s.eat(TokenKind::Implements) {
        let mut impls = vec![parse_type(s)?];
        while s.eat(TokenKind::Comma) {
            impls.push(parse_type(s)?);
        }
        impls
    } else {
        vec![]
    };

    s.expect(TokenKind::LBrace)?;
    let mut body: Vec<ClassMember> = vec![];
    while !s.check(TokenKind::RBrace) && !s.is_eof() {
        while s.eat(TokenKind::Semicolon) || s.eat(TokenKind::Comma) {}
        if s.check(TokenKind::RBrace) {
            break;
        }
        let member_start = s.range_with_doc();
        match parse_class_member(s, false, member_start)? {
            ClassMember::Property {
                key,
                key_range,
                type_ann,
                init,
                modifiers,
                decorators,
                range: field_range,
            } if !modifiers.is_static => {
                if !decorators.is_empty() {
                    return Err(ParseError::invalid(
                        "decorators are not supported on struct fields".to_owned(),
                    ));
                }
                let repeated = body
                    .iter()
                    .any(|m| matches!(m, ClassMember::Field { key: other, .. } if *other == key));
                if repeated {
                    return Err(ParseError::invalid(format!(
                        "the struct already has a field named '{}'",
                        s.interner.resolve(key)
                    )));
                }
                let Some(type_ann) = type_ann else {
                    return Err(ParseError::invalid(format!(
                        "the field '{}' of a struct needs a type: write `name: Type`",
                        s.interner.resolve(key)
                    )));
                };
                body.push(ClassMember::Field {
                    key,
                    key_range,
                    type_ann,
                    default: init,
                    visibility: modifiers.visibility,
                    range: field_range,
                });
            }
            ClassMember::Constructor { .. } => {
                return Err(ParseError::invalid(
                    "a struct has no constructor: its fields, in order, are its constructor",
                ));
            }
            ClassMember::Setter { .. } => {
                return Err(ParseError::invalid(
                    "a struct is immutable: it cannot declare a setter".to_owned(),
                ));
            }
            member @ (ClassMember::Property { .. }
            | ClassMember::Method { .. }
            | ClassMember::Getter { .. }
            | ClassMember::StaticBlock { .. }
            | ClassMember::Field { .. }) => body.push(member),
        }
    }
    s.expect(TokenKind::RBrace)?;

    let full_range = s.span_from(range);

    Ok(ClassDecl {
        id: Some(id),
        ast_id: s.next_ast_id(),
        id_offset,
        type_params,
        super_class: None,
        super_type_args: vec![],
        implements,
        body,
        modifiers: Modifiers::default(),
        decorators,
        range: full_range,
        is_struct: true,
    })
}
