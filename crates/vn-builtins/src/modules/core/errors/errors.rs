use vn_core::RuntimeErrorKind;
use vn_macros::natives;
use vn_types::{NativeCtx, VmValue};

fn init_error(
    ctx: &mut dyn NativeCtx,
    this: VmValue,
    message: Option<&str>,
    cause: Option<VmValue>,
    class_name: &str,
) {
    let msg = match message {
        Some(m) => ctx.alloc_str(m),
        None => VmValue::null(),
    };
    ctx.set_field(this, vn_core::MemberKey::Message.as_str(), msg);
    let name = ctx.alloc_str(class_name);
    ctx.set_field(this, vn_core::MemberKey::Name.as_str(), name);
    let stack = ctx.alloc_str("");
    ctx.set_field(this, vn_core::MemberKey::Stack.as_str(), stack);
    ctx.set_field(
        this,
        vn_core::MemberKey::Cause.as_str(),
        cause.unwrap_or_else(VmValue::null),
    );
}







natives! {
    class Error {
        constructor(ctx, this, message, cause) => {
            init_error(ctx, this, message, cause, RuntimeErrorKind::Error.class_name());
            this
        },
        toString(ctx, this) => {
            let name = ctx
                .get_field(this, vn_core::MemberKey::Name.as_str())
                .and_then(|v| ctx.str_owned(v))
                .unwrap_or_else(|| RuntimeErrorKind::Error.class_name().to_owned());
            let message = ctx
                .get_field(this, vn_core::MemberKey::Message.as_str())
                .and_then(|v| ctx.str_owned(v))
                .unwrap_or_default();
            if message.is_empty() {
                name
            } else {
                format!("{name}: {message}")
            }
        },
    }
    class TypeError {
        constructor(ctx, this, message, cause) => {
            init_error(ctx, this, message, cause, RuntimeErrorKind::TypeError.class_name());
            this
        },
    }
    class RangeError {
        constructor(ctx, this, message, cause) => {
            init_error(ctx, this, message, cause, RuntimeErrorKind::RangeError.class_name());
            this
        },
    }
    class IntegerOverflow {
        constructor(ctx, this, message, cause) => {
            init_error(ctx, this, message, cause, RuntimeErrorKind::IntegerOverflow.class_name());
            this
        },
    }
    class DivisionByZero {
        constructor(ctx, this, message, cause) => {
            init_error(ctx, this, message, cause, RuntimeErrorKind::DivisionByZero.class_name());
            this
        },
    }
    class MatchError {
        constructor(ctx, this, message, cause) => {
            init_error(ctx, this, message, cause, RuntimeErrorKind::MatchError.class_name());
            this
        },
    }
}
