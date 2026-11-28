use crate::doc::Doc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Flat,
    Break,
}

struct Command<'a> {
    indent: usize,
    mode: Mode,
    doc: &'a Doc,
}

pub struct Printer {
    width: usize,
    indent_width: usize,
}

impl Printer {
    pub fn new(width: usize, indent_width: usize) -> Self {
        Self {
            width,
            indent_width,
        }
    }

    pub fn print(&self, root: &Doc) -> String {
        let mut out = String::new();
        let mut column = 0usize;
        let mut suffix = String::new();
        let mut stack = vec![Command {
            indent: 0,
            mode: Mode::Break,
            doc: root,
        }];
        while let Some(command) = stack.pop() {
            match command.doc {
                Doc::Nil | Doc::BreakParent => {}
                Doc::Suffix(text) => suffix.push_str(text),
                Doc::Text(text) => {
                    out.push_str(text);
                    column += text.chars().count();
                }
                Doc::Raw(text) => {
                    out.push_str(text);
                    column = match text.rfind('\n') {
                        Some(at) => text[at + 1..].chars().count(),
                        None => column + text.chars().count(),
                    };
                }
                Doc::Space => {
                    out.push(' ');
                    column += 1;
                }
                Doc::Line | Doc::Soft | Doc::Hard => {
                    let flat = command.mode == Mode::Flat && !matches!(command.doc, Doc::Hard);
                    if flat {
                        if matches!(command.doc, Doc::Line) {
                            out.push(' ');
                            column += 1;
                        }
                    } else {
                        out.push_str(&std::mem::take(&mut suffix));
                        while out.ends_with(' ') {
                            out.pop();
                        }
                        out.push('\n');
                        for _ in 0..command.indent {
                            out.push(' ');
                        }
                        column = command.indent;
                    }
                }
                Doc::Concat(parts) => {
                    for part in parts.iter().rev() {
                        stack.push(Command {
                            indent: command.indent,
                            mode: command.mode,
                            doc: part,
                        });
                    }
                }
                Doc::Indent(inner) => stack.push(Command {
                    indent: command.indent + self.indent_width,
                    mode: command.mode,
                    doc: inner,
                }),
                Doc::Group(inner, forced) => {
                    let mode = if *forced {
                        Mode::Break
                    } else if command.mode == Mode::Flat || self.fits(inner, column, &stack) {
                        Mode::Flat
                    } else {
                        Mode::Break
                    };
                    stack.push(Command {
                        indent: command.indent,
                        mode,
                        doc: inner,
                    });
                }
                Doc::Hugging {
                    head,
                    hugged,
                    expanded,
                } => {
                    let hug = command.mode == Mode::Flat || self.fits(head, column, &[]);
                    stack.push(Command {
                        indent: command.indent,
                        mode: command.mode,
                        doc: if hug { hugged } else { expanded },
                    });
                }
                Doc::IfBreak { broken, flat } => stack.push(Command {
                    indent: command.indent,
                    mode: command.mode,
                    doc: if command.mode == Mode::Break {
                        broken
                    } else {
                        flat
                    },
                }),
            }
        }
        out.push_str(&suffix);
        while out.ends_with(' ') {
            out.pop();
        }
        out
    }

    fn fits(&self, group: &Doc, column: usize, rest: &[Command<'_>]) -> bool {
        let Some(mut remaining) = self.width.checked_sub(column) else {
            return false;
        };
        let mut work: Vec<(Mode, &Doc)> = vec![(Mode::Flat, group)];
        let mut rest_index = rest.len();
        loop {
            let (mode, doc) = match work.pop() {
                Some(item) => item,
                None => {
                    if rest_index == 0 {
                        return true;
                    }
                    rest_index -= 1;
                    (rest[rest_index].mode, rest[rest_index].doc)
                }
            };
            match doc {
                Doc::Nil | Doc::BreakParent | Doc::Suffix(_) => {}
                Doc::Text(text) => {
                    let len = text.chars().count();
                    if len > remaining {
                        return false;
                    }
                    remaining -= len;
                }
                Doc::Raw(text) => {
                    let first = text.split('\n').next().unwrap_or("").chars().count();
                    if first > remaining {
                        return false;
                    }
                    if text.contains('\n') {
                        return true;
                    }
                    remaining -= first;
                }
                Doc::Space => {
                    if remaining == 0 {
                        return false;
                    }
                    remaining -= 1;
                }
                Doc::Line => {
                    if mode == Mode::Break {
                        return true;
                    }
                    if remaining == 0 {
                        return false;
                    }
                    remaining -= 1;
                }
                Doc::Soft => {
                    if mode == Mode::Break {
                        return true;
                    }
                }
                Doc::Hard => return true,
                Doc::Concat(parts) => {
                    for part in parts.iter().rev() {
                        work.push((mode, part));
                    }
                }
                Doc::Indent(inner) => work.push((mode, inner)),
                Doc::Group(inner, forced) => {
                    work.push((if *forced { Mode::Break } else { mode }, inner));
                }
                Doc::IfBreak { broken, flat } => {
                    work.push((mode, if mode == Mode::Break { broken } else { flat }));
                }
                Doc::Hugging { hugged, .. } => work.push((mode, hugged)),
            }
        }
    }
}
