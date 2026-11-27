use vn_pipeline::CapabilitySet;

pub struct Policy {
    pub capabilities: CapabilitySet,
    pub trace: bool,
    pub gc: bool,
    pub silent: bool,
    pub fs_root: Option<std::path::PathBuf>,
}

impl Policy {
    pub fn new(capabilities: CapabilitySet) -> Policy {
        Policy {
            capabilities,
            trace: false,
            gc: false,
            silent: false,
            fs_root: None,
        }
    }
}
