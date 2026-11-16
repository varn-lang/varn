#![allow(unused_crate_dependencies)]

// Ley 11 exime a estos dos: fijan determinismo de internado (mismo contenido,
// mismo id, en cualquier orden e hilo), una propiedad que preserva el
// comportamiento y por tanto es ciega a tests/main.vn. Sin ellos, un cambio de
// orden generaría bytecode distinto sin que ninguna suite lo notara.
// (L3-Auditoría: el resto de invariantes de la tabla sí tienen cobertura
// conductual y se migraron o borraron.)

use vn_core::TypeKind;
use vn_sem::types::{CheckerTyId, CheckerTyTable};

#[test]
fn content_ids_are_order_independent() {
    let mut left = CheckerTyTable::new();
    let mut right = CheckerTyTable::new();

    let l_int = left.intern(TypeKind::Primitive(vn_core::LangPrimitive::INT));
    let l_arr = left.intern(TypeKind::Array(l_int));
    let l_list = left.intern_list(&[l_int, CheckerTyId::STR]);
    let l_union = left.intern(TypeKind::Union(l_list));

    let r_str = right.intern(TypeKind::Primitive(vn_core::LangPrimitive::Str));
    let r_int = right.intern(TypeKind::Primitive(vn_core::LangPrimitive::INT));
    let r_list = right.intern_list(&[r_int, r_str]);
    let r_union = right.intern(TypeKind::Union(r_list));
    let r_arr = right.intern(TypeKind::Array(r_int));

    assert_eq!(l_int, r_int);
    assert_eq!(l_arr, r_arr, "Array<int> es el mismo id en ambas tablas");
    assert_eq!(l_union, r_union, "Union<int,str> es el mismo id");
}

#[test]
fn ids_agree_across_threads_and_interning_order() {
    fn build(union_first: bool) -> (CheckerTyId, CheckerTyId, CheckerTyId) {
        let mut t = CheckerTyTable::new();
        if union_first {
            let l = t.intern_list(&[CheckerTyId::INT, CheckerTyId::STR]);
            let _u = t.intern(TypeKind::Union(l));
            let a = t.intern(TypeKind::Array(CheckerTyId::INT));
            let l2 = t.intern_list(&[CheckerTyId::INT, CheckerTyId::STR]);
            (
                a,
                t.intern(TypeKind::Union(l2)),
                t.intern(TypeKind::Array(a)),
            )
        } else {
            let a = t.intern(TypeKind::Array(CheckerTyId::INT));
            let l = t.intern_list(&[CheckerTyId::INT, CheckerTyId::STR]);
            let u = t.intern(TypeKind::Union(l));
            let a2 = t.intern(TypeKind::Array(CheckerTyId::INT));
            (a2, u, t.intern(TypeKind::Array(a)))
        }
    }

    let (left, right) = std::thread::scope(|s| {
        let h1 = s.spawn(|| build(true));
        let h2 = s.spawn(|| build(false));
        (h1.join().unwrap(), h2.join().unwrap())
    });
    assert_eq!(left, right, "mismos ids en hilos y órdenes distintos");
}
