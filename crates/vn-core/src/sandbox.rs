crate::named_enum! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum Capability {
        FsRead => "fs.read",
        FsWrite => "fs.write",
        NetClient => "net.client",
        NetServer => "net.server",
        SysEnv => "sys.env",
        SysExec => "sys.exec",
        SysFfi => "sys.ffi",
        Ui => "ui",
    }
}

impl Capability {
    pub const fn bit(self) -> u64 {
        1u64 << (self as u64)
    }

    pub fn canonicalize_all(domains: Vec<String>) -> Result<Vec<String>, String> {
        let mut out = Vec::with_capacity(domains.len());
        for d in domains {
            let Some(cap) = Self::of(&d) else {
                return Err(format!("unknown capability `{d}`"));
            };
            let name = cap.name().to_string();
            if !out.contains(&name) {
                out.push(name);
            }
        }
        out.sort();
        Ok(out)
    }
}
