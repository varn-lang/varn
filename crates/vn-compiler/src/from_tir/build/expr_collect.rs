use super::context::{Builder, Result};
use crate::ssa::ir::{InstKind, Value};
use vn_tir::TirExpr;
use vn_tir::{BackendTy, DynReason};

const ARRAY_CHUNK: usize = 64;

impl<'m> Builder<'m> {
    pub(super) fn lower_array_lit(
        &mut self,
        els: &[vn_tir::TirArrayEl],
        ty: BackendTy,
    ) -> Result<Value> {
        let any_spread = els
            .iter()
            .any(|el| matches!(el, vn_tir::TirArrayEl::Spread(_)));
        let mut chunks = els.chunks(ARRAY_CHUNK);
        let first = chunks.next().unwrap_or(&[]);
        let mut array = self.build_array_chunk(first, ty, any_spread)?;
        for chunk in chunks {
            if any_spread {
                let mut parts = vec![vn_types::ssa::SsaSpread {
                    value: array,
                    spread: true,
                }];
                parts.extend(self.array_chunk_parts(chunk)?);
                array = self.emit(InstKind::BuildArraySpread { elements: parts }, ty);
            } else {
                for el in chunk {
                    let value = self.array_element(el)?;
                    self.emit_effect(InstKind::ArrayPush { array, value });
                }
            }
        }
        Ok(array)
    }

    fn array_element(&mut self, el: &vn_tir::TirArrayEl) -> Result<Value> {
        match el {
            vn_tir::TirArrayEl::Expr(x) | vn_tir::TirArrayEl::Spread(x) => self.lower_expr(x),
            vn_tir::TirArrayEl::Hole => {
                Ok(self.emit(InstKind::ConstNull, BackendTy::Dynamic(DynReason::Boxed)))
            }
        }
    }

    fn array_chunk_parts(
        &mut self,
        chunk: &[vn_tir::TirArrayEl],
    ) -> Result<Vec<vn_types::ssa::SsaSpread<Value>>> {
        chunk
            .iter()
            .map(|el| {
                Ok(vn_types::ssa::SsaSpread {
                    value: self.array_element(el)?,
                    spread: matches!(el, vn_tir::TirArrayEl::Spread(_)),
                })
            })
            .collect()
    }

    fn build_array_chunk(
        &mut self,
        chunk: &[vn_tir::TirArrayEl],
        ty: BackendTy,
        spread: bool,
    ) -> Result<Value> {
        if spread {
            let parts = self.array_chunk_parts(chunk)?;
            Ok(self.emit(InstKind::BuildArraySpread { elements: parts }, ty))
        } else {
            let elements = chunk
                .iter()
                .map(|el| self.array_element(el))
                .collect::<Result<Vec<_>>>()?;
            let storage = self.array_storage(ty, &elements);
            Ok(self.emit(InstKind::BuildArray { elements, storage }, ty))
        }
    }

    fn array_storage(&self, ty: BackendTy, elements: &[Value]) -> vn_core::ArrayStorage {
        use vn_core::{ArrayStorage, IntKind, NumKind};
        let BackendTy::Array(element_id) = ty else {
            return ArrayStorage::Boxed;
        };
        if !self.tir.types.contains(element_id) {
            return ArrayStorage::Boxed;
        }
        let element = self.tir.types.get(element_id);
        let exact = elements.iter().all(|e| self.value_ty(*e) == element);
        match element {
            BackendTy::Num(NumKind::Int(IntKind::U64)) => ArrayStorage::Boxed,
            BackendTy::Num(NumKind::Int(_)) if exact => ArrayStorage::Int,
            BackendTy::Num(NumKind::Float(_)) if exact => ArrayStorage::Float,
            BackendTy::Char if exact => ArrayStorage::Char,
            BackendTy::Bool if exact => ArrayStorage::Bool,
            BackendTy::Class(_)
            | BackendTy::Array(_)
            | BackendTy::Map(_, _)
            | BackendTy::Set(_)
            | BackendTy::Tuple(_)
                if exact =>
            {
                ArrayStorage::Ref
            }
            BackendTy::Literal(_)
            | BackendTy::Num(_)
            | BackendTy::Bool
            | BackendTy::Char
            | BackendTy::Str
            | BackendTy::Bytes
            | BackendTy::Decimal
            | BackendTy::BigInt
            | BackendTy::Class(_)
            | BackendTy::Array(_)
            | BackendTy::Map(_, _)
            | BackendTy::Set(_)
            | BackendTy::Tuple(_)
            | BackendTy::Enum(_)
            | BackendTy::Fn(_)
            | BackendTy::Nullable(_)
            | BackendTy::Void
            | BackendTy::Never
            | BackendTy::Dynamic(_) => ArrayStorage::Boxed,
        }
    }

    pub(super) fn lower_tuple_lit(&mut self, xs: &[TirExpr], ty: BackendTy) -> Result<Value> {
        let mut vals = Vec::with_capacity(xs.len());
        for x in xs {
            vals.push(self.lower_expr(x)?);
        }
        Ok(self.emit(InstKind::BuildTuple { elements: vals }, ty))
    }

    pub(super) fn lower_object_lit(
        &mut self,
        entries: &[vn_tir::TirObjectEntry],
        ty: BackendTy,
    ) -> Result<Value> {
        if matches!(ty, BackendTy::Map(..)) {
            return Err(crate::OptError::Unsupported(
                "from_tir: an object literal never builds a Map: use `new Map([(k, v)])`",
            ));
        }
        let any_spread = entries
            .iter()
            .any(|e| matches!(e, vn_tir::TirObjectEntry::Spread(_)));
        if any_spread {
            let mut parts: Vec<vn_types::ssa::SsaObjectSpreadPart<Value>> =
                Vec::with_capacity(entries.len());
            for entry in entries {
                match entry {
                    vn_tir::TirObjectEntry::Field { name, value } => {
                        let v = self.lower_expr(value)?;
                        parts.push(vn_types::ssa::SsaObjectSpreadPart {
                            key: Some(name.clone()),
                            value: v,
                        });
                    }
                    vn_tir::TirObjectEntry::Spread(x) => {
                        let v = self.lower_expr(x)?;
                        parts.push(vn_types::ssa::SsaObjectSpreadPart {
                            key: None,
                            value: v,
                        });
                    }
                }
            }
            Ok(self.emit(InstKind::BuildObjectSpread { parts }, ty))
        } else {
            let mut pairs = Vec::with_capacity(entries.len());
            for entry in entries {
                if let vn_tir::TirObjectEntry::Field { name, value } = entry {
                    let v = self.lower_expr(value)?;
                    pairs.push((name.clone(), v));
                }
            }
            Ok(self.emit(InstKind::BuildObject { pairs }, ty))
        }
    }

    pub(super) fn lower_assign_expr(&mut self, target: &TirExpr, value: &TirExpr) -> Result<Value> {
        let v = self.lower_expr(value)?;
        self.lower_assign(target, v)?;
        Ok(v)
    }
}
