pub fn is_value_name(name: &str) -> bool {
    let body = name.trim_start_matches('_');
    if body.is_empty() {
        return true;
    }
    let body = body.strip_suffix('$').unwrap_or(body);
    let mut chars = body.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase()) && chars.all(|c| c.is_ascii_alphanumeric())
}

pub fn is_constant_name(name: &str) -> bool {
    let body = name.trim_start_matches('_');
    let body = body.strip_suffix('$').unwrap_or(body);
    let mut chars = body.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

pub fn is_type_name(name: &str) -> bool {
    let body = name.trim_start_matches('_');
    let body = body.strip_suffix('$').unwrap_or(body);
    let mut chars = body.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase()) && chars.all(|c| c.is_ascii_alphanumeric())
}
