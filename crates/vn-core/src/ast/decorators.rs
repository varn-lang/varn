use super::arena::{AstArena, ExprId};
use super::expr::{Arg, ExprKind};
use super::types::Decorator;
use crate::atom::{Atom, AtomInterner};
use crate::source::SourceRange;

pub enum BuiltinDecorator {
    Deprecated { message: Option<String> },
    Pure,
    Inline,
    Capability { domains: Vec<String> },
    Test,
    Throws { classes: Vec<String> },
    Decodes { op: String },
}

pub struct BuiltinMatch {
    pub range: SourceRange,
    pub result: Option<Result<BuiltinDecorator, String>>,
}

fn head_atom(arena: &AstArena, id: ExprId) -> Option<Atom> {
    match &arena.expr(id).kind {
        ExprKind::Identifier { name } => Some(*name),
        ExprKind::Call { .. }
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
        | ExprKind::Member { .. }
        | ExprKind::New { .. }
        | ExprKind::Function { .. }
        | ExprKind::Arrow { .. }
        | ExprKind::Sequence { .. }
        | ExprKind::Paren { .. }
        | ExprKind::Await { .. }
        | ExprKind::Spawn { .. }
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
        | ExprKind::MetaAccess { .. } => None,
    }
}

pub fn decorator_parts(arena: &AstArena, id: ExprId) -> Option<(Atom, Option<&[Arg]>)> {
    match &arena.expr(id).kind {
        ExprKind::Identifier { name } => Some((*name, None)),
        ExprKind::Call { callee, args, .. } => {
            Some((head_atom(arena, *callee)?, Some(args.as_slice())))
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
        | ExprKind::New { .. }
        | ExprKind::Function { .. }
        | ExprKind::Arrow { .. }
        | ExprKind::Sequence { .. }
        | ExprKind::Paren { .. }
        | ExprKind::Await { .. }
        | ExprKind::Spawn { .. }
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
        | ExprKind::MetaAccess { .. } => None,
    }
}

pub fn decorator_head(arena: &AstArena, interner: &AtomInterner, d: &Decorator) -> Option<Atom> {
    let (head, _) = decorator_parts(arena, d.expression)?;
    let name = interner.resolve(head);
    super::decorator_kind::BuiltinDecoratorKind::of(name)
        .is_some()
        .then_some(head)
}

pub fn is_builtin(arena: &AstArena, interner: &AtomInterner, d: &Decorator) -> bool {
    match_builtin(arena, interner, std::slice::from_ref(d))
        .into_iter()
        .any(|m| m.result.is_some())
}

pub fn wraps_at_runtime(
    arena: &AstArena,
    interner: &AtomInterner,
    decorators: &[Decorator],
) -> bool {
    match_builtin(arena, interner, decorators)
        .into_iter()
        .any(|m| m.result.is_none())
}

fn str_arg(arena: &AstArena, id: ExprId) -> Option<String> {
    match &arena.expr(id).kind {
        ExprKind::StrLiteral { value } => Some(value.clone()),
        ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BigIntLiteral { .. }
        | ExprKind::DecimalLiteral { .. }
        | ExprKind::CharLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::NullLiteral
        | ExprKind::Template { .. }
        | ExprKind::TaggedTemplate { .. }
        | ExprKind::Identifier { .. }
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
        | ExprKind::Member { .. }
        | ExprKind::Call { .. }
        | ExprKind::New { .. }
        | ExprKind::Function { .. }
        | ExprKind::Arrow { .. }
        | ExprKind::Sequence { .. }
        | ExprKind::Paren { .. }
        | ExprKind::Await { .. }
        | ExprKind::Spawn { .. }
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
        | ExprKind::MetaAccess { .. } => None,
    }
}

fn positional_strs(arena: &AstArena, args: &[Arg]) -> Option<Vec<String>> {
    let mut out = Vec::with_capacity(args.len());
    for a in args {
        let Arg::Positional(e) = a else {
            return None;
        };
        out.push(str_arg(arena, *e)?);
    }
    Some(out)
}

pub fn match_builtin(
    arena: &AstArena,
    interner: &AtomInterner,
    decorators: &[Decorator],
) -> Vec<BuiltinMatch> {
    let mut out = Vec::new();
    for d in decorators {
        let Some((head, args)) = decorator_parts(arena, d.expression) else {
            out.push(BuiltinMatch {
                range: d.range,
                result: None,
            });
            continue;
        };
        let head = interner.resolve(head);
        let Some(kind) = super::decorator_kind::BuiltinDecoratorKind::of(head) else {
            out.push(BuiltinMatch {
                range: d.range,
                result: None,
            });
            continue;
        };
        use super::decorator_kind::BuiltinDecoratorKind as K;
        let result = match kind {
            K::Deprecated => match args {
                None | Some([]) => Ok(BuiltinDecorator::Deprecated { message: None }),
                Some([single]) => match single {
                    Arg::Positional(e) => match str_arg(arena, *e) {
                        Some(msg) => Ok(BuiltinDecorator::Deprecated { message: Some(msg) }),
                        None => Err("`@deprecated` takes an optional string message".to_string()),
                    },
                    Arg::Spread(_) | Arg::Named { .. } => {
                        Err("`@deprecated` takes an optional string message".to_string())
                    }
                },
                Some(_) => Err("`@deprecated` takes an optional string message".to_string()),
            },
            K::Pure => match args {
                None | Some([]) => Ok(BuiltinDecorator::Pure),
                Some(_) => Err("`@pure` takes no arguments".to_string()),
            },
            K::Inline => match args {
                None | Some([]) => Ok(BuiltinDecorator::Inline),
                Some(_) => Err("`@inline` takes no arguments".to_string()),
            },
            K::Capability => match args {
                Some(a) if !a.is_empty() => match positional_strs(arena, a) {
                    Some(domains) => match crate::sandbox::Capability::canonicalize_all(domains) {
                        Ok(domains) => Ok(BuiltinDecorator::Capability { domains }),
                        Err(bad) => Err(bad),
                    },
                    None => Err("`@capability` takes one or more string domains".to_string()),
                },
                Some(_) | None => Err("`@capability` takes one or more string domains".to_string()),
            },
            K::Throws => match args {
                Some(a) if !a.is_empty() => {
                    let mut out: Vec<String> = Vec::new();
                    let mut malformed = false;
                    let mut unknown: Option<String> = None;
                    for arg in a {
                        let Arg::Positional(e) = arg else {
                            malformed = true;
                            break;
                        };
                        let Some(name) = head_atom(arena, *e) else {
                            malformed = true;
                            break;
                        };
                        let name = interner.resolve(name).to_string();
                        if crate::ErrorClass::of_name(&name).is_none() {
                            unknown = Some(name);
                            break;
                        }
                        if !out.contains(&name) {
                            out.push(name);
                        }
                    }
                    if let Some(name) = unknown {
                        Err(format!(
                            "`@throws({name})`: `{name}` is not an error class of the canonical tables"
                        ))
                    } else if malformed || out.is_empty() {
                        Err("`@throws` takes error class names".to_string())
                    } else {
                        Ok(BuiltinDecorator::Throws { classes: out })
                    }
                }
                Some(_) | None => Err("`@throws` takes error class names".to_string()),
            },
            K::Decodes => match args {
                Some([single]) => match single {
                    Arg::Positional(e) => match str_arg(arena, *e) {
                        Some(op) if crate::runtime_ops::ALL.contains(&op.as_str()) => {
                            Ok(BuiltinDecorator::Decodes { op })
                        }
                        Some(op) => Err(format!(
                            "`@decodes` unknown op `{op}`; expected one of: {}",
                            crate::runtime_ops::ALL.join(", ")
                        )),
                        None => Err("`@decodes` takes exactly one string op".to_string()),
                    },
                    Arg::Spread(_) | Arg::Named { .. } => {
                        Err("`@decodes` takes exactly one string op".to_string())
                    }
                },
                Some(_) | None => Err("`@decodes` takes exactly one string op".to_string()),
            },
            K::Test => match args {
                None | Some([]) => Ok(BuiltinDecorator::Test),
                Some(_) => Err("`@test` takes no arguments".to_string()),
            },
        };
        out.push(BuiltinMatch {
            range: d.range,
            result: Some(result),
        });
    }
    out
}
