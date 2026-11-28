use crate::naming;
use crate::scope::{Kind, Scopes};
use crate::Source;
use std::collections::HashMap;
use vn_core::ast::{AssignOp, AstArena, ExprId, ExprKind, Program, StmtKind};
use vn_core::{AtomInterner, Diagnostic, ErrorCode, Suggestion, Trivia};

pub fn run(
    source: &Source<'_>,
    program: &Program,
    arena: &AstArena,
    interner: &AtomInterner,
    trivia: &[Trivia],
) -> Vec<Diagnostic> {
    let mut scopes = Scopes::new(arena, interner);
    scopes.program(&program.body);
    let mut found = Vec::new();
    bindings(source, &scopes, interner, &mut found);
    prefer_const(source, &scopes, &mut found);
    empty_catch(source, arena, trivia, &mut found);
    constant_condition(source, arena, &mut found);
    self_assign(source, arena, &mut found);
    found
}

fn bindings(
    source: &Source<'_>,
    scopes: &Scopes<'_>,
    interner: &AtomInterner,
    found: &mut Vec<Diagnostic>,
) {
    let mut last_read: HashMap<usize, usize> = HashMap::new();
    for binding in &scopes.bindings {
        if let (Some((function, position)), true) = (binding.param_of, binding.reads > 0) {
            let entry = last_read.entry(function).or_insert(position);
            *entry = (*entry).max(position);
        }
    }
    for binding in &scopes.bindings {
        let name = interner.resolve(binding.name);
        let located = binding.end > binding.start;
        let written = name
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$');
        if !located || !written || binding.kind == Kind::Exempt {
            continue;
        }
        let range = source.range(binding.start, binding.end);
        let after_last_read = binding.param_of.is_none_or(|(function, position)| {
            last_read.get(&function).is_none_or(|last| position > *last)
        });
        let unused = binding.reads == 0
            && !binding.exported
            && !name.starts_with('_')
            && after_last_read
            && !matches!(binding.kind, Kind::Exempt);
        if unused {
            let message = match binding.kind {
                Kind::Import => format!("`{name}` is imported but never used"),
                Kind::Param => format!("the parameter `{name}` is never used"),
                Kind::Function => format!("the function `{name}` is never used"),
                Kind::Class => format!("the class `{name}` is never used"),
                Kind::Let | Kind::Const | Kind::Exempt => {
                    format!("`{name}` is declared but never read")
                }
            };
            let mut diagnostic =
                Diagnostic::warning(ErrorCode::LintUnusedBinding, message).with_range(range);
            if let Some(fix) = unused_fix(source, scopes, binding, name, range) {
                diagnostic = diagnostic.with_suggestion(fix);
            }
            found.push(diagnostic);
            continue;
        }
        let acceptable = match binding.kind {
            Kind::Let | Kind::Const | Kind::Param | Kind::Function => {
                naming::is_value_name(name)
                    || naming::is_constant_name(name)
                    || naming::is_type_name(name)
            }
            Kind::Class => naming::is_type_name(name),
            Kind::Import | Kind::Exempt => true,
        };
        if !acceptable {
            let expected = match binding.kind {
                Kind::Class => "PascalCase",
                Kind::Let
                | Kind::Const
                | Kind::Param
                | Kind::Function
                | Kind::Import
                | Kind::Exempt => {
                    "camelCase, PascalCase or UPPER_SNAKE_CASE (no underscores inside a word)"
                }
            };
            found.push(
                Diagnostic::warning(
                    ErrorCode::LintNamingConvention,
                    format!("`{name}` should be {expected}"),
                )
                .with_range(range),
            );
        }
    }
}

