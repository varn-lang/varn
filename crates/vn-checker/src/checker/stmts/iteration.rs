use super::super::Checker;
use vn_core::{BuiltinType, LangPrimitive, MemberKey, TypeKind};
use vn_sem::bind::BindResult;
use vn_sem::output::ForOfPlan;
use vn_sem::types::Type;

impl Checker<'_> {
    pub(crate) fn for_of_element(
        &mut self,
        source: Type,
        is_await: bool,
        bind: &BindResult,
    ) -> Option<(ForOfPlan, Type)> {
        if let vn_core::TypeKind::Union(members) = self.ty_table.get(source.0) {
            let ids = self.ty_table.get_list(members).to_vec();
            if ids.is_empty() {
                return None;
            }
            let mut plans = Vec::with_capacity(ids.len());
            let mut elems = Vec::with_capacity(ids.len());
            for id in ids {
                let member = Type::resolved(id);
                if member == Type::Null {
                    return None;
                }
                let (plan, elem) = self.for_of_element(member, is_await, bind)?;
                plans.push(plan);
                elems.push(elem);
            }
            let (first_plan, _) = plans.split_first()?;
            let same_kind = plans.iter().all(|plan| match (plan, first_plan) {
                (
                    ForOfPlan::Collection { method: a, .. },
                    ForOfPlan::Collection { method: b, .. },
                ) => a == b,
                (ForOfPlan::IntRange, ForOfPlan::IntRange) => true,
                (ForOfPlan::Protocol { .. }, ForOfPlan::Protocol { .. }) => plan == first_plan,
                (ForOfPlan::Collection { .. }, ForOfPlan::IntRange)
                | (ForOfPlan::Collection { .. }, ForOfPlan::Protocol { .. })
                | (ForOfPlan::IntRange, ForOfPlan::Collection { .. })
                | (ForOfPlan::IntRange, ForOfPlan::Protocol { .. })
                | (ForOfPlan::Protocol { .. }, ForOfPlan::Collection { .. })
                | (ForOfPlan::Protocol { .. }, ForOfPlan::IntRange) => false,
            });
            if !same_kind {
                return None;
            }
            let table = std::sync::Arc::make_mut(&mut self.ty_table);
            let elem = Type::union(elems, table);
            let plan = match *first_plan {
                ForOfPlan::Collection { method, .. } => ForOfPlan::Collection { method, elem },
                ForOfPlan::IntRange => ForOfPlan::IntRange,
                ForOfPlan::Protocol {
                    iterator,
                    next,
                    step,
                } => ForOfPlan::Protocol {
                    iterator,
                    next,
                    step,
                },
            };
            return Some((plan, elem));
        }
        match self.ty_table.get(source.0) {
            TypeKind::Array(inner) => {
                let elem = Type::resolved(inner);
                return Some((ForOfPlan::Collection { method: None, elem }, elem));
            }
            TypeKind::Primitive(LangPrimitive::Str) | TypeKind::TemplateLiteral(_) => {
                let elem = Type::Char;
                return Some((
                    ForOfPlan::Collection {
                        method: Some(MemberKey::Chars),
                        elem,
                    },
                    elem,
                ));
            }
            TypeKind::Tuple(list) => {
                let members = self
                    .ty_table
                    .get_list(list)
                    .iter()
                    .map(|&id| Type::resolved(id))
                    .collect();
                let elem = Type::union(members, std::sync::Arc::make_mut(&mut self.ty_table));
                return Some((ForOfPlan::Collection { method: None, elem }, elem));
            }
            TypeKind::Record(_)
            | TypeKind::Interface(_)
            | TypeKind::Primitive(_)
            | TypeKind::Literal(_)
            | TypeKind::This
            | TypeKind::Union(_)
            | TypeKind::Intersection(_)
            | TypeKind::Named(..)
            | TypeKind::Generic(..)
            | TypeKind::Fn(_)
            | TypeKind::Typeof(_)
            | TypeKind::KeyOf(_)
            | TypeKind::IndexedAccess { .. }
            | TypeKind::Mapped { .. }
            | TypeKind::Conditional { .. }
            | TypeKind::Infer(_)
            | TypeKind::EnumVariant { .. }
            | TypeKind::TypePredicate { .. } => {}
        }
        if let Some(builtin) = source.builtin_of(&self.ty_table) {
            match builtin {
                BuiltinType::Bytes => {
                    return Some((
                        ForOfPlan::Collection {
                            method: None,
                            elem: Type::Int,
                        },
                        Type::Int,
                    ));
                }
                BuiltinType::Map | BuiltinType::ReadonlyMap => {
                    let key = source.builtin_arg(0, &self.ty_table);
                    let value = source.builtin_arg(1, &self.ty_table);
                    let table = std::sync::Arc::make_mut(&mut self.ty_table);
                    let pair = table.intern_list(&[key.0, value.0]);
                    let entry = Type::resolved(table.intern(TypeKind::Tuple(pair)));
                    return Some((
                        ForOfPlan::Collection {
                            method: Some(MemberKey::Entries),
                            elem: entry,
                        },
                        entry,
                    ));
                }
                BuiltinType::ReadonlyArray => {
                    let elem = source.builtin_arg(0, &self.ty_table);
                    return Some((ForOfPlan::Collection { method: None, elem }, elem));
                }
                BuiltinType::Set | BuiltinType::ReadonlySet => {
                    let elem = source.builtin_arg(0, &self.ty_table);
                    return Some((
                        ForOfPlan::Collection {
                            method: Some(MemberKey::Values),
                            elem,
                        },
                        elem,
                    ));
                }
                BuiltinType::Range => {
                    let elem = source.builtin_arg(0, &self.ty_table);
                    if elem == Type::Int || elem.is_unresolved() {
                        return Some((ForOfPlan::IntRange, Type::Int));
                    }
                    return Some((
                        ForOfPlan::Collection {
                            method: Some(MemberKey::ToArray),
                            elem,
                        },
                        elem,
                    ));
                }
                BuiltinType::Array
                | BuiltinType::Generator
                | BuiltinType::AsyncGenerator
                | BuiltinType::Task
                | BuiltinType::TaskHandle
                | BuiltinType::Class => {}
            }
        }
        let iter_result = self.method_return(&source, MemberKey::Iter.as_str(), bind);
        let iterator = iter_result.unwrap_or(source);
        let next = self.method_return(&iterator, MemberKey::IterNext.as_str(), bind)?;
        let step = if is_await {
            vn_sem::types::awaited(&next, &self.ty_table)
        } else {
            next
        };
        if !step.is_nullable(&self.ty_table) {
            return None;
        }
        let elem = step.non_nullified(std::sync::Arc::make_mut(&mut self.ty_table));
        Some((
            ForOfPlan::Protocol {
                iterator: iter_result,
                next,
                step,
            },
            elem,
        ))
    }

    fn method_return(&mut self, receiver: &Type, name: &str, bind: &BindResult) -> Option<Type> {
        let (member, _) = self.find_member_info(receiver, name, bind)?;
        match self.ty_table.get(member.0) {
            TypeKind::Fn(fid) => Some(Type::resolved(self.ty_table.get_function(fid).return_type)),
            TypeKind::Primitive(_)
            | TypeKind::Literal(_)
            | TypeKind::This
            | TypeKind::Array(_)
            | TypeKind::Union(_)
            | TypeKind::Intersection(_)
            | TypeKind::Tuple(_)
            | TypeKind::Named(..)
            | TypeKind::Generic(..)
            | TypeKind::TemplateLiteral(_)
            | TypeKind::Record(_)
            | TypeKind::Interface(_)
            | TypeKind::Typeof(_)
            | TypeKind::KeyOf(_)
            | TypeKind::IndexedAccess { .. }
            | TypeKind::Mapped { .. }
            | TypeKind::Conditional { .. }
            | TypeKind::Infer(_)
            | TypeKind::EnumVariant { .. }
            | TypeKind::TypePredicate { .. } => None,
        }
    }
}
