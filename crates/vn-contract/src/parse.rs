use vn_core::ast::{
    AstArena, ClassDecl, ClassMember, Decl, ExportDecl, ExprId, ExprKind, ExtensionMember,
    FunctionDecl, Param as AstParam, Pattern, StmtKind, TypeNode,
};
use vn_core::AtomInterner;

use crate::decorators::{capability, throws};
use crate::model::{ClassContract, ContractError, Kind, Member, ModuleContract, Param};
use crate::ContractTy;

pub fn parse(source: &str, name: &str) -> Result<ModuleContract, ContractError> {
    let (tokens, lexeme_buf, _) = vn_lexer::scan(source, name);
    let (program, interner, arena) =
        vn_parser::parse(tokens, lexeme_buf, name, AtomInterner::new())
            .map_err(|_| ContractError::Unparsable(format!("cannot parse contract `{name}`")))?;
    let mut contract = ModuleContract::default();
    for &stmt in &program.body {
        let StmtKind::Decl(decl) = &arena.stmt(stmt).kind else {
            continue;
        };
        let (declaration, exported) = match decl.as_ref() {
            Decl::Export(ExportDecl::Decl { declaration, .. }) => (declaration.as_ref(), true),
            Decl::Export(_) => {
                contract.has_code = true;
                continue;
            }
            Decl::Variable(_)
            | Decl::Function(_)
            | Decl::Class(_)
            | Decl::Interface(_)
            | Decl::TypeAlias(_)
            | Decl::Enum(_)
            | Decl::Namespace(_)
            | Decl::Import(_)
            | Decl::Extension(_) => (decl.as_ref(), false),
        };
        let (names, declared) = declare(declaration, &mut contract, &arena, &interner)?;
        if exported && !declared {
            contract.has_code = true;
            contract.code_exports.extend(names);
        }
    }
    contract.exports.sort();
    contract.exports.dedup();
    contract.code_exports.sort();
    contract.code_exports.dedup();
    Ok(contract)
}

fn declare(
    decl: &Decl,
    out: &mut ModuleContract,
    arena: &AstArena,
    interner: &AtomInterner,
) -> Result<(Vec<String>, bool), ContractError> {
    let name = |a: vn_core::Atom| interner.resolve(a).to_string();
    let (names, declared) = match decl {
        Decl::Function(f) => {
            out.functions.push(function_member(f, arena, interner)?);
            (vec![name(f.id)], f.modifiers.is_declare)
        }
        Decl::Class(c) => {
            if let Some(id) = c.id {
                out.classes.insert(
                    name(id),
                    ClassContract {
                        extends: c.super_class.and_then(|e| identifier(e, arena, interner)),
                        members: class_members(c, arena, interner)?,
                        is_struct: c.is_struct,
                    },
                );
            }
            (c.id.map(name).into_iter().collect(), c.modifiers.is_declare)
        }
        Decl::Interface(i) => (vec![name(i.id)], true),
        Decl::TypeAlias(t) => (vec![name(t.id)], true),
        Decl::Namespace(n) => (vec![name(n.id)], n.is_declare),
        Decl::Enum(e) => {
            out.enums.insert(name(e.id));
            (vec![name(e.id)], e.is_declare)
        }
        Decl::Variable(v) => (
            v.declarators
                .iter()
                .filter_map(|d| match &d.id {
                    Pattern::Identifier { name: n, .. } => Some(name(*n)),
                    Pattern::Array { .. }
                    | Pattern::Tuple { .. }
                    | Pattern::Object { .. }
                    | Pattern::Assignment { .. }
                    | Pattern::Rest { .. } => None,
                })
                .collect(),
            v.is_declare,
        ),
        Decl::Extension(ext) => (
            Vec::new(),
            ext.members.iter().all(
                |member| matches!(member, ExtensionMember::Method(f) if f.modifiers.is_declare),
            ),
        ),
        Decl::Import(_) | Decl::Export(_) => (Vec::new(), false),
    };
    out.exports.extend(names.iter().cloned());
    Ok((names, declared))
}

fn identifier(expr: ExprId, arena: &AstArena, interner: &AtomInterner) -> Option<String> {
    let ExprKind::Identifier { name } = &arena.expr(expr).kind else {
        return None;
    };
    Some(interner.resolve(*name).to_string())
}

fn ty(node: Option<&TypeNode>, interner: &AtomInterner, absent: ContractTy) -> ContractTy {
    node.map_or(absent, |n| ContractTy::from_node(n, interner))
}

