use super::super::cells::CellSpace;
use super::super::obj::HeapObj;
use super::super::structs::{Heap, HeapInner};
use super::super::young::YoungGen;

use super::RCBOX_PREFIX;
impl Heap {
    pub(crate) fn jit_instance_alloc() -> vn_jit::JitInstanceAlloc {
        super::super::young::YoungGen::probe_born_layout();
        let cls = std::rc::Rc::new(vn_types::ClassObj::new("jit-probe"));
        let cls_id = cls.id;
        let cls_val = std::rc::Rc::as_ptr(&cls) as usize;
        let rc_base = cls_val - 2 * std::mem::size_of::<usize>();
        let cls_slot = HeapObj::Class(cls);
        let cls_size = std::mem::size_of::<HeapObj>();
        let cls_bytes =
            unsafe { std::slice::from_raw_parts(&cls_slot as *const _ as *const u8, cls_size) };
        let class_tag = cls_bytes[0] as usize;
        let class_ref_off = (0..=cls_size - 8)
            .find(|&off| {
                usize::from_ne_bytes(cls_bytes[off..off + 8].try_into().unwrap()) == rc_base
            })
            .expect("class payload probe failed");
        let class_id_off =
            2 * std::mem::size_of::<usize>() + std::mem::offset_of!(vn_types::ClassObj, id);
        assert_eq!(
            unsafe { *((rc_base + class_id_off) as *const u32) },
            cls_id,
            "class id offset does not resolve to ClassObj.id"
        );
        vn_jit::JitInstanceAlloc {
            cells_off: RCBOX_PREFIX + std::mem::offset_of!(HeapInner, cells),
            classes_off: std::mem::offset_of!(CellSpace, classes),
            young_off: RCBOX_PREFIX + std::mem::offset_of!(HeapInner, young),
            born_off: std::mem::offset_of!(YoungGen, born),
            native_off: std::mem::offset_of!(CellSpace, native),
            class_tag,
            class_ref_off,
            class_id_off,
            hotspot_off: RCBOX_PREFIX + std::mem::offset_of!(HeapInner, hotspot),
        }
    }
}
