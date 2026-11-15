use vn_core::ast::decorator_kind::ContractDecorator;
use vn_core::ast::{Arg, AstArena, Decorator, ExprKind};
use vn_core::AtomInterner;

use crate::ContractError;

fn named<'a>(
    decorator: &Decorator,
    arena: &'a AstArena,
    interner: &AtomInterner,
) -> Option<(ContractDecorator, &'a [Arg])> {
    let ExprKind::Call { callee, args, .. } = &arena.expr(decorator.expression).kind else {
        return None;
    };
    let ExprKind::Identifier { name } = &arena.expr(*callee).kind else {
        return None;
    };
    ContractDecorator::of(interner.resolve(*name)).map(|kind| (kind, args.as_slice()))
}

pub(crate) fn throws(
    decorators: &[Decorator],
    arena: &AstArena,
    interner: &AtomInterner,
) -> Result<Vec<String>, ContractError> {
    let bad = || ContractError::BadDecorator("`@throws` takes error class names".to_string());
    let mut out = Vec::new();
    for d in decorators {
        let Some((ContractDecorator::Throws, args)) = named(d, arena, interner) else {
            continue;
        };
        for arg in args {
            let Arg::Positional(arg) = arg else {
                return Err(bad());
            };
            let ExprKind::Identifier { name } = &arena.expr(*arg).kind else {
                return Err(bad());
            };
            let class = interner.resolve(*name).to_string();
            if vn_core::ErrorClass::of_name(&class).is_none() {
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
    decorators: &[Decorator],
    arena: &AstArena,
    interner: &AtomInterner,
) -> Result<Option<String>, ContractError> {
    let bad =
        || ContractError::BadDecorator("`@permission` takes exactly one string domain".to_string());
    for d in decorators {
        let Some((ContractDecorator::Permission, args)) = named(d, arena, interner) else {
            continue;
        };
        let [Arg::Positional(arg)] = args else {
            return Err(bad());
        };
        let ExprKind::StrLiteral { value } = &arena.expr(*arg).kind else {
            return Err(bad());
        };
        return Ok(Some(value.clone()));
    }
    Ok(None)
}
