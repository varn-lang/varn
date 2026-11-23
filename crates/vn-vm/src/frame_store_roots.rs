use super::frame_store::FrameStore;
use vn_types::HeapRef;

impl FrameStore {
    pub fn collect_roots(&self, top_dyn: usize, top_ref: usize, out: &mut Vec<HeapRef>) {
        out.extend(self.refs[..top_ref.min(self.refs.len())].iter().flatten());
        for v in &self.dyn_[..top_dyn.min(self.dyn_.len())] {
            if v.is_heap() {
                out.push(v.as_heap());
            }
        }
    }
}
