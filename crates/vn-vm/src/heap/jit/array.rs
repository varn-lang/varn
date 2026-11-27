use super::super::obj::HeapObj;
use super::super::structs::Heap;
use crate::value::VmValue;

impl Heap {
    pub(crate) fn jit_array_layout() -> vn_jit::JitArrayLayout {
        fn vec_word_offsets<T>(v: &Vec<T>) -> (usize, usize) {
            assert_eq!(v.len(), 3);
            assert_eq!(v.capacity(), 7);
            let words: [usize; 3] = unsafe { std::mem::transmute_copy(v) };
            let ptr = v.as_ptr() as usize;
            let mut ptr_off = usize::MAX;
            let mut len_off = usize::MAX;
            for (i, w) in words.iter().enumerate() {
                if *w == ptr {
                    ptr_off = i * 8;
                } else if *w == 3 {
                    len_off = i * 8;
                }
            }
            assert!(
                ptr_off != usize::MAX && len_off != usize::MAX,
                "Vec layout probe failed"
            );
            (ptr_off, len_off)
        }

        let mut slots_probe: Vec<Option<HeapObj>> = Vec::with_capacity(7);
        for _ in 0..3 {
            slots_probe.push(None);
        }

        let (slots_ptr_off, _) = vec_word_offsets(&slots_probe);

        let (disc_off, elems_ptr_off, elems_len_off) = {
            use vn_types::vm_value::{ArrayRepr, CellVec};
            let mut boxed_vec: Vec<VmValue> = Vec::with_capacity(7);
            for _ in 0..3 {
                boxed_vec.push(VmValue::null());
            }
            let vec_ptr = boxed_vec.as_ptr() as usize;
            let repr = ArrayRepr::Boxed(vn_types::BoxedElems::new(CellVec::from_vec(boxed_vec)));
            let base = &repr as *const _ as *const u8;
            let disc = unsafe { *base.add(ArrayRepr::DISC_OFF) };
            assert_eq!(
                disc, 0,
                "ArrayRepr::Boxed discriminant must read 0 at DISC_OFF"
            );
            let ptr_off = ArrayRepr::ELEMS_UNION_OFF + CellVec::<VmValue>::PTR_OFF;
            let len_off = ArrayRepr::ELEMS_UNION_OFF + CellVec::<VmValue>::LEN_OFF;
            let probed_ptr = unsafe { *(base.add(ptr_off) as *const usize) };
            let probed_len = unsafe { *(base.add(len_off) as *const usize) };
            assert_eq!(
                probed_ptr, vec_ptr,
                "elems tripwire: Boxed Vec ptr mismatch"
            );
            assert_eq!(probed_len, 3, "elems tripwire: Boxed Vec len mismatch");

            let i64_repr = ArrayRepr::I64(CellVec::from_vec(vec![0, 0, 0]));
            let i64_disc =
                unsafe { *(&i64_repr as *const _ as *const u8).add(ArrayRepr::DISC_OFF) };
            assert_eq!(i64_disc, 1, "ArrayRepr::I64 discriminant must read 1");
            let f64_repr = ArrayRepr::F64(CellVec::from_vec(vec![0.0, 0.0, 0.0]));
            let f64_disc =
                unsafe { *(&f64_repr as *const _ as *const u8).add(ArrayRepr::DISC_OFF) };
            assert_eq!(f64_disc, 2, "ArrayRepr::F64 discriminant must read 2");
            let char_repr = ArrayRepr::Char(CellVec::from_vec(vec!['a', 'b', 'c']));
            let char_disc =
                unsafe { *(&char_repr as *const _ as *const u8).add(ArrayRepr::DISC_OFF) };
            assert_eq!(char_disc, 3, "ArrayRepr::Char discriminant must read 3");
            let bool_repr = ArrayRepr::Bool(CellVec::from_vec(vec![true, false, true]));
            let bool_disc =
                unsafe { *(&bool_repr as *const _ as *const u8).add(ArrayRepr::DISC_OFF) };
            assert_eq!(bool_disc, 4, "ArrayRepr::Bool discriminant must read 4");
            let ref_repr = ArrayRepr::Ref(CellVec::new());
            let ref_disc =
                unsafe { *(&ref_repr as *const _ as *const u8).add(ArrayRepr::DISC_OFF) };
            assert_eq!(ref_disc, 5, "ArrayRepr::Ref discriminant must read 5");

            (ArrayRepr::DISC_OFF, ptr_off, len_off)
        };

        let mut repr_cell = std::mem::MaybeUninit::<vn_types::vm_value::ArrayRepr>::uninit();
        let arr = unsafe {
            vn_types::VmArray::init_at(
                repr_cell.as_mut_ptr(),
                vn_types::vm_value::ArrayRepr::boxed(vec![VmValue::null()]),
            )
        };
        let data = repr_cell.as_ptr() as usize;
        let slot: Option<HeapObj> = Some(HeapObj::Array(arr));
        let size = std::mem::size_of::<Option<HeapObj>>();
        let bytes = unsafe { std::slice::from_raw_parts(&slot as *const _ as *const u8, size) };
        let array_tag = bytes[0] as usize;
        let payload_off = (0..=size - 8)
            .find(|&off| usize::from_ne_bytes(bytes[off..off + 8].try_into().unwrap()) == data)
            .expect("array payload probe failed")
            + vn_types::cell::HEADER_BYTES;
        let tuple_slot: Option<HeapObj> = Some(HeapObj::Tuple(arr));
        let tuple_tag = unsafe { *(&tuple_slot as *const _ as *const u8) } as usize;
        assert_ne!(
            tuple_tag, array_tag,
            "HeapObj::Tuple and HeapObj::Array share a tag"
        );
        unsafe { arr.drop_at() };

        let none_slot: Option<HeapObj> = None;
        let none_tag = unsafe { *(&none_slot as *const _ as *const u8) } as usize;
        assert_ne!(array_tag, none_tag, "Option<HeapObj> niche probe failed");

        let str_slot: Option<HeapObj> = Some(HeapObj::Str(super::super::str::HeapStr::inline("")));
        let str_tag = unsafe { *(&str_slot as *const _ as *const u8) } as usize;
        assert_ne!(str_tag, none_tag, "Option<HeapObj> niche probe failed");
        assert_ne!(
            str_tag, array_tag,
            "HeapObj::Str and HeapObj::Array share a tag"
        );

        vn_jit::JitArrayLayout {
            state_off: vn_types::cell::HEADER_STATE_OFF,
            kind_off: vn_types::cell::HEADER_KIND_OFF,
            young_state: super::super::cells::SlotState::Young as usize,
            vec_ptr_off: slots_ptr_off,
            array_tag,
            tuple_tag,
            str_tag,
            payload_off,
            disc_off,
            elems_ptr_off,
            elems_len_off,
        }
    }
}
