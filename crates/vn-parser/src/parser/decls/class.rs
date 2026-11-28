use super::class_member::parse_class_member;
use crate::error::{PResult, ParseError};
use crate::expressions::parse_new_callee_expr;
use crate::stream::TokenStream;
use crate::types::{parse_type, parse_type_args, parse_type_params};
use vn_core::ast::operators::Modifiers;
use vn_core::ast::{ClassDecl, ClassMember, Decorator, StmtKind};
use vn_core::TokenKind;

pub fn parse_class_decl(
    s: &mut TokenStream,
    decorators: Vec<Decorator>,
    is_declare: bool,
) -> PResult<ClassDecl> {
    let range = s.range();
    let is_abstract = s.eat(TokenKind::Abstract);
    s.expect(TokenKind::Class)?;

    let (id, id_offset) = if s.check(TokenKind::Identifier) {
        let id_offset = s.token().range.start.offset;
        let id = s.consume_name();
        (Some(id), id_offset)
    } else {
        (None, 0)
    };
    let type_params = if s.check(TokenKind::LAngle) {
        parse_type_params(s)?
    } else {
        vec![]
    };

    let primary_params = if s.check(TokenKind::LParen) {
        Some(super::super::params::parse_params(s)?)
    } else {
        None
    };

    let super_class = if s.eat(TokenKind::Extends) {
        Some(parse_new_callee_expr(s)?)
    } else {
        None
    };
    let super_type_args = if s.check(TokenKind::LAngle) {
        parse_type_args(s)?
    } else {
        vec![]
    };

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
    let body = {
        let mut parsed = vec![];
        while !s.check(TokenKind::RBrace) && !s.is_eof() {
            while s.eat(TokenKind::Semicolon) {}
            let member_start = s.range();
            let doc = super::class_member::member_doc(s);
            if s.check(TokenKind::RBrace) {
                break;
            }
            parsed.push(parse_class_member(s, is_declare, member_start, doc)?);
        }
        parsed
    };
    s.expect(TokenKind::RBrace)?;

    let full_range = s.span_from(range);
    let body = if let Some(params) = primary_params {
        if body
            .iter()
            .any(|m| matches!(m, ClassMember::Constructor { .. }))
        {
            return Err(ParseError::invalid(
                "a class cannot have both a primary constructor and an explicit constructor: use one form".to_owned(),
            ));
        }
        let empty_body = s.stmt(full_range, StmtKind::Block { stmts: vec![] });
        let mut desugared = Vec::with_capacity(body.len() + 1);
        desugared.push(ClassMember::Constructor {
            params,
            body: empty_body,
            decorators: vec![],
            doc: None,
            range: full_range,
        });
        desugared.extend(body);
        desugared
    } else {
        body
    };

    Ok(ClassDecl {
        id,
        ast_id: s.next_ast_id(),
        id_offset,
        type_params,
        super_class,
        super_type_args,
        implements,
        body,
        modifiers: Modifiers {
            is_abstract,
            is_declare,
            ..Default::default()
        },
        decorators,
        doc: None,
        range: full_range,
        is_struct: false,
    })
}