fn prefer_const(source: &Source<'_>, scopes: &Scopes<'_>, found: &mut Vec<Diagnostic>) {
    for group in &scopes.groups {
        if !group.fixable || group.members.is_empty() {
            continue;
        }
        let eligible = group.members.iter().all(|index| {
            let binding = &scopes.bindings[*index];
            binding.kind == Kind::Let && binding.initialized && binding.writes == 0
        });
        let start = group.keyword_start as usize;
        if !eligible || source.text.get(start..start + 3) != Some("let") {
            continue;
        }
        let range = source.range(group.keyword_start, group.keyword_start + 3);
        found.push(
            Diagnostic::warning(
                ErrorCode::LintPreferConst,
                "this binding is never reassigned: use `const`",
            )
            .with_range(range)
            .with_suggestion(
                Suggestion::new("Replace `let` with `const`")
                    .with_range(range)
                    .replace("const"),
            ),
        );
    }
}

fn empty_catch(
    source: &Source<'_>,
    arena: &AstArena,
    trivia: &[Trivia],
    found: &mut Vec<Diagnostic>,
) {
    for node in arena.stmts() {
        let StmtKind::Try { catches, .. } = &node.kind else {
            continue;
        };
        for clause in catches {
            let body = arena.stmt(clause.body);
            let StmtKind::Block { stmts } = &body.kind else {
                continue;
            };
            let commented = trivia.iter().any(|item| {
                item.range.start.offset >= body.range.start.offset
                    && item.range.end.offset <= body.range.end.offset
            });
            if stmts.is_empty() && !commented {
                found.push(
                    Diagnostic::warning(
                        ErrorCode::LintEmptyCatch,
                        "this `catch` swallows the error: handle it or explain why with a comment",
                    )
                    .with_range(
                        source.range(clause.range.start.offset, body.range.start.offset + 1),
                    ),
                );
            }
        }
    }
}

fn is_literal(arena: &AstArena, id: ExprId) -> bool {
    matches!(
        arena.expr(id).kind,
        ExprKind::BoolLiteral { .. }
            | ExprKind::NullLiteral
            | ExprKind::IntLiteral { .. }
            | ExprKind::FloatLiteral { .. }
            | ExprKind::StrLiteral { .. }
            | ExprKind::CharLiteral { .. }
    )
}

fn is_true(arena: &AstArena, id: ExprId) -> bool {
    matches!(arena.expr(id).kind, ExprKind::BoolLiteral { value: true })
}

fn constant_condition(source: &Source<'_>, arena: &AstArena, found: &mut Vec<Diagnostic>) {
    let mut report = |id: ExprId| {
        let range = arena.expr(id).range;
        found.push(
            Diagnostic::warning(
                ErrorCode::LintConstantCondition,
                "this condition is a literal: one branch can never run",
            )
            .with_range(source.range(range.start.offset, range.end.offset)),
        );
    };
    for node in arena.stmts() {
        match &node.kind {
            StmtKind::If { test, .. } if is_literal(arena, *test) => report(*test),
            StmtKind::While { test, .. } | StmtKind::DoWhile { test, .. }
                if is_literal(arena, *test) && !is_true(arena, *test) =>
            {
                report(*test)
            }
            StmtKind::For {
                test: Some(test), ..
            } if is_literal(arena, *test) && !is_true(arena, *test) => report(*test),
            StmtKind::Block { .. }
            | StmtKind::Empty
            | StmtKind::Expr { .. }
            | StmtKind::Decl(_)
            | StmtKind::Error
            | StmtKind::If { .. }
            | StmtKind::While { .. }
            | StmtKind::DoWhile { .. }
            | StmtKind::For { .. }
            | StmtKind::ForIn { .. }
            | StmtKind::Return { .. }
            | StmtKind::Break { .. }
            | StmtKind::Continue { .. }
            | StmtKind::Throw { .. }
            | StmtKind::Try { .. }
            | StmtKind::Using { .. }
            | StmtKind::Labeled { .. }
            | StmtKind::Debugger => {}
        }
    }
    for node in arena.exprs() {
        if let ExprKind::Conditional { test, .. } = &node.kind
            && is_literal(arena, *test)
        {
            report(*test);
        }
    }
}

