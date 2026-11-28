use super::TypeContext;
use crate::error::{PResult, ParseError};
use crate::stream::TokenStream;
use vn_core::ast::{QualifiedName, TypeNode};
use vn_core::{TokenKind, TypeKind};

fn starts_ternary(s: &TokenStream, ctx: TypeContext) -> bool {
    if ctx == TypeContext::Annotation || s.kind() != TokenKind::Question {
        return false;
    }
    let mut off = 1;
    if s.peek_kind(off) == TokenKind::Colon {
        return false;
    }
    let mut depth = 0usize;
    loop {
        let kind = s.peek_kind(off);
        let opens = matches!(
            kind,
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace
        );
        let closes = matches!(
            kind,
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace
        );
        if opens {
            depth += 1;
        } else if closes && depth > 0 {
            depth -= 1;
        } else if kind == TokenKind::Colon && depth == 0 {
            return true;
        } else if closes
            || kind == TokenKind::Semicolon
            || kind == TokenKind::EOF
            || kind == TokenKind::Eq
            || (depth == 0 && (kind == TokenKind::Comma || kind == TokenKind::Question))
            || kind.starts_statement()
        {
            return false;
        }
        off += 1;
    }
}

pub(crate) fn parse_array_type(s: &mut TokenStream, ctx: TypeContext) -> PResult<TypeNode> {
    let start = s.range();
    if s.check(TokenKind::Readonly) {
        s.advance();
        let inner = parse_array_type(s, ctx)?;
        let TypeKind::Array(element) = inner.kind else {
            return Err(ParseError::invalid(
                "`readonly` applies to an array type: write `readonly T[]`".to_owned(),
            ));
        };
        let name = s
            .interner
            .intern(vn_core::BuiltinType::ReadonlyArray.name());
        let full_range = s.span_from(start);
        return Ok(s.type_node(
            full_range,
            TypeKind::Generic(QualifiedName::single(name, start), vec![*element], ()),
        ));
    }
    let mut ty = super::primary::parse_primary_type(s, ctx)?;
    if matches!(ty.kind, TypeKind::Literal(_)) && s.check(TokenKind::LBracket) {
        return Err(ParseError::invalid(
            "literal types cannot be used as array element types or indexed".to_owned(),
        ));
    }

    loop {
        if s.check(TokenKind::LBracket) && s.peek_kind(1) == TokenKind::RBracket {
            s.advance();
            s.advance();
            let full_range = s.span_from(start);
            ty = s.type_node(full_range, TypeKind::Array(Box::new(ty)));
        } else if s.check(TokenKind::LBracket) && s.peek_kind(1) != TokenKind::RBracket {
            s.advance();
            let index = super::entry::parse_type(s)?;
            s.expect(TokenKind::RBracket)?;
            if matches!(index.kind, TypeKind::Literal(vn_core::TypeLiteral::Int(_))) {
                s.push_diagnostic(
                    vn_core::ErrorCode::InvalidTypeOperator,
                    "a type has no fixed length: write `T[]`, or a tuple `(T, T, T)` for a fixed set of values".to_owned(),
                    s.span_from(start),
                );
            } else {
                s.push_error(
                    vn_core::ForeignForm::IndexedAccessType.message(),
                    s.span_from(start),
                );
            }
        } else if s.check(TokenKind::Question) && !starts_ternary(s, ctx) {
            let q_range = s.range();
            s.advance();
            let full_range = s.span_from(start);
            let null_node = s.type_node(q_range, TypeKind::Primitive(vn_core::LangPrimitive::Null));
            ty = s.type_node(full_range, TypeKind::Union(vec![ty, null_node]));
        } else {
            break;
        }
    }
    Ok(ty)
}
