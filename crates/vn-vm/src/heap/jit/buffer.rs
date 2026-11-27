use super::super::obj::HeapObj;
use super::super::structs::Heap;

use super::RCBOX_PREFIX;
impl Heap {
    pub(crate) fn jit_buffer_layout() -> vn_jit::JitBufferLayout {
        use vn_types::value::VmBuffer;

        let base = VmBuffer::from_bytes(&[0xABu8; 300]);
        let sub = base.slice(100, 250);
        assert_eq!(sub.len(), 150);
        let size = std::mem::size_of::<VmBuffer>();
        assert_eq!(size, 16, "VmBuffer layout changed size");
        let bytes = unsafe { std::slice::from_raw_parts(&sub as *const _ as *const u8, size) };
        let at =
            |off: usize| -> u32 { u32::from_ne_bytes(bytes[off..off + 4].try_into().unwrap()) };
        assert_eq!(at(8), 100, "VmBuffer offset field tripwire");
        assert_eq!(at(12), 150, "VmBuffer length field tripwire");
        let buf_rc_off = 0usize;
        let buf_offset_off = 8usize;
        let buf_len_off = 12usize;

        let mut grown = Vec::with_capacity(500);
        grown.extend([0xCDu8; 300].iter().copied());
        let vec_ptr = grown.as_ptr() as usize;
        let cell = std::rc::Rc::new(std::cell::RefCell::new(grown));
        let rcbox = (std::rc::Rc::as_ptr(&cell) as usize) - RCBOX_PREFIX;
        assert_eq!(
            unsafe { *(rcbox as *const usize) },
            1,
            "RcBox strong counter tripwire"
        );
        assert_eq!(
            unsafe { *((rcbox + 8) as *const usize) },
            1,
            "RcBox weak counter tripwire"
        );
        let rc_value_off = RCBOX_PREFIX;
        let val = rcbox + rc_value_off;
        let borrow = unsafe { *(val as *const usize) };
        assert_eq!(borrow, 0, "RefCell borrow flag tripwire");
        let vec_off = std::mem::size_of::<usize>();
        let mut vec_ptr_off = usize::MAX;
        let mut vec_len_off = usize::MAX;
        for off in [vec_off, vec_off + 8, vec_off + 16] {
            let w = unsafe { *((val + off) as *const usize) };
            if w == vec_ptr {
                assert_eq!(
                    vec_ptr_off,
                    usize::MAX,
                    "Vec ptr tripwire: ambiguous layout"
                );
                vec_ptr_off = off;
            } else if w == 300 {
                vec_len_off = off;
            }
        }
        assert_ne!(vec_ptr_off, usize::MAX, "Vec ptr tripwire inside RcBox");
        assert_ne!(vec_len_off, usize::MAX, "Vec len tripwire inside RcBox");
        assert_ne!(
            vec_ptr_off, vec_len_off,
            "Vec field tripwire: ptr and len overlap"
        );

        let slot: Option<HeapObj> = Some(HeapObj::Buffer(sub));
        let slot_size = std::mem::size_of::<Option<HeapObj>>();
        let slot_bytes =
            unsafe { std::slice::from_raw_parts(&slot as *const _ as *const u8, slot_size) };
        let buffer_tag = slot_bytes[0] as usize;
        let none_slot: Option<HeapObj> = None;
        let none_tag = unsafe { *(&none_slot as *const _ as *const u8) } as usize;
        assert_ne!(buffer_tag, none_tag, "Option<HeapObj> niche probe failed");
        let pair = (0..=slot_size - 8).find(|&off| {
            u32::from_ne_bytes(slot_bytes[off..off + 4].try_into().unwrap()) == 100
                && u32::from_ne_bytes(slot_bytes[off + 4..off + 8].try_into().unwrap()) == 150
        });
        let Some(pair_off) = pair else {
            panic!("VmBuffer offset/len tripwire: (100, 150) not found in slot");
        };
        assert!(pair_off >= 8, "VmBuffer rc prefix tripwire");
        let payload_off = pair_off - 8 + vn_types::cell::HEADER_BYTES;
        vn_jit::JitBufferLayout {
            payload_off,
            buffer_tag,
            buf_rc_off,
            buf_offset_off,
            buf_len_off,
            rc_value_off,
            vec_ptr_off,
            vec_len_off,
        }
    }
}