fn params(params: &[AstParam], interner: &AtomInterner) -> Vec<Param> {
    params
        .iter()
        .map(|p| {
            let (name, rest) = match &p.pattern {
                Pattern::Identifier { name, .. } => (interner.resolve(*name).to_string(), false),
                Pattern::Rest { .. } => (String::new(), true),
                Pattern::Array { .. }
                | Pattern::Tuple { .. }
                | Pattern::Object { .. }
                | Pattern::Assignment { .. } => (String::new(), false),
            };
            Param {
                name,
                ty: ty(p.type_ann.as_ref(), interner, ContractTy::Dynamic),
                optional: p.is_optional,
                rest: p.is_rest || rest,
            }
        })
        .collect()
}

fn class_members(
    decl: &ClassDecl,
    arena: &AstArena,
    interner: &AtomInterner,
) -> Result<Vec<Member>, ContractError> {
    let mut out = Vec::new();
    let positional = decl.positional_constructor();
    if let Some(constructor) = &positional {
        out.push(Member {
            symbol: vn_core::TokenKind::Constructor.as_str().to_string(),
            kind: Kind::Constructor,
            params: params(&constructor.params, interner),
            ret: ContractTy::Dynamic,
            throws: Vec::new(),
            capability: Vec::new(),
        });
    }
    for m in &decl.body {
        let member = match m {
            ClassMember::Method {
                key,
                params: ps,
                return_type,
                modifiers,
                decorators,
                ..
            } => Member {
                symbol: interner.resolve(*key).to_string(),
                kind: if modifiers.is_static {
                    Kind::StaticMethod
                } else {
                    Kind::Method
                },
                params: params(ps, interner),
                ret: ty(return_type.as_ref(), interner, ContractTy::Void),
                throws: throws(Kind::Method, decorators, arena, interner)?,
                capability: capability(Kind::Method, decorators, arena, interner)?,
            },
            ClassMember::Getter {
                key,
                return_type,
                modifiers,
                decorators,
                ..
            } => Member {
                symbol: interner.resolve(*key).to_string(),
                kind: if modifiers.is_static {
                    Kind::StaticGetter
                } else {
                    Kind::Getter
                },
                params: Vec::new(),
                ret: ty(return_type.as_ref(), interner, ContractTy::Dynamic),
                throws: throws(Kind::Getter, decorators, arena, interner)?,
                capability: capability(Kind::Getter, decorators, arena, interner)?,
            },
            ClassMember::Property {
                key,
                type_ann,
                init: None,
                modifiers,
                decorators,
                ..
            } => Member {
                symbol: interner.resolve(*key).to_string(),
                kind: if modifiers.is_static {
                    Kind::StaticGetter
                } else if modifiers.is_readonly {
                    Kind::Getter
                } else {
                    Kind::Property
                },
                params: Vec::new(),
                ret: ty(type_ann.as_ref(), interner, ContractTy::Dynamic),
                throws: throws(Kind::Property, decorators, arena, interner)?,
                capability: capability(Kind::Property, decorators, arena, interner)?,
            },
            ClassMember::Constructor {
                params: ps,
                decorators,
                ..
            } => Member {
                symbol: vn_core::TokenKind::Constructor.as_str().to_string(),
                kind: Kind::Constructor,
                params: params(ps, interner),
                ret: ContractTy::Dynamic,
                throws: throws(Kind::Constructor, decorators, arena, interner)?,
                capability: capability(Kind::Constructor, decorators, arena, interner)?,
            },
            ClassMember::Property { .. }
            | ClassMember::Setter { .. }
            | ClassMember::StaticBlock { .. }
            | ClassMember::Field { .. } => continue,
        };
        out.push(member);
    }
    if let Some(constructor) = &positional {
        for p in &constructor.params {
            let vn_core::ast::Pattern::Identifier { name, .. } = &p.pattern else {
                continue;
            };
            out.push(Member {
                symbol: interner.resolve(*name).to_string(),
                kind: Kind::Property,
                params: Vec::new(),
                ret: ty(p.type_ann.as_ref(), interner, ContractTy::Dynamic),
                throws: Vec::new(),
                capability: Vec::new(),
            });
        }
    }
    Ok(out)
}

fn function_member(
    f: &FunctionDecl,
    arena: &AstArena,
    interner: &AtomInterner,
) -> Result<Member, ContractError> {
    Ok(Member {
        symbol: interner.resolve(f.id).to_string(),
        kind: Kind::Function,
        params: params(&f.params, interner),
        ret: ty(f.return_type.as_ref(), interner, ContractTy::Void),
        throws: throws(Kind::Function, &f.decorators, arena, interner)?,
        capability: capability(Kind::Function, &f.decorators, arena, interner)?,
    })
}