fn simple_reference(arena: &AstArena, id: ExprId) -> bool {
    match &arena.expr(id).kind {
        ExprKind::Identifier { .. } | ExprKind::This => true,
        ExprKind::Member {
            object,
            computed: false,
            optional: false,
            ..
        } => simple_reference(arena, *object),
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
        | ExprKind::Super
        | ExprKind::ImportMeta(_)
        | ExprKind::Array { .. }
        | ExprKind::Object { .. }
        | ExprKind::Tuple { .. }
        | ExprKind::Unary { .. }
        | ExprKind::Update { .. }
        | ExprKind::Binary { .. }
        | ExprKind::Logical { .. }
        | ExprKind::Assign { .. }
        | ExprKind::Conditional { .. }
        | ExprKind::Member { .. }
        | ExprKind::Call { .. }
        | ExprKind::New { .. }
        | ExprKind::Function { .. }
        | ExprKind::Arrow { .. }
        | ExprKind::Sequence { .. }
        | ExprKind::Paren { .. }
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
        | ExprKind::MetaAccess { .. } => false,
    }
}

fn self_assign(source: &Source<'_>, arena: &AstArena, found: &mut Vec<Diagnostic>) {
    for node in arena.exprs() {
        let ExprKind::Assign {
            op: AssignOp::Assign,
            target,
            value,
        } = &node.kind
        else {
            continue;
        };
        if !simple_reference(arena, *target) || !simple_reference(arena, *value) {
            continue;
        }
        let text = |id: ExprId| {
            let range = arena.expr(id).range;
            source
                .text
                .get(range.start.offset as usize..range.end.offset as usize)
                .map(|text| text.split_whitespace().collect::<String>())
        };
        if text(*target).is_some() && text(*target) == text(*value) {
            found.push(
                Diagnostic::warning(ErrorCode::LintSelfAssign, "this assigns a value to itself")
                    .with_range(source.range(node.range.start.offset, node.range.end.offset)),
            );
        }
    }
}

fn unused_fix(
    source: &Source<'_>,
    scopes: &Scopes<'_>,
    binding: &crate::scope::Binding,
    name: &str,
    range: vn_core::SourceRange,
) -> Option<Suggestion> {
    if let Some((declaration, position)) = binding.import {
        let info = &scopes.imports[declaration];
        let unused_all = info.specifiers.len() == 1
            || scopes
                .bindings
                .iter()
                .filter(|other| other.import.is_some_and(|(d, _)| d == declaration))
                .all(|other| other.reads == 0);
        let text = source.text;
        if unused_all {
            let mut end = info.end as usize;
            if text[end..].starts_with(';') {
                end += 1;
            }
            if text[end..].starts_with("\r\n") {
                end += 2;
            } else if text[end..].starts_with('\n') {
                end += 1;
            }
            return Some(
                Suggestion::new("Remove the unused import")
                    .with_range(source.range(info.start, end as u32))
                    .replace(""),
            );
        }
        let (start, end) = info.specifiers[position];
        let (mut from, mut to) = (start as usize, end as usize);
        let after = text[to..].trim_start();
        if after.starts_with(',') {
            to = text.len() - after.len() + 1;
            to += text[to..].len() - text[to..].trim_start().len();
        } else {
            let before = text[..from].trim_end();
            if before.ends_with(',') {
                from = before.len() - 1;
            }
        }
        return Some(
            Suggestion::new(format!("Remove `{name}` from the import"))
                .with_range(source.range(from as u32, to as u32))
                .replace(""),
        );
    }
    if binding.renamable
        && binding.writes == 0
        && matches!(binding.kind, Kind::Let | Kind::Const | Kind::Param)
    {
        return Some(
            Suggestion::new(format!("Rename to `_{name}`"))
                .with_range(range)
                .replace(format!("_{name}")),
        );
    }
    None
}
