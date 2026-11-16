#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Capability {
    FsRead,
    FsWrite,
    NetClient,
    NetServer,
    SysEnv,
    SysExec,
    SysFfi,
    Ui,
}

impl Capability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FsRead => "fs.read",
            Self::FsWrite => "fs.write",
            Self::NetClient => "net.client",
            Self::NetServer => "net.server",
            Self::SysEnv => "sys.env",
            Self::SysExec => "sys.exec",
            Self::SysFfi => "sys.ffi",
            Self::Ui => "ui",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "fs.read" => Self::FsRead,
            "fs.write" => Self::FsWrite,
            "net.client" => Self::NetClient,
            "net.server" => Self::NetServer,
            "sys.env" => Self::SysEnv,
            "sys.exec" => Self::SysExec,
            "sys.ffi" => Self::SysFfi,
            "ui" => Self::Ui,
            _ => return None,
        })
    }
}
