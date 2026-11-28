use super::*;

impl<'a> Scopes<'a> {
    pub(super) fn assign_target(&mut self, id: ExprId, also_read: bool) {
        let arena = self.arena;
        match &arena.expr(id).kind {
            ExprKind::Identifier { name } => {
                self.write(*name);
                if also_read {
                    self.read(*name);
                }
            }
            ExprKind::Paren { expression } => self.assign_target(*expression, also_read),
            ExprKind::Member {
                object,
                property,
                computed,
                ..
            } => {
                self.assign_target(*object, true);
                if *computed {
                    self.expr(*property);
                }
            }
            ExprKind::Array { elements } => {
                for element in elements {
                    match element {
                        ArrayEl::Expr(inner) | ArrayEl::Spread(inner) => {
                            self.assign_target(*inner, also_read)
                        }
                        ArrayEl::Hole => {}
                    }
                }
            }
            ExprKind::Tuple { elements } => {
                for element in elements {
                    self.assign_target(*element, also_read);
                }
            }
            ExprKind::Object { properties, .. } => {
                for property in properties {
                    match property {
                        ObjectProp::Property { value, .. } => self.assign_target(*value, also_read),
                        ObjectProp::Spread { argument, .. } => {
                            self.assign_target(*argument, also_read)
                        }
                        ObjectProp::Method { .. } => {}
                    }
                }
            }
            ExprKind::IntLiteral { .. }
            | ExprKind::FloatLiteral { .. }
            | ExprKind::BigIntLiteral { .. }
            | ExprKind::DecimalLiteral { .. }
            | ExprKind::StrLiteral { .. }
            | ExprKind::CharLiteral { .. }
            | ExprKind::BoolLiteral { .. }
            | ExprKind::NullLiteral
            | ExprKind::Template { .. }
            | ExprKind::TaggedTemplate { .. }
            | ExprKind::Missing
            | ExprKind::This
            | ExprKind::Super
            | ExprKind::ImportMeta(_)
            | ExprKind::Unary { .. }
            | ExprKind::Update { .. }
            | ExprKind::Binary { .. }
            | ExprKind::Logical { .. }
            | ExprKind::Assign { .. }
            | ExprKind::Conditional { .. }
            | ExprKind::Call { .. }
            | ExprKind::New { .. }
            | ExprKind::Function { .. }
            | ExprKind::Arrow { .. }
            | ExprKind::Sequence { .. }
            | ExprKind::Await { .. }
            | ExprKind::Yield { .. }
            | ExprKind::Spread { .. }
            | ExprKind::Pipeline { .. }
            | ExprKind::Range { .. }
            | ExprKind::NonNull { .. }
            | ExprKind::Try { .. }
            | ExprKind::As { .. }
            | ExprKind::Satisfies { .. }
            | ExprKind::ClassExpr { .. }
            | ExprKind::Match { .. }
            | ExprKind::Is { .. }
            | ExprKind::With { .. }
            | ExprKind::MetaAccess { .. } => self.expr(id),
        }
    }
    pub(super) fn arguments(&mut self, args: &[Arg]) {
        for arg in args {
            match arg {
                Arg::Positional(value) | Arg::Spread(value) | Arg::Named { value, .. } => {
                    self.expr(*value)
                }
            }
        }
    }
    pub(super) fn properties(&mut self, properties: &[ObjectProp]) {
        for property in properties {
            match property {
                ObjectProp::Property { key, value, .. } => {
                    if let PropKey::Computed(key) = key {
                        self.expr(*key);
                    }
                    self.expr(*value);
                }
                ObjectProp::Method {
                    key,
                    params,
                    body,
                    return_type,
                    ..
                } => {
                    if let PropKey::Computed(key) = key {
                        self.expr(*key);
                    }
                    self.stack.push(HashMap::new());
                    self.params(params, false, false);
                    if let Some(ty) = return_type {
                        self.ty(ty);
                    }
                    self.stmt(*body);
                    self.stack.pop();
                }
                ObjectProp::Spread { argument, .. } => self.expr(*argument),
            }
        }
    }
    pub(super) fn match_pattern(&mut self, pattern: &MatchPattern) {
        match pattern {
            MatchPattern::Wildcard => {}
            MatchPattern::Literal(expression) => self.expr(*expression),
            MatchPattern::Identifier(name) => self.exempt(*name, 0, 0),
            MatchPattern::Tuple(items) => {
                for item in items {
                    self.match_pattern(item);
                }
            }
            MatchPattern::Array { elements, rest } => {
                for element in elements {
                    self.match_pattern(element);
                }
                if let vn_core::ast::SequenceRest::Bind(name) = rest {
                    self.exempt(*name, 0, 0);
                }
            }
            MatchPattern::Record { fields, .. } => self.match_fields(fields),
            MatchPattern::EnumVariant {
                enum_name, payload, ..
            } => {
                if let Some(name) = enum_name {
                    self.read_qualified(*name);
                }
                match payload {
                    VariantPayload::Positional(items) => {
                        for item in items {
                            self.match_pattern(item);
                        }
                    }
                    VariantPayload::Named { fields, .. } => self.match_fields(fields),
                }
            }
        }
    }
    pub(super) fn match_fields(&mut self, fields: &[vn_core::ast::MatchField]) {
        for field in fields {
            match &field.pattern {
                Some(inner) => self.match_pattern(inner),
                None => self.exempt(field.name, 0, 0),
            }
        }
    }
    pub fn expr(&mut self, id: ExprId) {
        let arena = self.arena;
        match &arena.expr(id).kind {
            ExprKind::Identifier { name } => self.read(*name),
            ExprKind::IntLiteral { .. }
            | ExprKind::FloatLiteral { .. }
            | ExprKind::BigIntLiteral { .. }
            | ExprKind::DecimalLiteral { .. }
            | ExprKind::StrLiteral { .. }
            | ExprKind::CharLiteral { .. }
            | ExprKind::BoolLiteral { .. }
            | ExprKind::NullLiteral
            | ExprKind::Missing
            | ExprKind::This
            | ExprKind::Super
            | ExprKind::ImportMeta(_) => {}
            ExprKind::Template { parts } => {
                for part in parts {
                    if let TemplatePart::Interpolation(inner) = part {
                        self.expr(*inner);
                    }
                }
            }
            ExprKind::TaggedTemplate { tag, template } => {
                self.expr(*tag);
                self.expr(*template);
            }
            ExprKind::Array { elements } => {
                for element in elements {
                    match element {
                        ArrayEl::Expr(inner) | ArrayEl::Spread(inner) => self.expr(*inner),
                        ArrayEl::Hole => {}
                    }
                }
            }
            ExprKind::Object { properties, .. } => self.properties(properties),
            ExprKind::Tuple { elements }
            | ExprKind::Sequence {
                expressions: elements,
            } => {
                for element in elements {
                    self.expr(*element);
                }
            }
            ExprKind::Unary { operand, .. } => self.expr(*operand),
            ExprKind::Update { operand, .. } => self.assign_target(*operand, true),
            ExprKind::Binary { left, right, .. }
            | ExprKind::Logical { left, right, .. }
            | ExprKind::Pipeline { left, right } => {
                self.expr(*left);
                self.expr(*right);
            }
            ExprKind::Assign { op, target, value } => {
                self.expr(*value);
                let compound = !matches!(op, vn_core::ast::AssignOp::Assign);
                self.assign_target(*target, compound);
            }
            ExprKind::Conditional {
                test,
                consequent,
                alternate,
            } => {
                self.expr(*test);
                self.expr(*consequent);
                self.expr(*alternate);
            }
            ExprKind::Member {
                object,
                property,
                computed,
                ..
            } => {
                self.expr(*object);
                if *computed {
                    self.expr(*property);
                }
            }
            ExprKind::Call {
                callee,
                type_args,
                args,
                ..
            }
            | ExprKind::New {
                callee,
                type_args,
                args,
            } => {
                self.expr(*callee);
                for ty in type_args {
                    self.ty(ty);
                }
                self.arguments(args);
            }
            ExprKind::Function {
                fn_id,
                params,
                return_type,
                body,
                ..
            } => {
                self.stack.push(HashMap::new());
                if let Some(name) = fn_id {
                    self.exempt(*name, 0, 0);
                }
                self.params(params, true, true);
                if let Some(ty) = return_type {
                    self.ty(ty);
                }
                self.stmt(*body);
                self.stack.pop();
            }
            ExprKind::Arrow {
                params,
                body,
                return_type,
                ..
            } => {
                self.stack.push(HashMap::new());
                self.params(params, true, true);
                if let Some(ty) = return_type {
                    self.ty(ty);
                }
                match body.as_ref() {
                    ArrowBody::Expr(inner) => self.expr(*inner),
                    ArrowBody::Block(block) => self.stmt(*block),
                }
                self.stack.pop();
            }
            ExprKind::Paren { expression }
            | ExprKind::Await {
                argument: expression,
            }
            | ExprKind::Spread {
                argument: expression,
            }
            | ExprKind::NonNull { expression }
            | ExprKind::Try { expression } => self.expr(*expression),
            ExprKind::Yield { argument } => {
                if let Some(argument) = argument {
                    self.expr(*argument);
                }
            }
            ExprKind::Range { start, end, .. } => {
                self.expr(*start);
                self.expr(*end);
            }
            ExprKind::As {
                expression,
                type_ann,
            }
            | ExprKind::Satisfies {
                expression,
                type_ann,
            }
            | ExprKind::Is {
                expression,
                type_ann,
            } => {
                self.expr(*expression);
                self.ty(type_ann);
            }
            ExprKind::ClassExpr { declaration } => self.class(declaration),
            ExprKind::Match { subject, cases } => {
                self.expr(*subject);
                for case in cases {
                    self.stack.push(HashMap::new());
                    self.match_pattern(&case.pattern);
                    if let Some(guard) = case.guard {
                        self.expr(guard);
                    }
                    match &case.body {
                        MatchBody::Expr(inner) => self.expr(*inner),
                        MatchBody::Block(block) => self.stmt(*block),
                    }
                    self.stack.pop();
                }
            }
            ExprKind::With { object, properties } => {
                self.expr(*object);
                self.properties(properties);
            }
            ExprKind::MetaAccess { target, .. } => self.expr(*target),
        }
    }
}
