use std::path::Path;
use std::sync::Arc;

use vn_core::ast::{AstArena, ExprId, ExprKind};
use vn_core::{Diagnostic, DiagnosticKind, ErrorCode};
use vn_lint::{TypeFacts, Unit};
use vn_modules::project::LintConfig;
use vn_sem::bind::BindResult;
use vn_sem::output::CheckResult;

use crate::{Analysis, Session, Source};

const MAX_FIX_PASSES: usize = 5;

#[derive(Clone, Debug, Default)]
pub struct LintPolicy {
    allowed: Vec<ErrorCode>,
    denied: Vec<ErrorCode>,
}

fn rule_codes(names: &[String], section: &str) -> Result<Vec<ErrorCode>, String> {
    names
        .iter()
        .map(|name| {
            vn_lint::rule_of(name)
                .ok_or_else(|| format!("`{name}` in [lint] {section} is not a lint rule"))
        })
        .collect()
}

impl LintPolicy {
    pub fn from_config(config: &LintConfig) -> Result<Self, String> {
        Ok(Self {
            allowed: rule_codes(&config.allow, "allow")?,
            denied: rule_codes(&config.deny, "deny")?,
        })
    }

    pub fn apply(&self, found: Vec<Diagnostic>) -> Vec<Diagnostic> {
        found
            .into_iter()
            .filter(|diagnostic| !self.allowed.contains(&diagnostic.code))
            .map(|mut diagnostic| {
                if self.denied.contains(&diagnostic.code) {
                    diagnostic.kind = DiagnosticKind::Error;
                }
                diagnostic
            })
            .collect()
    }
}

fn assigned_places(arena: &AstArena) -> Vec<ExprId> {
    let mut places = Vec::new();
    for node in arena.exprs() {
        match &node.kind {
            ExprKind::Assign { target, .. } => places.push(*target),
            ExprKind::Update { operand, .. } => places.push(*operand),
            ExprKind::Identifier { .. }
            | ExprKind::IntLiteral { .. }
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
            | ExprKind::Array { .. }
            | ExprKind::Object { .. }
            | ExprKind::Tuple { .. }
            | ExprKind::Unary { .. }
            | ExprKind::Binary { .. }
            | ExprKind::Logical { .. }
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
            | ExprKind::MetaAccess { .. } => {}
        }
    }
    places
}

fn member_objects(arena: &AstArena, place: ExprId, out: &mut Vec<ExprId>) {
    match &arena.expr(place).kind {
        ExprKind::Member { object, .. } => {
            out.push(*object);
            member_objects(arena, *object, out);
        }
        ExprKind::Paren { expression } => member_objects(arena, *expression, out),
        ExprKind::Identifier { .. }
        | ExprKind::IntLiteral { .. }
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
        | ExprKind::Array { .. }
        | ExprKind::Object { .. }
        | ExprKind::Tuple { .. }
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
        | ExprKind::MetaAccess { .. } => {}
    }
}

fn type_facts(
    arena: &AstArena,
    checked: &CheckResult,
    resolver: &crate::ModuleResolver,
) -> TypeFacts {
    let mut objects = Vec::new();
    for place in assigned_places(arena) {
        member_objects(arena, place, &mut objects);
    }
    let mut facts = TypeFacts::new();
    if objects.is_empty() {
        return facts;
    }
    let bind: &BindResult = &checked.bind;
    let mut table = Arc::clone(&bind.ty_table);
    let table = Arc::make_mut(&mut table);
    for object in objects {
        let Some(entry) = checked.expr_table.get(&object.index()) else {
            continue;
        };
        if entry.ty.is_unresolved() {
            continue;
        }
        if vn_sem::types::is_reference_object(&entry.ty, table, bind, resolver) {
            let range = arena.expr(object).range;
            facts.mark_reference(range.start.offset, range.end.offset);
        }
    }
    facts
}

fn first_syntax_error(analysis: &Analysis) -> Option<String> {
    analysis
        .syntax
        .iter()
        .find(|diagnostic| diagnostic.kind == DiagnosticKind::Error)
        .map(|error| {
            format!(
                "{}:{}: {}",
                error.range.start.line, error.range.start.column, error.message
            )
        })
}

impl Session {
    pub fn lint_policy(&self) -> Result<LintPolicy, String> {
        LintPolicy::from_config(&self.project().manifest.lint)
    }

    pub fn lint_admits(&self, path: &Path) -> bool {
        let project = self.project();
        let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        let relative = absolute
            .strip_prefix(&project.root)
            .unwrap_or(&absolute)
            .to_string_lossy()
            .into_owned();
        project.manifest.lint.files().admits(&relative)
    }

    pub fn lint_analysis(
        &self,
        source: &Source,
        analysis: &Analysis,
        policy: &LintPolicy,
    ) -> Result<Vec<Diagnostic>, String> {
        if let Some(message) = first_syntax_error(analysis) {
            return Err(message);
        }
        let facts = type_facts(&analysis.parsed.arena, &analysis.checked, self.resolver());
        let unit = Unit {
            text: source.text(),
            filename: source.path(),
            program: &analysis.parsed.program,
            arena: &analysis.parsed.arena,
            interner: &analysis.parsed.interner,
            trivia: &analysis.tokens.trivia,
        };
        Ok(policy.apply(vn_lint::lint(&unit, &facts)))
    }

    pub fn lint(&self, source: &Source, policy: &LintPolicy) -> Result<Vec<Diagnostic>, String> {
        let analysis = self.analyze(source);
        self.lint_analysis(source, &analysis, policy)
    }

    pub fn fix(
        &self,
        path: &str,
        text: &str,
        policy: &LintPolicy,
    ) -> Result<(String, usize), String> {
        let mut text = text.to_owned();
        let mut total = 0;
        for _ in 0..MAX_FIX_PASSES {
            let found = self.lint(&Source::virtual_source(path, &text), policy)?;
            let (rewritten, applied) = vn_lint::apply_fixes(&text, &found);
            if applied == 0 {
                break;
            }
            total += applied;
            text = rewritten;
        }
        Ok((text, total))
    }
}
