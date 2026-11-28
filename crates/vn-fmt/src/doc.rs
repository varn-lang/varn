#[derive(Clone, Debug)]
pub enum Doc {
    Nil,
    Text(String),
    Raw(String),
    Suffix(String),
    BreakParent,
    Space,
    Line,
    Soft,
    Hard,
    Concat(Vec<Doc>),
    Group(Box<Doc>, bool),
    Indent(Box<Doc>),
    IfBreak { broken: Box<Doc>, flat: Box<Doc> },
}

impl Doc {
    pub fn text(value: impl Into<String>) -> Doc {
        Doc::Text(value.into())
    }

    pub fn group(inner: Doc) -> Doc {
        let forced = inner.forces_break();
        Doc::Group(Box::new(inner), forced)
    }

    pub fn indent(inner: Doc) -> Doc {
        Doc::Indent(Box::new(inner))
    }

    pub fn if_break(broken: Doc, flat: Doc) -> Doc {
        Doc::IfBreak {
            broken: Box::new(broken),
            flat: Box::new(flat),
        }
    }

    pub fn forces_break(&self) -> bool {
        match self {
            Doc::Hard | Doc::BreakParent => true,
            Doc::Concat(parts) => parts.iter().any(Doc::forces_break),
            Doc::Group(_, forced) => *forced,
            Doc::Indent(inner) => inner.forces_break(),
            Doc::IfBreak { broken, flat } => broken.forces_break() || flat.forces_break(),
            Doc::Nil
            | Doc::Text(_)
            | Doc::Raw(_)
            | Doc::Suffix(_)
            | Doc::Space
            | Doc::Line
            | Doc::Soft => false,
        }
    }
}
