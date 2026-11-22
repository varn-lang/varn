use crate::error::{PResult, ParseError};
use crate::stream::TokenStream;
use vn_core::ast::ExportSpecifier;
use vn_core::ast::{Decorator, ExportDecl, StmtKind};
use vn_core::TokenKind;

pub fn parse_export_decl(s: &mut TokenStream, decorators: Vec<Decorator>) -> PResult<ExportDecl> {
    let range = s.range();
    s.expect(TokenKind::Export)?;
    let is_declare = s.eat(TokenKind::Declare);

    if s.check(TokenKind::Type) && s.peek_kind(1) == TokenKind::LBrace {
        if !decorators.is_empty() {
            return Err(ParseError::invalid(
                "decorators are not supported on type exports".to_owned(),
            ));
        }
        s.advance();
        s.advance();
        while !s.check(TokenKind::RBrace) && !s.is_eof() {
            s.advance();
            if s.eat(TokenKind::As) {
                s.advance();
            }
            s.eat(TokenKind::Comma);
        }
        s.expect(TokenKind::RBrace)?;
        let source = if s.eat(TokenKind::From) {
            let text = s.consume_str();
            Some(s.interner.intern(&text))
        } else {
            None
        };
        s.eat_semicolon();
        let full_range = s.span_from(range);
        return Ok(ExportDecl::Named {
            ast_id: s.next_ast_id(),
            specifiers: vec![],
            source,
            range: full_range,
        });
    }

    if s.check(TokenKind::Default) {
        return Err(ParseError::invalid(
            vn_core::ForeignForm::ExportDefault.message(),
        ));
    }

    if s.eat(TokenKind::Star) {
        if !decorators.is_empty() {
            return Err(ParseError::invalid(
                "decorators are not supported on export all".to_owned(),
            ));
        }
        let alias = if s.eat(TokenKind::As) {
            Some(s.consume_lexeme())
        } else {
            None
        };
        s.expect(TokenKind::From)?;
        let source = s.consume_str();
        let source = s.interner.intern(&source);
        let full_range = s.span_from(range);
        return Ok(ExportDecl::All {
            ast_id: s.next_ast_id(),
            source,
            alias,
            range: full_range,
        });
    }

    if s.check(TokenKind::LBrace) {
        if !decorators.is_empty() {
            return Err(ParseError::invalid(
                "decorators are not supported on named exports".to_owned(),
            ));
        }
        s.advance();
        let mut specifiers = vec![];
        while !s.check(TokenKind::RBrace) && !s.is_eof() {
            let spec_range = s.range();
            let local = s.consume_lexeme();
            let exported = if s.eat(TokenKind::As) {
                s.consume_lexeme()
            } else {
                local
            };
            let full_spec_range = s.span_from(spec_range);
            specifiers.push(ExportSpecifier {
                local,
                exported,
                range: full_spec_range,
            });
            if !s.eat(TokenKind::Comma) {
                break;
            }
        }
        s.expect(TokenKind::RBrace)?;
        let source = if s.eat(TokenKind::From) {
            let text = s.consume_str();
            Some(s.interner.intern(&text))
        } else {
            None
        };
        let full_range = s.span_from(range);
        return Ok(ExportDecl::Named {
            ast_id: s.next_ast_id(),
            specifiers,
            source,
            range: full_range,
        });
    }

    let has_outer_decorators = !decorators.is_empty();
    let decl = if is_declare {
        match super::super::stmt_decls::try_parse_decl_stmt_mode(
            s,
            s.kind(),
            s.peek_kind(1),
            decorators,
            true,
        ) {
            Some(Ok(stmt)) => stmt,
            Some(Err(e)) => return Err(e),
            None => {
                return Err(ParseError::invalid(
                    "Expected declaration after `export declare`".to_owned(),
                ))
            }
        }
    } else {
        match super::super::stmt_decls::try_parse_decl_stmt_mode(
            s,
            s.kind(),
            s.peek_kind(1),
            decorators,
            false,
        ) {
            Some(Ok(stmt)) => stmt,
            Some(Err(e)) => return Err(e),
            None => {
                if has_outer_decorators {
                    return Err(ParseError::invalid(
                        "expected a declaration after decorators".to_owned(),
                    ));
                }
                super::super::stmts::parse_stmt_or_decl_inner(s)?
            }
        }
    };
    if let StmtKind::Decl(d) = s.arena.stmt(decl).kind.clone() {
        let full_range = s.span_from(range);
        return Ok(ExportDecl::Decl {
            ast_id: s.next_ast_id(),
            declaration: d,
            range: full_range,
        });
    }

    Err(ParseError::invalid(
        "Expected declaration after `export`".to_owned(),
    ))
}
