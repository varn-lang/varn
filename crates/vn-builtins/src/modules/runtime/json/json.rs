use vn_core::StdErrorClass;
use vn_macros::natives;
use vn_types::NativeError;


natives! {
    parse(ctx, text) => (ctx.parse_json(text)).map_err(|e| NativeError::std(StdErrorClass::Parse, e)),
    stringify(ctx, value) => (ctx.stringify_json(value)).map_err(NativeError::range_error),
}
