use super::super::structs::Heap;

impl Heap {
    pub(crate) fn jit_str_layout() -> vn_jit::JitStrLayout {
        use super::super::str::{ascii_flag, HeapStr};
        use std::sync::Arc;

        fn first_byte(v: &HeapStr) -> u8 {
            unsafe { *(v as *const HeapStr as *const u8) }
        }
        fn bytes_of(v: &HeapStr) -> Vec<u8> {
            let size = std::mem::size_of::<HeapStr>();
            unsafe { std::slice::from_raw_parts(v as *const _ as *const u8, size).to_vec() }
        }

        let yes_text = "probe-ascii-01234567ab";
        assert_eq!(yes_text.len(), 22);
        let no_text = "é".repeat(11);
        assert_eq!(no_text.len(), 22);
        let yes_arc: Arc<str> = Arc::from(yes_text);
        let no_arc: Arc<str> = Arc::from(no_text.as_str());
        let yes_inner = Arc::as_ptr(&yes_arc) as *const u8 as usize;
        let no_inner = Arc::as_ptr(&no_arc) as *const u8 as usize;
        let shared_yes = HeapStr::shared(yes_arc);
        let shared_no = HeapStr::shared(no_arc);
        let inline_probe = HeapStr::inline("ab");
        let ext_buf = std::rc::Rc::new(std::cell::UnsafeCell::new("ext-probe".to_string()));
        let ext_probe = HeapStr::ext(std::rc::Rc::clone(&ext_buf), 9, ascii_flag::YES);
        let view_base: Arc<str> = Arc::from("abcdef");
        let view_probe = HeapStr::view(view_base, 0, 3, true);

        let shared_disc = first_byte(&shared_yes) as usize;
        for (other, name) in [
            (&inline_probe, "Inline"),
            (&ext_probe, "Ext"),
            (&view_probe, "View"),
        ] {
            assert_ne!(
                first_byte(other),
                shared_disc as u8,
                "HeapStr::Shared shares a discriminant with {name}"
            );
        }

        let yes_bytes = bytes_of(&shared_yes);
        let no_bytes = bytes_of(&shared_no);
        assert_eq!(yes_bytes.len(), no_bytes.len());
        let data_ptr = yes_inner;
        let base_ptr = data_ptr - 2 * std::mem::size_of::<usize>();
        let arc_off = (0..=yes_bytes.len() - 8)
            .find(|&off| {
                usize::from_ne_bytes(yes_bytes[off..off + 8].try_into().unwrap()) == base_ptr
            })
            .expect("Shared Arc base pointer probe failed");
        assert_eq!(
            usize::from_ne_bytes(yes_bytes[arc_off + 8..arc_off + 16].try_into().unwrap()),
            22,
            "Shared Arc length probe failed (fat pointer order)"
        );
        assert_eq!(
            usize::from_ne_bytes(no_bytes[arc_off..arc_off + 8].try_into().unwrap()),
            no_inner - 2 * std::mem::size_of::<usize>(),
            "Shared Arc pointer probe failed on the non-ascii twin"
        );
        let base = base_ptr;
        assert_eq!(
            unsafe { *(base as *const usize) },
            1,
            "Arc strong counter tripwire"
        );
        assert_eq!(
            unsafe { *((base + std::mem::size_of::<usize>()) as *const usize) },
            1,
            "Arc weak counter tripwire"
        );
        let data_off = 2 * std::mem::size_of::<usize>();
        assert_eq!(
            unsafe { std::slice::from_raw_parts((base + data_off) as *const u8, 22) },
            yes_text.as_bytes(),
            "Arc data tripwire"
        );

        let twins: Vec<(Vec<u8>, Vec<u8>)> = (0..8u8)
            .map(|k| {
                let yes = format!("probe-ascii-{k:02}-01234567");
                let no = "é".repeat(11 + usize::from(k));
                (
                    bytes_of(&HeapStr::shared(Arc::from(yes.as_str()))),
                    bytes_of(&HeapStr::shared(Arc::from(no.as_str()))),
                )
            })
            .collect();
        let flags_at = |off: usize| {
            yes_bytes[off] == ascii_flag::YES
                && no_bytes[off] == ascii_flag::NO
                && twins
                    .iter()
                    .all(|(y, n)| y[off] == ascii_flag::YES && n[off] == ascii_flag::NO)
        };
        let mut candidates = (0..yes_bytes.len())
            .filter(|&off| !(arc_off..arc_off + 16).contains(&off) && flags_at(off));
        let ascii_off = candidates.next().expect("Shared ascii flag probe failed");
        assert!(
            candidates.next().is_none(),
            "Shared ascii flag probe is ambiguous"
        );
        assert_eq!(yes_bytes[ascii_off], ascii_flag::YES);
        assert_eq!(no_bytes[ascii_off], ascii_flag::NO);

        vn_jit::JitStrLayout {
            shared_disc,
            shared_arc_off: arc_off,
            shared_ascii_off: ascii_off,
            arc_data_off: data_off,
            ascii_yes: ascii_flag::YES as usize,
        }
    }
}
