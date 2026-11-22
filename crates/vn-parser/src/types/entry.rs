use crate::error::PResult;
use crate::stream::TokenStream;
use vn_core::ast::TypeNode;
use vn_core::{TokenKind, TypeKind};

pub fn parse_type(s: &mut TokenStream) -> PResult<TypeNode> {
    let start = s.range();

    if s.check(TokenKind::Identifier) && s.peek_kind(1) == TokenKind::Is {
        let param_name = s.consume_lexeme();
        s.advance();
        let target_ty = parse_type(s)?;
        let full_range = s.span_from(start);
        return Ok(s.type_node(
            full_range,
            TypeKind::TypePredicate {
                parameter_name: param_name,
                target_type: Box::new(target_ty),
            },
        ));
    }

    let ty = super::combinators::parse_union_type(s)?;

    if s.eat(TokenKind::Extends) {
        super::combinators::parse_union_type(s)?;
        s.expect(TokenKind::Question)?;
        parse_type(s)?;
        s.expect(TokenKind::Colon)?;
        parse_type(s)?;
        s.push_error(
            vn_core::ForeignForm::ConditionalType.message(),
            s.span_from(start),
        );
        return Ok(ty);
    }

    Ok(ty)
}
