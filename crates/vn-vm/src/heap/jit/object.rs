use super::super::obj::HeapObj;
use super::super::structs::Heap;
use crate::value::VmValue;
use std::rc::Rc;

use super::RCBOX_PREFIX;
impl Heap {
    pub(crate) fn jit_object_layout() -> vn_jit::JitObjectLayout {
        const SENTINEL_FIELD: u64 = 0xFEED_BEEF_CAFE_1234;
        const TAIL: usize = 3;

        let shape = vn_types::Shape::create(None, rustc_hash::FxHashMap::default());
        let shape_id = shape.id;

        let mut probe = vec![0u64; vn_types::ObjData::bytes_for(TAIL) / 8];
        let oref = unsafe {
            vn_types::ObjData::init_at(
                probe.as_mut_ptr() as *mut u8,
                Rc::clone(&shape),
                TAIL,
                &[VmValue::from_raw_parts(SENTINEL_FIELD, SENTINEL_FIELD); TAIL],
            )
        };
        let data = probe.as_ptr() as usize;
        let shape_ptr = Rc::as_ptr(&shape) as *const u8 as usize - RCBOX_PREFIX;
        assert_eq!(
            unsafe { *(shape_ptr as *const usize) },
            2,
            "Rc<Shape> strong counter tripwire: RcBox strong must sit at value_ptr - 16"
        );

        let values_off = vn_types::OBJ_VALUES_OFF;
        let shape_off = vn_types::OBJ_SHAPE_OFF;
        let len_off = vn_types::OBJ_INLINE_LEN_OFF;
        let block = unsafe { std::slice::from_raw_parts(data as *const u8, 64) };
        let word_at =
            |off: usize| -> u64 { u64::from_ne_bytes(block[off..off + 8].try_into().unwrap()) };
        assert_eq!(
            word_at(values_off),
            SENTINEL_FIELD,
            "OBJ_VALUES_OFF tripwire: tail sentinel not at derived offset"
        );
        assert!(
            (word_at(len_off) & 0xFFFF_FFFF) as usize == TAIL && len_off != values_off,
            "OBJ_INLINE_LEN_OFF tripwire: inline_len not at derived offset"
        );
        assert_eq!(
            word_at(shape_off) as usize,
            shape_ptr,
            "OBJ_SHAPE_OFF tripwire: shape control not at derived offset"
        );

        let shape_id_off = RCBOX_PREFIX + vn_types::SHAPE_ID_OFF;
        assert_eq!(
            unsafe { *((shape_ptr + shape_id_off) as *const u32) },
            shape_id,
            "shape id offset does not resolve to Shape.id"
        );

        let slot: Option<HeapObj> = Some(HeapObj::Object(oref));
        let size = std::mem::size_of::<Option<HeapObj>>();
        let bytes = unsafe { std::slice::from_raw_parts(&slot as *const _ as *const u8, size) };
        let object_tag = bytes[0] as usize;
        let payload_off = (0..=size - 8)
            .find(|&off| usize::from_ne_bytes(bytes[off..off + 8].try_into().unwrap()) == data)
            .expect("object payload probe failed")
            + vn_types::cell::HEADER_BYTES;
        let data_off = vn_types::cell::HEADER_BYTES + std::mem::size_of::<HeapObj>();
        let bare_obj = HeapObj::Object(oref);
        let bare_size = std::mem::size_of::<HeapObj>();
        let bare =
            unsafe { std::slice::from_raw_parts(&bare_obj as *const _ as *const u8, bare_size) };
        let bare_payload = (0..=bare_size - 8)
            .find(|&off| usize::from_ne_bytes(bare[off..off + 8].try_into().unwrap()) == data)
            .expect("bare HeapObj payload probe failed");
        assert_eq!(
            payload_off - vn_types::cell::HEADER_BYTES,
            bare_payload,
            "Option<HeapObj> must share HeapObj field offsets (niche layout)"
        );
        assert!(
            data_off >= vn_types::cell::HEADER_BYTES + bare_payload + 8,
            "ObjData must start after the whole HeapObj slot"
        );
        let payload_len_off = bare_payload + 8 + vn_types::cell::HEADER_BYTES;
        assert_eq!(
            usize::from_ne_bytes(
                bare[bare_payload + 8..bare_payload + 16]
                    .try_into()
                    .unwrap()
            ),
            TAIL,
            "ObjRef length metadata must sit right after the payload address"
        );
        std::mem::forget(bare_obj);
        unsafe { vn_types::ObjData::drop_at(oref) };

        let none_tag = unsafe { *(&(None::<HeapObj>) as *const _ as *const u8) } as usize;
        assert_ne!(object_tag, none_tag, "Option<HeapObj> niche probe failed");

        vn_jit::JitObjectLayout {
            object_tag,
            payload_off,
            payload_len_off,
            data_off,
            len_off,
            values_off,
            shape_off,
            shape_id_off,
        }
    }
}
