crate::named_enum! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum StdErrorClass {
        Std => "StdError",
        Io => "IoError",
        Fs => "FsError",
        FsNotFound => "FsNotFoundError",
        FsPermission => "FsPermissionError",
        FsExists => "FsExistsError",
        FsEncoding => "FsEncodingError",
        BadDescriptor => "BadDescriptorError",
        Closed => "ClosedError",
        Net => "NetError",
        Resolve => "ResolveError",
        Connect => "ConnectError",
        Tls => "TlsError",
        Protocol => "ProtocolError",
        Capability => "CapabilityError",
        Timeout => "TimeoutError",
        Cancelled => "CancelledError",
        Parse => "ParseError",
        Decode => "DecodeError",
        Sql => "SqlError",
        SqlConstraint => "SqlConstraintError",
    }
}

impl StdErrorClass {
    pub const MODULE: &'static str = "std:error";

    pub const fn code(self) -> &'static str {
        match self {
            Self::Std => "std",
            Self::Io => "io",
            Self::Fs => "fs",
            Self::FsNotFound => "fs.not_found",
            Self::FsPermission => "fs.permission_denied",
            Self::FsExists => "fs.exists",
            Self::FsEncoding => "fs.encoding",
            Self::BadDescriptor => "io.bad_descriptor",
            Self::Closed => "io.closed",
            Self::Net => "net",
            Self::Resolve => "net.resolve",
            Self::Connect => "net.connect",
            Self::Tls => "net.tls",
            Self::Protocol => "net.protocol",
            Self::Capability => "capability",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
            Self::Parse => "parse",
            Self::Decode => "parse.decode",
            Self::Sql => "sql",
            Self::SqlConstraint => "sql.constraint",
        }
    }

    pub const fn parent(self) -> Option<Self> {
        match self {
            Self::Std => None,
            Self::Io
            | Self::Capability
            | Self::Timeout
            | Self::Cancelled
            | Self::Parse
            | Self::Sql => Some(Self::Std),
            Self::Fs | Self::BadDescriptor | Self::Closed | Self::Net => Some(Self::Io),
            Self::FsNotFound | Self::FsPermission | Self::FsExists | Self::FsEncoding => {
                Some(Self::Fs)
            }
            Self::Resolve | Self::Connect | Self::Tls | Self::Protocol => Some(Self::Net),
            Self::Decode => Some(Self::Parse),
            Self::SqlConstraint => Some(Self::Sql),
        }
    }
}

crate::named_enum! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum ErrorField {
        Code => "code",
        Path => "path",
        Host => "host",
        Port => "port",
        Capability => "capability",
        Target => "target",
        Position => "position",
        Sql => "sql",
    }
}
