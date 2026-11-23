use vn_core::ast::decorator_kind::{BuiltinDecoratorKind, DecoratorTarget};
use vn_core::ast::{Arg, AstArena, Decorator, ExprKind};
use vn_core::AtomInterner;

use crate::{ContractError, Kind};

fn named<'a>(
    decorator: &Decorator,
    arena: &'a AstArena,
    interner: &AtomInterner,
) -> Option<(BuiltinDecoratorKind, &'a [Arg])> {
    let ExprKind::Call { callee, args, .. } = &arena.expr(decorator.expression).kind else {
        return None;
    };
    let ExprKind::Identifier { name } = &arena.expr(*callee).kind else {
        return None;
    };
    BuiltinDecoratorKind::of(interner.resolve(*name)).map(|kind| (kind, args.as_slice()))
}

fn bare_kind(
    decorator: &Decorator,
    arena: &AstArena,
    interner: &AtomInterner,
) -> Option<BuiltinDecoratorKind> {
    let (head, args) = vn_core::ast::decorators::decorator_parts(arena, decorator.expression)?;
    if args.is_some() {
        return None;
    }
    BuiltinDecoratorKind::of(interner.resolve(head))
}

fn target_of(kind: Kind) -> DecoratorTarget {
    match kind {
        Kind::Function => DecoratorTarget::Function,
        Kind::Method | Kind::StaticMethod => DecoratorTarget::Method,
        Kind::Getter | Kind::StaticGetter => DecoratorTarget::Getter,
        Kind::Constructor => DecoratorTarget::Constructor,
        Kind::Property => DecoratorTarget::Property,
    }
}

pub(crate) fn check_placement(
    kind: Kind,
    decorators: &[Decorator],
    arena: &AstArena,
    interner: &AtomInterner,
) -> Result<(), ContractError> {
    let target = target_of(kind);
    for d in decorators {
        let found = vn_core::ast::decorators::decorator_parts(arena, d.expression)
            .and_then(|(head, _)| BuiltinDecoratorKind::of(interner.resolve(head)));
        let Some(found) = found else {
            continue;
        };
        if !found.supports(target) {
            return Err(ContractError::BadDecorator(format!(
                "`@{}` is only supported on functions, not on this contract member",
                found.name(),
            )));
        }
    }
    Ok(())
}

pub(crate) fn throws(
    kind: Kind,
    decorators: &[Decorator],
    arena: &AstArena,
    interner: &AtomInterner,
) -> Result<Vec<String>, ContractError> {
    check_placement(kind, decorators, arena, interner)?;
    let bad = || ContractError::BadDecorator("`@throws` takes error class names".to_string());
    let mut out = Vec::new();
    for d in decorators {
        if bare_kind(d, arena, interner) == Some(BuiltinDecoratorKind::Throws) {
            return Err(bad());
        }
        let Some((BuiltinDecoratorKind::Throws, args)) = named(d, arena, interner) else {
            continue;
        };
        if args.is_empty() {
            return Err(bad());
        }
        for arg in args {
            let Arg::Positional(arg) = arg else {
                return Err(bad());
            };
            let ExprKind::Identifier { name } = &arena.expr(*arg).kind else {
                return Err(bad());
            };
            let class = interner.resolve(*name).to_string();
            if vn_core::ErrorClass::of(&class).is_none() {
                return Err(ContractError::BadDecorator(format!(
                    "`@throws({class})`: `{class}` is not an error class of the canonical tables"
                )));
            }
            if !out.contains(&class) {
                out.push(class);
            }
        }
    }
    Ok(out)
}

pub(crate) fn capability(
    kind: Kind,
    decorators: &[Decorator],
    arena: &AstArena,
    interner: &AtomInterner,
) -> Result<Vec<String>, ContractError> {
    check_placement(kind, decorators, arena, interner)?;
    let bad = || {
        ContractError::BadDecorator("`@capability` takes one or more string domains".to_string())
    };
    let mut out = Vec::new();
    for d in decorators {
        if bare_kind(d, arena, interner) == Some(BuiltinDecoratorKind::Capability) {
            return Err(bad());
        }
        let Some((BuiltinDecoratorKind::Capability, args)) = named(d, arena, interner) else {
            continue;
        };
        if args.is_empty() {
            return Err(bad());
        }
        for arg in args {
            let Arg::Positional(arg) = arg else {
                return Err(bad());
            };
            let ExprKind::StrLiteral { value } = &arena.expr(*arg).kind else {
                return Err(bad());
            };
            if vn_core::sandbox::Capability::of(value).is_none() {
                return Err(ContractError::BadDecorator(format!(
                    "unknown capability `{value}`"
                )));
            }
            if !out.contains(value) {
                out.push(value.clone());
            }
        }
    }
    out.sort();
    Ok(out)
}
