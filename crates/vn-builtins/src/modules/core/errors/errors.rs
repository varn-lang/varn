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
    ctx.set_field(this, vn_core::MemberKey::Message.name(), msg);
    let name = ctx.alloc_str(class_name);
    ctx.set_field(this, vn_core::MemberKey::Name.name(), name);
    let stack = ctx.alloc_str("");
    ctx.set_field(this, vn_core::MemberKey::Stack.name(), stack);
    ctx.set_field(
        this,
        vn_core::MemberKey::Cause.name(),
        cause.unwrap_or_else(VmValue::null),
    );
}







natives! {
    class Error {
        constructor(ctx, this, message, cause) => {
            init_error(ctx, this, message, cause, RuntimeErrorKind::Error.name());
            this
        },
        toString(ctx, this) => {
            let name = ctx
                .get_field(this, vn_core::MemberKey::Name.name())
                .and_then(|v| ctx.str_owned(v))
                .unwrap_or_else(|| RuntimeErrorKind::Error.name().to_owned());
            let message = ctx
                .get_field(this, vn_core::MemberKey::Message.name())
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
            init_error(ctx, this, message, cause, RuntimeErrorKind::TypeError.name());
            this
        },
    }
    class RangeError {
        constructor(ctx, this, message, cause) => {
            init_error(ctx, this, message, cause, RuntimeErrorKind::RangeError.name());
            this
        },
    }
    class IntegerOverflow {
        constructor(ctx, this, message, cause) => {
            init_error(ctx, this, message, cause, RuntimeErrorKind::IntegerOverflow.name());
            this
        },
    }
    class DivisionByZero {
        constructor(ctx, this, message, cause) => {
            init_error(ctx, this, message, cause, RuntimeErrorKind::DivisionByZero.name());
            this
        },
    }
    class MatchError {
        constructor(ctx, this, message, cause) => {
            init_error(ctx, this, message, cause, RuntimeErrorKind::MatchError.name());
            this
        },
    }
}
