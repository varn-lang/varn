use vn_core::NumKind;

pub const EXTENSION: &str = "tpl";

const HEADER: &str = "// @kinds ";
const SIGNED: &str = " // @signed";
const UNSIGNED: &str = " // @unsigned";
const PLACEHOLDER: &str = "$T";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Integer,
    Float,
}

impl Family {
    fn of(name: &str) -> Option<Self> {
        match name {
            "integer" => Some(Self::Integer),
            "float" => Some(Self::Float),
            _ => None,
        }
    }

    fn holds(self, kind: NumKind) -> bool {
        match (self, kind) {
            (Self::Integer, NumKind::Int(_)) | (Self::Float, NumKind::Float(_)) => true,
            (Self::Integer, NumKind::Float(_)) | (Self::Float, NumKind::Int(_)) => false,
        }
    }
}

pub fn kinds(template: &str) -> Result<Vec<NumKind>, String> {
    let name = template
        .lines()
        .next()
        .and_then(|line| line.strip_prefix(HEADER))
        .map(str::trim)
        .ok_or_else(|| {
            format!(
                "a template starts with `{}integer` or `{}float`",
                HEADER, HEADER
            )
        })?;
    let family = Family::of(name).ok_or_else(|| format!("unknown template family `{name}`"))?;
    Ok(NumKind::ALL
        .into_iter()
        .filter(|kind| family.holds(*kind) && !kind.is_default_width())
        .collect())
}

pub fn instantiate(template: &str, kind: NumKind) -> String {
    let signed = match kind {
        NumKind::Int(width) => width.is_signed(),
        NumKind::Float(_) => true,
    };
    let mut out = String::with_capacity(template.len());
    for line in template.lines().skip(1) {
        let kept = if let Some(rest) = line.strip_suffix(SIGNED) {
            signed.then_some(rest)
        } else if let Some(rest) = line.strip_suffix(UNSIGNED) {
            (!signed).then_some(rest)
        } else {
            Some(line)
        };
        if let Some(line) = kept {
            out.push_str(&line.replace(PLACEHOLDER, kind.name()));
            out.push('\n');
        }
    }
    out
}
