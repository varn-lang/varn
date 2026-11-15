use vn_core::StdErrorClass;
use vn_macros::natives;
use vn_types::NativeError;


natives! {
    parse(ctx, text, delimiter, has_header, trim) => {
        let delim_byte = delimiter.as_bytes().first().copied().unwrap_or(b',');
        (ctx.parse_csv(text, delim_byte, has_header, trim)).map_err(|e| NativeError::std(StdErrorClass::Parse, e))
    },
    stringify(ctx, data, delimiter) => {
        let delim_byte = delimiter.as_bytes().first().copied().unwrap_or(b',');
        (ctx.stringify_csv(data, delim_byte)).map_err(NativeError::range_error)
    },
}
