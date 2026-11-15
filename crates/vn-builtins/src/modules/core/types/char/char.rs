use vn_macros::natives;
use vn_types::{VmValue};


natives! {
    class char {
        fromCode(code) => char::from_u32(code as u32).unwrap_or(char::REPLACEMENT_CHARACTER),
        toString(this) => this.to_string(),
        toStr(this) => this.to_string(),
        charCodeAt(this, pos) => {
            if let Some(p) = pos
                && p != 0 {
                    return -1;
                }
            this as i64
        },
        isAlphabetic(this) => this.is_alphabetic(),
        isAlphanumeric(this) => this.is_alphanumeric(),
        isDigit(this) => this.is_ascii_digit(),
        isWhitespace(this) => this.is_whitespace(),
        isUppercase(this) => this.is_uppercase(),
        isLowercase(this) => this.is_lowercase(),
        isAscii(this) => this.is_ascii(),
        isPunctuation(this) => this.is_ascii_punctuation(),
        toUppercase(this) => {
            let u = this.to_uppercase().next().unwrap_or(this);
            VmValue::from_char(u)
        },
        toLowercase(this) => {
            let l = this.to_lowercase().next().unwrap_or(this);
            VmValue::from_char(l)
        },
    }
}
